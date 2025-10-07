mod connection_data;
mod connection_traits;
mod connections;
mod handle_authentication;
mod handle_connection;
mod handle_received_messages;

pub use connection_traits::*;
pub use connections::WebsocketServer;

use tokio::{sync::Mutex, task::JoinHandle};

use std::{
    collections::{BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};

use crate::{
    data::{Area, Color, Coordinate},
    protocol::P2Encodable,
    ratelimit::RatelimitSettings,
    server::connection_data::ActiveConnectionData,
    users::UserId,
};

pub use handle_authentication::AuthenticationError;

const DELAY_BETWEEN_UPDATES: Duration = Duration::from_millis(10);

/// Shared state, can be shared using `.clone()`.
pub struct Server<W: P2Write + Unpin> {
    ratelimit: RatelimitSettings,
    /// NOTE: You may not wait for a lock on this Mutex while holding a lock to a Mutex
    /// which is (or was) contained in the HashMap, as this may result in a deadlock.
    /// Always lock this Mutex before you lock an inner Mutex, if you have to hold two locks at the same time.
    active_connections: Arc<Mutex<HashMap<UserId, Arc<Mutex<ActiveConnectionData<W>>>>>>,
    /// Recently modified pixels
    modified_pixels: Arc<Mutex<BTreeMap<Coordinate, Color>>>,
    /// used to batch updates together so that more groups can be built
    update_task: Arc<Mutex<Option<JoinHandle<()>>>>,
}

impl<W: P2Write + Unpin> Server<W> {
    pub fn new(ratelimit: RatelimitSettings) -> Self {
        Self {
            ratelimit,
            active_connections: Default::default(),
            modified_pixels: Arc::new(Mutex::new(BTreeMap::new())),
            update_task: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn put(&self, coord: Coordinate, color: Color) {
        self.modified_pixels.lock().await.insert(coord, color);
        let mut update_task = self.update_task.lock().await;
        let modified_pixels = Arc::clone(&self.modified_pixels);
        let active_connections = Arc::clone(&self.active_connections);
        if update_task.as_ref().is_none_or(|task| task.is_finished()) {
            *update_task = Some(tokio::task::spawn(async move {
                tokio::time::sleep(DELAY_BETWEEN_UPDATES).await;
                Self::transmit_modified_pixels(&*modified_pixels, &*active_connections).await;
            }));
        }
    }

    async fn transmit_modified_pixels(
        modified_pixels: &Mutex<BTreeMap<Coordinate, Color>>,
        active_connections: &Mutex<HashMap<UserId, Arc<Mutex<ActiveConnectionData<W>>>>>,
    ) {
        let mut modified_pixels = modified_pixels.lock().await;
        if modified_pixels.is_empty() {
            return;
        }

        // find connected groups of pixels, but not necessarily rectangles
        let mut groups = Vec::<BTreeMap<Coordinate, Color>>::new();
        for (coord, color) in modified_pixels.iter() {
            let left = coord.x.checked_sub(1).map(|x| Coordinate { x, y: coord.y });
            let right = coord.x.checked_add(1).map(|x| Coordinate { x, y: coord.y });
            let above = coord.y.checked_sub(1).map(|y| Coordinate { x: coord.x, y });
            let below = coord.y.checked_add(1).map(|y| Coordinate { x: coord.x, y });
            let mut main_group: Option<usize> = None;
            for group_index in 0..groups.len() {
                for neighbor in [left, right, above, below].into_iter().flatten() {
                    let group = &mut groups[group_index];
                    if group.contains_key(&neighbor) {
                        if let Some(main_group) = main_group {
                            let group = group.clone();
                            groups[main_group].extend(group);
                        } else {
                            group.insert(*coord, *color);
                            main_group = Some(group_index);
                        }
                    }
                }
            }
            if main_group.is_none() {
                let mut new_group = BTreeMap::new();
                new_group.insert(*coord, *color);
                groups.push(new_group);
            }
        }
        modified_pixels.clear();
        drop(modified_pixels);

        // extract rectangles from the groups
        // TODO: improve this
        let mut row_groups = Vec::new();
        for group in groups {
            let rows = group.into_iter().fold(
                BTreeMap::<i16, BTreeMap<i16, Color>>::new(),
                |mut rows, (coord, color)| {
                    rows.entry(coord.y).or_default().insert(coord.x, color);
                    rows
                },
            );
            for (y, row) in rows {
                let mut current_group = Vec::<(i16, Color)>::new();
                for (x, color) in row {
                    if current_group.len() < 15
                        && current_group
                            .last()
                            .is_none_or(|last| last.0.saturating_add(1) == x)
                    {
                        current_group.push((x, color));
                    } else if !current_group.is_empty() {
                        row_groups
                            .push((y, std::mem::replace(&mut current_group, vec![(x, color)])));
                    }
                }
                if !current_group.is_empty() {
                    row_groups.push((y, current_group));
                }
            }
        }

        // send messages
        let mut messages = Vec::with_capacity(row_groups.len());
        for (y, row) in row_groups {
            let coord = Coordinate { x: row[0].0, y };
            let h = 1;
            let w = row.len() as u8;
            let mut message = vec![0xFF, (((h - 1) & 0xF) << 4) | (w & 0xF)];
            let x2 = row.last().unwrap().0;
            coord.write_p2encoded(&mut message).await.unwrap();
            for (_, color) in row {
                color.write_p2encoded(&mut message).await.unwrap();
            }
            messages.push((
                Area {
                    top_left: coord,
                    bottom_right: Coordinate { x: x2, y },
                },
                message,
            ));
        }
        let active_connections = active_connections.lock().await;
        for (_, connection) in active_connections.iter() {
            let mut connection = connection.lock().await;
            if !connection.replaced {
                let mut sent_any = false;
                for (area, message) in messages.iter() {
                    if connection
                        .subscribed_area
                        .is_some_and(|subscribed_area| area.intersects(subscribed_area))
                    {
                        sent_any = true;
                        if connection.write.write_all(&message).await.is_err() {
                            connection.replaced = true;
                        }
                    }
                }
                if sent_any {
                    if connection.write.flush().await.is_err() {
                        connection.replaced = true;
                    }
                }
            }
        }
        drop(active_connections);
    }
}

impl<W: P2Write + Unpin> Clone for Server<W> {
    fn clone(&self) -> Self {
        Self {
            ratelimit: self.ratelimit,
            active_connections: Arc::clone(&self.active_connections),
            modified_pixels: Arc::clone(&self.modified_pixels),
            update_task: Arc::clone(&self.update_task),
        }
    }
}

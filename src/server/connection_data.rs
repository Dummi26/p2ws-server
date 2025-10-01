use tokio::time::Instant;

use crate::{data::Area, server::P2Write};

pub struct ActiveConnectionData<W: P2Write + Unpin> {
    pub replaced: bool,
    pub subscribed_area: Option<Area>,
    pub write: W,
    pub last_action: Instant,
}
impl<W: P2Write + Unpin> ActiveConnectionData<W> {
    pub fn new(write: W) -> Self {
        Self {
            replaced: false,
            subscribed_area: None,
            write,
            last_action: Instant::now(),
        }
    }

    pub fn has_acted(&mut self) {
        self.last_action = Instant::now();
    }
}

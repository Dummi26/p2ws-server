#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Coordinate {
    pub x: i16,
    pub y: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Area {
    pub top_left: Coordinate,
    pub bottom_right: Coordinate,
}

impl Area {
    pub fn try_new(top_left: Coordinate, bottom_right: Coordinate) -> Option<Self> {
        if top_left.x > bottom_right.x || top_left.y > bottom_right.y {
            None
        } else {
            Some(Self {
                top_left,
                bottom_right,
            })
        }
    }

    pub fn contains(&self, coord: Coordinate) -> bool {
        self.top_left.x <= coord.x
            && coord.x <= self.bottom_right.x
            && self.top_left.y <= coord.y
            && coord.y <= self.bottom_right.y
    }
}

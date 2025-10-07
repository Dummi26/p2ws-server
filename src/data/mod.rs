#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
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

    pub fn left(&self) -> i16 {
        self.top_left.x
    }
    pub fn right(&self) -> i16 {
        self.bottom_right.x
    }
    pub fn top(&self) -> i16 {
        self.top_left.y
    }
    pub fn bottom(&self) -> i16 {
        self.bottom_right.y
    }

    pub fn contains(&self, coord: Coordinate) -> bool {
        self.left() <= coord.x
            && coord.x <= self.right()
            && self.top() <= coord.y
            && coord.y <= self.bottom()
    }

    pub fn intersects(&self, other: Area) -> bool {
        !(other.right() < self.left()
            || self.right() < other.left()
            || other.bottom() < self.top()
            || self.bottom() < other.top())
    }
}

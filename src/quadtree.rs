use super::*;

pub struct QuadTree {
    root: Cell,
}

impl QuadTree {
    pub fn build(data: Vec<TraversedVolume>) -> Self {

    }
}

struct AABB {
    min: Vec2,
    max: Vec2,
}

pub struct Cell {
    area: AABB,
    children: [Option<Box<Cell>>; 4],
}

impl Cell {
    pub fn insert(object: TraversedVolume) {
         
    }
}
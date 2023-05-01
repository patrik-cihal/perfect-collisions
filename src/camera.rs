use std::ops::Deref;

use super::*;

#[derive(Default)]
pub struct Camera(pub GTransform);

impl Deref for Camera {
    type Target = GTransform;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Camera {
    pub fn screen_to_world(&self, screen_pos: Vec2) -> Vec2 {
        self.inv_transform(screen_pos)
    }
}

use super::*;

#[derive(Clone, Debug)]
pub struct Object {
    pub mass: f32,
    pub position: Vec2,
    pub velocity: Vec2,
    pub acceleration: Vec2,
    pub rotation: f32,
    pub rot_velocity: f32,
    pub shape: Shape<Txts>,
}

impl Object {
    pub fn new(position: Vec2, velocity: Vec2, rotation: f32, shape: Shape<Txts>) -> Self {
        Self {
            mass: 1.,
            position,
            velocity,
            acceleration: Vec2::ZERO,
            rotation,
            rot_velocity: 0.,
            shape,
        }
    }
    pub fn update(&mut self, dt: f32) {
        self.position += self.velocity * dt;
        self.velocity += self.acceleration * dt;
        self.acceleration = Vec2::ZERO;
    }
}

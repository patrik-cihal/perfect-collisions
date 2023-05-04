#![feature(async_fn_in_trait)]

mod camera;
mod object;

use std::{cmp::Reverse, collections::BinaryHeap, f32::consts::PI, ops::Deref};

use camera::Camera;
use ellipsoid::prelude::{winit::event::MouseButton, winit::window::Window, *};
use object::Object;

mod geometry;
use geometry::*;

#[derive(PartialEq, PartialOrd)]
struct F32Ord(f32);

impl Deref for F32Ord {
    type Target = f32;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Eq for F32Ord {}

impl Ord for F32Ord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Default, strum::Display, strum::EnumIter, Debug)]
#[strum(serialize_all = "snake_case")]
pub enum AppTextures {
    #[default]
    White,
    Blue,
}

impl Textures for AppTextures {}

impl Into<u32> for AppTextures {
    fn into(self) -> u32 {
        self as u32
    }
}

type Txts = AppTextures;

struct CollisionSimulator {
    objects: Vec<Object>,
    camera: Camera,
    graphics: Graphics<Txts>,
    middle_clicked: bool,
    cursor_position: Vec2,
    last_cursor_position: Vec2,
    time_elapsed: f32,
    frame: usize,
    debug_points: Vec<Vec2>
}

impl App<Txts> for CollisionSimulator {
    async fn new(window: Window) -> Self {
        let graphics = Graphics::new(window).await;
        Self {
            objects: vec![],
            graphics,
            middle_clicked: false,
            cursor_position: Vec2::ZERO,
            last_cursor_position: Vec2::ZERO,
            camera: Camera::default(),
            time_elapsed: 0.,
            frame: 0,
            debug_points: vec![]
        }
    }

    fn update(&mut self, dt: f32) {
        self.time_elapsed += dt;

        self.update_camera();
        self.update_objects(dt);

        //----------- late update ------------//
        self.last_cursor_position = self.cursor_position;
    }
    fn draw(&mut self) {
        self.draw_ui();
        self.draw_objects();
        self.draw_debug();
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        // detect mouse down, and set clicked to the mouse position
        // detect mouse up, and set clicked to None
        // detect mouse move, and set cursor_position to the mouse position
        match event {
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Middle,
                ..
            } => match state {
                winit::event::ElementState::Pressed => {
                    self.middle_clicked = true;
                }
                winit::event::ElementState::Released => {
                    self.middle_clicked = false;
                }
            },
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Right,
                ..
            } => match state {
                winit::event::ElementState::Pressed => {
                    let object_velocity =
                        (self.cursor_position - self.last_cursor_position) / self.camera.scale;
                    let object = Object::new(
                        self.camera.screen_to_world(self.cursor_position),
                        object_velocity,
                        rand::random::<f32>() % (PI * 2.),
                        Shape::from_polygon(rand::random::<usize>() % 5 + 3),
                    );
                    self.objects.push(object);
                }
                _ => (),
            },
            WindowEvent::CursorMoved { position, .. } => {
                let x = position.x as f32 / self.graphics.window().inner_size().width as f32;
                let y = position.y as f32 / self.graphics.window().inner_size().height as f32;
                self.cursor_position = vec2(x, -y) * 2. - vec2(1., -1.);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let y_offset = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => *y,
                    winit::event::MouseScrollDelta::PixelDelta(offset) => offset.y as f32,
                };

                self.camera.0 = self.camera.inflate(1.1f32.powf(y_offset));
            }
            _ => (),
        }
        false
    }

    fn graphics(&self) -> &Graphics<Txts> {
        &self.graphics
    }
    fn graphics_mut(&mut self) -> &mut Graphics<Txts> {
        &mut self.graphics
    }
}

struct TraversedVolume {
    points: Vec<Vec2>,
}

impl TraversedVolume {
    pub fn from_object(object: Object, dt: f32) -> Self {
        let mut future_object = object.clone();
        future_object.update(dt);

        let points = convex_hull(
            object
                .shape
                .points
                .into_iter()
                .map(|(p, _)| p.rotate_rad(object.rotation) + object.position)
                .chain(
                    future_object
                        .shape
                        .points
                        .into_iter()
                        .map(|(p, _)| p.rotate_rad(object.rotation) + future_object.position),
                )
                .collect::<Vec<_>>(),
        );
        Self { points }
    }
}

impl CollisionSimulator {
    pub fn update_objects(&mut self, dt: f32) {
        let dt = 0.5;
        if (self.time_elapsed / dt) as usize <= self.frame {
            return;
        }
        self.frame += 1;
        let mut collisions_pq = BinaryHeap::new();
        for i in 0..self.objects.len() {
            for j in 0..self.objects.len() {
                if i == j {
                    continue; 
                }
                if let Some(time) = self.check_collision(i, j, dt) {
                    collisions_pq.push((Reverse(F32Ord(time)), (i, j)));
                }
            }
        }

        while let Some((Reverse(t), (i, j))) = collisions_pq.pop() {
            println!("{} {} {}", *t, i, j);
        }

        for object in &mut self.objects {
            object.update(dt);
        }
    }
    pub fn update_camera(&mut self) {
        if self.middle_clicked {
            let delta = (self.cursor_position - self.last_cursor_position) / self.camera.scale;
            self.camera.0 = self.camera.translate(delta);
        }
    }
    pub fn draw_ui(&mut self) {
        egui::Window::new("Simulation Info").show(&self.graphics.egui_platform.context(), |ui| {
            ui.label(format!("Time: {}", self.time_elapsed));
            ui.label(format!("Energy: {}", self.total_energy()));
        });
    }
    pub fn draw_objects(&mut self) {
        for object in &self.objects {
            let traversed_volume = TraversedVolume::from_object(object.clone(), 0.5);
            self.graphics.add_geometry(
                Shape::new(
                    traversed_volume
                        .points
                )
                .set_texture(Txts::Blue)
                .apply(self.camera.0)
                .into(),
            );

            let object_gtransform =
                GTransform::from_translation(object.position).rotate(object.rotation);
            self.graphics.add_geometry(
                object
                    .shape
                    .clone()
                    .apply(object_gtransform)
                    .apply(self.camera.0)
                    .into(),
            );
        }
    }
    pub fn draw_debug(&mut self) {
        for point in &self.debug_points {
            let circle = Shape::from_circle(20).set_texture(Txts::Blue).apply(GTransform::from_translation(*point).inflate(0.05)).apply(self.camera.0);
            self.graphics.add_geometry(circle.into());
        }
    }
    pub fn total_energy(&mut self) -> f32 {
        let mut total_energy = 0.;
        for object in &self.objects {
            total_energy += 0.5 * object.mass * object.velocity.length_squared();
        }
        total_energy
    }
    pub fn check_collision(&mut self, i: usize, j: usize, dt: f32) -> Option<f32> {
        let mut static_object = self.objects[i].clone();
        let mut moving_object = self.objects[j].clone();

        moving_object.velocity -= static_object.velocity;
        moving_object.acceleration -= static_object.acceleration;
        static_object.velocity = Vec2::ZERO;
        static_object.acceleration = Vec2::ZERO;

        let traversed_vol = TraversedVolume::from_object(moving_object.clone(), dt);

        let static_object_points = static_object
            .shape
            .points
            .iter()
            .map(|(p, _)| p.rotate_rad(static_object.rotation) + static_object.position)
            .collect::<Vec<_>>();

        let moving_object_points = moving_object
            .shape
            .points
            .iter()
            .map(|(p, _)| p.rotate_rad(moving_object.rotation) + moving_object.position)
            .collect::<Vec<_>>();


        let mut answer = None;

        let check = |p: Vec2, v: Vec2, a: Vec2, b: Vec2, debug_points: &mut Vec<Vec2>| -> Option<f32> {
            let slope_1 = (b.y - a.y) / (b.x - a.x);
            let y_1 = a.y - a.x * slope_1;
            let slope_2 = (v.y - p.y) / (v.x - p.x);
            let y_2 = v.y - v.x * slope_2;

            let intercept = (y_2 - y_1) / (slope_1 - slope_2);

            if intercept >= a.x.min(b.x) && intercept <= a.x.max(b.x) {
                let time = (intercept - p.x) / (v.x);
                if time > 0. && time < dt {
                    debug_points.push(vec2(intercept, intercept*slope_1+y_1));
                    Some(time)
                } else {
                    None
                }
            } else {
                None
            }
        };

        for p in &moving_object_points {
            for i in 0..static_object_points.len() {
                let a = static_object_points[i];
                let b = static_object_points[(i + 1) % static_object_points.len()];

                if let Some(time) = check(*p, moving_object.velocity, a, b, &mut self.debug_points) {
                    answer = Some(answer.unwrap_or(f32::MAX).min(time));
                }
            }
        }

        answer
    }
}

#[tokio::main]
async fn main() {
    ellipsoid::run::<Txts, CollisionSimulator>().await;
}
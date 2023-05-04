#![feature(async_fn_in_trait)]

mod camera;
mod object;

use std::{cmp::Reverse, collections::BinaryHeap, f32::consts::PI, ops::Deref};

use camera::Camera;
use ellipsoid::prelude::{winit::event::MouseButton, winit::window::Window, *};
use object::Object;

mod geometry;
use geometry::*;


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
                        Shape::from_polygon(rand::random::<usize>() % 1 + 3),
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

#[derive(PartialEq, PartialOrd, Debug, Clone, Copy)]
struct CollisionInfo {
    time: f32,
    object_1: usize,
    point_1: usize,
    object_2: usize,
    line_2: usize
}

impl Eq for CollisionInfo {}

impl Ord for CollisionInfo {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
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
                if let Some(col_info) = self.check_collision(i, j, dt) {
                    collisions_pq.push(Reverse(col_info));
                }
            }
        }

        while let Some(Reverse(col_info)) = collisions_pq.pop() {
            let object = &self.objects[col_info.object_1];
            let col_position = object.shape.points[col_info.point_1].0.rotate_rad(object.rotation)+object.position+object.velocity*col_info.time;
            self.debug_points.push(col_position);
            println!("{:?}", col_info);
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
    /// Checks whether obj 1 collides with obj 2 with one of its corners
    pub fn check_collision(&mut self, obj_1_id: usize, obj_2_id: usize, dt: f32) -> Option<CollisionInfo> {
        let mut obj_1 = self.objects[obj_1_id].clone();
        let mut obj_2 = self.objects[obj_2_id].clone();

        obj_1.velocity -= obj_2.velocity;
        obj_2.velocity = Vec2::ZERO;

        let obj_1_points = obj_1
            .shape
            .points
            .iter()
            .map(|(p, _)| p.rotate_rad(obj_1.rotation) + obj_1.position)
            .collect::<Vec<_>>();

        let obj_2_points = obj_2
            .shape
            .points
            .iter()
            .map(|(p, _)| p.rotate_rad(obj_2.rotation) + obj_2.position)
            .collect::<Vec<_>>();


        let mut collision: Option<CollisionInfo> = None;

        let check = |p: Vec2, v: Vec2, a: Vec2, b: Vec2| -> Option<f32> {
            let slope_1 = (b.y - a.y) / (b.x - a.x);
            let y_1 = a.y - a.x * slope_1;
            let slope_2 = (v.y) / (v.x);
            let y_2 = p.y-p.x * slope_2;

            let intercept = (y_2 - y_1) / (slope_1 - slope_2);

            if intercept >= a.x.min(b.x) && intercept <= a.x.max(b.x) {
                let time = (intercept - p.x) / (v.x);
                if time > 0. && time < dt {
                    Some(time)
                } else {
                    None
                }
            } else {
                None
            }
        };

        for (i, p) in obj_1_points.into_iter().enumerate() {
            for j in 0..obj_2_points.len() {
                let a = obj_2_points[j];
                let b = obj_2_points[(j + 1) % obj_2_points.len()];

                if let Some(time) = check(p, obj_1.velocity, a, b) {
                    let candidate = CollisionInfo {
                        time,
                        object_1: obj_1_id,
                        point_1: i,
                        object_2: obj_2_id,
                        line_2: j
                    };
                    if let Some(cur_answer) = &mut collision {
                        *cur_answer = (*cur_answer).min(candidate);
                    }
                    else {
                        collision = Some(candidate);
                    }
                }
            }
        }

        collision
    }
}

#[tokio::main]
async fn main() {
    ellipsoid::run::<Txts, CollisionSimulator>().await;
}
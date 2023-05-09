#![feature(async_fn_in_trait)]

mod camera;
mod object;

use std::{cmp::Reverse, collections::BinaryHeap, f32::consts::PI, time::Instant};

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
    last_cursor_pressed_position: Vec2,
    time_elapsed: f32,
    frame_rate: usize,
    frame: usize,
    debug_points: Vec<Vec2>,
    spawning_object: Option<Object>
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
            last_cursor_pressed_position: Vec2::ZERO,
            camera: Camera::default(),
            time_elapsed: 0.,
            frame: 0,
            debug_points: vec![],
            spawning_object: None,
            frame_rate: 0
        }
    }

    fn update(&mut self, dt: f32) {
        self.update_camera();

        self.last_cursor_position = self.cursor_position;

        self.frame_rate = (1./dt) as usize;
        self.time_elapsed += dt;
        self.frame += 1;
        self.update_objects(dt);
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
                    self.spawning_object = Some(Object::new(
                        self.camera.screen_to_world(self.cursor_position),
                        Vec2::ZERO,
                        rand::random::<f32>() % (PI * 2.),
                        Shape::from_polygon(rand::random::<usize>() % 5 + 3),
                    ));
                    self.last_cursor_pressed_position = self.cursor_position;
                }
                winit::event::ElementState::Released => {
                    let Some(spawning_object) = &mut self.spawning_object else {
                        return false;
                    };

                    let object_velocity =
                        -(self.cursor_position - self.last_cursor_pressed_position) / self.camera.scale;

                    spawning_object.cur_time = self.time_elapsed;
                    spawning_object.velocity = object_velocity;

                    self.objects.push(spawning_object.clone());

                    self.spawning_object = None;
                }
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
    pub fn from_object(object: Object, target_time: f32) -> Self {
        let mut future_object = object.clone();
        future_object.update(target_time);

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
    object_1_col_stamp: usize,
    point_1: usize,
    object_2: usize,
    object_2_col_stamp: usize,
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
        let mut collisions_pq = BinaryHeap::new();
        for i in 0..self.objects.len() {
            for j in 0..self.objects.len() {
                if i == j {
                    continue; 
                }
                if let Some(col_info) = self.check_collision(i, j) {
                    collisions_pq.push(Reverse(col_info));
                }
            }
        }

        let time_measure = Instant::now();
        while let Some(Reverse(col_info)) = collisions_pq.pop() {
            let sharp_obj = &self.objects[col_info.object_1];
            let other_obj = &self.objects[col_info.object_2];

            if col_info.object_1_col_stamp != sharp_obj.updated || col_info.object_2_col_stamp != other_obj.updated {
                continue;
            }

            let col_position = sharp_obj.shape.points[col_info.point_1].0.rotate_rad(sharp_obj.rotation)+sharp_obj.position+sharp_obj.velocity*(col_info.time-sharp_obj.cur_time);
            self.debug_points.push(col_position);

            let col_line_a = other_obj.shape.points[col_info.line_2].0.rotate_rad(other_obj.rotation);
            let col_line_b = other_obj.shape.points[(col_info.line_2+1)%other_obj.shape.points.len()].0.rotate_rad(other_obj.rotation);

            let normal = (col_line_a-col_line_b).perp().normalize();

            let rel_velocity = sharp_obj.velocity-other_obj.velocity;

            let impulse_numerator = -2. * rel_velocity.dot(normal);
            let impulse_denominator = (1./sharp_obj.mass) + (1./other_obj.mass);
            let impulse = impulse_numerator / impulse_denominator;

            self.objects[col_info.object_1].update(col_info.time);
            self.objects[col_info.object_2].update(col_info.time);

            let mass1 = self.objects[col_info.object_1].mass;
            let mass2 = self.objects[col_info.object_2].mass;
            self.objects[col_info.object_1].velocity += impulse * normal / mass1;
            self.objects[col_info.object_2].velocity -= impulse * normal / mass2;

            self.objects[col_info.object_1].position += normal * 0.005;
            self.objects[col_info.object_2].position -= normal * 0.005;

            for col in self.check_collisions(col_info.object_1).into_iter().chain(self.check_collisions(col_info.object_2)) {
                collisions_pq.push(Reverse(col));
            }
        }



        for object in &mut self.objects {
            object.update(self.time_elapsed+0.001);
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
            ui.label(format!("Frame rate: {}", self.frame_rate));
            ui.label(format!("Objects count: {}", self.objects.len()));
        });
    }
    pub fn draw_objects(&mut self) {
        for object in &self.objects {
            let traversed_volume = TraversedVolume::from_object(object.clone(), self.time_elapsed+0.001);
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
        if let Some(spawning_object) = &self.spawning_object {
            let object_gtransform =
                GTransform::from_translation(spawning_object.position).rotate(spawning_object.rotation);
            self.graphics.add_geometry(
                spawning_object
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

    fn check_collisions(&self, obj_id: usize) -> Vec<CollisionInfo> {
        let mut result = vec![];

        for i in 0..self.objects.len() {
            if obj_id == i {
                continue;
            }
            if let Some(collision) = self.check_collision(obj_id, i) {
                result.push(collision);
            }
            if let Some(collision) = self.check_collision(i, obj_id) {
                result.push(collision);
            }
        }

        result
    }

    /// Checks whether obj 1 collides with obj 2 with one of its corners
    fn check_collision(&self, sharp_obj_id: usize, other_obj_id: usize) -> Option<CollisionInfo> {
        let mut sharp_obj = self.objects[sharp_obj_id].clone();
        let mut other_obj = self.objects[other_obj_id].clone();

        let cur_time = sharp_obj.cur_time.max(other_obj.cur_time);

        sharp_obj.position += sharp_obj.velocity * (cur_time-sharp_obj.cur_time);
        other_obj.position += other_obj.velocity * (cur_time-other_obj.cur_time);

        sharp_obj.velocity -= other_obj.velocity;
        other_obj.velocity = Vec2::ZERO;

        let sharp_obj_points = sharp_obj
            .shape
            .points
            .iter()
            .map(|(p, _)| p.rotate_rad(sharp_obj.rotation) + sharp_obj.position)
            .collect::<Vec<_>>();

        let other_obj_points = other_obj
            .shape
            .points
            .iter()
            .map(|(p, _)| p.rotate_rad(other_obj.rotation) + other_obj.position)
            .collect::<Vec<_>>();


        let mut collision: Option<CollisionInfo> = None;

        let check = |p: Vec2, v: Vec2, a: Vec2, b: Vec2| -> Option<f32> {
            let slope_1 = (b.y - a.y) / (b.x - a.x);
            let y_1 = a.y - a.x * slope_1;
            let slope_2 = (v.y) / (v.x);
            let y_2 = p.y-p.x * slope_2;

            let intercept = (y_2 - y_1) / (slope_1 - slope_2);

            if intercept >= a.x.min(b.x) && intercept <= a.x.max(b.x) {
                let time = (intercept - p.x) / (v.x) + cur_time;
                if time > cur_time && time < self.time_elapsed {
                    Some(time)
                } else {
                    None
                }
            } else {
                None
            }
        };

        for (i, p) in sharp_obj_points.into_iter().enumerate() {
            for j in 0..other_obj_points.len() {
                let a = other_obj_points[j];
                let b = other_obj_points[(j + 1) % other_obj_points.len()];

                if let Some(time) = check(p, sharp_obj.velocity, a, b) {
                    let candidate = CollisionInfo {
                        time,
                        object_1: sharp_obj_id,
                        object_1_col_stamp: self.objects[sharp_obj_id].updated,
                        point_1: i,
                        object_2: other_obj_id,
                        object_2_col_stamp: self.objects[other_obj_id].updated,
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
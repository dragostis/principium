use std::{f32, time::Duration};

use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

const FOV_Y: f32 = f32::consts::FRAC_PI_4;
const NEAR: f32 = 1.0;
const FAR: f32 = 1000.0;
const RADIANS_PER_DOT: f32 = 1.0 / 180.0;

#[derive(Debug)]
pub struct Camera {
    pub eye: glam::Vec3,
    dir: glam::Vec3,
    vel: glam::Vec3,
    cursor_pos_delta: glam::Vec2,
}

impl Default for Camera {
    fn default() -> Self {
        let eye = glam::Vec3::new(0.0, 150.0, 0.0);

        Self {
            eye,
            dir: glam::Vec3::ZERO,
            vel: glam::Vec3::ZERO,
            cursor_pos_delta: glam::Vec2::new(
                (f32::consts::FRAC_PI_2 + f32::consts::FRAC_PI_4) * RADIANS_PER_DOT.recip(),
                0.0,
            ),
        }
    }
}

impl Camera {
    pub fn handle_key_event(&mut self, event: KeyEvent) {
        let dir_from_state =
            |state: ElementState| (state == ElementState::Pressed) as u8 as f32 * 2.0 - 1.0;

        match event {
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyW),
                state,
                repeat: false,
                ..
            } => self.vel.x += dir_from_state(state),
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyA),
                state,
                repeat: false,
                ..
            } => self.vel.z += dir_from_state(state),
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyS),
                state,
                repeat: false,
                ..
            } => self.vel.x -= dir_from_state(state),
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyD),
                state,
                repeat: false,
                ..
            } => self.vel.z -= dir_from_state(state),
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyQ),
                state,
                repeat: false,
                ..
            } => self.vel.y += dir_from_state(state),
            KeyEvent {
                physical_key: PhysicalKey::Code(KeyCode::KeyE),
                state,
                repeat: false,
                ..
            } => self.vel.y -= dir_from_state(state),
            _ => (),
        }
    }

    pub fn handle_mouse_motion(&mut self, delta: (f64, f64)) {
        self.cursor_pos_delta += glam::Vec2::new(delta.0 as f32, delta.1 as f32);
    }

    pub fn update(&mut self, dt: Duration) {
        let forward = self.dir;
        let left = glam::Vec3::Y.cross(self.dir);

        let vel = forward * self.vel.x + glam::Vec3::Y * self.vel.y + left * self.vel.z;

        self.eye += vel * 10.0 * dt.as_secs_f32();

        let pitch = (-self.cursor_pos_delta.y * RADIANS_PER_DOT)
            .clamp(-f32::consts::PI / 2.0, f32::consts::PI / 2.0);
        let yaw = -self.cursor_pos_delta.x * RADIANS_PER_DOT;

        self.dir = glam::Quat::from_euler(glam::EulerRot::ZYX, 0.0, yaw, pitch)
            .mul_vec3(glam::Vec3::NEG_Z);
    }

    pub fn clip_from_world(&self, aspect_ratio: f32) -> glam::Mat4 {
        let view = glam::Mat4::look_to_rh(self.eye, self.dir, glam::Vec3::Y);
        let proj = glam::Mat4::perspective_rh(FOV_Y, aspect_ratio, NEAR, FAR);
        let flip_z = glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, 1.0))
            * glam::Mat4::from_scale(glam::Vec3::new(1.0, 1.0, -1.0));

        flip_z * proj * view
    }

    pub fn clip_from_world_with_margin(&self, aspect_ratio: f32, margin: f32) -> glam::Mat4 {
        let dist = margin / (FOV_Y / 2.0).sin();

        let eye = self.eye - self.dir * dist;

        let view = glam::Mat4::look_to_rh(eye, self.dir, glam::Vec3::Y);
        let proj = glam::Mat4::perspective_rh_gl(
            FOV_Y,
            aspect_ratio,
            NEAR + dist - margin,
            FAR + dist + margin,
        );

        proj * view
    }
}

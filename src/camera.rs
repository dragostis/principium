use std::{f32, time::Duration};

use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Debug)]
pub struct Camera {
    eye: glam::Vec3,
    dir: glam::Vec3,
    vel: glam::Vec3,
    cursor_pos_delta: glam::Vec2,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            eye: glam::Vec3::new(0.0, 1.0, 2.0),
            dir: glam::Vec3::NEG_Z,
            vel: glam::Vec3::ZERO,
            cursor_pos_delta: glam::Vec2::ZERO,
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

        const RADIANS_PER_DOT: f32 = 1.0 / 180.0;

        let pitch = (-self.cursor_pos_delta.y * RADIANS_PER_DOT)
            .clamp(-f32::consts::PI / 2.0, f32::consts::PI / 2.0);
        let yaw = -self.cursor_pos_delta.x * RADIANS_PER_DOT;

        self.cursor_pos_delta = glam::Vec2::ZERO;

        self.dir = glam::Quat::from_euler(glam::EulerRot::ZYX, 0.0, yaw, pitch).mul_vec3(self.dir);
    }

    pub fn clip_from_world(&self, config: &wgpu::SurfaceConfiguration) -> glam::Mat4 {
        let view = glam::Mat4::look_to_rh(self.eye, self.dir, glam::Vec3::Y);
        let proj = glam::Mat4::perspective_rh(
            f32::consts::FRAC_PI_4,
            config.width as f32 / config.height as f32,
            1.0,
            100.0,
        );

        proj * view
    }
}

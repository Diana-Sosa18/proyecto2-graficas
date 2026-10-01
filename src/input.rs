use crate::camera::Camera;
use minifb::{Key, KeyRepeat, Window};

const ORBIT_SPEED: f32 = 1.4; // radianes por segundo
const ZOOM_SPEED: f32 = 1.8; // factor exponencial por segundo

#[derive(Default)]
pub struct Actions {
    pub screenshot: bool,
    pub reset_camera: bool,
}

/// Lee teclado y rueda del mouse y los traduce en movimientos de camara.
pub fn handle_input(window: &Window, camera: &mut Camera, dt: f32) -> Actions {
    let down = |k: Key| window.is_key_down(k);

    let mut d_yaw = 0.0;
    let mut d_pitch = 0.0;
    if down(Key::A) || down(Key::Left) {
        d_yaw -= ORBIT_SPEED * dt;
    }
    if down(Key::D) || down(Key::Right) {
        d_yaw += ORBIT_SPEED * dt;
    }
    if down(Key::W) || down(Key::Up) {
        d_pitch += ORBIT_SPEED * 0.7 * dt;
    }
    if down(Key::S) || down(Key::Down) {
        d_pitch -= ORBIT_SPEED * 0.7 * dt;
    }
    if d_yaw != 0.0 || d_pitch != 0.0 {
        camera.orbit(d_yaw, d_pitch);
    }

    if down(Key::Equal) || down(Key::NumPadPlus) || down(Key::E) {
        camera.zoom((-ZOOM_SPEED * dt).exp());
    }
    if down(Key::Minus) || down(Key::NumPadMinus) || down(Key::Q) {
        camera.zoom((ZOOM_SPEED * dt).exp());
    }
    if let Some((_, scroll)) = window.get_scroll_wheel() {
        camera.zoom((-scroll * 0.08).exp());
    }

    Actions {
        screenshot: window.is_key_pressed(Key::P, KeyRepeat::No),
        reset_camera: window.is_key_pressed(Key::R, KeyRepeat::No),
    }
}

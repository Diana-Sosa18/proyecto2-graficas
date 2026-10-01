use crate::ray::Ray;
use nalgebra_glm::{vec3, Vec3};

const MIN_PITCH: f32 = 0.08;
const MAX_PITCH: f32 = 1.45;

/// Camara orbital: gira alrededor de `target` sobre una esfera de radio `distance`.
/// Los valores `goal_*` son el destino; los actuales se interpolan hacia ellos
/// para que la rotacion y el zoom se sientan suaves.
pub struct Camera {
    pub target: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    goal_yaw: f32,
    goal_pitch: f32,
    goal_distance: f32,
    pub fov_deg: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

/// Base ortonormal de la camara precalculada para un cuadro.
pub struct CameraFrame {
    eye: Vec3,
    forward: Vec3,
    right: Vec3,
    up: Vec3,
    tan_half_fov: f32,
}

impl Camera {
    pub fn new(target: Vec3, yaw: f32, pitch: f32, distance: f32) -> Self {
        Camera {
            target,
            yaw,
            pitch,
            distance,
            goal_yaw: yaw,
            goal_pitch: pitch,
            goal_distance: distance,
            fov_deg: 50.0,
            min_distance: 40.0,
            max_distance: 420.0,
        }
    }

    pub fn eye(&self) -> Vec3 {
        let (sy, cy) = self.yaw.sin_cos();
        let (sp, cp) = self.pitch.sin_cos();
        self.target + vec3(cp * sy, sp, cp * cy) * self.distance
    }

    pub fn orbit(&mut self, d_yaw: f32, d_pitch: f32) {
        self.goal_yaw += d_yaw;
        self.goal_pitch = (self.goal_pitch + d_pitch).clamp(MIN_PITCH, MAX_PITCH);
    }

    /// factor < 1 acerca, factor > 1 aleja.
    pub fn zoom(&mut self, factor: f32) {
        self.goal_distance = (self.goal_distance * factor).clamp(self.min_distance, self.max_distance);
    }

    /// Fija posicion sin interpolar (exportacion de cuadros).
    pub fn set_orbit(&mut self, yaw: f32, pitch: f32, distance: f32) {
        self.goal_yaw = yaw;
        self.goal_pitch = pitch.clamp(MIN_PITCH, MAX_PITCH);
        self.goal_distance = distance.clamp(self.min_distance, self.max_distance);
        self.yaw = self.goal_yaw;
        self.pitch = self.goal_pitch;
        self.distance = self.goal_distance;
    }

    /// Interpola hacia el destino. Devuelve true si la camara sigue en movimiento.
    pub fn update(&mut self, dt: f32) -> bool {
        let k = 1.0 - (-12.0 * dt).exp();
        self.yaw += (self.goal_yaw - self.yaw) * k;
        self.pitch += (self.goal_pitch - self.pitch) * k;
        self.distance += (self.goal_distance - self.distance) * k;

        let moving = (self.goal_yaw - self.yaw).abs() > 1e-3
            || (self.goal_pitch - self.pitch).abs() > 1e-3
            || (self.goal_distance - self.distance).abs() > 0.05;
        if !moving {
            self.yaw = self.goal_yaw;
            self.pitch = self.goal_pitch;
            self.distance = self.goal_distance;
        }
        moving
    }

    pub fn frame(&self) -> CameraFrame {
        let eye = self.eye();
        let forward = (self.target - eye).normalize();
        let right = forward.cross(&vec3(0.0, 1.0, 0.0)).normalize();
        let up = right.cross(&forward);
        CameraFrame {
            eye,
            forward,
            right,
            up,
            tan_half_fov: (self.fov_deg.to_radians() * 0.5).tan(),
        }
    }
}

impl CameraFrame {
    /// Rayo primario por el punto (px, py) del plano de imagen, en pixeles.
    pub fn primary_ray(&self, px: f32, py: f32, width: usize, height: usize) -> Ray {
        let aspect = width as f32 / height as f32;
        let ndc_x = (2.0 * px / width as f32 - 1.0) * aspect * self.tan_half_fov;
        let ndc_y = (1.0 - 2.0 * py / height as f32) * self.tan_half_fov;
        Ray::new(self.eye, self.forward + self.right * ndc_x + self.up * ndc_y)
    }
}

use crate::ray::Ray;
use nalgebra_glm::{vec3, Vec3};

/// Escena provisional del hito 1: plano a cuadros y cielo en degradado.
pub fn trace(ray: &Ray) -> Vec3 {
    if ray.dir.y < -1e-4 {
        let t = -ray.origin.y / ray.dir.y;
        let p = ray.at(t);
        let checker = ((p.x / 8.0).floor() as i32 + (p.z / 8.0).floor() as i32) & 1;
        return if checker == 0 { vec3(0.35, 0.55, 0.25) } else { vec3(0.25, 0.42, 0.18) };
    }
    let t = ray.dir.y.max(0.0);
    vec3(0.75, 0.85, 0.95) * (1.0 - t) + vec3(0.25, 0.45, 0.85) * t
}

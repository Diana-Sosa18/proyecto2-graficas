use crate::material;
use crate::ray::Ray;
use crate::voxel_grid::VoxelGrid;
use nalgebra_glm::{vec3, Vec3};

pub struct Scene {
    pub grid: VoxelGrid,
}

pub fn trace(scene: &Scene, ray: &Ray) -> Vec3 {
    if let Some(hit) = scene.grid.trace(ray, f32::INFINITY, material::AIR) {
        let light = vec3(0.5, 0.8, 0.3).normalize();
        let diffuse = hit.normal.dot(&light).max(0.0);
        return material::debug_albedo(hit.material_id) * (0.3 + 0.7 * diffuse);
    }
    let t = ray.dir.y.max(0.0);
    vec3(0.75, 0.85, 0.95) * (1.0 - t) + vec3(0.25, 0.45, 0.85) * t
}

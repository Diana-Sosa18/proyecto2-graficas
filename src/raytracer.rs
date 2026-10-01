use crate::material::{self, Material};
use crate::ray::Ray;
use crate::texture::{self, SurfacePoint};
use crate::voxel_grid::VoxelGrid;
use nalgebra_glm::{vec3, Vec3};

pub struct Scene {
    pub grid: VoxelGrid,
    pub materials: Vec<Material>,
}

pub fn trace(scene: &Scene, ray: &Ray) -> Vec3 {
    if let Some(hit) = scene.grid.trace(ray, f32::INFINITY, material::AIR) {
        let mat = &scene.materials[hit.material_id as usize];
        let sp = SurfacePoint { p: hit.point - scene.grid.origin, normal: hit.normal, cell: hit.cell };
        let albedo = texture::albedo(mat, &sp);
        let light = vec3(0.5, 0.8, 0.3).normalize();
        let diffuse = hit.normal.dot(&light).max(0.0);
        return albedo * (0.3 + 0.7 * diffuse) + albedo * mat.emission;
    }
    let t = ray.dir.y.max(0.0);
    vec3(0.75, 0.85, 0.95) * (1.0 - t) + vec3(0.25, 0.45, 0.85) * t
}

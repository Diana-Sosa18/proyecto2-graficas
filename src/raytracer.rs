use crate::lighting::{self, Lighting};
use crate::material::{self, Material};
use crate::ray::Ray;
use crate::texture::{self, SurfacePoint};
use crate::voxel_grid::{Hit, VoxelGrid};
use nalgebra_glm::{vec3, Vec3};

pub struct Scene {
    pub grid: VoxelGrid,
    pub materials: Vec<Material>,
    pub lighting: Lighting,
}

pub fn trace(scene: &Scene, ray: &Ray) -> Vec3 {
    match scene.grid.trace(ray, f32::INFINITY, material::AIR) {
        Some(hit) => shade(scene, ray, &hit),
        None => sky(ray),
    }
}

/// Iluminacion local: ambiente con oclusion + difuso Lambert y especular
/// Blinn-Phong del sol, ambos atenuados por el shadow ray.
fn shade(scene: &Scene, ray: &Ray, hit: &Hit) -> Vec3 {
    let mat = &scene.materials[hit.material_id as usize];
    let sp = SurfacePoint { p: hit.point - scene.grid.origin, normal: hit.normal, cell: hit.cell };
    let albedo = texture::albedo(mat, &sp);
    if mat.emission > 0.0 {
        return albedo * mat.emission;
    }

    let light = &scene.lighting;
    let n = hit.normal;
    let ao = lighting::ambient_occlusion(&scene.grid, hit);
    let mut color = albedo.component_mul(&light.ambient(&n)) * ao;

    let n_dot_l = n.dot(&light.sun_dir);
    if n_dot_l > 0.0 {
        let vis = light.sun_visibility(&scene.grid, hit.point, n);
        if vis > 0.0 {
            let half = (light.sun_dir - ray.dir).normalize();
            let spec = mat.specular * n.dot(&half).max(0.0).powf(mat.shininess);
            color += (albedo * n_dot_l + Vec3::repeat(spec)).component_mul(&light.sun_color) * vis;
        }
    }
    color
}

fn sky(ray: &Ray) -> Vec3 {
    let t = ray.dir.y.max(0.0);
    vec3(0.75, 0.85, 0.95) * (1.0 - t) + vec3(0.25, 0.45, 0.85) * t
}

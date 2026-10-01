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

/// Profundidad maxima de recursion para reflexion/refraccion.
pub const MAX_DEPTH: u32 = 3;
/// Desplazamiento para que los rayos secundarios no choquen con su propia cara.
const EPS: f32 = 2e-3;

pub fn trace(scene: &Scene, ray: &Ray) -> Vec3 {
    cast(scene, ray, 0, material::AIR)
}

/// Lanza un rayo que viaja por `medium` y devuelve el color que ve.
fn cast(scene: &Scene, ray: &Ray, depth: u32, medium: u8) -> Vec3 {
    match scene.grid.trace(ray, f32::INFINITY, medium) {
        Some(hit) => shade(scene, ray, &hit, depth),
        None => sky(ray),
    }
}

/// R = I - 2 (I . N) N
pub fn reflect(i: Vec3, n: Vec3) -> Vec3 {
    i - n * (2.0 * i.dot(&n))
}

/// Iluminacion local: ambiente con oclusion + difuso Lambert y especular
/// Blinn-Phong del sol, ambos atenuados por el shadow ray.
fn shade(scene: &Scene, ray: &Ray, hit: &Hit, depth: u32) -> Vec3 {
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

    // Reflexion recursiva, limitada por MAX_DEPTH.
    if mat.reflectivity > 0.0 && depth < MAX_DEPTH {
        let dir = reflect(ray.dir, n);
        let reflected = cast(scene, &Ray::new(hit.point + n * EPS, dir), depth + 1, material::AIR);
        let tint = if mat.metallic { albedo } else { Vec3::repeat(1.0) };
        color = color * (1.0 - mat.reflectivity) + reflected.component_mul(&tint) * mat.reflectivity;
    }
    color
}

fn sky(ray: &Ray) -> Vec3 {
    let t = ray.dir.y.max(0.0);
    vec3(0.75, 0.85, 0.95) * (1.0 - t) + vec3(0.25, 0.45, 0.85) * t
}

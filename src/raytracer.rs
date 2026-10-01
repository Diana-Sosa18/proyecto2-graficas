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
    let Some(hit) = scene.grid.trace(ray, f32::INFINITY, medium) else {
        return sky(ray);
    };
    let entering_transparent = scene.materials[hit.material_id as usize].transparency > 0.0;
    let color = if medium != material::AIR && hit.material_id == material::AIR {
        // El rayo sale del agua hacia el aire.
        shade_interface(scene, ray, &hit, depth, medium, material::AIR)
    } else if entering_transparent && medium == material::AIR {
        shade_interface(scene, ray, &hit, depth, medium, hit.material_id)
    } else {
        shade(scene, ray, &hit, depth)
    };
    if medium != material::AIR {
        // Absorcion de Beer-Lambert segun la distancia recorrida dentro del medio.
        let k = WATER_ABSORPTION;
        color.component_mul(&vec3((-k.x * hit.t).exp(), (-k.y * hit.t).exp(), (-k.z * hit.t).exp()))
    } else {
        color
    }
}

/// Coeficientes de absorcion del agua por canal (el rojo se pierde primero).
const WATER_ABSORPTION: Vec3 = Vec3::new(0.50, 0.17, 0.11);

/// Ley de Snell en forma vectorial. `n` apunta hacia el lado incidente y
/// `eta = n1 / n2`. Devuelve None si hay reflexion total interna.
pub fn refract(i: Vec3, n: Vec3, eta: f32) -> Option<Vec3> {
    let cos_i = -i.dot(&n);
    let k = 1.0 - eta * eta * (1.0 - cos_i * cos_i);
    if k < 0.0 {
        None
    } else {
        Some(i * eta + n * (eta * cos_i - k.sqrt()))
    }
}

/// Aproximacion de Schlick para la reflectancia de Fresnel.
pub fn fresnel_schlick(cos_i: f32, n1: f32, n2: f32) -> f32 {
    let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
    let mut cos = cos_i;
    if n1 > n2 {
        // Pasando a un medio menos denso se usa el angulo transmitido.
        let eta = n1 / n2;
        let sin2_t = eta * eta * (1.0 - cos_i * cos_i);
        if sin2_t > 1.0 {
            return 1.0;
        }
        cos = (1.0 - sin2_t).sqrt();
    }
    r0 + (1.0 - r0) * (1.0 - cos).powi(5)
}

fn ior_of(scene: &Scene, medium: u8) -> f32 {
    if medium == material::AIR {
        1.0
    } else {
        scene.materials[medium as usize].ior
    }
}

/// Interfaz entre dos medios (aire <-> agua): mezcla reflexion y refraccion
/// segun Fresnel; si hay reflexion total interna solo se refleja.
fn shade_interface(scene: &Scene, ray: &Ray, hit: &Hit, depth: u32, from: u8, to: u8) -> Vec3 {
    let liquid = if to != material::AIR { to } else { from };
    let mat = &scene.materials[liquid as usize];
    let (n1, n2) = (ior_of(scene, from), ior_of(scene, to));

    // La normal de la grilla siempre mira hacia el lado de donde viene el rayo.
    let mut n = hit.normal;
    if n.y.abs() > 0.5 {
        let waved = texture::water_normal(hit.point - scene.grid.origin, n.y.signum());
        if ray.dir.dot(&waved) < 0.0 {
            n = waved;
        }
    }
    let light = &scene.lighting;
    if depth >= MAX_DEPTH {
        return mat.albedo.component_mul(&light.ambient(&n));
    }

    let cos_i = (-ray.dir.dot(&n)).clamp(0.0, 1.0);
    let fresnel = fresnel_schlick(cos_i, n1, n2).max(mat.reflectivity * 0.4);
    let reflected = cast(scene, &Ray::new(hit.point + n * EPS, reflect(ray.dir, n)), depth + 1, from);

    let mut color = match refract(ray.dir, n, n1 / n2) {
        Some(t) => {
            let transmitted = cast(scene, &Ray::new(hit.point - n * EPS, t), depth + 1, to);
            let body = mat.albedo.component_mul(&light.ambient(&n));
            let through = transmitted * mat.transparency + body * (1.0 - mat.transparency);
            reflected * fresnel + through * (1.0 - fresnel)
        }
        None => reflected, // reflexion total interna
    };

    // Destello especular del sol sobre la superficie vista desde el aire.
    if from == material::AIR && n.dot(&light.sun_dir) > 0.0 {
        let vis = light.sun_visibility(&scene.grid, hit.point, hit.normal);
        let half = (light.sun_dir - ray.dir).normalize();
        let spec = mat.specular * n.dot(&half).max(0.0).powf(mat.shininess);
        color += light.sun_color * (spec * vis);
    }
    color
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

use crate::material::{AIR, LANTERN, WATER};
use crate::ray::Ray;
use crate::voxel_grid::{Hit, VoxelGrid};
use nalgebra_glm::{vec3, Vec3};

pub struct Lighting {
    /// Direccion normalizada desde la superficie hacia el sol.
    pub sun_dir: Vec3,
    pub sun_color: Vec3,
    pub sky_ambient: Vec3,
    pub ground_ambient: Vec3,
}

impl Default for Lighting {
    fn default() -> Self {
        Lighting {
            sun_dir: vec3(-0.72, 0.52, 0.32).normalize(),
            sun_color: vec3(1.0, 0.88, 0.70) * 2.6,
            sky_ambient: vec3(0.50, 0.62, 0.85) * 0.75,
            ground_ambient: vec3(0.42, 0.36, 0.28) * 0.4,
        }
    }
}

impl Lighting {
    /// Luz ambiental hemisferica: cielo arriba, rebote del suelo abajo.
    pub fn ambient(&self, normal: &Vec3) -> Vec3 {
        let t = 0.5 + 0.5 * normal.y;
        self.ground_ambient * (1.0 - t) + self.sky_ambient * t
    }

    /// Fraccion de luz solar que llega a `point` (0 = sombra total).
    /// Lanza un shadow ray hacia el sol; el agua deja pasar parte de la luz.
    pub fn sun_visibility(&self, grid: &VoxelGrid, point: Vec3, normal: Vec3) -> f32 {
        let mut ray = Ray::new(point + normal * 2e-3, self.sun_dir);
        let mut medium = AIR;
        let mut transmit = 1.0;
        for _ in 0..4 {
            match grid.trace(&ray, f32::INFINITY, medium) {
                None => return transmit,
                Some(hit) if hit.material_id == WATER && medium == AIR => {
                    transmit *= 0.8;
                    medium = WATER;
                    ray.origin = hit.point - hit.normal * 2e-3;
                }
                Some(hit) if medium == WATER && hit.material_id == AIR => {
                    medium = AIR;
                    ray.origin = hit.point - hit.normal * 2e-3;
                }
                Some(_) => return 0.0,
            }
        }
        transmit
    }
}

fn blocks_light(m: u8) -> bool {
    m != AIR && m != WATER && m != LANTERN
}

/// Oclusion ambiental por voxel: revisa los 8 vecinos de la cara impactada
/// e interpola los valores de las 4 esquinas segun la posicion en la cara.
pub fn ambient_occlusion(grid: &VoxelGrid, hit: &Hit) -> f32 {
    let axis = if hit.normal.x != 0.0 {
        0
    } else if hit.normal.y != 0.0 {
        1
    } else {
        2
    };
    let (t1, t2) = match axis {
        0 => (1, 2),
        1 => (0, 2),
        _ => (0, 1),
    };
    let mut layer = hit.cell;
    layer[axis] += hit.normal[axis] as i32;

    let local = hit.point - grid.origin;
    let fu = (local[t1] - hit.cell[t1] as f32).clamp(0.0, 1.0);
    let fv = (local[t2] - hit.cell[t2] as f32).clamp(0.0, 1.0);

    let occ = |du: i32, dv: i32| -> bool {
        let mut c = layer;
        c[t1] += du;
        c[t2] += dv;
        blocks_light(grid.get(c[0], c[1], c[2]))
    };
    let corner = |du: i32, dv: i32| -> f32 {
        let s1 = occ(du, 0);
        let s2 = occ(0, dv);
        if s1 && s2 {
            0.0
        } else {
            (3 - s1 as i32 - s2 as i32 - occ(du, dv) as i32) as f32 / 3.0
        }
    };
    let c00 = corner(-1, -1);
    let c10 = corner(1, -1);
    let c01 = corner(-1, 1);
    let c11 = corner(1, 1);
    let a = c00 + (c10 - c00) * fu;
    let b = c01 + (c11 - c01) * fu;
    let ao = a + (b - a) * fv;
    0.45 + 0.55 * ao
}

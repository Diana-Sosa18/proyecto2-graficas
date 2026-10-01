//! Texturas procedurales: cada material calcula su color en el punto de
//! impacto a partir de coordenadas de la cara del voxel y ruido determinista.

use crate::material::{Material, TextureKind};
use crate::noise::{fbm2, hash2, hash3, value_noise2};
use nalgebra_glm::{vec3, Vec3};

/// Datos geometricos del punto de impacto, en espacio de grilla.
pub struct SurfacePoint {
    pub p: Vec3,
    pub normal: Vec3,
    pub cell: [i32; 3],
}

impl SurfacePoint {
    /// Coordenadas (u, v) sobre la cara; en caras laterales v es la altura.
    fn uv(&self) -> (f32, f32) {
        if self.normal.x != 0.0 {
            (self.p.z, self.p.y)
        } else if self.normal.y != 0.0 {
            (self.p.x, self.p.z)
        } else {
            (self.p.x, self.p.y)
        }
    }

    fn is_top(&self) -> bool {
        self.normal.y > 0.5
    }
}

fn fract(v: f32) -> f32 {
    v - v.floor()
}

/// Mortero: 0 en la junta, 1 dentro del bloque.
fn mortar(u: f32, v: f32, width: f32) -> f32 {
    let du = fract(u).min(1.0 - fract(u));
    let dv = fract(v).min(1.0 - fract(v));
    let d = du.min(dv);
    ((d - width) / width).clamp(0.0, 1.0)
}

/// Patron de ladrillos con hileras desplazadas; devuelve (mortero, id del ladrillo).
fn bricks(u: f32, v: f32, len: f32, height: f32) -> (f32, f32) {
    let row = (v / height).floor();
    let shift = if row as i32 % 2 == 0 { 0.0 } else { 0.5 };
    let bu = u / len + shift;
    let col = bu.floor();
    (mortar(bu, v / height, 0.045), hash2(col as i32, row as i32, 17))
}

pub fn albedo(mat: &Material, sp: &SurfacePoint) -> Vec3 {
    let (u, v) = sp.uv();
    let [cx, cy, cz] = sp.cell;
    // Variacion de tono por bloque para que no todos se vean iguales.
    let cell_var = 0.92 + 0.16 * hash3(cx, cy, cz, 3);
    let fine = value_noise2(u * 9.0, v * 9.0, 21);
    let base = mat.albedo;

    let color = match mat.texture {
        TextureKind::None => base,
        TextureKind::WallBricks => {
            if sp.is_top() {
                let m = mortar(u, v, 0.04);
                let tile = 0.9 + 0.2 * hash2(u.floor() as i32, v.floor() as i32, 5);
                base * (0.6 + 0.4 * m) * tile * (0.9 + 0.15 * fine)
            } else {
                let (m, id) = bricks(u, v, 1.0, 0.5);
                let weather = fbm2(u * 0.35, v * 0.35, 3, 41);
                let tint = vec3(0.95 + 0.1 * id, 0.95 + 0.07 * id, 0.93);
                let brick = base.component_mul(&tint) * (0.82 + 0.3 * id) * (0.88 + 0.2 * fine);
                let mortar_col = base * 0.55;
                (mortar_col * (1.0 - m) + brick * m) * (0.85 + 0.25 * weather)
            }
        }
        TextureKind::BigBlocks => {
            let (m, id) = bricks(u, v, 1.5, 1.0);
            base * (0.65 + 0.35 * m) * (0.85 + 0.25 * id) * (0.85 + 0.25 * fine)
        }
        TextureKind::Paving => {
            let m = mortar(u, v, 0.035);
            let id = hash2(u.floor() as i32, v.floor() as i32, 9);
            base * (0.7 + 0.3 * m) * (0.88 + 0.2 * id) * (0.9 + 0.15 * fine)
        }
        TextureKind::Tiles => {
            // Filas de tejas curvas: oscurecen en los bordes de cada fila.
            let row = fract(v * 3.0);
            let col = fract(u * 4.0 + if (v * 3.0).floor() as i32 % 2 == 0 { 0.0 } else { 0.5 });
            let curve = (col * std::f32::consts::PI).sin().powf(0.6);
            let edge = 0.65 + 0.35 * (1.0 - row).powf(0.5);
            base * curve.mul_add(0.35, 0.65) * edge * (0.9 + 0.15 * fine)
        }
        TextureKind::Grass => {
            let blades = hash2((u * 8.0).floor() as i32, (v * 8.0).floor() as i32, cx as u32 ^ 77);
            let green = base.component_mul(&vec3(0.85 + 0.3 * blades, 0.88 + 0.22 * blades, 0.9)) * (0.85 + 0.25 * fine);
            if sp.is_top() || sp.normal.y < -0.5 {
                green
            } else {
                // Costado: franja de cesped arriba y tierra abajo.
                let lip = 0.72 - 0.18 * hash2((u * 8.0).floor() as i32, cy, 31);
                if fract(v) > lip {
                    green
                } else {
                    vec3(0.46, 0.33, 0.21) * (0.85 + 0.25 * fine)
                }
            }
        }
        TextureKind::Dirt => {
            let speck = hash2((u * 8.0).floor() as i32, (v * 8.0).floor() as i32, 13);
            base * (0.8 + 0.3 * speck) * (0.85 + 0.2 * fine)
        }
        TextureKind::Rock => {
            let n = fbm2(u * 1.3, v * 1.3, 4, 51);
            let crack = ((n - 0.5).abs() < 0.03) as i32 as f32;
            base * (0.75 + 0.45 * n) * (1.0 - 0.35 * crack)
        }
        TextureKind::Water => {
            let w = (u * 2.1 + (v * 1.3).sin()).sin() * (v * 1.7 + (u * 0.9).cos()).sin();
            base * (0.9 + 0.12 * w)
        }
        TextureKind::Metal => {
            let brushed = value_noise2(u * 30.0, v * 2.0, 61);
            base * (0.92 + 0.12 * brushed)
        }
        TextureKind::Lacquer => {
            let grain = value_noise2(u * 3.0, v * 24.0, 71);
            base * (0.85 + 0.25 * grain)
        }
        TextureKind::Leaves => {
            let leaf = hash2((u * 6.0).floor() as i32, (v * 6.0).floor() as i32, (cx * 31 + cz) as u32);
            let hole = if leaf < 0.12 { 0.55 } else { 1.0 };
            base * (0.75 + 0.45 * leaf) * hole
        }
        TextureKind::Snow => base * (0.94 + 0.08 * fine),
        TextureKind::Lantern => {
            let frame = mortar(u * 2.0, v * 2.0, 0.08);
            base * (0.35 + 0.65 * frame)
        }
        TextureKind::Cloth => base * (0.85 + 0.15 * (u * 6.0 + v * 2.0).sin()),
        TextureKind::Sand => {
            let pebble = hash2((u * 6.0).floor() as i32, (v * 6.0).floor() as i32, 81);
            let p = if pebble > 0.93 { 0.75 } else { 1.0 };
            base * (0.86 + 0.2 * fine) * p
        }
        TextureKind::Planks => {
            let plank = fract(v * 2.0);
            let gap = if plank < 0.06 { 0.5 } else { 1.0 };
            let grain = value_noise2(u * 12.0, v * 2.0 * 6.0, 91);
            base * (0.8 + 0.3 * grain) * gap
        }
        TextureKind::Bark => {
            let stripes = value_noise2(u * 10.0, v * 1.5, 101);
            base * (0.7 + 0.45 * stripes)
        }
    };
    color * cell_var
}

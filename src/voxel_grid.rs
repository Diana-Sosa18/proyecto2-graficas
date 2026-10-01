use crate::ray::Ray;
use crate::voxel::Voxel;
use nalgebra_glm::{vec3, Vec3};

/// Resultado de intersectar un rayo con la grilla.
#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub t: f32,
    pub point: Vec3,
    /// Normal de la cara atravesada (siempre alineada a un eje).
    pub normal: Vec3,
    pub cell: [i32; 3],
    pub material_id: u8,
}

/// Grilla 3D densa: X = ancho, Y = altura, Z = profundidad.
/// La celda (x, y, z) ocupa el cubo [origin + (x,y,z), origin + (x+1,y+1,z+1)].
pub struct VoxelGrid {
    pub nx: i32,
    pub ny: i32,
    pub nz: i32,
    pub origin: Vec3,
    voxels: Vec<Voxel>,
    /// Distancia de Chebyshev (en celdas, saturada) a la celda ocupada mas
    /// cercana. Permite saltar espacio vacio durante el recorrido.
    empty_dist: Vec<u8>,
}

/// Distancia maxima almacenada en el campo de distancias.
const MAX_SKIP: u8 = 12;

impl VoxelGrid {
    pub fn new(nx: i32, ny: i32, nz: i32) -> Self {
        VoxelGrid {
            nx,
            ny,
            nz,
            origin: vec3(-nx as f32 * 0.5, 0.0, -nz as f32 * 0.5),
            voxels: vec![Voxel::EMPTY; (nx * ny * nz) as usize],
            empty_dist: Vec::new(),
        }
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && y >= 0 && z >= 0 && x < self.nx && y < self.ny && z < self.nz
    }

    #[inline]
    fn index(&self, x: i32, y: i32, z: i32) -> usize {
        ((y * self.nz + z) * self.nx + x) as usize
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32, z: i32) -> u8 {
        if self.in_bounds(x, y, z) {
            self.voxels[self.index(x, y, z)].material_id
        } else {
            0
        }
    }

    pub fn set(&mut self, x: i32, y: i32, z: i32, material_id: u8) {
        if self.in_bounds(x, y, z) {
            let i = self.index(x, y, z);
            self.voxels[i] = Voxel::new(material_id);
        }
    }

    /// Rellena la caja de celdas [min, max] (ambos inclusive).
    pub fn fill_box(&mut self, min: [i32; 3], max: [i32; 3], material_id: u8) {
        for y in min[1]..=max[1] {
            for z in min[2]..=max[2] {
                for x in min[0]..=max[0] {
                    self.set(x, y, z, material_id);
                }
            }
        }
    }

    /// Calcula el campo de distancias con tres pasadas separables (una por eje):
    /// d(p) = min sobre celdas ocupadas q de max(|px-qx|, |py-qy|, |pz-qz|).
    /// Debe llamarse despues de terminar de construir la escena.
    pub fn build_distance_field(&mut self) {
        let (nx, ny, nz) = (self.nx, self.ny, self.nz);
        let n = (nx * ny * nz) as usize;
        let cap = MAX_SKIP as i32;
        let mut cur: Vec<u8> = self.voxels.iter().map(|v| if v.occupied() { 0 } else { MAX_SKIP }).collect();
        let mut next = vec![MAX_SKIP; n];
        let dims = [nx, ny, nz];
        for axis in 0..3 {
            for y in 0..ny {
                for z in 0..nz {
                    for x in 0..nx {
                        let p = [x, y, z];
                        let mut best = cur[self.index(x, y, z)] as i32;
                        for k in 1..cap.min(best + 1) {
                            if k >= best {
                                break;
                            }
                            for sgn in [-1, 1] {
                                let mut q = p;
                                q[axis] += sgn * k;
                                if q[axis] < 0 || q[axis] >= dims[axis] {
                                    continue;
                                }
                                let d = (cur[self.index(q[0], q[1], q[2])] as i32).max(k);
                                best = best.min(d);
                            }
                        }
                        next[self.index(x, y, z)] = best as u8;
                    }
                }
            }
            std::mem::swap(&mut cur, &mut next);
        }
        self.empty_dist = cur;
    }

    /// Interseccion rayo-AABB por el metodo de slabs, en espacio de grilla.
    /// Devuelve (t_entrada, t_salida, eje de entrada).
    fn intersect_bounds(&self, origin: Vec3, dir: Vec3) -> Option<(f32, f32, usize)> {
        let size = [self.nx as f32, self.ny as f32, self.nz as f32];
        let mut t_min = f32::NEG_INFINITY;
        let mut t_max = f32::INFINITY;
        let mut axis = 0;
        for a in 0..3 {
            if dir[a].abs() < 1e-9 {
                if origin[a] < 0.0 || origin[a] > size[a] {
                    return None;
                }
                continue;
            }
            let inv = 1.0 / dir[a];
            let mut t0 = -origin[a] * inv;
            let mut t1 = (size[a] - origin[a]) * inv;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            if t0 > t_min {
                t_min = t0;
                axis = a;
            }
            t_max = t_max.min(t1);
        }
        if t_max < t_min.max(0.0) {
            None
        } else {
            Some((t_min, t_max, axis))
        }
    }

    /// Recorre la grilla con DDA (Amanatides & Woo) y devuelve la primera celda
    /// cuyo material difiere de `medium` (el medio por donde viaja el rayo:
    /// 0 = aire, o el id del agua cuando el rayo esta refractado dentro de ella).
    /// Si el rayo empieza dentro de la grilla, la celda inicial se ignora.
    pub fn trace(&self, ray: &Ray, max_t: f32, medium: u8) -> Option<Hit> {
        let o = ray.origin - self.origin;
        let d = ray.dir;
        let (t_enter, t_exit, enter_axis) = self.intersect_bounds(o, d)?;
        let t_exit = t_exit.min(max_t);
        let starts_outside = t_enter > 0.0;
        let t_start = t_enter.max(0.0);
        if t_start > t_exit {
            return None;
        }

        let p = o + d * (t_start + 1e-4);
        let mut cell = [
            (p.x.floor() as i32).clamp(0, self.nx - 1),
            (p.y.floor() as i32).clamp(0, self.ny - 1),
            (p.z.floor() as i32).clamp(0, self.nz - 1),
        ];
        let step = [sign(d.x), sign(d.y), sign(d.z)];
        let mut t_max = [0.0f32; 3];
        let mut t_delta = [f32::INFINITY; 3];
        for a in 0..3 {
            if step[a] != 0 {
                let next = cell[a] as f32 + if step[a] > 0 { 1.0 } else { 0.0 };
                t_max[a] = (next - o[a]) / d[a];
                t_delta[a] = (1.0 / d[a]).abs();
            } else {
                t_max[a] = f32::INFINITY;
            }
        }

        let make_hit = |t: f32, axis: usize, cell: [i32; 3], material_id: u8| {
            let mut normal = Vec3::zeros();
            normal[axis] = -(step[axis] as f32);
            Hit { t, point: ray.at(t), normal, cell, material_id }
        };

        if starts_outside {
            let m = self.get(cell[0], cell[1], cell[2]);
            if m != medium {
                return Some(make_hit(t_start, enter_axis, cell, m));
            }
        }

        let use_skip = medium == 0 && !self.empty_dist.is_empty();
        // t en el que el rayo entro a la celda actual.
        let mut t_cell = t_start;
        loop {
            // Salto de espacio vacio: si la celda actual esta a distancia `dist`
            // de cualquier bloque, el rayo puede avanzar dist-1 unidades sin chocar.
            if use_skip {
                let dist = self.empty_dist[self.index(cell[0], cell[1], cell[2])];
                if dist >= 3 {
                    let t_jump = t_cell + dist as f32 - 1.05;
                    if t_jump > t_exit {
                        return None;
                    }
                    let p = o + d * t_jump;
                    let jumped = [p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32];
                    if !self.in_bounds(jumped[0], jumped[1], jumped[2]) {
                        return None;
                    }
                    t_cell = t_jump;
                    if jumped != cell {
                        cell = jumped;
                        for a in 0..3 {
                            if step[a] != 0 {
                                let next = cell[a] as f32 + if step[a] > 0 { 1.0 } else { 0.0 };
                                t_max[a] = (next - o[a]) / d[a];
                            }
                        }
                    }
                }
            }
            let axis = if t_max[0] < t_max[1] {
                if t_max[0] < t_max[2] { 0 } else { 2 }
            } else if t_max[1] < t_max[2] {
                1
            } else {
                2
            };
            let t = t_max[axis];
            if t > t_exit {
                return None;
            }
            cell[axis] += step[axis];
            if cell[axis] < 0 || cell[axis] >= [self.nx, self.ny, self.nz][axis] {
                return None;
            }
            t_max[axis] += t_delta[axis];
            t_cell = t;
            let m = self.get(cell[0], cell[1], cell[2]);
            if m != medium {
                return Some(make_hit(t, axis, cell, m));
            }
        }
    }

    /// Centro de una celda en coordenadas de mundo.
    pub fn cell_center(&self, x: i32, y: i32, z: i32) -> Vec3 {
        self.origin + vec3(x as f32 + 0.5, y as f32 + 0.5, z as f32 + 0.5)
    }
}

#[inline]
fn sign(v: f32) -> i32 {
    if v > 0.0 {
        1
    } else if v < 0.0 {
        -1
    } else {
        0
    }
}

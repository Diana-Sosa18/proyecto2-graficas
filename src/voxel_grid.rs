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
}

impl VoxelGrid {
    pub fn new(nx: i32, ny: i32, nz: i32) -> Self {
        VoxelGrid {
            nx,
            ny,
            nz,
            origin: vec3(-nx as f32 * 0.5, 0.0, -nz as f32 * 0.5),
            voxels: vec![Voxel::EMPTY; (nx * ny * nz) as usize],
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

        loop {
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

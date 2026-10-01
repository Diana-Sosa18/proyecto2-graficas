//! Construccion procedural del diorama de la Gran Muralla.
//!
//! La escena se arma por capas: terreno -> caminos -> muralla -> torres ->
//! puertas -> agua -> puente -> casas -> pabellon -> vegetacion -> rocas.

use crate::material::*;
use crate::noise::{fbm2, hash2, smoothstep};
use crate::voxel_grid::VoxelGrid;

pub const GRID_X: i32 = 192;
pub const GRID_Y: i32 = 80;
pub const GRID_Z: i32 = 144;

/// Ultima fila (y) ocupada por agua.
const WATER_LEVEL: i32 = 7;
/// Medio ancho de la muralla en celdas.
const WALL_HALF: f32 = 4.5;
/// Altura del camino de la muralla sobre el terreno.
const WALL_HEIGHT: i32 = 8;
/// Posiciones X de las torres; la de indice GATE_TOWER tiene la puerta principal.
const TOWERS_X: [i32; 4] = [14, 52, 94, 170];
const GATE_TOWER: usize = 2;
const LAKE: (f32, f32, f32, f32) = (150.0, 118.0, 18.0, 12.0); // centro x, z, radio x, z
const BRIDGE_Z: i32 = 96;

fn wall_center(x: f32) -> f32 {
    64.0 + 10.0 * (x * 0.03 + 0.5).sin() + 4.0 * (x * 0.08).sin()
}

fn river_x(z: f32) -> f32 {
    134.0 + 8.0 * (z * 0.05 + 1.0).sin()
}

/// Factor 0..1 de cuanto pertenece (x, z) al cauce del rio o al lago.
fn water_factor(x: f32, z: f32) -> f32 {
    let (cx, cz, rx, rz) = LAKE;
    let river = if z < cz { 1.0 - smoothstep(3.5, 12.0, (x - river_x(z)).abs()) } else { 0.0 };
    let q = ((x - cx) / rx).powi(2) + ((z - cz) / rz).powi(2);
    let lake = 1.0 - smoothstep(0.75, 1.7, q);
    river.max(lake)
}

fn terrain_height(x: f32, z: f32) -> f32 {
    let back = smoothstep(84.0, 8.0, z);
    let mountains = back * (14.0 + 38.0 * fbm2(x * 0.024, z * 0.024, 5, 7).powf(1.3) * 1.4);
    let hills = 6.0 * fbm2(x * 0.06 + 31.0, z * 0.06, 4, 3);
    let bump = |cx: f32, r: f32, a: f32| {
        let dx = x - cx;
        let dz = z - wall_center(cx);
        a * (-(dx * dx + dz * dz * 0.5) / (r * r)).exp()
    };
    let land = 6.5 + hills + mountains + bump(52.0, 20.0, 15.0) + bump(172.0, 18.0, 11.0);
    let bed = 3.6 + fbm2(x * 0.2, z * 0.2, 2, 11) * 1.2;
    let f = water_factor(x, z);
    land * (1.0 - f) + bed * f
}

struct Builder {
    height: Vec<i32>,
    wall_top: Vec<i32>,
    reserved: Vec<bool>,
}

impl Builder {
    fn h(&self, x: i32, z: i32) -> i32 {
        let x = x.clamp(0, GRID_X - 1);
        let z = z.clamp(0, GRID_Z - 1);
        self.height[(z * GRID_X + x) as usize]
    }

    fn reserve(&mut self, x0: i32, z0: i32, x1: i32, z1: i32) {
        for z in z0.max(0)..=z1.min(GRID_Z - 1) {
            for x in x0.max(0)..=x1.min(GRID_X - 1) {
                self.reserved[(z * GRID_X + x) as usize] = true;
            }
        }
    }

    fn is_reserved(&self, x: i32, z: i32) -> bool {
        x < 0 || z < 0 || x >= GRID_X || z >= GRID_Z || self.reserved[(z * GRID_X + x) as usize]
    }

    fn max_h(&self, x0: i32, z0: i32, x1: i32, z1: i32) -> i32 {
        let mut m = i32::MIN;
        for z in z0..=z1 {
            for x in x0..=x1 {
                m = m.max(self.h(x, z));
            }
        }
        m
    }
}

pub fn build_scene() -> VoxelGrid {
    let mut g = VoxelGrid::new(GRID_X, GRID_Y, GRID_Z);
    let mut b = Builder {
        height: Vec::with_capacity((GRID_X * GRID_Z) as usize),
        wall_top: Vec::new(),
        reserved: vec![false; (GRID_X * GRID_Z) as usize],
    };
    build_terrain(&mut g, &mut b);
    build_paths(&mut g, &mut b);
    build_wall(&mut g, &mut b);
    build_towers(&mut g, &mut b);
    build_water_gate(&mut g, &b);
    build_water(&mut g, &mut b);
    build_bridge(&mut g, &mut b);
    build_houses(&mut g, &mut b);
    build_pavilion(&mut g, &mut b);
    build_trees(&mut g, &mut b);
    build_rocks(&mut g, &mut b);
    g
}

// ---------------------------------------------------------------- terreno

fn build_terrain(g: &mut VoxelGrid, b: &mut Builder) {
    for z in 0..GRID_Z {
        for x in 0..GRID_X {
            let h = terrain_height(x as f32 + 0.5, z as f32 + 0.5).round() as i32;
            b.height.push(h.clamp(2, GRID_Y - 12));
        }
    }
    for z in 0..GRID_Z {
        for x in 0..GRID_X {
            let h = b.h(x, z);
            let slope = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .map(|(dx, dz)| (b.h(x + dx, z + dz) - h).abs())
                .max()
                .unwrap();
            let wet = water_factor(x as f32 + 0.5, z as f32 + 0.5);
            let top = if h > 47 {
                SNOW
            } else if wet > 0.35 && h <= WATER_LEVEL + 1 {
                if hash2(x, z, 5) < 0.25 { ROCK } else { PATH }
            } else if slope >= 3 || (slope >= 2 && h > 26) {
                ROCK
            } else if h > 43 && hash2(x, z, 9) < 0.5 {
                SNOW
            } else {
                GRASS
            };
            g.set(x, 0, z, BASE);
            for y in 1..h {
                let m = if y >= h - 3 { DIRT } else if y == 1 { STONE_DARK } else { ROCK };
                g.set(x, y, z, m);
            }
            g.set(x, h, z, top);
            if top == SNOW && h > 49 {
                g.set(x, h + 1, z, SNOW);
            }
        }
    }
}

// ---------------------------------------------------------------- caminos

/// Curva de Bezier cuadratica.
fn bezier(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), t: f32) -> (f32, f32) {
    let u = 1.0 - t;
    (
        u * u * p0.0 + 2.0 * u * t * p1.0 + t * t * p2.0,
        u * u * p0.1 + 2.0 * u * t * p1.1 + t * t * p2.1,
    )
}

fn path_curves() -> Vec<[(f32, f32); 3]> {
    let gx = TOWERS_X[GATE_TOWER] as f32;
    let gz = wall_center(gx);
    let bw = river_x(BRIDGE_Z as f32);
    vec![
        [(gx, gz + 8.0), (88.0, 102.0), (74.0, 143.0)],
        [(86.0, 110.0), (108.0, 98.0), (bw - 9.0, BRIDGE_Z as f32)],
        [(bw + 9.0, BRIDGE_Z as f32), (154.0, 95.0), (160.0, 99.0)],
        [(gx, gz - 8.0), (95.0, gz - 16.0), (102.0, gz - 24.0)],
        [(82.0, 120.0), (62.0, 119.0), (40.0, 117.0)],
    ]
}

fn build_paths(g: &mut VoxelGrid, b: &mut Builder) {
    for curve in path_curves() {
        for i in 0..=200 {
            let (px, pz) = bezier(curve[0], curve[1], curve[2], i as f32 / 200.0);
            for dz in -3..=3 {
                for dx in -3..=3 {
                    let (x, z) = (px as i32 + dx, pz as i32 + dz);
                    if !g.in_bounds(x, 0, z) {
                        continue;
                    }
                    let d2 = (x as f32 + 0.5 - px).powi(2) + (z as f32 + 0.5 - pz).powi(2);
                    if d2 < 4.4 {
                        let h = b.h(x, z);
                        if h > WATER_LEVEL {
                            g.set(x, h, z, if hash2(x, z, 77) < 0.12 { DIRT } else { PATH });
                        }
                    }
                    if d2 < 9.0 {
                        b.reserve(x, z, x, z);
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------- muralla

fn wall_cells(x: i32) -> impl Iterator<Item = (i32, f32)> {
    let zc = wall_center(x as f32 + 0.5);
    let slope = (wall_center(x as f32 + 1.0) - wall_center(x as f32)).abs();
    let cos = 1.0 / (1.0 + slope * slope).sqrt();
    ((zc - 8.0) as i32..=(zc + 8.0) as i32).filter_map(move |z| {
        let d = (z as f32 + 0.5 - zc) * cos;
        if d.abs() <= WALL_HALF {
            Some((z, d))
        } else {
            None
        }
    })
}

fn build_wall(g: &mut VoxelGrid, b: &mut Builder) {
    // Altura del camino: terreno maximo bajo la seccion + altura de muralla,
    // limitada a subir/bajar un bloque por columna para formar escalones.
    let mut top: Vec<i32> = (0..GRID_X)
        .map(|x| {
            let lo = (x - 2).max(0);
            let hi = (x + 2).min(GRID_X - 1);
            (lo..=hi)
                .flat_map(|xx| wall_cells(xx).map(move |(z, _)| (xx, z)))
                .map(|(xx, z)| b.h(xx, z))
                .max()
                .unwrap()
                + WALL_HEIGHT
        })
        .collect();
    for x in 1..GRID_X as usize {
        top[x] = top[x].max(top[x - 1] - 1);
    }
    for x in (0..GRID_X as usize - 1).rev() {
        top[x] = top[x].max(top[x + 1] - 1);
    }

    for x in 0..GRID_X {
        let t = top[x as usize];
        for (z, d) in wall_cells(x) {
            let ground = b.h(x, z);
            for y in 1..t {
                let m = if y <= ground + 1 { STONE_DARK } else { STONE_WALL };
                g.set(x, y, z, m);
            }
            let edge = d.abs() > WALL_HALF - 1.0;
            g.set(x, t, z, if edge { STONE_WALL } else { PAVING });
            if edge {
                g.set(x, t + 1, z, STONE_WALL);
                // Almenas en el lado exterior (hacia las montanas).
                if d < 0.0 && x % 3 != 0 {
                    g.set(x, t + 2, z, STONE_WALL);
                }
                // Antorchas sobre el parapeto interior.
                if d > 0.0 && x % 16 == 8 {
                    g.set(x, t + 2, z, WOOD);
                    g.set(x, t + 3, z, LANTERN);
                }
            }
            b.reserve(x - 2, z - 2, x + 2, z + 2);
        }
    }
    b.wall_top = top;
}

// ---------------------------------------------------------------- torres

fn build_towers(g: &mut VoxelGrid, b: &mut Builder) {
    for (i, &cx) in TOWERS_X.iter().enumerate() {
        let cz = wall_center(cx as f32 + 0.5).floor() as i32;
        let floor = b.wall_top[cx as usize];
        let ground = (cz - 6..=cz + 6).map(|z| b.h(cx, z)).min().unwrap();
        build_tower(g, cx, cz, floor, ground, i == GATE_TOWER);
        b.reserve(cx - 9, cz - 9, cx + 9, cz + 9);
    }
}

fn build_tower(g: &mut VoxelGrid, cx: i32, cz: i32, floor: i32, ground: i32, gate: bool) {
    const R: i32 = 6;
    let top = floor + 9;

    // Cuerpo macizo con zocalo oscuro y una franja decorativa.
    g.fill_box([cx - R, 1, cz - R], [cx + R, top, cz + R], STONE_WALL);
    g.fill_box([cx - R, 1, cz - R], [cx + R, ground + 2, cz + R], STONE_DARK);
    for k in -R..=R {
        for (x, z) in [(cx + k, cz - R), (cx + k, cz + R), (cx - R, cz + k), (cx + R, cz + k)] {
            g.set(x, floor, z, STONE_DARK);
            g.set(x, top, z, STONE_DARK);
        }
    }

    // Sala interior, pasos del camino de la muralla y ventanas.
    g.fill_box([cx - R + 1, floor + 1, cz - R + 1], [cx + R - 1, top - 1, cz + R - 1], AIR);
    g.fill_box([cx - R, floor + 1, cz - 2], [cx + R, floor + 4, cz + 2], AIR);
    g.fill_box([cx - R, floor + 5, cz - 1], [cx + R, floor + 5, cz + 1], AIR);
    for wx in [cx - 3, cx + 2] {
        for z in [cz - R, cz + R] {
            g.fill_box([wx, floor + 3, z], [wx + 1, floor + 5, z], AIR);
        }
    }
    for wz in [cz - 4, cz + 4] {
        for x in [cx - R, cx + R] {
            g.fill_box([x, floor + 6, wz], [x, floor + 7, wz], AIR);
        }
    }

    // Almenas del techo de la torre.
    for k in -R..=R {
        if k.rem_euclid(2) == 0 {
            for (x, z) in [(cx + k, cz - R), (cx + k, cz + R), (cx - R, cz + k), (cx + R, cz + k)] {
                g.set(x, top + 1, z, STONE_WALL);
            }
        }
    }

    // Pabellon: columnas rojas, faroles y techo de tejas escalonado.
    let base = top + 1;
    for (x, z) in [(cx - 3, cz - 3), (cx + 3, cz - 3), (cx - 3, cz + 3), (cx + 3, cz + 3)] {
        g.fill_box([x, base, z], [x, base + 3, z], WOOD);
    }
    g.fill_box([cx - 2, base, cz - 2], [cx + 2, base + 2, cz + 2], WOOD);
    g.fill_box([cx - 1, base, cz - 3], [cx + 1, base + 2, cz + 3], AIR);
    g.fill_box([cx - 3, base, cz - 1], [cx + 3, base + 2, cz + 1], AIR);
    g.fill_box([cx - 1, base, cz - 1], [cx + 1, base + 2, cz + 1], WOOD);
    let roof = base + 4;
    for k in 0..6 {
        let r = 5 - k;
        g.fill_box([cx - r, roof + k, cz - r], [cx + r, roof + k, cz + r], ROOF_TILE);
    }
    // Aleros curvados hacia arriba en las esquinas.
    for (sx, sz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
        g.set(cx + sx * 6, roof + 1, cz + sz * 6, ROOF_TILE);
        g.set(cx + sx * 5, roof, cz + sz * 5, ROOF_TILE);
        g.set(cx + sx * 6, roof + 2, cz + sz * 6, GOLD);
        g.set(cx + sx * 4, roof - 1, cz + sz * 4, LANTERN);
    }
    // Remate dorado, asta y bandera.
    g.fill_box([cx, roof + 6, cz], [cx, roof + 7, cz], GOLD);
    g.fill_box([cx, roof + 8, cz], [cx, roof + 13, cz], TRUNK);
    g.fill_box([cx + 1, roof + 11, cz], [cx + 4, roof + 13, cz], FLAG);
    g.set(cx + 4, roof + 11, cz, AIR);

    // Puerta principal: tunel en arco que atraviesa la torre y la muralla.
    if gate {
        let arch_top = (ground + 7).min(floor - 1);
        for z in cz - 9..=cz + 9 {
            g.fill_box([cx - 2, ground + 1, z], [cx + 2, arch_top - 2, z], AIR);
            g.fill_box([cx - 1, arch_top - 1, z], [cx + 1, arch_top - 1, z], AIR);
            g.set(cx, arch_top, z, AIR);
            for x in cx - 2..=cx + 2 {
                g.set(x, ground, z, PAVING);
            }
        }
        // Marco dorado en ambos lados del arco y puertas de madera abiertas.
        for z in [cz - R - 1, cz + R + 1] {
            g.fill_box([cx - 3, ground + 1, z], [cx - 3, arch_top - 1, z], WOOD);
            g.fill_box([cx + 3, ground + 1, z], [cx + 3, arch_top - 1, z], WOOD);
            g.fill_box([cx - 2, arch_top + 1, z], [cx + 2, arch_top + 1, z], GOLD);
        }
    }
}

// ---------------------------------------------------------------- agua

fn build_water_gate(g: &mut VoxelGrid, b: &Builder) {
    let xr = (100..170)
        .min_by(|&a, &c| {
            let fa = (a as f32 - river_x(wall_center(a as f32))).abs();
            let fc = (c as f32 - river_x(wall_center(c as f32))).abs();
            fa.partial_cmp(&fc).unwrap()
        })
        .unwrap();
    let zc = wall_center(xr as f32).floor() as i32;
    let bed = b.h(xr, zc);
    let spring = bed + 4;
    for x in xr - 4..=xr + 4 {
        let dx = (x - xr) as f32;
        let arch = spring + (4.5f32 * 4.5 - dx * dx).max(0.0).sqrt().round() as i32;
        for z in zc - 8..=zc + 8 {
            let m = g.get(x, bed + 1, z);
            if m == STONE_WALL || m == STONE_DARK {
                for y in b.h(x, z) + 1..=arch {
                    g.set(x, y, z, AIR);
                }
            }
        }
    }
}

fn build_water(g: &mut VoxelGrid, b: &mut Builder) {
    for z in 0..GRID_Z {
        for x in 0..GRID_X {
            let f = water_factor(x as f32 + 0.5, z as f32 + 0.5);
            if f > 0.15 {
                b.reserve(x - 1, z - 1, x + 1, z + 1);
            }
            if f < 0.3 {
                continue;
            }
            for y in b.h(x, z) + 1..=WATER_LEVEL {
                if g.get(x, y, z) == AIR {
                    g.set(x, y, z, WATER);
                }
            }
        }
    }
}

fn build_bridge(g: &mut VoxelGrid, b: &mut Builder) {
    let xc = river_x(BRIDGE_Z as f32).round() as i32;
    for dx in -9..=9 {
        let x = xc + dx;
        let t = dx as f32 / 9.0;
        let deck = WATER_LEVEL + 2 + (2.6 * (1.0 - t * t)).round() as i32;
        for z in BRIDGE_Z - 2..=BRIDGE_Z + 2 {
            let rail = z == BRIDGE_Z - 2 || z == BRIDGE_Z + 2;
            g.set(x, deck, z, if rail { STONE_WALL } else { PAVING });
            if rail {
                g.set(x, deck + 1, z, if dx % 3 == 0 { STONE_WALL } else { AIR });
                if dx.abs() == 9 {
                    g.set(x, deck + 2, z, LANTERN);
                    g.set(x, deck + 1, z, STONE_WALL);
                }
            }
            // Pilares en los extremos y arco central abierto.
            if dx.abs() >= 7 {
                for y in b.h(x, z) + 1..deck {
                    g.set(x, y, z, STONE_DARK);
                }
            } else if dx.abs() >= 5 {
                g.set(x, deck - 1, z, STONE_WALL);
            }
        }
    }
    b.reserve(xc - 11, BRIDGE_Z - 4, xc + 11, BRIDGE_Z + 4);
}

// ---------------------------------------------------------------- construcciones

fn build_houses(g: &mut VoxelGrid, b: &mut Builder) {
    let houses = [(44, 107, false), (60, 106, false), (38, 125, true), (56, 127, true), (70, 128, true)];
    for (x0, z0, faces_north) in houses {
        build_house(g, b, x0, z0, faces_north);
    }
}

/// Casa de 9x7: zocalo, muros claros, columnas rojas y techo a dos aguas.
fn build_house(g: &mut VoxelGrid, b: &mut Builder, x0: i32, z0: i32, faces_north: bool) {
    let (x1, z1) = (x0 + 8, z0 + 6);
    let base = b.max_h(x0, z0, x1, z1) + 1;
    g.fill_box([x0 - 1, 1, z0 - 1], [x1 + 1, base - 1, z1 + 1], STONE_DARK);
    g.fill_box([x0, base, z0], [x1, base + 3, z1], PAVING);
    g.fill_box([x0 + 1, base, z0 + 1], [x1 - 1, base + 3, z1 - 1], AIR);
    for (x, z) in [(x0, z0), (x1, z0), (x0, z1), (x1, z1)] {
        g.fill_box([x, base, z], [x, base + 3, z], WOOD);
    }
    let door_z = if faces_north { z0 } else { z1 };
    g.fill_box([x0 + 4, base, door_z], [x0 + 4, base + 2, door_z], AIR);
    for wx in [x0 + 2, x0 + 6] {
        g.set(wx, base + 2, door_z, AIR);
        g.set(wx, base + 2, if faces_north { z1 } else { z0 }, AIR);
    }
    g.set(x0 + 4, base + 3, door_z + if faces_north { -1 } else { 1 }, LANTERN);
    // Techo escalonado a dos aguas con alero.
    for k in 0..4 {
        g.fill_box([x0 - 1, base + 4 + k, z0 - 1 + k], [x1 + 1, base + 4 + k, z1 + 1 - k], ROOF_TILE);
    }
    for z in [z0 - 1, z1 + 1] {
        g.set(x0 - 1, base + 5, z, ROOF_TILE);
        g.set(x1 + 1, base + 5, z, ROOF_TILE);
    }
    g.fill_box([x0 - 1, base + 7, z0 + 3], [x1 + 1, base + 7, z0 + 3], STONE_DARK);
    b.reserve(x0 - 3, z0 - 3, x1 + 3, z1 + 3);
}

/// Pabellon junto al lago con techo doble y remates dorados reflectivos.
fn build_pavilion(g: &mut VoxelGrid, b: &mut Builder) {
    let (cx, cz) = (164, 102);
    let base = b.max_h(cx - 4, cz - 4, cx + 4, cz + 4) + 1;
    g.fill_box([cx - 4, 1, cz - 4], [cx + 4, base, cz + 4], STONE_DARK);
    g.fill_box([cx - 4, base, cz - 4], [cx + 4, base, cz + 4], PAVING);
    for (x, z) in [(cx - 3, cz - 3), (cx + 3, cz - 3), (cx - 3, cz + 3), (cx + 3, cz + 3)] {
        g.fill_box([x, base + 1, z], [x, base + 4, z], WOOD);
    }
    g.fill_box([cx - 1, base + 1, cz - 1], [cx + 1, base + 2, cz + 1], GOLD);
    for (r0, y0) in [(5, base + 5), (3, base + 9)] {
        for k in 0..3 {
            let r = r0 - k;
            g.fill_box([cx - r, y0 + k, cz - r], [cx + r, y0 + k, cz + r], ROOF_TILE);
        }
        for (sx, sz) in [(-1, -1), (1, -1), (-1, 1), (1, 1)] {
            g.set(cx + sx * r0, y0 + 1, cz + sz * r0, GOLD);
        }
    }
    g.fill_box([cx - 3, base + 8, cz - 3], [cx + 3, base + 8, cz + 3], WOOD);
    g.fill_box([cx, base + 12, cz], [cx, base + 14, cz], GOLD);
    b.reserve(cx - 6, cz - 6, cx + 6, cz + 6);
}

// ---------------------------------------------------------------- vegetacion

fn build_trees(g: &mut VoxelGrid, b: &mut Builder) {
    for z in 2..GRID_Z - 2 {
        for x in 2..GRID_X - 2 {
            let h = b.h(x, z);
            let top = g.get(x, h, z);
            if top != GRASS || b.is_reserved(x, z) {
                continue;
            }
            let r = hash2(x, z, 1234);
            let mountain = h > 18;
            let density = if mountain { 0.035 } else { 0.014 };
            if r > density {
                continue;
            }
            let size = hash2(x, z, 99);
            if mountain || size < 0.3 {
                build_pine(g, x, h + 1, z, 5 + (size * 5.0) as i32);
            } else {
                build_round_tree(g, x, h + 1, z, 2.2 + size * 1.6);
            }
            b.reserve(x - 3, z - 3, x + 3, z + 3);
        }
    }
}

fn build_pine(g: &mut VoxelGrid, x: i32, y: i32, z: i32, height: i32) {
    g.fill_box([x, y, z], [x, y + height - 3, z], TRUNK);
    let mut r = (height / 3).clamp(1, 3);
    let mut yy = y + 2;
    while yy <= y + height + 1 {
        for dz in -r..=r {
            for dx in -r..=r {
                if dx * dx + dz * dz <= r * r + 1 && g.get(x + dx, yy, z + dz) == AIR {
                    g.set(x + dx, yy, z + dz, PINE);
                }
            }
        }
        if (yy - y) % 2 == 1 {
            r = (r - 1).max(0);
        }
        yy += 1;
    }
    g.set(x, y + height + 1, z, PINE);
}

fn build_round_tree(g: &mut VoxelGrid, x: i32, y: i32, z: i32, radius: f32) {
    let trunk = 3;
    g.fill_box([x, y, z], [x, y + trunk, z], TRUNK);
    let cy = y as f32 + trunk as f32 + radius * 0.8;
    let ri = radius.ceil() as i32 + 1;
    for dy in -ri..=ri {
        for dz in -ri..=ri {
            for dx in -ri..=ri {
                let (px, py, pz) = (x + dx, cy as i32 + dy, z + dz);
                let d = ((dx * dx + dz * dz) as f32 + (dy as f32 * 1.2).powi(2)).sqrt();
                let jitter = hash2(px * 7 + py, pz, 3) * 0.9;
                if d + jitter < radius + 0.4 && g.get(px, py, pz) == AIR {
                    g.set(px, py, pz, LEAVES);
                }
            }
        }
    }
}

fn build_rocks(g: &mut VoxelGrid, b: &mut Builder) {
    for z in 3..GRID_Z - 3 {
        for x in 3..GRID_X - 3 {
            let near_water = water_factor(x as f32, z as f32) > 0.05 && water_factor(x as f32, z as f32) < 0.3;
            let p = if near_water { 0.02 } else { 0.0025 };
            if hash2(x, z, 4321) > p || (b.is_reserved(x, z) && !near_water) {
                continue;
            }
            let h = b.h(x, z);
            let r = 0.8 + hash2(x, z, 8) * 1.6;
            let ri = r.ceil() as i32;
            for dy in 0..=ri {
                for dz in -ri..=ri {
                    for dx in -ri..=ri {
                        let d = ((dx * dx + dz * dz) as f32 + (dy as f32 * 1.4).powi(2)).sqrt();
                        if d <= r && g.get(x + dx, h + dy, z + dz) == AIR {
                            g.set(x + dx, h + dy, z + dz, ROCK);
                        }
                    }
                }
            }
            b.reserve(x - 2, z - 2, x + 2, z + 2);
        }
    }
}

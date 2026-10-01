//! Construccion procedural del diorama de la Gran Muralla.

use crate::material::*;
use crate::voxel_grid::VoxelGrid;

pub const GRID_X: i32 = 192;
pub const GRID_Y: i32 = 80;
pub const GRID_Z: i32 = 144;

pub fn build_scene() -> VoxelGrid {
    let mut g = VoxelGrid::new(GRID_X, GRID_Y, GRID_Z);
    build_terrain(&mut g);
    build_wall(&mut g);
    g
}

fn build_terrain(g: &mut VoxelGrid) {
    g.fill_box([0, 0, 0], [GRID_X - 1, 3, GRID_Z - 1], DIRT);
    g.fill_box([0, 4, 0], [GRID_X - 1, 4, GRID_Z - 1], GRASS);
}

/// Seccion recta inicial de muralla con camino y almenas.
fn build_wall(g: &mut VoxelGrid) {
    let z0 = GRID_Z / 2 - 4;
    let z1 = GRID_Z / 2 + 4;
    let top = 16;
    g.fill_box([20, 5, z0], [GRID_X - 21, top, z1], STONE_WALL);
    g.fill_box([20, top, z0 + 1], [GRID_X - 21, top, z1 - 1], PAVING);
    for x in (20..GRID_X - 20).step_by(2) {
        g.set(x, top + 1, z0, STONE_WALL);
        g.set(x, top + 1, z1, STONE_WALL);
    }
}

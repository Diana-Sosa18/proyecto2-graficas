use nalgebra_glm::{vec3, Vec3};

// Identificadores de material guardados en cada voxel (0 = aire).
pub const AIR: u8 = 0;
pub const STONE_WALL: u8 = 1;
pub const STONE_DARK: u8 = 2;
pub const ROOF_TILE: u8 = 3;
pub const GRASS: u8 = 4;
pub const DIRT: u8 = 5;
pub const ROCK: u8 = 6;
pub const WATER: u8 = 7;
pub const GOLD: u8 = 8;
pub const WOOD: u8 = 9;
pub const LEAVES: u8 = 10;
pub const PINE: u8 = 11;
pub const SNOW: u8 = 12;
pub const LANTERN: u8 = 13;
pub const FLAG: u8 = 14;
pub const PATH: u8 = 15;
pub const BASE: u8 = 16;
pub const PAVING: u8 = 17;
pub const TRUNK: u8 = 18;
pub const MATERIAL_COUNT: usize = 19;

/// Color base provisional por material (el sistema completo llega en el hito 4).
pub fn debug_albedo(id: u8) -> Vec3 {
    match id {
        STONE_WALL => vec3(0.62, 0.57, 0.48),
        STONE_DARK => vec3(0.42, 0.40, 0.37),
        ROOF_TILE => vec3(0.55, 0.12, 0.08),
        GRASS => vec3(0.30, 0.52, 0.20),
        DIRT => vec3(0.45, 0.32, 0.20),
        ROCK => vec3(0.50, 0.49, 0.47),
        WATER => vec3(0.15, 0.35, 0.55),
        GOLD => vec3(0.90, 0.70, 0.25),
        WOOD => vec3(0.55, 0.15, 0.10),
        LEAVES => vec3(0.25, 0.50, 0.15),
        PINE => vec3(0.12, 0.32, 0.15),
        SNOW => vec3(0.92, 0.94, 0.97),
        LANTERN => vec3(1.0, 0.6, 0.2),
        FLAG => vec3(0.85, 0.10, 0.08),
        PATH => vec3(0.68, 0.58, 0.42),
        BASE => vec3(0.28, 0.18, 0.12),
        PAVING => vec3(0.70, 0.66, 0.58),
        TRUNK => vec3(0.35, 0.22, 0.12),
        _ => vec3(1.0, 0.0, 1.0),
    }
}

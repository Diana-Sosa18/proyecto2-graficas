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
pub const FLOWER: u8 = 19;
pub const MATERIAL_COUNT: usize = 20;

/// Patron procedural que se aplica sobre el albedo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextureKind {
    None,
    WallBricks,
    BigBlocks,
    Paving,
    Tiles,
    Grass,
    Dirt,
    Rock,
    Water,
    Metal,
    Lacquer,
    Leaves,
    Snow,
    Lantern,
    Cloth,
    Sand,
    Planks,
    Bark,
    Flower,
}

#[derive(Clone, Copy, Debug)]
pub struct Material {
    pub name: &'static str,
    pub albedo: Vec3,
    /// Intensidad del brillo especular (Blinn-Phong).
    pub specular: f32,
    /// Exponente del brillo especular: mayor = brillo mas concentrado.
    pub shininess: f32,
    /// Fraccion de luz reflejada (0 = mate, 1 = espejo).
    pub reflectivity: f32,
    /// Fraccion de luz transmitida por refraccion.
    pub transparency: f32,
    /// Indice de refraccion.
    pub ior: f32,
    /// Luz propia (faroles).
    pub emission: f32,
    /// Si es metal, la reflexion se tiñe con el albedo.
    pub metallic: bool,
    pub texture: TextureKind,
}

impl Material {
    const fn base(name: &'static str, albedo: Vec3, texture: TextureKind) -> Self {
        Material {
            name,
            albedo,
            specular: 0.05,
            shininess: 8.0,
            reflectivity: 0.0,
            transparency: 0.0,
            ior: 1.0,
            emission: 0.0,
            metallic: false,
            texture,
        }
    }
}

/// Tabla indexada por id de material.
pub fn material_table() -> Vec<Material> {
    use TextureKind::*;
    let mut t = vec![Material::base("aire", vec3(0.0, 0.0, 0.0), None); MATERIAL_COUNT];

    // --- Los 5 materiales principales de la rubrica ---
    // 1. Piedra de la muralla: mate, casi sin brillo.
    t[STONE_WALL as usize] = Material { specular: 0.06, shininess: 10.0, ..Material::base("piedra muralla", vec3(0.66, 0.60, 0.50), WallBricks) };
    // 2. Tejas vidriadas: brillo moderado y un poco de reflejo.
    t[ROOF_TILE as usize] = Material { specular: 0.45, shininess: 32.0, reflectivity: 0.06, ..Material::base("tejas", vec3(0.54, 0.17, 0.11), Tiles) };
    // 3. Cesped: completamente difuso.
    t[GRASS as usize] = Material { specular: 0.02, ..Material::base("cesped", vec3(0.45, 0.56, 0.31), Grass) };
    // 4. Agua: transparente, refracta con IOR 1.33 y refleja segun Fresnel.
    t[WATER as usize] = Material {
        specular: 1.2,
        shininess: 180.0,
        reflectivity: 0.25,
        transparency: 0.9,
        ior: 1.33,
        ..Material::base("agua", vec3(0.10, 0.36, 0.42), Water)
    };
    // 5. Oro pulido: superficie altamente reflectiva.
    t[GOLD as usize] = Material {
        specular: 1.0,
        shininess: 120.0,
        reflectivity: 0.72,
        metallic: true,
        ..Material::base("oro", vec3(1.0, 0.76, 0.33), Metal)
    };

    // --- Materiales complementarios ---
    t[STONE_DARK as usize] = Material { specular: 0.05, ..Material::base("piedra base", vec3(0.43, 0.41, 0.38), BigBlocks) };
    t[PAVING as usize] = Material { specular: 0.12, shininess: 16.0, ..Material::base("losas", vec3(0.72, 0.68, 0.60), Paving) };
    t[DIRT as usize] = Material::base("tierra", vec3(0.46, 0.33, 0.21), Dirt);
    t[ROCK as usize] = Material { specular: 0.08, shininess: 12.0, ..Material::base("roca", vec3(0.52, 0.51, 0.49), Rock) };
    t[WOOD as usize] = Material { specular: 0.5, shininess: 40.0, reflectivity: 0.04, ..Material::base("madera lacada", vec3(0.56, 0.15, 0.10), Lacquer) };
    t[LEAVES as usize] = Material { specular: 0.04, ..Material::base("hojas", vec3(0.40, 0.53, 0.27), Leaves) };
    t[PINE as usize] = Material { specular: 0.04, ..Material::base("pino", vec3(0.20, 0.37, 0.24), Leaves) };
    t[SNOW as usize] = Material { specular: 0.25, shininess: 20.0, ..Material::base("nieve", vec3(0.93, 0.95, 0.98), Snow) };
    t[LANTERN as usize] = Material { emission: 3.0, ..Material::base("farol", vec3(1.0, 0.55, 0.18), Lantern) };
    t[FLAG as usize] = Material { specular: 0.1, ..Material::base("bandera", vec3(0.80, 0.12, 0.08), Cloth) };
    t[PATH as usize] = Material::base("camino", vec3(0.70, 0.60, 0.43), Sand);
    t[BASE as usize] = Material { specular: 0.3, shininess: 30.0, ..Material::base("base diorama", vec3(0.30, 0.19, 0.12), Planks) };
    t[TRUNK as usize] = Material::base("tronco", vec3(0.36, 0.23, 0.13), Bark);
    t[FLOWER as usize] = Material { specular: 0.1, ..Material::base("flores", vec3(1.0, 1.0, 1.0), Flower) };

    // Los colores se escriben en sRGB; el render trabaja en espacio lineal.
    for m in &mut t {
        m.albedo = m.albedo.map(|c| c.powf(2.2));
    }
    t
}

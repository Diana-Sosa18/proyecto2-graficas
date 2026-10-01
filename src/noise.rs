//! Ruido determinista (hash + value noise + fBm) usado por el terreno,
//! las texturas procedurales y las nubes del skybox.

#[inline]
fn mix32(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x7FEB_352D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x846C_A68B);
    h ^= h >> 16;
    h
}

/// Valor pseudoaleatorio en [0, 1) para una celda 2D.
#[inline]
pub fn hash2(x: i32, z: i32, seed: u32) -> f32 {
    let h = mix32((x as u32).wrapping_mul(0x8DA6_B343) ^ (z as u32).wrapping_mul(0xD816_3841) ^ seed.wrapping_mul(0xCB1A_B31F));
    (h >> 8) as f32 / 16_777_216.0
}

/// Valor pseudoaleatorio en [0, 1) para una celda 3D.
#[inline]
pub fn hash3(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let h = mix32(
        (x as u32).wrapping_mul(0x8DA6_B343)
            ^ (y as u32).wrapping_mul(0xD816_3841)
            ^ (z as u32).wrapping_mul(0xCB1A_B31F)
            ^ seed.wrapping_mul(0x1656_67B1),
    );
    (h >> 8) as f32 / 16_777_216.0
}

#[inline]
fn fade(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

#[inline]
fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// Value noise 2D suavizado, en [0, 1].
pub fn value_noise2(x: f32, z: f32, seed: u32) -> f32 {
    let (xi, zi) = (x.floor() as i32, z.floor() as i32);
    let (tx, tz) = (fade(x - xi as f32), fade(z - zi as f32));
    let a = hash2(xi, zi, seed);
    let b = hash2(xi + 1, zi, seed);
    let c = hash2(xi, zi + 1, seed);
    let d = hash2(xi + 1, zi + 1, seed);
    lerp(lerp(a, b, tx), lerp(c, d, tx), tz)
}

/// Value noise 3D suavizado, en [0, 1].
pub fn value_noise3(x: f32, y: f32, z: f32, seed: u32) -> f32 {
    let (xi, yi, zi) = (x.floor() as i32, y.floor() as i32, z.floor() as i32);
    let (tx, ty, tz) = (fade(x - xi as f32), fade(y - yi as f32), fade(z - zi as f32));
    let h = |dx, dy, dz| hash3(xi + dx, yi + dy, zi + dz, seed);
    let x00 = lerp(h(0, 0, 0), h(1, 0, 0), tx);
    let x10 = lerp(h(0, 1, 0), h(1, 1, 0), tx);
    let x01 = lerp(h(0, 0, 1), h(1, 0, 1), tx);
    let x11 = lerp(h(0, 1, 1), h(1, 1, 1), tx);
    lerp(lerp(x00, x10, ty), lerp(x01, x11, ty), tz)
}

/// Suma fractal de octavas (fBm) normalizada a [0, 1].
pub fn fbm2(x: f32, z: f32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for i in 0..octaves {
        sum += amp * value_noise2(x * freq, z * freq, seed.wrapping_add(i * 101));
        norm += amp;
        amp *= 0.5;
        freq *= 2.03;
    }
    sum / norm
}

pub fn fbm3(x: f32, y: f32, z: f32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for i in 0..octaves {
        sum += amp * value_noise3(x * freq, y * freq, z * freq, seed.wrapping_add(i * 101));
        norm += amp;
        amp *= 0.5;
        freq *= 2.03;
    }
    sum / norm
}

pub fn smoothstep(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

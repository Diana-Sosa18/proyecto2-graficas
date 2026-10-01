//! Skybox procedural: un cubemap de 6 caras generado al iniciar (degradado,
//! nubes, cordilleras lejanas y resplandor solar) y muestreado por direccion.

use crate::noise::{fbm2, smoothstep};
use nalgebra_glm::{vec3, Vec3};

const FACE_SIZE: usize = 256;

/// Cada cara se define por su eje principal M y los ejes U, V de la imagen:
/// direccion = M + a*U + b*V, con a, b en [-1, 1].
const FACES: [([f32; 3], [f32; 3], [f32; 3]); 6] = [
    ([1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0]),  // +X
    ([-1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0]),  // -X
    ([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]),  // +Y
    ([0.0, -1.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]),  // -Y
    ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),   // +Z
    ([0.0, 0.0, -1.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0]), // -Z
];

fn v(a: [f32; 3]) -> Vec3 {
    vec3(a[0], a[1], a[2])
}

pub struct Skybox {
    faces: Vec<Vec<Vec3>>,
    sun_dir: Vec3,
    sun_color: Vec3,
}

impl Skybox {
    pub fn new(sun_dir: Vec3, sun_color: Vec3) -> Self {
        let faces = std::thread::scope(|s| {
            let handles: Vec<_> = FACES
                .iter()
                .map(|&(m, u, w)| {
                    s.spawn(move || {
                        let mut img = Vec::with_capacity(FACE_SIZE * FACE_SIZE);
                        for j in 0..FACE_SIZE {
                            for i in 0..FACE_SIZE {
                                let a = (i as f32 + 0.5) / FACE_SIZE as f32 * 2.0 - 1.0;
                                let b = (j as f32 + 0.5) / FACE_SIZE as f32 * 2.0 - 1.0;
                                let dir = (v(m) + v(u) * a + v(w) * b).normalize();
                                img.push(procedural_sky(dir, sun_dir));
                            }
                        }
                        img
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });
        Skybox { faces, sun_dir, sun_color }
    }

    /// Color del cielo en la direccion `dir` (normalizada).
    pub fn sample(&self, dir: Vec3) -> Vec3 {
        let ax = dir.abs();
        let face = if ax.x >= ax.y && ax.x >= ax.z {
            if dir.x > 0.0 { 0 } else { 1 }
        } else if ax.y >= ax.z {
            if dir.y > 0.0 { 2 } else { 3 }
        } else if dir.z > 0.0 {
            4
        } else {
            5
        };
        let (m, u, w) = FACES[face];
        let major = dir.dot(&v(m));
        let a = dir.dot(&v(u)) / major;
        let b = dir.dot(&v(w)) / major;
        let mut color = self.bilinear(face, a, b);

        // Disco solar analitico para que el borde se vea nitido.
        let cos_sun = dir.dot(&self.sun_dir);
        if cos_sun > 0.9994 {
            color += self.sun_color * (6.0 * smoothstep(0.9994, 0.9997, cos_sun));
        }
        color
    }

    fn bilinear(&self, face: usize, a: f32, b: f32) -> Vec3 {
        let n = FACE_SIZE as f32;
        let x = ((a + 1.0) * 0.5 * n - 0.5).clamp(0.0, n - 1.0);
        let y = ((b + 1.0) * 0.5 * n - 0.5).clamp(0.0, n - 1.0);
        let (x0, y0) = (x as usize, y as usize);
        let (x1, y1) = ((x0 + 1).min(FACE_SIZE - 1), (y0 + 1).min(FACE_SIZE - 1));
        let (fx, fy) = (x - x0 as f32, y - y0 as f32);
        let img = &self.faces[face];
        let p = |xx: usize, yy: usize| img[yy * FACE_SIZE + xx];
        let top = p(x0, y0) * (1.0 - fx) + p(x1, y0) * fx;
        let bottom = p(x0, y1) * (1.0 - fx) + p(x1, y1) * fx;
        top * (1.0 - fy) + bottom * fy
    }
}

/// Cielo procedural evaluado una sola vez por texel del cubemap.
fn procedural_sky(dir: Vec3, sun_dir: Vec3) -> Vec3 {
    let zenith = vec3(0.10, 0.25, 0.62);
    let horizon = vec3(0.62, 0.70, 0.80);
    let haze = vec3(0.70, 0.66, 0.60);
    let y = dir.y;

    let mut color = if y >= 0.0 {
        let t = y.powf(0.45);
        horizon * (1.0 - t) + zenith * t
    } else {
        // Bajo el horizonte: bruma que se oscurece hacia abajo.
        let t = (-y).powf(0.6);
        horizon * (1.0 - t) + vec3(0.20, 0.24, 0.30) * t
    };

    // Resplandor alrededor del sol.
    let cos_sun = dir.dot(&sun_dir).max(0.0);
    color += vec3(1.0, 0.78, 0.50) * (cos_sun.powf(8.0) * 0.35 + cos_sun.powf(64.0) * 0.6);

    // Nubes: fBm proyectado sobre un plano a cierta altura.
    if y > 0.01 {
        let t = 1.0 / y;
        let (px, pz) = (dir.x * t * 0.9, dir.z * t * 0.9);
        let n = fbm2(px * 1.2 + 10.0, pz * 1.2, 5, 1337);
        let cover = smoothstep(0.48, 0.72, n) * smoothstep(0.02, 0.22, y);
        if cover > 0.0 {
            let shade = fbm2(px * 1.2 + 10.3, pz * 1.2 + 0.2, 4, 1337);
            let lit = 0.75 + 0.6 * (n - shade).max(-0.2) + 0.25 * cos_sun;
            let cloud = vec3(1.0, 0.97, 0.94) * lit;
            color = color * (1.0 - cover * 0.9) + cloud * (cover * 0.9);
        }
    }

    // Cordilleras lejanas en el horizonte (dos capas con bruma).
    let phi = dir.z.atan2(dir.x);
    let (c, s) = (phi.cos(), phi.sin());
    let far = 0.035 + 0.11 * fbm2(c * 2.2 + 5.0, s * 2.2, 5, 404).powf(1.6) * 1.6;
    let near = 0.005 + 0.07 * fbm2(c * 3.5 - 7.0, s * 3.5, 5, 909).powf(1.4) * 1.5;
    let edge = 0.006; // borde suave para evitar escalones en el cubemap
    let far_a = smoothstep(far + edge, far - edge, y);
    if far_a > 0.0 {
        let mist = smoothstep(far - 0.12, far, y);
        let mountain = vec3(0.36, 0.44, 0.56) * (1.0 - mist) + haze * 0.85 * mist;
        color = color * (1.0 - 0.85 * far_a) + mountain * (0.85 * far_a);
    }
    let near_a = smoothstep(near + edge, near - edge, y);
    if near_a > 0.0 {
        let mist = smoothstep(near - 0.1, near, y) * 0.5;
        let mountain = vec3(0.20, 0.28, 0.30) * (1.0 - mist) + vec3(0.45, 0.52, 0.60) * mist;
        let below = smoothstep(0.0, -0.35, y);
        let ground = mountain * (1.0 - below) + vec3(0.12, 0.15, 0.18) * below;
        color = color * (1.0 - near_a) + ground * near_a;
    }
    color
}

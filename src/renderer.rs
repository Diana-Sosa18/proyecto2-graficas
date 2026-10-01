use crate::camera::Camera;
use crate::raytracer::{self, Scene};
use nalgebra_glm::Vec3;
use std::sync::Mutex;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer { width, height, pixels: vec![0; width * height] }
    }

    /// Copia este buffer a `dst` escalando por vecino mas cercano.
    pub fn upscale_into(&self, dst: &mut Framebuffer) {
        for y in 0..dst.height {
            let sy = y * self.height / dst.height;
            let src_row = &self.pixels[sy * self.width..(sy + 1) * self.width];
            let dst_row = &mut dst.pixels[y * dst.width..(y + 1) * dst.width];
            for (x, p) in dst_row.iter_mut().enumerate() {
                *p = src_row[x * self.width / dst.width];
            }
        }
    }
}

/// Convierte color lineal HDR a 0x00RRGGBB con tone mapping filmico (ACES,
/// aproximacion de Narkowicz) y correccion gamma.
pub fn color_to_u32(c: Vec3) -> u32 {
    let map = |v: f32| {
        let v = v.max(0.0) * 0.9;
        let v = (v * (2.51 * v + 0.03)) / (v * (2.43 * v + 0.59) + 0.14);
        (v.clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0 + 0.5) as u32
    };
    (map(c.x) << 16) | (map(c.y) << 8) | map(c.z)
}

/// Filas por banda de trabajo: bandas pequenas balancean mejor la carga.
const ROWS_PER_BAND: usize = 4;

/// Renderiza en paralelo con `std::thread::scope`. El framebuffer se divide
/// en bandas de filas; cada hilo toma la siguiente banda libre de una cola
/// protegida por Mutex, asi ningun hilo se queda ocioso.
pub fn render(fb: &mut Framebuffer, scene: &Scene, camera: &Camera) {
    let frame = camera.frame();
    let (w, h) = (fb.width, fb.height);
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let bands = Mutex::new(fb.pixels.chunks_mut(w * ROWS_PER_BAND).enumerate());

    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let next = bands.lock().unwrap().next();
                let Some((band, pixels)) = next else { break };
                let y0 = band * ROWS_PER_BAND;
                for (i, px) in pixels.iter_mut().enumerate() {
                    let (x, y) = (i % w, y0 + i / w);
                    let ray = frame.primary_ray(x as f32 + 0.5, y as f32 + 0.5, w, h);
                    *px = color_to_u32(raytracer::trace(scene, &ray));
                }
            });
        }
    });
}

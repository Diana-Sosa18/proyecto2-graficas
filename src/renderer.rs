use crate::camera::Camera;
use crate::raytracer::{self, Scene};
use nalgebra_glm::Vec3;
use std::sync::Mutex;

/// Posiciones de muestreo dentro del pixel (antialiasing progresivo).
const JITTER: [(f32, f32); 5] = [(0.5, 0.5), (0.25, 0.25), (0.75, 0.25), (0.25, 0.75), (0.75, 0.75)];
pub const MAX_SAMPLES: u32 = JITTER.len() as u32;

/// Filas por banda de trabajo: bandas pequenas balancean mejor la carga.
const ROWS_PER_BAND: usize = 4;

pub struct Framebuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
    /// Suma de colores lineales de todas las muestras de cada pixel.
    accum: Vec<Vec3>,
    /// Muestras acumuladas por pixel; 0 = hay que renderizar de nuevo.
    pub samples: u32,
}

impl Framebuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Framebuffer {
            width,
            height,
            pixels: vec![0; width * height],
            accum: vec![Vec3::zeros(); width * height],
            samples: 0,
        }
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
        dst.samples = 0;
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

/// Renderiza un cuadro nuevo (una muestra por pixel).
pub fn render(fb: &mut Framebuffer, scene: &Scene, camera: &Camera) {
    fb.samples = 0;
    add_sample(fb, scene, camera);
}

/// Renderiza con todas las muestras de antialiasing (exportacion de imagenes).
pub fn render_full_quality(fb: &mut Framebuffer, scene: &Scene, camera: &Camera) {
    render(fb, scene, camera);
    while add_sample(fb, scene, camera) {}
}

/// Agrega una muestra desplazada a cada pixel y actualiza la imagen con el
/// promedio. Devuelve false si ya se alcanzo MAX_SAMPLES.
///
/// El trabajo se reparte con `std::thread::scope`: el framebuffer se divide en
/// bandas de filas y cada hilo toma la siguiente banda libre de una cola
/// protegida por Mutex, asi ningun hilo se queda ocioso.
pub fn add_sample(fb: &mut Framebuffer, scene: &Scene, camera: &Camera) -> bool {
    if fb.samples >= MAX_SAMPLES {
        return false;
    }
    let frame = camera.frame();
    let (w, h) = (fb.width, fb.height);
    let index = fb.samples;
    let (jx, jy) = JITTER[index as usize];
    let inv = 1.0 / (index + 1) as f32;
    let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let band = w * ROWS_PER_BAND;
    let bands = Mutex::new(fb.pixels.chunks_mut(band).zip(fb.accum.chunks_mut(band)).enumerate());

    std::thread::scope(|s| {
        for _ in 0..threads {
            s.spawn(|| loop {
                let next = bands.lock().unwrap().next();
                let Some((b, (pixels, accum))) = next else { break };
                let y0 = b * ROWS_PER_BAND;
                for (i, (px, acc)) in pixels.iter_mut().zip(accum.iter_mut()).enumerate() {
                    let (x, y) = (i % w, y0 + i / w);
                    let ray = frame.primary_ray(x as f32 + jx, y as f32 + jy, w, h);
                    let color = raytracer::trace(scene, &ray);
                    *acc = if index == 0 { color } else { *acc + color };
                    *px = color_to_u32(*acc * inv);
                }
            });
        }
    });
    fb.samples += 1;
    true
}

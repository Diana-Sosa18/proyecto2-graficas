use crate::camera::Camera;
use crate::raytracer;
use nalgebra_glm::Vec3;

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

/// Convierte color lineal HDR a 0x00RRGGBB con tone mapping y correccion gamma.
pub fn color_to_u32(c: Vec3) -> u32 {
    let map = |v: f32| {
        let v = v.max(0.0);
        let v = v / (1.0 + v * 0.35); // tone mapping suave tipo Reinhard
        (v.powf(1.0 / 2.2).min(1.0) * 255.0 + 0.5) as u32
    };
    (map(c.x) << 16) | (map(c.y) << 8) | map(c.z)
}

pub fn render(fb: &mut Framebuffer, camera: &Camera) {
    let frame = camera.frame();
    let (w, h) = (fb.width, fb.height);
    for y in 0..h {
        for x in 0..w {
            let ray = frame.primary_ray(x as f32 + 0.5, y as f32 + 0.5, w, h);
            fb.pixels[y * w + x] = color_to_u32(raytracer::trace(&ray));
        }
    }
}

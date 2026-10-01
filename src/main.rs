mod camera;
mod image_export;
mod input;
mod ray;
mod raytracer;
mod renderer;
mod scene_wall;
mod texture;
mod voxel;
mod voxel_grid;
mod material;
mod noise;

use camera::Camera;
use minifb::{Key, Window, WindowOptions};
use nalgebra_glm::vec3;
use raytracer::Scene;
use renderer::Framebuffer;
use std::path::Path;
use std::time::Instant;

const WIDTH: usize = 640;
const HEIGHT: usize = 480;
const PREVIEW_DIV: usize = 2; // 320x240 mientras la camara se mueve

fn default_camera() -> Camera {
    Camera::new(vec3(0.0, 16.0, 4.0), 0.55, 0.5, 235.0)
}

fn build_scene() -> Scene {
    let start = Instant::now();
    let grid = scene_wall::build_scene();
    println!("Escena construida en {:.2?}", start.elapsed());
    Scene { grid, materials: material::material_table() }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--render") {
        let out = args.get(i + 1).map(String::as_str).unwrap_or("render.png");
        let mut camera = default_camera();
        if let Some(v) = args.iter().position(|a| a == "--view").and_then(|j| args.get(j + 1)) {
            let p: Vec<f32> = v.split(',').filter_map(|n| n.trim().parse().ok()).collect();
            if p.len() == 3 {
                camera.set_orbit(p[0], p[1], p[2]);
            }
        }
        render_to_file(Path::new(out), &camera);
        return;
    }
    run_window();
}

/// Renderiza un cuadro sin abrir ventana (util para pruebas y capturas).
fn render_to_file(path: &Path, camera: &Camera) {
    let scene = build_scene();
    let mut fb = Framebuffer::new(WIDTH, HEIGHT);
    let start = Instant::now();
    renderer::render(&mut fb, &scene, camera);
    println!("Render {}x{} en {:.2?}", WIDTH, HEIGHT, start.elapsed());
    image_export::save_png(path, fb.width, fb.height, &fb.pixels).expect("no se pudo guardar el PNG");
    println!("Guardado en {}", path.display());
}

fn run_window() {
    let mut window = Window::new(
        "Gran Muralla China - Diorama Raytracing",
        WIDTH,
        HEIGHT,
        WindowOptions::default(),
    )
    .expect("no se pudo crear la ventana");
    window.set_target_fps(60);

    let scene = build_scene();
    let mut camera = default_camera();
    let mut full = Framebuffer::new(WIDTH, HEIGHT);
    let mut preview = Framebuffer::new(WIDTH / PREVIEW_DIV, HEIGHT / PREVIEW_DIV);
    let mut full_is_current = false;
    let mut last = Instant::now();
    let mut screenshot_count = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.1);
        last = now;

        let actions = input::handle_input(&window, &mut camera, dt);
        let moving = camera.update(dt);

        let start = Instant::now();
        if moving {
            renderer::render(&mut preview, &scene, &camera);
            preview.upscale_into(&mut full);
            full_is_current = false;
        } else if !full_is_current {
            renderer::render(&mut full, &scene, &camera);
            full_is_current = true;
        }
        let ms = start.elapsed().as_secs_f32() * 1000.0;
        if moving || ms > 1.0 {
            window.set_title(&format!(
                "Gran Muralla China - Raytracing | {} | {:.0} ms",
                if moving { "preview 320x240" } else { "640x480" },
                ms
            ));
        }

        if actions.screenshot {
            let name = format!("captura_{:03}.png", screenshot_count);
            screenshot_count += 1;
            match image_export::save_png(Path::new(&name), full.width, full.height, &full.pixels) {
                Ok(()) => println!("Captura guardada: {name}"),
                Err(e) => eprintln!("Error al guardar captura: {e}"),
            }
        }

        window
            .update_with_buffer(&full.pixels, WIDTH, HEIGHT)
            .expect("no se pudo actualizar la ventana");
    }
}

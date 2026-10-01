mod camera;
mod image_export;
mod input;
mod lighting;
mod material;
mod noise;
mod ray;
mod raytracer;
mod renderer;
mod scene_wall;
mod skybox;
mod texture;
mod voxel;
mod voxel_grid;

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
const TITLE: &str = "Gran Muralla China - Diorama Raytracing";

fn default_camera() -> Camera {
    Camera::new(vec3(0.0, 16.0, 4.0), 0.55, 0.48, 215.0)
}

fn build_scene() -> Scene {
    let start = Instant::now();
    let grid = scene_wall::build_scene();
    let lighting = lighting::Lighting::default();
    let skybox = skybox::Skybox::new(lighting.sun_dir, lighting.sun_color);
    println!(
        "Escena {}x{}x{} voxeles y skybox listos en {:.2?}",
        grid.nx,
        grid.ny,
        grid.nz,
        start.elapsed()
    );
    Scene { grid, materials: material::material_table(), lighting, skybox }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg_after = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();

    let mut camera = default_camera();
    if let Some(v) = arg_after("--view") {
        // --view yaw,pitch,distancia[,objetivo_x,objetivo_y,objetivo_z]
        let p: Vec<f32> = v.split(',').filter_map(|n| n.trim().parse().ok()).collect();
        if p.len() >= 6 {
            camera.target = vec3(p[3], p[4], p[5]);
        }
        if p.len() >= 3 {
            camera.set_orbit(p[0], p[1], p[2]);
        }
    }

    if args.iter().any(|a| a == "--bench") {
        bench();
    } else if args.iter().any(|a| a == "--render") {
        let out = arg_after("--render").filter(|s| !s.starts_with("--")).unwrap_or("render.png".into());
        render_to_file(Path::new(&out), &camera);
    } else if args.iter().any(|a| a == "--export-frames") {
        let count = arg_after("--export-frames").and_then(|s| s.parse().ok()).unwrap_or(240);
        export_frames(count, &mut camera);
    } else {
        run_window();
    }
}

/// Renderiza un cuadro sin abrir ventana (util para pruebas y capturas).
fn render_to_file(path: &Path, camera: &Camera) {
    let scene = build_scene();
    let mut fb = Framebuffer::new(WIDTH, HEIGHT);
    let start = Instant::now();
    renderer::render_full_quality(&mut fb, &scene, camera);
    println!("Render {}x{} ({} muestras/pixel) en {:.2?}", WIDTH, HEIGHT, fb.samples, start.elapsed());
    image_export::save_png(path, fb.width, fb.height, &fb.pixels).expect("no se pudo guardar el PNG");
    println!("Guardado en {}", path.display());
}

/// Exporta una orbita completa alrededor del diorama como PNG numerados,
/// listos para unirse en un video (por ejemplo con ffmpeg).
fn export_frames(count: usize, camera: &mut Camera) {
    let scene = build_scene();
    let dir = Path::new("frames");
    std::fs::create_dir_all(dir).expect("no se pudo crear la carpeta frames");
    let mut fb = Framebuffer::new(WIDTH, HEIGHT);
    let (yaw0, pitch, dist) = (camera.yaw, camera.pitch, camera.distance);
    let start = Instant::now();
    for i in 0..count {
        let t = i as f32 / count as f32;
        let yaw = yaw0 + t * std::f32::consts::TAU;
        // Leve vaiven de altura y zoom para que el recorrido sea mas dinamico.
        let wave = (t * std::f32::consts::TAU * 2.0).sin();
        camera.set_orbit(yaw, pitch + 0.08 * wave, dist * (1.0 - 0.12 * wave));
        renderer::render_full_quality(&mut fb, &scene, camera);
        let path = dir.join(format!("frame_{i:04}.png"));
        image_export::save_png(&path, fb.width, fb.height, &fb.pixels).expect("no se pudo guardar el cuadro");
        println!("{}/{} {}", i + 1, count, path.display());
    }
    println!("Exportados {count} cuadros en {:.1?}", start.elapsed());
    println!("Video: ffmpeg -framerate 30 -i frames/frame_%04d.png -pix_fmt yuv420p diorama.mp4");
}

/// Mide el tiempo promedio de render en 12 vistas alrededor del diorama.
fn bench() {
    let scene = build_scene();
    let mut camera = default_camera();
    let mut fb = Framebuffer::new(WIDTH, HEIGHT);
    let mut total = 0.0;
    let views = 12;
    for i in 0..views {
        let yaw = i as f32 / views as f32 * std::f32::consts::TAU;
        camera.set_orbit(yaw, 0.35 + 0.25 * (i % 3) as f32, 140.0 + 40.0 * (i % 2) as f32);
        let start = Instant::now();
        renderer::render(&mut fb, &scene, &camera);
        total += start.elapsed().as_secs_f64() * 1000.0;
    }
    println!("Promedio {}x{}: {:.1} ms por cuadro ({} vistas)", WIDTH, HEIGHT, total / views as f64, views);
}

fn print_help(scene: &Scene) {
    println!("\nMateriales:");
    for (id, m) in scene.materials.iter().enumerate().skip(1) {
        println!(
            "  {id:2} {:<15} specular {:.2}  reflectividad {:.2}  transparencia {:.2}  ior {:.2}",
            m.name, m.specular, m.reflectivity, m.transparency, m.ior
        );
    }
    println!("\nControles:");
    println!("  A / D o flechas     rotar alrededor del diorama");
    println!("  W / S o flechas     inclinar la camara");
    println!("  + / - , Q / E, rueda zoom");
    println!("  R                   reiniciar camara");
    println!("  P                   guardar captura PNG");
    println!("  ESC                 salir\n");
}

fn run_window() {
    let scene = build_scene();
    print_help(&scene);

    let mut window = Window::new(TITLE, WIDTH, HEIGHT, WindowOptions::default()).expect("no se pudo crear la ventana");
    window.set_target_fps(60);

    let mut camera = default_camera();
    let mut full = Framebuffer::new(WIDTH, HEIGHT);
    let mut preview = Framebuffer::new(WIDTH / PREVIEW_DIV, HEIGHT / PREVIEW_DIV);
    let mut last = Instant::now();
    let mut screenshot_count = 0;

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.1);
        last = now;

        let actions = input::handle_input(&window, &mut camera, dt);
        if actions.reset_camera {
            camera = default_camera();
            full.samples = 0;
        }
        let moving = camera.update(dt);

        // En movimiento: preview a media resolucion. Quieta: resolucion completa
        // y una muestra extra de antialiasing por cuadro hasta MAX_SAMPLES.
        let start = Instant::now();
        let mut rendered = true;
        if moving {
            renderer::render(&mut preview, &scene, &camera);
            preview.upscale_into(&mut full);
        } else if full.samples == 0 {
            renderer::render(&mut full, &scene, &camera);
        } else {
            rendered = renderer::add_sample(&mut full, &scene, &camera);
        }
        if rendered {
            let ms = start.elapsed().as_secs_f32() * 1000.0;
            let mode = if moving {
                format!("preview {}x{}", WIDTH / PREVIEW_DIV, HEIGHT / PREVIEW_DIV)
            } else {
                format!("{WIDTH}x{HEIGHT} AA {}/{}", full.samples, renderer::MAX_SAMPLES)
            };
            window.set_title(&format!("{TITLE} | {mode} | {ms:.0} ms"));
        }

        if actions.screenshot {
            let name = format!("captura_{screenshot_count:03}.png");
            screenshot_count += 1;
            match image_export::save_png(Path::new(&name), full.width, full.height, &full.pixels) {
                Ok(()) => println!("Captura guardada: {name}"),
                Err(e) => eprintln!("Error al guardar captura: {e}"),
            }
        }

        window.update_with_buffer(&full.pixels, WIDTH, HEIGHT).expect("no se pudo actualizar la ventana");
    }
}

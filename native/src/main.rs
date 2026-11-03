mod assets;
mod gpu;
mod mesh;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use image::RgbaImage;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};

use assets::{load_image_path, noise_tile, Catalog};
use gpu::{save_png, FrameInput, Gpu, ViewModel};
use mesh::Mesh;

const HEIGHTMAPS: &[(&str, &str)] = &[
    ("Planeta", "planet.png"),
    ("Relieves", "bumps.png"),
    ("Sonrisa", "smile.png"),
    ("Mano", "hand.png"),
    ("Onda", "ripple.png"),
    ("Cabeza", "head.png"),
    ("Atomium", "atomium.png"),
    ("Auto", "car.png"),
    ("Delfín", "dolphin.png"),
    ("Árbol", "tree.png"),
    ("Barco", "ship.png"),
];

const TILES: &[(&str, &str)] = &[
    ("Empapelado", "wallpaper.png"),
    ("Espacio", "space.png"),
    ("Follaje", "foliage.png"),
    ("Jirafa", "giraffe.png"),
    ("Piedras", "stones.png"),
    ("Hojas", "leaves.png"),
];

const MAX_CANVAS_CSS_PX: f32 = 512.0;

const MODELS: &[(&str, &str, [f32; 3], f32, f32)] = &[
    ("Primitivas", "primitives.obj", [-1.3, 0.0, -0.54], 0.6, 2.2),
    ("Cubo", "cube.obj", [-2.0, 0.0, -1.0], 1.3, 3.0),
    ("Mono", "monkey.obj", [-1.3, 0.0, -0.5], 0.7, 2.0),
    ("Conejo", "bunny.obj", [-1.7, 0.0, -0.8], 0.9, 2.3),
    ("Atomium", "atomium.obj", [-2.5, 0.0, -0.6], 1.5, 3.4),
];

struct Settings {
    heightmap_index: usize,
    custom_heightmap: bool,
    moving: bool,
    model_index: usize,
    custom_mesh: bool,
    tile_index: usize,
    custom_tile: bool,
    noise: bool,
    noise_resolution: u32,
    noise_square: bool,
    noise_colored: bool,
    depth: f32,
    invert: bool,
    show_heightmap: bool,
    show_uv: bool,
    centered_stripe: bool,
    adaptive_stripes: bool,
    stripe_width: f32,
    stripe_count: f32,
    message: String,
}

struct App {
    window: Option<Arc<Window>>,
    gpu: Option<Gpu>,
    surface: Option<wgpu::Surface<'static>>,
    config: Option<wgpu::SurfaceConfiguration>,
    egui: Option<EguiHost>,
    catalog: Catalog,
    settings: Settings,
    heightmap: RgbaImage,
    tile: RgbaImage,
    tile_padding: u32,
    mesh: Option<Mesh>,
    mesh_id: u64,
    started: Instant,
}

struct EguiHost {
    state: egui_winit::State,
    renderer: egui_wgpu::Renderer,
    context: egui::Context,
}

fn main() {
    let mut args = std::env::args().skip(1);
    if args.next().as_deref() == Some("--snapshot") {
        let path = args
            .next()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("estereograma.png"));
        if let Err(error) = snapshot(&path) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }

    let event_loop = EventLoop::new().expect("no pude crear la ventana");
    let mut app = App::new().unwrap_or_else(|error| {
        eprintln!("{error}");
        std::process::exit(1);
    });
    event_loop
        .run_app(&mut app)
        .expect("la ventana se cerró con un error");
}

fn snapshot(path: &std::path::Path) -> Result<(), String> {
    let catalog = Catalog::discover()?;
    let mut gpu = Gpu::headless()?;
    let heightmap = catalog.load_image("heightmaps", "planet.png")?;
    let tile = catalog.load_image("tiles", "space.png")?;
    let image = gpu.render_image(&FrameInput {
        width: 512,
        height: 512,
        heightmap: &heightmap,
        tile: &tile,
        tile_padding: 0,
        colored_tile: true,
        show_uv: false,
        show_heightmap: false,
        invert: false,
        depth: 1.0,
        main_stripe_centered: true,
        adaptive_stripes: true,
        stripe_width: 80.0,
        fixed_stripes: 12.0,
        moving: false,
        mesh: None,
        view: view_of(0),
        elapsed_ms: 0.0,
    })?;
    save_png(path, &image)
}

impl App {
    fn new() -> Result<Self, String> {
        let catalog = Catalog::discover()?;
        let heightmap = catalog.load_image("heightmaps", "planet.png")?;
        let tile = catalog.load_image("tiles", "space.png")?;
        Ok(Self {
            window: None,
            gpu: None,
            surface: None,
            config: None,
            egui: None,
            catalog,
            settings: Settings {
                heightmap_index: 0,
                custom_heightmap: false,
                moving: false,
                model_index: 0,
                custom_mesh: false,
                tile_index: 1,
                custom_tile: false,
                noise: false,
                noise_resolution: 16,
                noise_square: true,
                noise_colored: true,
                depth: 1.0,
                invert: false,
                show_heightmap: false,
                show_uv: false,
                centered_stripe: true,
                adaptive_stripes: true,
                stripe_width: 80.0,
                stripe_count: 16.0,
                message: String::new(),
            },
            heightmap,
            tile,
            tile_padding: 0,
            mesh: None,
            mesh_id: 0,
            started: Instant::now(),
        })
    }

    fn load_model(&mut self) {
        if self.settings.custom_mesh {
            return;
        }
        let file_name = MODELS[self.settings.model_index].1;
        match self.catalog.load_obj(file_name) {
            Ok(mesh) => {
                self.mesh_id = self.mesh_id.wrapping_add(1).max(1);
                if let Some(gpu) = self.gpu.as_mut() {
                    gpu.set_mesh(self.mesh_id, &mesh);
                }
                self.mesh = Some(mesh);
                self.settings.message.clear();
            }
            Err(error) => self.settings.message = error,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title("Estereograma")
                        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 800.0)),
                )
                .expect("no pude crear la ventana"),
        );
        let (gpu, surface, config, egui) = open_window(&window).unwrap_or_else(|error| {
            eprintln!("{error}");
            std::process::exit(1);
        });
        self.gpu = Some(gpu);
        self.surface = Some(surface);
        self.config = Some(config);
        self.egui = Some(egui);
        self.window = Some(window);
        if self.settings.moving {
            self.load_model();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(window) = self.window.clone() else {
            return;
        };
        let egui_wants_frame = self
            .egui
            .as_mut()
            .is_some_and(|egui| egui.state.on_window_event(&window, &event).repaint);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let (Some(surface), Some(config), Some(gpu)) = (
                    self.surface.as_ref(),
                    self.config.as_mut(),
                    self.gpu.as_ref(),
                ) {
                    if size.width > 0 && size.height > 0 {
                        config.width = size.width;
                        config.height = size.height;
                        surface.configure(gpu.device(), config);
                    }
                }
                window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                if let Err(error) = self.redraw() {
                    self.settings.message = error;
                }
                if self.settings.moving {
                    window.request_redraw();
                }
            }
            _ => {
                if egui_wants_frame {
                    window.request_redraw();
                }
            }
        }
    }
}

impl App {
    fn redraw(&mut self) -> Result<(), String> {
        let window = self.window.clone().ok_or("Todavía no hay ventana.")?;
        let App {
            gpu: Some(gpu),
            surface: Some(surface),
            config: Some(config),
            egui: Some(egui),
            catalog,
            settings,
            heightmap,
            tile,
            tile_padding,
            mesh,
            mesh_id,
            started,
            ..
        } = self
        else {
            return Err("La ventana todavía no está lista.".into());
        };

        let raw_input = egui.state.take_egui_input(&window);
        let full_output = egui.context.run(raw_input, |ui| {
            draw_ui(
                ui,
                settings,
                catalog,
                heightmap,
                tile,
                tile_padding,
                mesh,
                mesh_id,
                gpu,
            );
        });

        let paint_jobs = egui
            .context
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        let frame = surface
            .get_current_texture()
            .map_err(|error| format!("No pude presentar el cuadro: {error}"))?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = gpu
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("window"),
            });

        let scale = full_output.pixels_per_point;
        let panel = 320.0 * scale;
        let available_width = (config.width as f32 - panel).max(1.0);
        let available_height = (config.height as f32).max(1.0);
        // The web canvas is square and never larger than 512 CSS pixels.
        let canvas_css = (available_width / scale)
            .min(available_height / scale)
            .min(MAX_CANVAS_CSS_PX)
            .max(1.0);
        let canvas_px = (canvas_css * scale).floor().max(1.0);
        let origin_x = panel + (available_width - canvas_px) * 0.5;
        let origin_y = (available_height - canvas_px) * 0.5;
        {
            let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("clear"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.08,
                            g: 0.08,
                            b: 0.09,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        let input = FrameInput {
            width: canvas_px as u32,
            height: canvas_px as u32,
            heightmap,
            tile,
            tile_padding: *tile_padding,
            colored_tile: !settings.noise || settings.noise_colored,
            show_uv: settings.show_uv,
            show_heightmap: settings.show_heightmap,
            invert: settings.invert,
            depth: settings.depth,
            main_stripe_centered: settings.centered_stripe,
            adaptive_stripes: settings.adaptive_stripes,
            stripe_width: settings.stripe_width * scale,
            fixed_stripes: settings.stripe_count,
            moving: settings.moving,
            mesh: mesh.as_ref(),
            view: view_of(settings.model_index),
            elapsed_ms: started.elapsed().as_secs_f32() * 1000.0,
        };
        if let Some(mesh) = input.mesh {
            gpu.set_mesh(*mesh_id, mesh);
        }
        gpu.render_to_view(
            &input,
            &view,
            &mut encoder,
            (origin_x, origin_y, canvas_px, canvas_px),
        );

        for (id, image) in &full_output.textures_delta.set {
            egui.renderer
                .update_texture(gpu.device(), gpu.queue(), *id, image);
        }
        egui.renderer.update_buffers(
            gpu.device(),
            gpu.queue(),
            &mut encoder,
            &paint_jobs,
            &screen,
        );
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();
            egui.renderer.render(&mut pass, &paint_jobs, &screen);
        }
        for id in &full_output.textures_delta.free {
            egui.renderer.free_texture(id);
        }
        gpu.queue().submit(Some(encoder.finish()));
        frame.present();
        Ok(())
    }
}

fn draw_ui(
    ui_ctx: &egui::Context,
    settings: &mut Settings,
    catalog: &Catalog,
    heightmap: &mut RgbaImage,
    tile: &mut RgbaImage,
    tile_padding: &mut u32,
    mesh: &mut Option<Mesh>,
    mesh_id: &mut u64,
    gpu: &mut Gpu,
) {
    egui::SidePanel::left("controles")
        .exact_width(320.0)
        .show(ui_ctx, |ui| {
            ui.heading("Estereograma");
            ui.label("Mapa de profundidad");
            ui.checkbox(&mut settings.show_heightmap, "Mostrar escala de grises");
            ui.add(egui::Slider::new(&mut settings.depth, 0.0..=1.0).text("Profundidad"));
            ui.checkbox(&mut settings.invert, "Invertir");
            ui.horizontal(|ui| {
                if ui.selectable_label(!settings.moving, "Quieto").clicked() {
                    settings.moving = false;
                }
                if ui
                    .selectable_label(settings.moving, "En movimiento")
                    .clicked()
                {
                    settings.moving = true;
                    if mesh.is_none() {
                        load_selected_model(settings, catalog, mesh, mesh_id, gpu);
                    }
                }
            });

            if settings.moving {
                egui::ComboBox::from_label("Modelo")
                    .selected_text(if settings.custom_mesh {
                        "Personalizado"
                    } else {
                        MODELS[settings.model_index].0
                    })
                    .show_ui(ui, |ui| {
                        for (index, model) in MODELS.iter().enumerate() {
                            if ui
                                .selectable_label(
                                    index == settings.model_index && !settings.custom_mesh,
                                    model.0,
                                )
                                .clicked()
                            {
                                settings.model_index = index;
                                settings.custom_mesh = false;
                                load_selected_model(settings, catalog, mesh, mesh_id, gpu);
                            }
                        }
                    });
                if ui.button("Subir un STL").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("STL", &["stl"])
                        .pick_file()
                    {
                        match std::fs::read(&path)
                            .map_err(|error| error.to_string())
                            .and_then(|bytes| Mesh::from_stl(&bytes))
                        {
                            Ok(loaded) => {
                                *mesh_id = mesh_id.wrapping_add(1).max(1);
                                gpu.set_mesh(*mesh_id, &loaded);
                                *mesh = Some(loaded);
                                settings.custom_mesh = true;
                                settings.message.clear();
                            }
                            Err(error) => settings.message = error,
                        }
                    }
                }
            } else {
                egui::ComboBox::from_label("Predefinido")
                    .selected_text(if settings.custom_heightmap {
                        "Personalizado"
                    } else {
                        HEIGHTMAPS[settings.heightmap_index].0
                    })
                    .show_ui(ui, |ui| {
                        for (index, item) in HEIGHTMAPS.iter().enumerate() {
                            if ui
                                .selectable_label(
                                    index == settings.heightmap_index && !settings.custom_heightmap,
                                    item.0,
                                )
                                .clicked()
                            {
                                match catalog.load_image("heightmaps", item.1) {
                                    Ok(image) => {
                                        *heightmap = image;
                                        settings.heightmap_index = index;
                                        settings.custom_heightmap = false;
                                        settings.message.clear();
                                    }
                                    Err(error) => settings.message = error,
                                }
                            }
                        }
                    });
                if ui.button("Subir un mapa de profundidad").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Imagen", &["png", "jpg", "jpeg", "bmp", "webp"])
                        .pick_file()
                    {
                        match load_image_path(&path) {
                            Ok(image) => {
                                *heightmap = image;
                                settings.custom_heightmap = true;
                                settings.message.clear();
                            }
                            Err(error) => settings.message = error,
                        }
                    }
                }
            }

            ui.separator();
            ui.label("Patrón");
            ui.horizontal(|ui| {
                if ui.selectable_label(!settings.noise, "Textura").clicked() {
                    settings.noise = false;
                    if let Ok(image) = catalog.load_image("tiles", TILES[settings.tile_index].1) {
                        *tile = image;
                        *tile_padding = 0;
                    }
                }
                if ui.selectable_label(settings.noise, "Ruido").clicked() {
                    settings.noise = true;
                    let (image, padding) =
                        noise_tile(settings.noise_resolution, settings.noise_square);
                    *tile = image;
                    *tile_padding = padding;
                }
            });
            if settings.noise {
                let mut resolution = settings.noise_resolution as i32;
                if ui
                    .add(egui::Slider::new(&mut resolution, 8..=64).text("Resolución"))
                    .changed()
                {
                    settings.noise_resolution = resolution as u32;
                    let (image, padding) =
                        noise_tile(settings.noise_resolution, settings.noise_square);
                    *tile = image;
                    *tile_padding = padding;
                }
                if ui
                    .checkbox(&mut settings.noise_square, "Cuadrado")
                    .changed()
                {
                    let (image, padding) =
                        noise_tile(settings.noise_resolution, settings.noise_square);
                    *tile = image;
                    *tile_padding = padding;
                }
                ui.checkbox(&mut settings.noise_colored, "Color");
            } else {
                egui::ComboBox::from_label("Textura")
                    .selected_text(if settings.custom_tile {
                        "Personalizado"
                    } else {
                        TILES[settings.tile_index].0
                    })
                    .show_ui(ui, |ui| {
                        for (index, item) in TILES.iter().enumerate() {
                            if ui
                                .selectable_label(
                                    index == settings.tile_index && !settings.custom_tile,
                                    item.0,
                                )
                                .clicked()
                            {
                                match catalog.load_image("tiles", item.1) {
                                    Ok(image) => {
                                        *tile = image;
                                        *tile_padding = 0;
                                        settings.tile_index = index;
                                        settings.custom_tile = false;
                                    }
                                    Err(error) => settings.message = error,
                                }
                            }
                        }
                    });
                if ui.button("Subir un patrón").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Imagen", &["png", "jpg", "jpeg", "bmp", "webp"])
                        .pick_file()
                    {
                        match load_image_path(&path) {
                            Ok(image) => {
                                *tile = image;
                                *tile_padding = 0;
                                settings.custom_tile = true;
                            }
                            Err(error) => settings.message = error,
                        }
                    }
                }
            }

            ui.separator();
            ui.label("Franjas");
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(!settings.centered_stripe, "Izquierda")
                    .clicked()
                {
                    settings.centered_stripe = false;
                }
                if ui
                    .selectable_label(settings.centered_stripe, "Centro")
                    .clicked()
                {
                    settings.centered_stripe = true;
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(settings.adaptive_stripes, "Adaptativo")
                    .clicked()
                {
                    settings.adaptive_stripes = true;
                }
                if ui
                    .selectable_label(!settings.adaptive_stripes, "Fijo")
                    .clicked()
                {
                    settings.adaptive_stripes = false;
                }
            });
            if settings.adaptive_stripes {
                ui.add(
                    egui::Slider::new(&mut settings.stripe_width, 20.0..=200.0)
                        .text("Ancho de franja"),
                );
            } else {
                ui.add(
                    egui::Slider::new(&mut settings.stripe_count, 8.0..=24.0)
                        .text("Cantidad de franjas"),
                );
            }
            ui.checkbox(&mut settings.show_uv, "Mostrar UV");

            ui.separator();
            if ui.button("Descargar imagen").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PNG", &["png"])
                    .set_file_name("estereograma.png")
                    .save_file()
                {
                    match gpu
                        .render_image(&FrameInput {
                            width: MAX_CANVAS_CSS_PX as u32,
                            height: MAX_CANVAS_CSS_PX as u32,
                            heightmap,
                            tile,
                            tile_padding: *tile_padding,
                            colored_tile: !settings.noise || settings.noise_colored,
                            show_uv: settings.show_uv,
                            show_heightmap: settings.show_heightmap,
                            invert: settings.invert,
                            depth: settings.depth,
                            main_stripe_centered: settings.centered_stripe,
                            adaptive_stripes: settings.adaptive_stripes,
                            stripe_width: settings.stripe_width,
                            fixed_stripes: settings.stripe_count,
                            moving: settings.moving,
                            mesh: mesh.as_ref(),
                            view: view_of(settings.model_index),
                            elapsed_ms: 0.0,
                        })
                        .and_then(|image| save_png(&path, &image))
                    {
                        Ok(()) => settings.message = format!("Guardado en {}", path.display()),
                        Err(error) => settings.message = error,
                    }
                }
            }
            if !settings.message.is_empty() {
                ui.separator();
                ui.colored_label(egui::Color32::from_rgb(220, 120, 90), &settings.message);
            }
            ui.label(format!("Patrón {}x{}", tile.width(), tile.height()));
        });
}

fn load_selected_model(
    settings: &mut Settings,
    catalog: &Catalog,
    mesh: &mut Option<Mesh>,
    mesh_id: &mut u64,
    gpu: &mut Gpu,
) {
    match catalog.load_obj(MODELS[settings.model_index].1) {
        Ok(loaded) => {
            *mesh_id = mesh_id.wrapping_add(1).max(1);
            gpu.set_mesh(*mesh_id, &loaded);
            *mesh = Some(loaded);
            settings.custom_mesh = false;
            settings.message.clear();
        }
        Err(error) => settings.message = error,
    }
}

fn view_of(index: usize) -> ViewModel {
    let model = MODELS[index];
    ViewModel {
        eye: model.2,
        near: model.3,
        far: model.4,
    }
}

fn open_window(
    window: &Arc<Window>,
) -> Result<
    (
        Gpu,
        wgpu::Surface<'static>,
        wgpu::SurfaceConfiguration,
        EguiHost,
    ),
    String,
> {
    let (gpu, surface, config) = Gpu::with_window(window.clone())?;
    let context = egui::Context::default();
    let state = egui_winit::State::new(
        context.clone(),
        egui::ViewportId::ROOT,
        window.as_ref(),
        Some(window.scale_factor() as f32),
        None,
        None,
    );
    let renderer = egui_wgpu::Renderer::new(gpu.device(), config.format, None, 1, false);
    Ok((
        gpu,
        surface,
        config,
        EguiHost {
            state,
            renderer,
            context,
        },
    ))
}

use std::sync::Arc;

use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window, WindowAttributes};

use clear_ui::color;
use glyphon::{
    Attrs, Buffer, Cache, FontSystem, Metrics, Resolution, SwashCache, TextArea, TextAtlas,
    TextBounds, TextRenderer, Viewport,
};

use clear_system_interface::app::{AppAction, AppState, ContentButton, PageContent};
use clear_system_interface::pages::{self, Page};

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x4,
    ];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

fn quad_vertices(x: f32, y: f32, w: f32, h: f32, sw: f32, sh: f32, c: [f32; 4]) -> [Vertex; 6] {
    let x0 = (x / sw) * 2.0 - 1.0;
    let y0 = 1.0 - (y / sh) * 2.0;
    let x1 = ((x + w) / sw) * 2.0 - 1.0;
    let y1 = 1.0 - ((y + h) / sh) * 2.0;
    [
        Vertex { position: [x0, y0], color: c },
        Vertex { position: [x1, y0], color: c },
        Vertex { position: [x0, y1], color: c },
        Vertex { position: [x1, y0], color: c },
        Vertex { position: [x1, y1], color: c },
        Vertex { position: [x0, y1], color: c },
    ]
}

fn make_text_buffer(fs: &mut FontSystem, text: &str, size: f32) -> Buffer {
    let metrics = Metrics::new(size, size * 1.4);
    let mut buf = Buffer::new(fs, metrics);
    buf.set_text(fs, text, Attrs::new(), glyphon::Shaping::Advanced);
    buf.shape_until_scroll(fs, true);
    buf
}

struct AppWidget {
    x: f32, y: f32, w: f32, h: f32,
    color: [f32; 4],
    hover_color: [f32; 4],
    hovering: bool,
    kind: WidgetKind,
}

#[derive(Clone)]
enum WidgetKind {
    PageButton(Page),
    ActionButton(AppAction),
    Static,
}

struct TextItem {
    buffer: Buffer,
    x: f32, y: f32,
    color: glyphon::Color,
}

struct SystemInterface {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertex_count: u32,

    app: AppState,

    font_system: FontSystem,
    swash_cache: SwashCache,
    text_atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_viewport: Viewport,

    widgets: Vec<AppWidget>,
    text_items: Vec<TextItem>,
    page_buttons: Vec<ContentButton>,

    sidebar_width: f32,
    header_height: f32,
    status_height: f32,

    cursor_x: f32,
    cursor_y: f32,

    rx_power: std::sync::mpsc::Receiver<pages::power::PowerState>,
    rx_audio: std::sync::mpsc::Receiver<pages::audio::AudioState>,
    rx_display: std::sync::mpsc::Receiver<pages::display::DisplayState>,
    rx_network: std::sync::mpsc::Receiver<pages::network::NetworkState>,
    rx_layout: std::sync::mpsc::Receiver<pages::layout::LayoutState>,
    rx_input: std::sync::mpsc::Receiver<pages::input::InputState>,
    rx_system: std::sync::mpsc::Receiver<pages::system_info::SystemState>,
    rx_status: std::sync::mpsc::Receiver<pages::status::StatusState>,
    rx_storage: std::sync::mpsc::Receiver<pages::storage::StorageState>,

    scale_factor: f64,
    width: u32,
    height: u32,
    needs_rebuild: bool,
}

impl SystemInterface {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let sw = size.width as f32;
        let sh = size.height as f32;

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let surface = instance.create_surface(window.clone()).expect("surface");
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.expect("adapter");
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("GPU Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
        }, None).await.expect("device");
        let config = surface.get_default_config(&adapter, size.width.max(1), size.height.max(1)).expect("config");
        surface.configure(&device, &config);

        let shader_code = clear_ui::SHADER;
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(shader_code)),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState { count: 1, mask: !0, alpha_to_coverage_enabled: false },
            multiview: None,
            cache: None,
        });

        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let mut text_atlas = TextAtlas::new(&device, &queue, &cache, config.format);
        let text_renderer = TextRenderer::new(&mut text_atlas, &device, wgpu::MultisampleState::default(), None);
        let mut text_viewport = Viewport::new(&device, &cache);
        text_viewport.update(&queue, Resolution { width: size.width, height: size.height });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 1,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // ── Initial state ──
        let app = AppState {
            power: pages::power::fetch_power_state().await,
            audio: pages::audio::fetch_audio_state().await,
            display: pages::display::fetch_display_state().await,
            network: pages::network::fetch_network_state().await,
            layout: pages::layout::read_layout_config(),
            input: pages::input::read_input_config(),
            system_info: pages::system_info::fetch_system_state().await,
            status: pages::status::fetch_status_state().await,
            storage: pages::storage::fetch_storage_state().await,
            current_page: Page::ALL[0],
        };

        // ── Background refresh channels ──
        fn spawn_bg<T, F>(period_secs: u64, f: fn() -> F) -> std::sync::mpsc::Receiver<T>
        where
            T: Send + 'static,
            F: std::future::Future<Output = T> + Send + 'static,
        {
            let (tx, rx) = std::sync::mpsc::channel::<T>();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(period_secs)).await;
                    let val = f().await;
                    if tx.send(val).is_err() { break; }
                }
            });
            rx
        }

        let rx_power = spawn_bg(5, || pages::power::fetch_power_state());
        let rx_audio = spawn_bg(3, || pages::audio::fetch_audio_state());
        let rx_display = spawn_bg(10, || pages::display::fetch_display_state());
        let rx_network = spawn_bg(5, || pages::network::fetch_network_state());
        let rx_layout = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::layout::LayoutState>();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    let val = tokio::task::spawn_blocking(|| pages::layout::read_layout_config()).await;
                    if let Ok(val) = val { if tx.send(val).is_err() { break; } }
                }
            });
            rx
        };
        let rx_input = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::input::InputState>();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                    let val = tokio::task::spawn_blocking(|| pages::input::read_input_config()).await;
                    if let Ok(val) = val { if tx.send(val).is_err() { break; } }
                }
            });
            rx
        };
        let rx_system = spawn_bg(5, || pages::system_info::fetch_system_state());
        let rx_status = spawn_bg(10, || pages::status::fetch_status_state());
        let rx_storage = spawn_bg(10, || pages::storage::fetch_storage_state());

        let scale_factor = (window.scale_factor() as f32).max(2.0) as f64;
        let mut this = Self {
            window, surface, device, queue, config, render_pipeline,
            vertex_buffer, vertex_count: 0,
            app,
            font_system, swash_cache, text_atlas, text_renderer, text_viewport,
            widgets: Vec::new(), text_items: Vec::new(), page_buttons: Vec::new(),
            sidebar_width: 140.0, header_height: 40.0, status_height: 28.0,
            cursor_x: 0.0, cursor_y: 0.0,
            scale_factor,
            rx_power, rx_audio, rx_display, rx_network, rx_layout, rx_input,
            rx_system, rx_status, rx_storage,
            width: size.width, height: size.height,
            needs_rebuild: true,
        };
        this.rebuild_layout(sw, sh);
        this
    }

    fn rebuild_layout(&mut self, sw: f32, sh: f32) {
        let s = self.scale_factor as f32;
        let mut widgets = Vec::new();
        let mut text_items = Vec::new();
        let mut page_buttons = Vec::new();

        let sb_w = self.sidebar_width * s;
        let hdr_h = self.header_height * s;
        let st_h = self.status_height * s;

        // Header
        widgets.push(AppWidget {
            x: 0.0, y: 0.0, w: sw, h: hdr_h,
            color: color::HEADER_BG, hover_color: color::HEADER_BG,
            hovering: false, kind: WidgetKind::Static,
        });
        text_items.push(TextItem {
            buffer: make_text_buffer(&mut self.font_system, "Clear System Interface", 14.0 * s),
            x: 12.0 * s, y: 12.0 * s,
            color: glyphon::Color::rgb(0xcc, 0xcc, 0xd4),
        });

        // Sidebar bg
        widgets.push(AppWidget {
            x: 0.0, y: hdr_h, w: sb_w, h: sh - hdr_h - st_h,
            color: [0.16, 0.16, 0.26, 1.0],
            hover_color: [0.16, 0.16, 0.26, 1.0],
            hovering: false, kind: WidgetKind::Static,
        });

        // Sidebar page buttons
        let btn_h = 32.0 * s;
        let btn_margin = 4.0 * s;
        let total = Page::ALL.len() as f32;
        let sb_h = sh - hdr_h - st_h;
        let start_y = hdr_h + (sb_h - total * (btn_h + btn_margin)) / 2.0;
        let start_y = start_y.max(hdr_h + 8.0 * s);

        for (i, page) in Page::ALL.iter().enumerate() {
            let active = *page == self.app.current_page;
            let y = start_y + i as f32 * (btn_h + btn_margin);
            let bg = if active { color::BUTTON_PRESS } else { [0.18, 0.18, 0.28, 1.0] };
            let hov = if active { color::BUTTON_IDLE } else { [0.22, 0.22, 0.34, 1.0] };
            widgets.push(AppWidget {
                x: 6.0 * s, y, w: sb_w - 12.0 * s, h: btn_h,
                color: bg, hover_color: hov,
                hovering: false,
                kind: WidgetKind::PageButton(*page),
            });
            text_items.push(TextItem {
                buffer: make_text_buffer(&mut self.font_system, page.label(), 12.0 * s),
                x: 16.0 * s, y: y + 9.0 * s,
                color: glyphon::Color::rgb(0x99, 0x99, 0xaa),
            });
        }

        // Content area (logical coords, scaled to physical later)
        let lcx = self.sidebar_width;
        let lcy = self.header_height;
        let lcw = sw / s - self.sidebar_width;
        let lch = sh / s - self.header_height - self.status_height;
        let p_cx = lcx * s;
        let p_cy = lcy * s;
        let p_cw = lcw * s;
        let p_ch = lch * s;
        widgets.push(AppWidget {
            x: p_cx, y: p_cy, w: p_cw, h: p_ch,
            color: color::CONTENT_BG, hover_color: color::CONTENT_BG,
            hovering: false, kind: WidgetKind::Static,
        });

        // Page content in LOGICAL coordinates, then scale to physical
        let pc = self.render_page_content(lcx, lcy, lcw, lch);

        for (c, x, y, w, h) in &pc.rects {
            widgets.push(AppWidget {
                x: *x * s, y: *y * s, w: *w * s, h: *h * s,
                color: *c, hover_color: *c,
                hovering: false, kind: WidgetKind::Static,
            });
        }
        for (t, size, x, y, tc) in &pc.texts {
            text_items.push(TextItem {
                buffer: make_text_buffer(&mut self.font_system, t, *size * s),
                x: *x * s, y: *y * s,
                color: glyphon::Color::rgb(
                    (tc[0] * 255.0) as u8, (tc[1] * 255.0) as u8, (tc[2] * 255.0) as u8,
                ),
            });
        }
        for btn in &pc.buttons {
            widgets.push(AppWidget {
                x: btn.x * s, y: btn.y * s, w: btn.w * s, h: btn.h * s,
                color: btn.bg, hover_color: btn.hover_bg,
                hovering: false,
                kind: WidgetKind::ActionButton(btn.action.clone()),
            });
            text_items.push(TextItem {
                buffer: make_text_buffer(&mut self.font_system, &btn.label, btn.label_size * s),
                x: btn.x * s + 10.0 * s, y: btn.y * s + 8.0 * s,
                color: glyphon::Color::rgb(
                    (btn.label_color[0] * 255.0) as u8,
                    (btn.label_color[1] * 255.0) as u8,
                    (btn.label_color[2] * 255.0) as u8,
                ),
            });
            let mut cb = btn.clone();
            cb.x *= s; cb.y *= s; cb.w *= s; cb.h *= s;
            page_buttons.push(cb);
        }

        // Status bar
        widgets.push(AppWidget {
            x: 0.0, y: sh - st_h, w: sw, h: st_h,
            color: color::STATUS_BG, hover_color: color::STATUS_BG,
            hovering: false, kind: WidgetKind::Static,
        });

        self.widgets = widgets;
        self.text_items = text_items;
        self.page_buttons = page_buttons;
        self.needs_rebuild = false;
    }

    fn render_page_content(&self, cx: f32, cy: f32, cw: f32, ch: f32) -> PageContent {
        use pages::*;
        match self.app.current_page {
            Page::Power => power::view(&self.app.power, cx, cy, cw, ch),
            Page::Audio => audio::view(&self.app.audio, cx, cy, cw, ch),
            Page::Display => display::view(&self.app.display, cx, cy, cw, ch),
            Page::Radios => network::view(&self.app.network, cx, cy, cw, ch),
            Page::Layout => layout::view(&self.app.layout, cx, cy, cw, ch),
            Page::Input => input::view(&self.app.input, cx, cy, cw, ch),
            Page::System => system_info::view(&self.app.system_info, cx, cy, cw, ch),
            Page::Status => status::view(&self.app.status, cx, cy, cw, ch),
            Page::Storage => storage::view(&self.app.storage, cx, cy, cw, ch),
        }
    }

    fn collect_vertices(&self) -> Vec<Vertex> {
        let sw = self.width as f32;
        let sh = self.height as f32;
        let mut verts = Vec::new();
        for w in &self.widgets {
            verts.extend(quad_vertices(w.x, w.y, w.w, w.h, sw, sh, w.color));
        }
        verts
    }

    fn upload_vertices(&mut self) {
        let verts = self.collect_vertices();
        self.vertex_count = verts.len() as u32;
        let data = bytemuck::cast_slice(&verts);
        let needed = data.len() as wgpu::BufferAddress;
        if needed > self.vertex_buffer.size() {
            self.vertex_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Vertex Buffer"),
                size: needed,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
        }
        self.queue.write_buffer(&self.vertex_buffer, 0, data);
    }

    fn prepare_text(&mut self) {
        let Self {
            ref mut text_renderer, ref device, ref queue,
            ref mut font_system, ref mut text_atlas,
            ref mut text_viewport, ref mut swash_cache,
            ref text_items, width, height, ..
        } = self;

        let w = *width as f32;
        let h = *height as f32;
        let viewport = Resolution { width: w as u32, height: h as u32 };
        text_viewport.update(queue, viewport);
        let bounds = TextBounds { left: 0, top: 0, right: w as i32, bottom: h as i32 };
        let areas: Vec<TextArea> = text_items.iter().map(|ti| TextArea {
            buffer: &ti.buffer,
            left: ti.x, top: ti.y, scale: 1.0, bounds,
            default_color: ti.color,
            custom_glyphs: &[],
        }).collect();
        text_renderer.prepare(device, queue, font_system, text_atlas, text_viewport, areas, swash_cache).unwrap();
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            self.width = size.width; self.height = size.height;
            self.config.width = size.width; self.config.height = size.height;
            self.surface.configure(&self.device, &self.config);
            self.needs_rebuild = true;
        }
    }

    fn poll_background_updates(&mut self) {
        use pages::*;
        while let Ok(s) = self.rx_power.try_recv() {
            power::update(&mut self.app.power, power::PowerMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_audio.try_recv() {
            audio::update(&mut self.app.audio, audio::AudioMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_display.try_recv() {
            display::update(&mut self.app.display, display::DisplayMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_network.try_recv() {
            network::update(&mut self.app.network, network::NetworkMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_layout.try_recv() {
            layout::update(&mut self.app.layout, layout::LayoutMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_input.try_recv() {
            input::update(&mut self.app.input, input::InputMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_system.try_recv() {
            system_info::update(&mut self.app.system_info, system_info::SystemMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_status.try_recv() {
            status::update(&mut self.app.status, status::StatusMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_storage.try_recv() {
            storage::update(&mut self.app.storage, storage::StorageMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
    }

    fn handle_action(&mut self, action: &AppAction) {
        use pages::*;
        match action {
            AppAction::Power(m) => power::update(&mut self.app.power, m.clone()),
            AppAction::Audio(m) => audio::update(&mut self.app.audio, m.clone()),
            AppAction::Display(m) => display::update(&mut self.app.display, m.clone()),
            AppAction::Radios(m) => network::update(&mut self.app.network, m.clone()),
            AppAction::Layout(m) => layout::update(&mut self.app.layout, m.clone()),
            AppAction::Input(m) => input::update(&mut self.app.input, m.clone()),
            AppAction::SystemInfo(m) => system_info::update(&mut self.app.system_info, m.clone()),
            AppAction::Status(m) => status::update(&mut self.app.status, m.clone()),
            AppAction::Storage(m) => storage::update(&mut self.app.storage, m.clone()),
        }
    }

    fn handle_event(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_x = position.x as f32;
                self.cursor_y = position.y as f32;
                let mut changed = false;
                for w in &mut self.widgets {
                    let was = w.hovering;
                    w.hovering = self.cursor_x >= w.x && self.cursor_x <= w.x + w.w
                        && self.cursor_y >= w.y && self.cursor_y <= w.y + w.h;
                    if w.hovering != was {
                        w.color = if w.hovering { w.hover_color } else { w.color };
                        changed = true;
                    }
                }
                if changed { self.needs_rebuild = true; }
                changed
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if *button != MouseButton::Left { return false; }
                if *state == ElementState::Released {
                    let (px, py) = (self.cursor_x, self.cursor_y);
                    for btn in &self.page_buttons.clone() {
                        if px >= btn.x && px <= btn.x + btn.w && py >= btn.y && py <= btn.y + btn.h {
                            self.handle_action(&btn.action);
                            self.needs_rebuild = true;
                            return true;
                        }
                    }
                    for w in &self.widgets {
                        if px >= w.x && px <= w.x + w.w && py >= w.y && py <= w.y + w.h {
                            if let WidgetKind::PageButton(p) = &w.kind {
                                if self.app.current_page != *p {
                                    self.app.current_page = *p;
                                    self.needs_rebuild = true;
                                    return true;
                                }
                            }
                        }
                    }
                }
                self.needs_rebuild = true;
                true
            }
            _ => false,
        }
    }

    fn render(&mut self) {
        self.poll_background_updates();

        let sw = self.width as f32;
        let sh = self.height as f32;

        if self.needs_rebuild {
            self.rebuild_layout(sw, sh);
            self.upload_vertices();
        }

        self.prepare_text();

        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            Err(wgpu::SurfaceError::Timeout) => return,
            Err(e) => { eprintln!("Surface error: {e:?}"); return; }
        };

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.06, g: 0.06, b: 0.08, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            pass.set_pipeline(&self.render_pipeline);
            pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            pass.draw(0..self.vertex_count, 0..1);

            self.text_renderer.render(&self.text_atlas, &self.text_viewport, &mut pass).unwrap();
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        output.present();
    }
}

struct App { state: Option<SystemInterface> }
impl App {
    fn new() -> Self { Self { state: None } }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.is_some() { return; }
        let window = Arc::new(event_loop.create_window(
            WindowAttributes::default()
                .with_title("Clear System Interface")
                .with_inner_size(winit::dpi::LogicalSize::new(820, 680)),
        ).unwrap());
        let state = pollster::block_on(SystemInterface::new(window));
        self.state = Some(state);
        self.state.as_ref().unwrap().window.request_redraw();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: winit::window::WindowId, event: WindowEvent) {
        let redraw = match &event {
            WindowEvent::CloseRequested => { event_loop.exit(); true }
            WindowEvent::Resized(s) => { if let Some(st) = &mut self.state { st.resize(*s); } true }
            WindowEvent::RedrawRequested => {
                if let Some(st) = &mut self.state { st.render(); st.window.request_redraw(); }
                true
            }
            _ => self.state.as_mut().map(|st| st.handle_event(&event)).unwrap_or(false)
        };
        if redraw { if let Some(st) = &mut self.state { st.window.request_redraw(); } }
    }
}

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let _guard = rt.enter();
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut App::new()).unwrap();
}

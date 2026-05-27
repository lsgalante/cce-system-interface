use std::sync::Arc;

use clear_ui::color;
use clear_ui::widget::{Spinbox, Widget};
use glyphon::{
    Attrs, Buffer, Cache, FontSystem, Metrics, Resolution, SwashCache, TextArea, TextAtlas,
    TextBounds, TextRenderer, Viewport,
};

use clear_system_interface::app::{AppAction, AppState, ContentButton, PageContent};
use clear_system_interface::pages::{self, Page};

use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_pointer, delegate_registry,
    delegate_seat, delegate_shm, delegate_xdg_shell, delegate_xdg_window, delegate_output,
    registry::{ProvidesRegistryState, RegistryState},
    output::{OutputHandler, OutputState},
    seat::{
        keyboard::KeyboardHandler,
        pointer::PointerHandler,
        Capability, SeatHandler, SeatState,
    },
    shell::{
        xdg::{
            window::{Window as XdgWindow, WindowConfigure, WindowHandler, WindowDecorations},
            XdgShell,
        },
        WaylandSurface,
    },
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    globals::registry_queue_init,
    protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface},
    Connection, QueueHandle, Proxy,
};
use calloop::EventLoop;
use calloop_wayland_source::WaylandSource;

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

fn find_cased_family(fs: &FontSystem, name: &str) -> Option<String> {
    let lower_name = name.to_lowercase();
    for face in fs.db().faces() {
        for (family, _) in &face.families {
            if family.to_lowercase() == lower_name {
                return Some(family.clone());
            }
        }
    }
    None
}

fn make_text_buffer_with_font(
    fs: &mut FontSystem,
    text: &str,
    size: f32,
    font: Option<&str>,
    sans_fallback: &str,
    serif_fallback: &str,
    mono_fallback: &str,
) -> Buffer {
    let metrics = Metrics::new(size, size * 1.4);
    let mut buf = Buffer::new(fs, metrics);
    let mut attrs = Attrs::new();
    let mut resolved_storage = None;
    if let Some(font_name) = font {
        let family = match font_name {
            "monospace" => {
                if !mono_fallback.is_empty() {
                    resolved_storage = find_cased_family(fs, mono_fallback);
                    if let Some(ref cased) = resolved_storage {
                        glyphon::Family::Name(cased)
                    } else {
                        glyphon::Family::Name(mono_fallback)
                    }
                } else {
                    glyphon::Family::Monospace
                }
            }
            "sans-serif" => {
                if !sans_fallback.is_empty() {
                    resolved_storage = find_cased_family(fs, sans_fallback);
                    if let Some(ref cased) = resolved_storage {
                        glyphon::Family::Name(cased)
                    } else {
                        glyphon::Family::Name(sans_fallback)
                    }
                } else {
                    glyphon::Family::SansSerif
                }
            }
            "serif" => {
                if !serif_fallback.is_empty() {
                    resolved_storage = find_cased_family(fs, serif_fallback);
                    if let Some(ref cased) = resolved_storage {
                        glyphon::Family::Name(cased)
                    } else {
                        glyphon::Family::Name(serif_fallback)
                    }
                } else {
                    glyphon::Family::Serif
                }
            }
            _ => glyphon::Family::Name(font_name),
        };
        attrs = attrs.family(family);
    }
    buf.set_text(fs, text, attrs, glyphon::Shaping::Advanced);
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

enum ColorSelectorAction {
    Background([u8; 3]),
    Border([u8; 3]),
}

struct SystemInterface {
    window: XdgWindow,
    surface: wl_surface::WlSurface,
    wgpu_surface: wgpu::Surface<'static>,
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

    rx_audio: std::sync::mpsc::Receiver<pages::audio::AudioState>,
    rx_display: std::sync::mpsc::Receiver<pages::display::DisplayState>,
    rx_network: std::sync::mpsc::Receiver<pages::network::NetworkState>,
    rx_layout: std::sync::mpsc::Receiver<pages::layout::LayoutState>,
    rx_input: std::sync::mpsc::Receiver<pages::input::InputState>,
    rx_fingers: std::sync::mpsc::Receiver<Vec<pages::input::Finger>>,
    rx_hardware: std::sync::mpsc::Receiver<pages::hardware::HardwareState>,
    rx_system: std::sync::mpsc::Receiver<pages::system_info::SystemState>,
    rx_status: std::sync::mpsc::Receiver<pages::status::StatusState>,
    rx_storage: std::sync::mpsc::Receiver<pages::storage::StorageState>,
    rx_notifications: std::sync::mpsc::Receiver<pages::notifications::NotificationsState>,
    rx_backup_state: std::sync::mpsc::Receiver<pages::backup::BackupState>,
    rx_typeface: std::sync::mpsc::Receiver<pages::typeface::TypefaceState>,
    rx_services: std::sync::mpsc::Receiver<Vec<pages::services::ServiceInfo>>,
    rx_colors: std::sync::mpsc::Receiver<pages::colors::ColorsState>,
    tx_backup: std::sync::mpsc::Sender<pages::backup::BackupMessage>,
    rx_backup: std::sync::mpsc::Receiver<pages::backup::BackupMessage>,
    tx_color_selector: std::sync::mpsc::Sender<ColorSelectorAction>,
    rx_color_selector: std::sync::mpsc::Receiver<ColorSelectorAction>,

    scale_factor: f64,
    width: u32,
    height: u32,
    needs_rebuild: bool,
    scroll_y: f32,
    max_scroll_y: f32,
    page_root_container: clear_ui::widget::Container,
    page_sec_containers: Vec<clear_ui::widget::Container>,
    sans_serif_family: String,
    serif_family: String,
    monospace_family: String,
}

impl SystemInterface {
    async fn new(
        conn: &Connection,
        qh: &QueueHandle<App>,
        compositor_state: &CompositorState,
        xdg_shell_state: &XdgShell,
        width: u32,
        height: u32,
        scale: f64,
    ) -> Self {
        let surface = compositor_state.create_surface(qh);
        surface.set_buffer_scale(scale as i32);
        let window = xdg_shell_state.create_window(surface.clone(), WindowDecorations::None, qh);
        window.set_title("Clear System Interface");
        window.set_app_id("clear-system-interface");
        window.set_min_size(Some((820, 680)));
        window.commit();

        let wayland_handle = Box::leak(Box::new(clear_ui::wayland::WaylandSurfaceHandle {
            display_ptr: conn.backend().display_id().as_ptr() as *mut std::ffi::c_void,
            surface_ptr: surface.id().as_ptr() as *mut std::ffi::c_void,
        }));

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let wgpu_surface = instance.create_surface(wayland_handle).expect("surface");
        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&wgpu_surface),
            force_fallback_adapter: false,
        }).await.expect("adapter");
        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("GPU Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults().using_resolution(adapter.limits()),
            memory_hints: wgpu::MemoryHints::MemoryUsage,
        }, None).await.expect("device");
        let config = wgpu_surface.get_default_config(&adapter, width.max(1), height.max(1)).expect("config");
        wgpu_surface.configure(&device, &config);

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
        text_viewport.update(&queue, Resolution { width, height });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Vertex Buffer"),
            size: 1,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let app = AppState {
            layout: pages::layout::read_layout_config(),
            input: pages::input::read_input_config(),
            ..Default::default()
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
                    let val = f().await;
                    if tx.send(val).is_err() { break; }
                    tokio::time::sleep(std::time::Duration::from_secs(period_secs)).await;
                }
            });
            rx
        }

        let rx_audio = spawn_bg(3, || pages::audio::fetch_audio_state());
        let rx_display = spawn_bg(10, || pages::display::fetch_display_state());
        let rx_network = spawn_bg(5, || pages::network::fetch_network_state());
        let rx_layout = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::layout::LayoutState>();
            tokio::spawn(async move {
                loop {
                    let val = tokio::task::spawn_blocking(|| pages::layout::read_layout_config()).await;
                    if let Ok(val) = val { if tx.send(val).is_err() { break; } }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            });
            rx
        };
        let rx_input = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::input::InputState>();
            tokio::spawn(async move {
                loop {
                    let val = tokio::task::spawn_blocking(|| pages::input::read_input_config()).await;
                    if let Ok(val) = val { if tx.send(val).is_err() { break; } }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            });
            rx
        };
        let rx_fingers = {
            let (tx, rx) = std::sync::mpsc::channel::<Vec<pages::input::Finger>>();
            tokio::spawn(async move {
                let socket_path = "/tmp/clear-input-coords.sock";
                loop {
                    if let Ok(stream) = tokio::net::UnixStream::connect(socket_path).await {
                        use tokio::io::AsyncBufReadExt;
                        let reader = tokio::io::BufReader::new(stream);
                        let mut lines = reader.lines();
                        while let Ok(Some(line)) = lines.next_line().await {
                            if let Ok(fingers) = serde_json::from_str::<Vec<pages::input::Finger>>(&line) {
                                if tx.send(fingers).is_err() {
                                    return;
                                }
                            }
                        }
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
            });
            rx
        };
        let rx_system = spawn_bg(5, || pages::system_info::fetch_system_state());
        let rx_hardware = spawn_bg(3, || pages::hardware::fetch_hardware_state());
        let rx_status = spawn_bg(10, || pages::status::fetch_status_state());
        let rx_storage = spawn_bg(10, || pages::storage::fetch_storage_state());
        let rx_notifications = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::notifications::NotificationsState>();
            tokio::spawn(async move {
                loop {
                    let val = tokio::task::spawn_blocking(|| pages::notifications::read_notifications_config()).await;
                    if let Ok(val) = val { if tx.send(val).is_err() { break; } }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            });
            rx
        };
        let rx_backup_state = spawn_bg(30, || pages::backup::fetch_backup_state());
        let rx_typeface = spawn_bg(30, || pages::typeface::fetch_typeface_state());
        let rx_services = spawn_bg(3, || pages::services::fetch_services());
        let rx_colors = {
            let (tx, rx) = std::sync::mpsc::channel::<pages::colors::ColorsState>();
            tokio::spawn(async move {
                loop {
                    let val = tokio::task::spawn_blocking(|| pages::colors::read_colors_config()).await;
                    if let Ok(val) = val { if tx.send(val).is_err() { break; } }
                    tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                }
            });
            rx
        };
        let (tx_backup, rx_backup) = std::sync::mpsc::channel();

        let (tx_color_selector, rx_color_selector) = std::sync::mpsc::channel();
        let scale_factor = scale;

        let (sans_family, serif_family, monospace_family, _, _, _, _) = pages::typeface::read_preferred_fonts();

        let mut this = Self {
            window, surface, wgpu_surface, device, queue, config, render_pipeline,
            vertex_buffer, vertex_count: 0,
            app,
            font_system, swash_cache, text_atlas, text_renderer, text_viewport,
            widgets: Vec::new(), text_items: Vec::new(), page_buttons: Vec::new(),
            sidebar_width: 140.0, header_height: 0.0, status_height: 0.0,
            cursor_x: 0.0, cursor_y: 0.0,
            scale_factor,
            rx_audio, rx_display, rx_network, rx_layout, rx_input, rx_fingers,
            rx_hardware, rx_system, rx_status, rx_storage, rx_notifications,
            rx_backup_state, rx_typeface, rx_services, rx_colors, tx_backup, rx_backup,
            tx_color_selector, rx_color_selector,
            width, height,
            needs_rebuild: true,
            scroll_y: 0.0,
            max_scroll_y: 0.0,
            page_root_container: clear_ui::widget::Container::new(),
            page_sec_containers: Vec::new(),
            sans_serif_family: sans_family,
            serif_family,
            monospace_family,
        };
        this.rebuild_layout(width as f32, height as f32);
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
                color: glyphon::Color::rgb(0x33, 0x33, 0x4a),
            });
        }

        // Content area
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

        let mut max_y = 0.0f32;
        for (_, _, y, _, h) in &pc.rects {
            max_y = max_y.max(y + h);
        }
        for (_, size, _, y, _, _) in &pc.texts {
            max_y = max_y.max(y + size);
        }
        for btn in &pc.buttons {
            max_y = max_y.max(btn.y + btn.h);
        }
        self.max_scroll_y = (max_y - lch).max(0.0);
        self.scroll_y = self.scroll_y.min(self.max_scroll_y);

        let scroll_offset_y = self.scroll_y;

        for (c, x, y, w, h) in &pc.rects {
            widgets.push(AppWidget {
                x: *x * s, y: (*y - scroll_offset_y) * s, w: *w * s, h: *h * s,
                color: *c, hover_color: *c,
                hovering: false, kind: WidgetKind::Static,
            });
        }
        for (t, size, x, y, tc, font_opt) in &pc.texts {
            text_items.push(TextItem {
                buffer: make_text_buffer_with_font(
                    &mut self.font_system,
                    t,
                    *size * s,
                    font_opt.as_deref(),
                    &self.sans_serif_family,
                    &self.serif_family,
                    &self.monospace_family,
                ),
                x: *x * s, y: (*y - scroll_offset_y) * s,
                color: glyphon::Color::rgb(
                    (tc[0] * 255.0) as u8, (tc[1] * 255.0) as u8, (tc[2] * 255.0) as u8,
                ),
            });
        }
        for btn in &pc.buttons {
            widgets.push(AppWidget {
                x: btn.x * s, y: (btn.y - scroll_offset_y) * s, w: btn.w * s, h: btn.h * s,
                color: btn.bg, hover_color: btn.hover_bg,
                hovering: false,
                kind: WidgetKind::ActionButton(btn.action.clone()),
            });
            let buf = make_text_buffer(&mut self.font_system, &btn.label, btn.label_size * s);
            let tw = buf.layout_runs().next().map(|r| r.line_w).unwrap_or(0.0);
            let lh = btn.label_size * s * 1.4;
            let mut left_align = btn.left_align;

            // Auto-detect if inside a ScrollBox to apply left alignment by default
            if !left_align && btn.w >= 60.0 {
                if self.app.current_page == Page::Typefaces {
                    let sb = &self.app.typeface.list_box;
                    let (sb_x, sb_y, sb_w, sb_h) = sb.rect();
                    if btn.x >= sb_x - 1.0 && btn.x + btn.w <= sb_x + sb_w + 1.0
                       && btn.y >= sb_y - 1.0 && btn.y + btn.h <= sb_y + sb_h + 1.0 {
                        left_align = true;
                    }
                } else if self.app.current_page == Page::Hardware {
                    let sb = &self.app.hardware.cpu_list_box;
                    let (sb_x, sb_y, sb_w, sb_h) = sb.rect();
                    if btn.x >= sb_x - 1.0 && btn.x + btn.w <= sb_x + sb_w + 1.0
                       && btn.y >= sb_y - 1.0 && btn.y + btn.h <= sb_y + sb_h + 1.0 {
                        left_align = true;
                    }
                } else if self.app.current_page == Page::Services {
                    let sb = &self.app.services.list_box;
                    let (sb_x, sb_y, sb_w, sb_h) = sb.rect();
                    if btn.x >= sb_x - 1.0 && btn.x + btn.w <= sb_x + sb_w + 1.0
                       && btn.y >= sb_y - 1.0 && btn.y + btn.h <= sb_y + sb_h + 1.0 {
                        left_align = true;
                    }
                }
            }

            let text_x = if left_align {
                btn.x * s + 8.0 * s
            } else {
                btn.x * s + (btn.w * s - tw) / 2.0
            };

            text_items.push(TextItem {
                buffer: buf,
                x: text_x, y: (btn.y - scroll_offset_y) * s + (btn.h * s - lh) / 2.0,
                color: glyphon::Color::rgb(
                    (btn.label_color[0] * 255.0) as u8,
                    (btn.label_color[1] * 255.0) as u8,
                    (btn.label_color[2] * 255.0) as u8,
                ),
            });
            let mut cb = btn.clone();
            cb.x *= s; cb.y = (cb.y - scroll_offset_y) * s; cb.w *= s; cb.h *= s;
            page_buttons.push(cb);
        }

        // ── Rebuild Widget Focus Hierarchy ──
        self.page_root_container.clear_children();
        self.page_root_container.set_parent(None);
        self.page_sec_containers.clear();

        // Clear all widgets' hierarchy links
        self.app.typeface.sans_box.clear_children(); self.app.typeface.sans_box.set_parent(None);
        self.app.typeface.serif_box.clear_children(); self.app.typeface.serif_box.set_parent(None);
        self.app.typeface.mono_box.clear_children(); self.app.typeface.mono_box.set_parent(None);
        self.app.typeface.borders_menu.clear_children(); self.app.typeface.borders_menu.set_parent(None);
        self.app.typeface.borders_box.clear_children(); self.app.typeface.borders_box.set_parent(None);
        self.app.typeface.status_menu.clear_children(); self.app.typeface.status_menu.set_parent(None);
        self.app.typeface.status_box.clear_children(); self.app.typeface.status_box.set_parent(None);
        self.app.typeface.fuzzel_menu.clear_children(); self.app.typeface.fuzzel_menu.set_parent(None);
        self.app.typeface.fuzzel_box.clear_children(); self.app.typeface.fuzzel_box.set_parent(None);
        self.app.typeface.terminal_menu.clear_children(); self.app.typeface.terminal_menu.set_parent(None);
        self.app.typeface.terminal_box.clear_children(); self.app.typeface.terminal_box.set_parent(None);
        self.app.typeface.search_box.clear_children(); self.app.typeface.search_box.set_parent(None);
        self.app.typeface.list_box.scroll_box.clear_children(); self.app.typeface.list_box.scroll_box.set_parent(None);

        self.app.services.search_box.clear_children(); self.app.services.search_box.set_parent(None);

        self.app.services.list_box.scroll_box.clear_children(); self.app.services.list_box.scroll_box.set_parent(None);

        self.app.hardware.cpu_list_box.scroll_box.clear_children(); self.app.hardware.cpu_list_box.scroll_box.set_parent(None);

        self.app.network.wifi_list_box.scroll_box.clear_children(); self.app.network.wifi_list_box.scroll_box.set_parent(None);

        for sb in &mut self.app.layout.spinboxes {
            sb.clear_children();
            sb.set_parent(None);
        }
        self.app.layout.cascade_offset_spinbox.clear_children(); self.app.layout.cascade_offset_spinbox.set_parent(None);
        self.app.layout.edge_gap_spinbox.clear_children(); self.app.layout.edge_gap_spinbox.set_parent(None);
        self.app.layout.top_gap_spinbox.clear_children(); self.app.layout.top_gap_spinbox.set_parent(None);

        for cs in &mut self.app.colors.color_selectors {
            cs.clear_children();
            cs.set_parent(None);
        }

        self.app.notifications.duration_spinbox.clear_children(); self.app.notifications.duration_spinbox.set_parent(None);

        self.app.input.rate_spinbox.clear_children(); self.app.input.rate_spinbox.set_parent(None);
        self.app.input.delay_spinbox.clear_children(); self.app.input.delay_spinbox.set_parent(None);
        self.app.input.scroll_friction_spinbox.clear_children(); self.app.input.scroll_friction_spinbox.set_parent(None);
        self.app.input.pointer_friction_spinbox.clear_children(); self.app.input.pointer_friction_spinbox.set_parent(None);
        self.app.input.trackpad_friction_spinbox.clear_children(); self.app.input.trackpad_friction_spinbox.set_parent(None);
        self.app.input.trackpoint_accel_speed_spinbox.clear_children(); self.app.input.trackpoint_accel_speed_spinbox.set_parent(None);
        self.app.input.trackpoint_accel_profile_menu.clear_children(); self.app.input.trackpoint_accel_profile_menu.set_parent(None);

        for sb in &mut self.app.audio.sink_spinboxes {
            sb.clear_children();
            sb.set_parent(None);
        }
        for sb in &mut self.app.audio.source_spinboxes {
            sb.clear_children();
            sb.set_parent(None);
        }

        self.app.display.brightness_spinbox.clear_children(); self.app.display.brightness_spinbox.set_parent(None);

        use clear_ui::widget::focus::link_parent_child;
        match self.app.current_page {
            Page::Typefaces => {
                self.page_sec_containers.resize_with(3, clear_ui::widget::Container::new);
                
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[0]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[1]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[2]);
                
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.typeface.sans_box);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.typeface.serif_box);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.typeface.mono_box);
                
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.borders_menu);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.borders_box);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.status_menu);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.status_box);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.fuzzel_menu);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.fuzzel_box);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.terminal_menu);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.typeface.terminal_box);
                
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.typeface.search_box);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.typeface.list_box.scroll_box);
            }
            Page::Services => {
                link_parent_child(&mut self.page_root_container, &mut self.app.services.search_box);
                link_parent_child(&mut self.page_root_container, &mut self.app.services.list_box.scroll_box);
            }
            Page::Hardware => {
                link_parent_child(&mut self.page_root_container, &mut self.app.hardware.cpu_list_box.scroll_box);
            }
            Page::Radios => {
                link_parent_child(&mut self.page_root_container, &mut self.app.network.wifi_list_box.scroll_box);
            }
            Page::Layout => {
                self.page_sec_containers.resize_with(6, clear_ui::widget::Container::new);
                
                for i in 0..6 {
                    link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[i]);
                }
                
                if !self.app.layout.spinboxes.is_empty() {
                    link_parent_child(&mut self.page_sec_containers[0], &mut self.app.layout.spinboxes[0]);
                }
                if self.app.layout.spinboxes.len() > 1 {
                    link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.spinboxes[1]);
                }
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.cascade_offset_spinbox);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.edge_gap_spinbox);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.layout.top_gap_spinbox);
                
                if self.app.layout.spinboxes.len() > 2 {
                    link_parent_child(&mut self.page_sec_containers[2], &mut self.app.layout.spinboxes[2]);
                }
                if self.app.layout.spinboxes.len() > 3 {
                    link_parent_child(&mut self.page_sec_containers[3], &mut self.app.layout.spinboxes[3]);
                }
                if self.app.layout.spinboxes.len() > 4 {
                    link_parent_child(&mut self.page_sec_containers[4], &mut self.app.layout.spinboxes[4]);
                }
                if self.app.layout.spinboxes.len() > 5 {
                    link_parent_child(&mut self.page_sec_containers[5], &mut self.app.layout.spinboxes[5]);
                }
            }
            Page::Colors => {
                for cs in &mut self.app.colors.color_selectors {
                    link_parent_child(&mut self.page_root_container, cs);
                }
            }
            Page::Notifications => {
                link_parent_child(&mut self.page_root_container, &mut self.app.notifications.duration_spinbox);
            }
            Page::Input => {
                self.page_sec_containers.resize_with(3, clear_ui::widget::Container::new);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[0]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[1]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[2]);
                
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.input.trackpoint_accel_speed_spinbox);
                link_parent_child(&mut self.page_sec_containers[0], &mut self.app.input.trackpoint_accel_profile_menu);
                
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.input.rate_spinbox);
                link_parent_child(&mut self.page_sec_containers[1], &mut self.app.input.delay_spinbox);
                
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.input.scroll_friction_spinbox);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.input.pointer_friction_spinbox);
                link_parent_child(&mut self.page_sec_containers[2], &mut self.app.input.trackpad_friction_spinbox);
            }
            Page::Audio => {
                self.page_sec_containers.resize_with(2, clear_ui::widget::Container::new);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[0]);
                link_parent_child(&mut self.page_root_container, &mut self.page_sec_containers[1]);
                
                for sb in &mut self.app.audio.sink_spinboxes {
                    link_parent_child(&mut self.page_sec_containers[0], sb);
                }
                for sb in &mut self.app.audio.source_spinboxes {
                    link_parent_child(&mut self.page_sec_containers[1], sb);
                }
            }
            Page::Display => {
                link_parent_child(&mut self.page_root_container, &mut self.app.display.brightness_spinbox);
            }
            _ => {}
        }

        // Render context menu overlay if visible
        if clear_ui::widget::context_menu::is_visible() {
            let cx = clear_ui::widget::context_menu::x();
            let cy = clear_ui::widget::context_menu::y();
            let cw = clear_ui::widget::context_menu::w();
            let ch = clear_ui::widget::context_menu::h();
            
            // Border
            widgets.push(AppWidget {
                x: cx * s, y: cy * s, w: cw * s, h: ch * s,
                color: [0.22, 0.22, 0.28, 1.0], hover_color: [0.22, 0.22, 0.28, 1.0],
                hovering: false, kind: WidgetKind::Static,
            });
            // Bg
            widgets.push(AppWidget {
                x: (cx + 1.0) * s, y: (cy + 1.0) * s, w: (cw - 2.0) * s, h: (ch - 2.0) * s,
                color: [0.06, 0.06, 0.09, 1.0], hover_color: [0.06, 0.06, 0.09, 1.0],
                hovering: false, kind: WidgetKind::Static,
            });
            
            // Hover highlight
            if let Some(h_idx) = clear_ui::widget::context_menu::hovered_item() {
                let iy = cy + h_idx as f32 * 24.0;
                widgets.push(AppWidget {
                    x: (cx + 2.0) * s, y: (iy + 2.0) * s, w: (cw - 4.0) * s, h: 20.0 * s,
                    color: [0.20, 0.40, 0.65, 0.6], hover_color: [0.20, 0.40, 0.65, 0.6],
                    hovering: false, kind: WidgetKind::Static,
                });
            }
            
            // Texts
            for (idx, opt) in clear_ui::widget::context_menu::options().iter().enumerate() {
                let iy = cy + idx as f32 * 24.0 + (24.0 - 12.0) / 2.0;
                let text_color = if clear_ui::widget::context_menu::hovered_item() == Some(idx) {
                    glyphon::Color::rgb(0xff, 0xff, 0xff)
                } else {
                    glyphon::Color::rgb(0xcc, 0xcc, 0xd4)
                };
                text_items.push(TextItem {
                    buffer: make_text_buffer(&mut self.font_system, opt, 12.0 * s),
                    x: (cx + 8.0) * s, y: iy * s,
                    color: text_color,
                });
            }
        }

        self.widgets = widgets;
        self.text_items = text_items;
        self.page_buttons = page_buttons;
        self.needs_rebuild = false;
    }

    fn render_page_content(&mut self, cx: f32, cy: f32, cw: f32, ch: f32) -> PageContent {
        use pages::*;
        let root_focused = clear_ui::widget::focus::is_focused(&self.page_root_container);
        let sec_focused: Vec<bool> = self.page_sec_containers.iter()
            .map(|c| clear_ui::widget::focus::is_focused(c))
            .collect();
        match self.app.current_page {
            Page::Audio => audio::view(&mut self.app.audio, cx, cy, cw, ch, &sec_focused),
            Page::Display => display::view(&mut self.app.display, cx, cy, cw, ch),
            Page::Radios => network::view(&mut self.app.network, cx, cy, cw, ch, root_focused),
            Page::Layout => layout::view(&mut self.app.layout, cx, cy, cw, ch, &sec_focused),
            Page::Hardware => hardware::view(&mut self.app.hardware, cx, cy, cw, ch, root_focused),
            Page::Input => input::view(&mut self.app.input, cx, cy, cw, ch, &sec_focused),
            Page::System => system_info::view(&self.app.system_info, cx, cy, cw, ch),
            Page::Status => status::view(&mut self.app.status, cx, cy, cw, ch),
            Page::Storage => storage::view(&self.app.storage, cx, cy, cw, ch),
            Page::Notifications => notifications::view(&mut self.app.notifications, cx, cy, cw, ch),
            Page::Backup => backup::view(&self.app.backup, cx, cy, cw, ch),
            Page::Typefaces => typeface::view(&mut self.app.typeface, cx, cy, cw, ch, &sec_focused),
            Page::Services => services::view(&mut self.app.services, cx, cy, cw, ch, root_focused),
            Page::Colors => colors::view(&mut self.app.colors, cx, cy, cw, ch),
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

    fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.width = width; self.height = height;
            self.config.width = width; self.config.height = height;
            self.wgpu_surface.configure(&self.device, &self.config);
            self.needs_rebuild = true;
        }
    }

    fn poll_background_updates(&mut self) {
        use pages::*;
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
        let mut got_fingers = None;
        while let Ok(s) = self.rx_fingers.try_recv() {
            got_fingers = Some(s);
        }
        if let Some(fingers) = got_fingers {
            input::update(&mut self.app.input, input::InputMessage::UpdateFingers(fingers));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_system.try_recv() {
            system_info::update(&mut self.app.system_info, system_info::SystemMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_hardware.try_recv() {
            hardware::update(&mut self.app.hardware, hardware::HardwareMessage::Refreshed(s));
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
        while let Ok(s) = self.rx_notifications.try_recv() {
            notifications::update(&mut self.app.notifications, notifications::NotificationsMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_backup_state.try_recv() {
            pages::backup::update(&mut self.app.backup, pages::backup::BackupMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_typeface.try_recv() {
            self.sans_serif_family = s.sans_serif.clone();
            self.serif_family = s.serif.clone();
            self.monospace_family = s.monospace.clone();
            typeface::update(&mut self.app.typeface, typeface::TypefaceMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_services.try_recv() {
            pages::services::update(&mut self.app.services, pages::services::ServicesMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(s) = self.rx_colors.try_recv() {
            colors::update(&mut self.app.colors, pages::colors::ColorsMessage::Refreshed(s));
            self.needs_rebuild = true;
        }
        while let Ok(m) = self.rx_backup.try_recv() {
            self.handle_action(&AppAction::Backup(m));
            self.needs_rebuild = true;
        }
        while let Ok(action) = self.rx_color_selector.try_recv() {
            match action {
                ColorSelectorAction::Background(rgb) => {
                    colors::update(&mut self.app.colors, pages::colors::ColorsMessage::SetLowColor(rgb));
                }
                ColorSelectorAction::Border(rgb) => {
                    colors::update(&mut self.app.colors, pages::colors::ColorsMessage::SetHighColor(rgb));
                }
            }
            self.needs_rebuild = true;
        }
    }

    fn handle_action(&mut self, action: &AppAction) {
        use pages::*;
        match action {
            AppAction::Audio(m) => audio::update(&mut self.app.audio, m.clone()),
            AppAction::Display(m) => display::update(&mut self.app.display, m.clone()),
            AppAction::Radios(m) => network::update(&mut self.app.network, m.clone()),
            AppAction::Layout(m) => layout::update(&mut self.app.layout, m.clone()),
            AppAction::Input(m) => input::update(&mut self.app.input, m.clone()),
            AppAction::SystemInfo(m) => system_info::update(&mut self.app.system_info, m.clone()),
            AppAction::Hardware(m) => hardware::update(&mut self.app.hardware, m.clone()),
            AppAction::Status(m) => status::update(&mut self.app.status, m.clone()),
            AppAction::Storage(m) => storage::update(&mut self.app.storage, m.clone()),
            AppAction::Notifications(m) => notifications::update(&mut self.app.notifications, m.clone()),
            AppAction::Typeface(m) => {
                typeface::update(&mut self.app.typeface, m.clone());
                self.sans_serif_family = self.app.typeface.sans_serif.clone();
                self.serif_family = self.app.typeface.serif.clone();
                self.monospace_family = self.app.typeface.monospace.clone();
            }
            AppAction::Services(m) => services::update(&mut self.app.services, m.clone()),
            AppAction::Colors(m) => colors::update(&mut self.app.colors, m.clone()),
            AppAction::Backup(m) => match m {
                pages::backup::BackupMessage::StartBackup => {
                    pages::backup::update(&mut self.app.backup, pages::backup::BackupMessage::StartBackup);
                    let tx = self.tx_backup.clone();
                    tokio::spawn(async move {
                        let res = pages::backup::run_backup().await;
                        let _ = tx.send(pages::backup::BackupMessage::BackupFinished(res));
                    });
                }
                _ => pages::backup::update(&mut self.app.backup, m.clone()),
            },
        }
    }

    fn handle_cursor_moved(&mut self, x: f32, y: f32) -> bool {
        self.cursor_x = x;
        self.cursor_y = y;
        let s = self.scale_factor as f32;
        
        if clear_ui::widget::context_menu::is_visible() {
            let lx_no_scroll = x / s;
            let ly_no_scroll = y / s;
            if clear_ui::widget::context_menu::cursor_moved(lx_no_scroll, ly_no_scroll) {
                self.needs_rebuild = true;
                return true;
            }
            return false;
        }

        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
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
        if self.app.current_page == Page::Layout {
            for sb in &mut self.app.layout.spinboxes {
                if sb.cursor_moved(lx, ly) {
                    changed = true;
                }
            }
            if self.app.layout.cascade_offset_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.layout.edge_gap_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.layout.top_gap_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Colors {
            for cp in &mut self.app.colors.color_selectors {
                if cp.cursor_moved(lx, ly) {
                    changed = true;
                }
            }
        }
        if self.app.current_page == Page::Input {
            if self.app.input.rate_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.delay_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.tap_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.scroll_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.scroll_friction_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.pointer_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.pointer_friction_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.trackpad_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.trackpad_friction_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.dwtp_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.trackpoint_accel_speed_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.input.trackpoint_accel_profile_menu.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Audio {
            for sb in &mut self.app.audio.sink_spinboxes {
                if sb.cursor_moved(lx, ly) {
                    changed = true;
                }
            }
            for sb in &mut self.app.audio.source_spinboxes {
                if sb.cursor_moved(lx, ly) {
                    changed = true;
                }
            }
        }
        if self.app.current_page == Page::Display {
            if self.app.display.brightness_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.display.night_light_label.cursor_moved(lx, ly) {
                changed = true;
            }
            for out in &mut self.app.display.outputs {
                if out.name_label.cursor_moved(lx, ly) {
                    changed = true;
                }
                if out.resolution_label.cursor_moved(lx, ly) {
                    changed = true;
                }
                if let Some(ref mut scale_lbl) = out.scale_label {
                    if scale_lbl.cursor_moved(lx, ly) {
                        changed = true;
                    }
                }
            }
        }
        if self.app.current_page == Page::Notifications {
            if self.app.notifications.enable_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.notifications.bell_toggle.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.notifications.duration_spinbox.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Hardware {
            if self.app.hardware.cpu_label.cursor_moved(lx, ly) {
                changed = true;
            }
            for gpu_lbl in &mut self.app.hardware.gpu_labels {
                if gpu_lbl.cursor_moved(lx, ly) {
                    changed = true;
                }
            }
            if self.app.hardware.cpu_list_box.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Status {
            if self.app.status.status_label.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.status.size_label.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Typefaces {
            if self.app.typeface.sans_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.serif_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.mono_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.borders_menu.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.borders_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.status_menu.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.status_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.fuzzel_menu.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.fuzzel_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.terminal_menu.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.terminal_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.search_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.list_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.borders_size_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.status_size_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.fuzzel_size_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.typeface.terminal_size_box.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if self.app.current_page == Page::Services {
            if self.app.services.search_box.cursor_moved(lx, ly) {
                changed = true;
            }
            if self.app.services.list_box.cursor_moved(lx, ly) {
                changed = true;
            }
        }
        if changed { self.needs_rebuild = true; }
        changed
    }

    fn handle_mouse_input(&mut self, button: clear_ui::widget::MouseButton, state: clear_ui::widget::ElementState) -> bool {
        let s = self.scale_factor as f32;
        let lx_no_scroll = self.cursor_x / s;
        let ly_no_scroll = self.cursor_y / s;

        if clear_ui::widget::context_menu::is_visible() {
            if clear_ui::widget::context_menu::mouse_input(button, state, lx_no_scroll, ly_no_scroll) {
                self.needs_rebuild = true;
                return true;
            }
        }

        if button != clear_ui::widget::MouseButton::Left && button != clear_ui::widget::MouseButton::Right { return false; }
        if button == clear_ui::widget::MouseButton::Left && state == clear_ui::widget::ElementState::Released {
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
                            clear_ui::widget::focus::clear_focus();
                            self.app.current_page = *p;
                            self.scroll_y = 0.0;
                            self.needs_rebuild = true;
                            return true;
                        }
                    }
                }
            }
        }
        let lx = self.cursor_x / s;
        let ly = self.cursor_y / s + self.scroll_y;
        let mut actions = Vec::new();

        if state == clear_ui::widget::ElementState::Pressed {
            let mut clicked_any_focusable = false;
            match self.app.current_page {
                Page::Layout => {
                    for sb in &mut self.app.layout.spinboxes {
                        if sb.hit_test(lx, ly) { clicked_any_focusable = true; }
                    }
                    if self.app.layout.cascade_offset_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.layout.edge_gap_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.layout.top_gap_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Colors => {
                    for cp in &mut self.app.colors.color_selectors {
                        if cp.hit_test(lx, ly) { clicked_any_focusable = true; }
                    }
                }
                Page::Input => {
                    if self.app.input.rate_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.delay_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.scroll_friction_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.pointer_friction_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.trackpad_friction_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.trackpoint_accel_speed_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.input.trackpoint_accel_profile_menu.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Notifications => {
                    if self.app.notifications.duration_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Audio => {
                    for sb in &mut self.app.audio.sink_spinboxes {
                        if sb.hit_test(lx, ly) { clicked_any_focusable = true; }
                    }
                    for sb in &mut self.app.audio.source_spinboxes {
                        if sb.hit_test(lx, ly) { clicked_any_focusable = true; }
                    }
                }
                Page::Display => {
                    if self.app.display.brightness_spinbox.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.display.night_light_label.hit_test(lx, ly) { clicked_any_focusable = true; }
                    for out in &mut self.app.display.outputs {
                        if out.name_label.hit_test(lx, ly) { clicked_any_focusable = true; }
                        if out.resolution_label.hit_test(lx, ly) { clicked_any_focusable = true; }
                        if let Some(ref mut scale_lbl) = out.scale_label {
                            if scale_lbl.hit_test(lx, ly) { clicked_any_focusable = true; }
                        }
                    }
                }
                Page::Status => {
                    if self.app.status.status_label.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if self.app.status.size_label.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Typefaces => {
                    let tf = &mut self.app.typeface;
                    if tf.sans_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.serif_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.mono_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.borders_menu.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.borders_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.status_menu.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.status_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.fuzzel_menu.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.fuzzel_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.terminal_menu.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.terminal_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.search_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if tf.list_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Services => {
                    let srv = &mut self.app.services;
                    if srv.search_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                    if srv.list_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Hardware => {
                    let hw = &mut self.app.hardware;
                    if hw.cpu_list_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                Page::Radios => {
                    let net = &mut self.app.network;
                    if net.wifi_list_box.hit_test(lx, ly) { clicked_any_focusable = true; }
                }
                _ => {}
            }

            if !clicked_any_focusable {
                clear_ui::widget::focus::clear_focus();
            }
        }

        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Layout {
            for (i, sb) in self.app.layout.spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetWidth(
                            pages::layout::WidthParam::ALL[i],
                            sb.value as u16,
                        )
                    ));
                }
            }
            let sb = &mut self.app.layout.cascade_offset_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetCascadeOffset(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.edge_gap_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetEdgeGap(sb.value as u16)
                ));
            }
            let sb = &mut self.app.layout.top_gap_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Layout(
                    pages::layout::LayoutMessage::SetTopGap(sb.value as u16)
                ));
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Colors {
            for (i, cp) in self.app.colors.color_selectors.iter_mut().enumerate() {
                let old = cp.color;
                if !cp.hit_test(lx, ly) { cp.unfocus(); }
                cp.mouse_input(button, state, lx, ly);
                if cp.take_click() {
                    actions.push(AppAction::Colors(match i {
                        0 => pages::colors::ColorsMessage::PickLowColor,
                        1 => pages::colors::ColorsMessage::PickHighColor,
                        _ => pages::colors::ColorsMessage::PickDisabledColor,
                    }));
                }
                if cp.color != old {
                    actions.push(AppAction::Colors(match i {
                        0 => pages::colors::ColorsMessage::SetLowColor(cp.color),
                        1 => pages::colors::ColorsMessage::SetHighColor(cp.color),
                        _ => pages::colors::ColorsMessage::SetDisabledColor(cp.color),
                    }));
                }
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Input {
            let sb = &mut self.app.input.rate_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
            }
            let sb = &mut self.app.input.delay_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyRepeat));
            }
            let sb = &mut self.app.input.scroll_friction_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyScrollFriction));
            }
            let sb = &mut self.app.input.pointer_friction_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyPointerFriction));
            }
            let sb = &mut self.app.input.trackpad_friction_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpadFriction));
            }
            let sb = &mut self.app.input.trackpoint_accel_speed_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelSpeed));
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Notifications {
            let sb = &mut self.app.notifications.duration_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Notifications(pages::notifications::NotificationsMessage::SetDuration(sb.value)));
            }
        }
        if self.app.current_page == Page::Input {
            let toggle = &mut self.app.input.tap_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleTapToClick));
            }
            let toggle = &mut self.app.input.scroll_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialScroll));
            }
            let toggle = &mut self.app.input.pointer_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialPointer));
            }
            let toggle = &mut self.app.input.trackpad_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleInertialTrackpad));
            }
            let toggle = &mut self.app.input.dwtp_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Input(pages::input::InputMessage::ToggleDwtp));
            }
            let menu = &mut self.app.input.trackpoint_accel_profile_menu;
            if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelProfile(menu.selected)));
            }
        }
        if self.app.current_page == Page::Notifications {
            let toggle = &mut self.app.notifications.enable_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Notifications(pages::notifications::NotificationsMessage::ToggleEnable));
            }
            let toggle = &mut self.app.notifications.bell_toggle;
            toggle.mouse_input(button, state, lx, ly);
            if toggle.take_click() {
                actions.push(AppAction::Notifications(pages::notifications::NotificationsMessage::ToggleBell));
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Audio {
            for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                    let id = self.app.audio.sinks[i].id;
                    actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                }
            }
            for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                if !sb.hit_test(lx, ly) { sb.unfocus(); }
                let old = sb.value;
                if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                    let id = self.app.audio.sources[i].id;
                    actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                }
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Display {
            let sb = &mut self.app.display.brightness_spinbox;
            if !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old = sb.value;
            if sb.mouse_input(button, state, lx, ly) && sb.value != old {
                actions.push(AppAction::Display(pages::display::DisplayMessage::BrightnessSet(sb.value as u32)));
            }
            let lbl = &mut self.app.display.night_light_label;
            if !lbl.hit_test(lx, ly) { lbl.unfocus(); }
            lbl.mouse_input(button, state, lx, ly);

            for out in &mut self.app.display.outputs {
                let lbl = &mut out.name_label;
                if !lbl.hit_test(lx, ly) { lbl.unfocus(); }
                lbl.mouse_input(button, state, lx, ly);

                let lbl2 = &mut out.resolution_label;
                if !lbl2.hit_test(lx, ly) { lbl2.unfocus(); }
                lbl2.mouse_input(button, state, lx, ly);

                if let Some(ref mut scale_lbl) = out.scale_label {
                    if !scale_lbl.hit_test(lx, ly) { scale_lbl.unfocus(); }
                    scale_lbl.mouse_input(button, state, lx, ly);
                }
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Status {
            let lbl1 = &mut self.app.status.status_label;
            if !lbl1.hit_test(lx, ly) { lbl1.unfocus(); }
            lbl1.mouse_input(button, state, lx, ly);

            let lbl2 = &mut self.app.status.size_label;
            if !lbl2.hit_test(lx, ly) { lbl2.unfocus(); }
            lbl2.mouse_input(button, state, lx, ly);
        }
        if self.app.current_page == Page::Typefaces {
            let tb = &mut self.app.typeface.sans_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSans(tb.text.clone())));
            }

            let tb = &mut self.app.typeface.serif_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSerif(tb.text.clone())));
            }

            let tb = &mut self.app.typeface.mono_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetMono(tb.text.clone())));
            }

            let menu = &mut self.app.typeface.borders_menu;
            if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetBordersMenu(menu.selected)));
            }

            let tb = &mut self.app.typeface.borders_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetBorders(tb.text.clone())));
            }

            let menu = &mut self.app.typeface.status_menu;
            if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetStatusMenu(menu.selected)));
            }

            let tb = &mut self.app.typeface.status_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetStatus(tb.text.clone())));
            }

            let menu = &mut self.app.typeface.fuzzel_menu;
            if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetFuzzelMenu(menu.selected)));
            }

            let tb = &mut self.app.typeface.fuzzel_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetFuzzel(tb.text.clone())));
            }

            let menu = &mut self.app.typeface.terminal_menu;
            if state == clear_ui::widget::ElementState::Pressed && !menu.hit_test(lx, ly) { menu.unfocus(); }
            if menu.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && menu.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetTerminalMenu(menu.selected)));
            }

            let tb = &mut self.app.typeface.terminal_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetTerminal(tb.text.clone())));
            }

            let tb = &mut self.app.typeface.search_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if state == clear_ui::widget::ElementState::Pressed && tb.take_change() {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSearch(tb.text.clone())));
            }

            let sb = &mut self.app.typeface.borders_size_box;
            if state == clear_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetBordersSize(sb.value)));
            }

            let sb = &mut self.app.typeface.status_size_box;
            if state == clear_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetStatusSize(sb.value)));
            }

            let sb = &mut self.app.typeface.fuzzel_size_box;
            if state == clear_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetFuzzelSize(sb.value)));
            }

            let sb = &mut self.app.typeface.terminal_size_box;
            if state == clear_ui::widget::ElementState::Pressed && !sb.hit_test(lx, ly) { sb.unfocus(); }
            let old_val = sb.value;
            if sb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            if sb.value != old_val {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetTerminalSize(sb.value)));
            }

            let tf = &mut self.app.typeface;
            if tf.list_box.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
        }
        if self.app.current_page == Page::Services {
            let tb = &mut self.app.services.search_box;
            if state == clear_ui::widget::ElementState::Pressed && !tb.hit_test(lx, ly) { tb.unfocus(); }
            if tb.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
            let srv = &mut self.app.services;
            if srv.list_box.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Hardware {
            let hw = &mut self.app.hardware;
            if hw.cpu_list_box.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
        }
        if state == clear_ui::widget::ElementState::Pressed && self.app.current_page == Page::Radios {
            let net = &mut self.app.network;
            if net.wifi_list_box.mouse_input(button, state, lx, ly) {
                self.needs_rebuild = true;
            }
        }
        for a in &actions {
            self.handle_action(a);
        }
        if !actions.is_empty() {
            self.needs_rebuild = true;
            return true;
        }
        self.needs_rebuild = true;
        true
    }

    fn handle_mouse_wheel(&mut self, delta: &clear_ui::widget::MouseScrollDelta) -> bool {
        let s = self.scale_factor as f32;
        if self.cursor_x >= self.sidebar_width * s {
            let lx = self.cursor_x / s;
            let ly = self.cursor_y / s + self.scroll_y;
            
            if self.app.current_page == Page::Input {
                let input = &self.app.input;
                if lx >= input.trackpad_x && lx <= input.trackpad_x + input.trackpad_w
                    && ly >= input.trackpad_y && ly <= input.trackpad_y + input.trackpad_h
                {
                    return true;
                }
            }

            if self.app.current_page == Page::Typefaces {
                let tf = &mut self.app.typeface;
                if tf.list_box.mouse_wheel(delta, lx, ly) {
                    self.needs_rebuild = true;
                    return true;
                }
            }
            if self.app.current_page == Page::Services {
                let srv = &mut self.app.services;
                if srv.list_box.mouse_wheel(delta, lx, ly) {
                    self.needs_rebuild = true;
                    return true;
                }
            }
            if self.app.current_page == Page::Hardware {
                let hw = &mut self.app.hardware;
                if hw.cpu_list_box.mouse_wheel(delta, lx, ly) {
                    self.needs_rebuild = true;
                    return true;
                }
            }
            if self.app.current_page == Page::Radios {
                let net = &mut self.app.network;
                if net.wifi_list_box.mouse_wheel(delta, lx, ly) {
                    self.needs_rebuild = true;
                    return true;
                }
            }

            let scroll_speed = 24.0;
            let dy = match delta {
                clear_ui::widget::MouseScrollDelta::LineDelta(_, y) => -y * scroll_speed,
                clear_ui::widget::MouseScrollDelta::PixelDelta(pos) => -pos.y as f32,
            };
            let old_scroll = self.scroll_y;
            self.scroll_y = (self.scroll_y + dy).max(0.0).min(self.max_scroll_y);
            if (self.scroll_y - old_scroll).abs() > 0.01 {
                self.needs_rebuild = true;
                return true;
            }
        }
        false
    }

    fn get_page_root_widget(&mut self) -> Option<*mut (dyn clear_ui::widget::Widget + 'static)> {
        match self.app.current_page {
            Page::Typefaces | Page::Services | Page::Hardware | Page::Radios |
            Page::Layout | Page::Colors | Page::Notifications | Page::Input |
            Page::Audio | Page::Display => {
                let ptr = &mut self.page_root_container as &mut dyn clear_ui::widget::Widget as *mut dyn clear_ui::widget::Widget;
                let static_ptr = unsafe {
                    std::mem::transmute::<*mut dyn clear_ui::widget::Widget, *mut (dyn clear_ui::widget::Widget + 'static)>(ptr)
                };
                Some(static_ptr)
            }
            _ => None,
        }
    }

    fn handle_key_input(&mut self, event: &clear_ui::widget::KeyEvent) -> bool {
        if event.state == clear_ui::widget::ElementState::Pressed && !event.repeat {
            let is_nav_key = match (&event.logical_key, event.ctrl) {
                (clear_ui::widget::Key::Character(c), true) if c == "j" || c == "J" || c == "k" || c == "K" || c == "u" || c == "U" || c == "i" || c == "I" => true,
                _ => false,
            };
            if is_nav_key {
                if clear_ui::widget::focus::has_focus() {
                    if clear_ui::widget::focus::navigate_focus(&event.logical_key, event.ctrl) {
                        self.needs_rebuild = true;
                        return true;
                    }
                } else {
                    if let Some(root_ptr) = self.get_page_root_widget() {
                        unsafe {
                            let root_ref = &mut *root_ptr;
                            clear_ui::widget::focus::set_focused(root_ref);
                            root_ref.focus();
                            self.needs_rebuild = true;
                            return true;
                        }
                    }
                }
            }
        }

        if self.app.current_page == Page::Layout {
            let mut changed = false;
            let mut actions = Vec::new();
            for (i, sb) in self.app.layout.spinboxes.iter_mut().enumerate() {
                let old = sb.value;
                if sb.keyboard_input(event) {
                    if sb.value != old {
                        actions.push(AppAction::Layout(
                            pages::layout::LayoutMessage::SetWidth(
                                pages::layout::WidthParam::ALL[i],
                                sb.value as u16,
                            )
                        ));
                    }
                    changed = true;
                }
            }
            let sb = &mut self.app.layout.cascade_offset_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetCascadeOffset(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.edge_gap_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetEdgeGap(sb.value as u16)
                    ));
                }
                changed = true;
            }
            let sb = &mut self.app.layout.top_gap_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                if sb.value != old {
                    actions.push(AppAction::Layout(
                        pages::layout::LayoutMessage::SetTopGap(sb.value as u16)
                    ));
                }
                changed = true;
            }
            for a in &actions {
                self.handle_action(a);
            }
            if changed {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Colors {
            let mut changed = false;
            let mut actions = Vec::new();
            for (i, cp) in self.app.colors.color_selectors.iter_mut().enumerate() {
                let old = cp.color;
                if cp.keyboard_input(event) {
                    if cp.color != old {
                        actions.push(AppAction::Colors(match i {
                            0 => pages::colors::ColorsMessage::SetLowColor(cp.color),
                            1 => pages::colors::ColorsMessage::SetHighColor(cp.color),
                            _ => pages::colors::ColorsMessage::SetDisabledColor(cp.color),
                        }));
                    }
                    changed = true;
                }
            }
            for a in &actions {
                self.handle_action(a);
            }
            if changed {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Notifications {
            let sb = &mut self.app.notifications.duration_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                let new_val = sb.value;
                drop(sb);
                if new_val != old {
                    self.handle_action(&AppAction::Notifications(pages::notifications::NotificationsMessage::SetDuration(new_val)));
                }
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Input {
            if self.app.input.rate_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.delay_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyRepeat));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.scroll_friction_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyScrollFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.pointer_friction_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyPointerFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.trackpad_friction_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyTrackpadFriction));
                self.needs_rebuild = true;
                return true;
            }
            if self.app.input.trackpoint_accel_speed_spinbox.keyboard_input(event) {
                self.handle_action(&AppAction::Input(pages::input::InputMessage::ApplyTrackpointAccelSpeed));
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Audio {
            let mut actions = Vec::new();
            for (i, sb) in self.app.audio.sink_spinboxes.iter_mut().enumerate() {
                let old = sb.value;
                if sb.keyboard_input(event) {
                    if sb.value != old {
                        let id = self.app.audio.sinks[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SinkVolume(id, sb.value as f32 / 100.0)));
                    }
                }
            }
            for (i, sb) in self.app.audio.source_spinboxes.iter_mut().enumerate() {
                let old = sb.value;
                if sb.keyboard_input(event) {
                    if sb.value != old {
                        let id = self.app.audio.sources[i].id;
                        actions.push(AppAction::Audio(pages::audio::AudioMessage::SourceVolume(id, sb.value as f32 / 100.0)));
                    }
                }
            }
            for a in &actions {
                self.handle_action(a);
            }
            if !actions.is_empty() {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Display {
            let sb = &mut self.app.display.brightness_spinbox;
            let old = sb.value;
            if sb.keyboard_input(event) {
                let new_val = sb.value;
                drop(sb);
                if new_val != old {
                    self.handle_action(&AppAction::Display(pages::display::DisplayMessage::BrightnessSet(new_val as u32)));
                }
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Typefaces {
            if event.state == clear_ui::widget::ElementState::Pressed {
                let is_down = match (&event.logical_key, event.ctrl) {
                    (clear_ui::widget::Key::Character(c), true) if c == "n" || c == "N" => true,
                    (clear_ui::widget::Key::Named(clear_ui::widget::NamedKey::ArrowDown), false) => true,
                    _ => false,
                };
                let is_up = match (&event.logical_key, event.ctrl) {
                    (clear_ui::widget::Key::Character(c), true) if c == "p" || c == "P" => true,
                    (clear_ui::widget::Key::Named(clear_ui::widget::NamedKey::ArrowUp), false) => true,
                    _ => false,
                };
                if is_down && clear_ui::widget::focus::is_focused(&self.app.typeface.list_box.scroll_box) {
                    let next_idx_font_scroll = {
                        let tf = &self.app.typeface;
                        let query = tf.search_box.text.to_lowercase();
                        let matching_fonts: Vec<&String> = tf.all_fonts.iter()
                            .filter(|font| font.to_lowercase().contains(&query))
                            .collect();
                        if !matching_fonts.is_empty() {
                            let current_idx = tf.selected_font.as_ref()
                                .and_then(|f| matching_fonts.iter().position(|&x| x == f));
                            let next_idx = match current_idx {
                                Some(idx) => (idx + 1).min(matching_fonts.len() - 1),
                                None => 0,
                            };
                            let font = matching_fonts[next_idx].clone();
                            
                            // Compute scroll
                            let btn_h = 24.0;
                            let btn_gap = 4.0;
                            let item_height_full = btn_h + btn_gap;
                            let item_y = next_idx as f32 * item_height_full;
                            let list_box_h = 320.0;
                            
                            let mut scroll_y = tf.list_box.scroll_y();
                            if item_y < scroll_y {
                                scroll_y = item_y;
                            } else if item_y + btn_h > scroll_y + list_box_h {
                                scroll_y = item_y + btn_h - list_box_h;
                            }
                            Some((font, scroll_y))
                        } else {
                            None
                        }
                    };

                    if let Some((font, scroll_y)) = next_idx_font_scroll {
                        self.app.typeface.list_box.set_scroll_y(scroll_y);
                        self.handle_action(&AppAction::Typeface(pages::typeface::TypefaceMessage::SelectFont(font)));
                        self.needs_rebuild = true;
                        return true;
                    }
                } else if is_up && clear_ui::widget::focus::is_focused(&self.app.typeface.list_box.scroll_box) {
                    let next_idx_font_scroll = {
                        let tf = &self.app.typeface;
                        let query = tf.search_box.text.to_lowercase();
                        let matching_fonts: Vec<&String> = tf.all_fonts.iter()
                            .filter(|font| font.to_lowercase().contains(&query))
                            .collect();
                        if !matching_fonts.is_empty() {
                            let current_idx = tf.selected_font.as_ref()
                                .and_then(|f| matching_fonts.iter().position(|&x| x == f));
                            let next_idx = match current_idx {
                                Some(idx) => idx.saturating_sub(1),
                                None => 0,
                            };
                            let font = matching_fonts[next_idx].clone();
                            
                            // Compute scroll
                            let btn_h = 24.0;
                            let btn_gap = 4.0;
                            let item_height_full = btn_h + btn_gap;
                            let item_y = next_idx as f32 * item_height_full;
                            let list_box_h = 320.0;
                            
                            let mut scroll_y = tf.list_box.scroll_y();
                            if item_y < scroll_y {
                                scroll_y = item_y;
                            } else if item_y + btn_h > scroll_y + list_box_h {
                                scroll_y = item_y + btn_h - list_box_h;
                            }
                            Some((font, scroll_y))
                        } else {
                            None
                        }
                    };

                    if let Some((font, scroll_y)) = next_idx_font_scroll {
                        self.app.typeface.list_box.set_scroll_y(scroll_y);
                        self.handle_action(&AppAction::Typeface(pages::typeface::TypefaceMessage::SelectFont(font)));
                        self.needs_rebuild = true;
                        return true;
                    }
                }
            }

            let mut actions = Vec::new();
            let mut consumed = false;
            
            let tf = &mut self.app.typeface;
            let tb = &mut tf.sans_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSans(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.serif_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSerif(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.mono_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetMono(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.borders_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetBorders(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.status_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetStatus(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.fuzzel_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetFuzzel(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.terminal_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetTerminal(tb.text.clone())));
                }
                consumed = true;
            }

            let tb = &mut self.app.typeface.search_box;
            if tb.keyboard_input(event) {
                if tb.take_change() {
                    actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetSearch(tb.text.clone())));
                }
                consumed = true;
            }

            let sb = &mut self.app.typeface.borders_size_box;
            if sb.keyboard_input(event) {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetBordersSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.typeface.status_size_box;
            if sb.keyboard_input(event) {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetStatusSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.typeface.fuzzel_size_box;
            if sb.keyboard_input(event) {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetFuzzelSize(sb.value)));
                consumed = true;
            }

            let sb = &mut self.app.typeface.terminal_size_box;
            if sb.keyboard_input(event) {
                actions.push(AppAction::Typeface(pages::typeface::TypefaceMessage::SetTerminalSize(sb.value)));
                consumed = true;
            }
            
            for a in &actions {
                self.handle_action(a);
            }
            if consumed {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Services {
            let srv = &mut self.app.services;
            if srv.list_box.keyboard_input(event) {
                self.needs_rebuild = true;
                return true;
            }
            let tb = &mut srv.search_box;
            if tb.keyboard_input(event) {
                tb.take_change();
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Hardware {
            let hw = &mut self.app.hardware;
            if hw.cpu_list_box.keyboard_input(event) {
                self.needs_rebuild = true;
                return true;
            }
        }
        if self.app.current_page == Page::Radios {
            let net = &mut self.app.network;
            if net.wifi_list_box.keyboard_input(event) {
                self.needs_rebuild = true;
                return true;
            }
        }
        false
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

        let output = match self.wgpu_surface.get_current_texture() {
            Ok(t) => t,
            Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                self.wgpu_surface.configure(&self.device, &self.config);
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
        output.present();
    }
}

struct PressedKey {
    logical_key: clear_ui::widget::Key,
    text: Option<String>,
    first_pressed: std::time::Instant,
    last_repeated: std::time::Instant,
}

fn is_repeatable_key(key: &clear_ui::widget::Key) -> bool {
    use clear_ui::widget::{Key, NamedKey};
    match key {
        Key::Named(NamedKey::Backspace) |
        Key::Named(NamedKey::Delete) |
        Key::Named(NamedKey::ArrowLeft) |
        Key::Named(NamedKey::ArrowRight) |
        Key::Named(NamedKey::ArrowUp) |
        Key::Named(NamedKey::ArrowDown) |
        Key::Named(NamedKey::Home) |
        Key::Named(NamedKey::End) |
        Key::Character(_) => true,
        _ => false,
    }
}

struct App {
    registry_state: RegistryState,
    compositor_state: CompositorState,
    xdg_shell_state: XdgShell,
    shm_state: Shm,
    seat_state: SeatState,
    output_state: OutputState,

    seats: Vec<wl_seat::WlSeat>,
    pointer: Option<wl_pointer::WlPointer>,
    keyboard: Option<wl_keyboard::WlKeyboard>,

    window: Option<XdgWindow>,
    surface: Option<wl_surface::WlSurface>,

    state: Option<SystemInterface>,
    exit: bool,
    redraw: bool,
    ctrl_pressed: bool,
    shift_pressed: bool,
    pressed_key: Option<PressedKey>,
}



impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        scale_factor: i32,
    ) {
        _surface.set_buffer_scale(scale_factor);
        if let Some(state) = &mut self.state {
            let old_scale = state.scale_factor;
            state.scale_factor = scale_factor as f64;
            let logical_w = state.width as f64 / old_scale;
            let logical_h = state.height as f64 / old_scale;
            let pw = (logical_w * state.scale_factor) as u32;
            let ph = (logical_h * state.scale_factor) as u32;
            state.resize(pw, ph);
        }
        self.redraw = true;
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {}

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {}

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {}

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {}
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        self.seats.push(seat);
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            let pointer = self.seat_state.get_pointer(qh, &seat).unwrap();
            self.pointer = Some(pointer);
        }
        if capability == Capability::Keyboard && self.keyboard.is_none() {
            let keyboard = self.seat_state.get_keyboard(qh, &seat, None).unwrap();
            self.keyboard = Some(keyboard);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer {
            self.pointer = None;
        }
        if capability == Capability::Keyboard {
            self.keyboard = None;
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        self.seats.retain(|s| s != &seat);
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm_state
    }
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[smithay_client_toolkit::seat::pointer::PointerEvent],
    ) {
        use smithay_client_toolkit::seat::pointer::PointerEventKind;
        for event in events {
            if let Some(st) = &mut self.state {
                let (cx, cy) = clear_ui::wayland::scale_pointer_pos(event.position, st.scale_factor);
                st.cursor_x = cx;
                st.cursor_y = cy;
            }
            match &event.kind {
                PointerEventKind::Motion { .. } => {
                    if let Some(st) = &mut self.state {
                        let cx = st.cursor_x;
                        let cy = st.cursor_y;
                        if st.handle_cursor_moved(cx, cy) {
                            self.redraw = true;
                        }
                    }
                }
                PointerEventKind::Press { button, .. } => {
                    let custom_btn = match button {
                        272 => clear_ui::widget::MouseButton::Left,
                        273 => clear_ui::widget::MouseButton::Right,
                        _ => continue,
                    };
                    if let Some(st) = &mut self.state {
                        if st.handle_mouse_input(custom_btn, clear_ui::widget::ElementState::Pressed) {
                            self.redraw = true;
                        }
                    }
                }
                PointerEventKind::Release { button, .. } => {
                    let custom_btn = match button {
                        272 => clear_ui::widget::MouseButton::Left,
                        273 => clear_ui::widget::MouseButton::Right,
                        _ => continue,
                    };
                    if let Some(st) = &mut self.state {
                        if st.handle_mouse_input(custom_btn, clear_ui::widget::ElementState::Released) {
                            self.redraw = true;
                        }
                    }
                }
                PointerEventKind::Axis { horizontal, vertical, .. } => {
                    let h_scroll = horizontal.absolute as f32;
                    let v_scroll = vertical.absolute as f32;
                    let delta = clear_ui::widget::MouseScrollDelta::LineDelta(-h_scroll / 10.0, -v_scroll / 10.0);
                    if let Some(st) = &mut self.state {
                        if st.handle_mouse_wheel(&delta) {
                            self.redraw = true;
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

impl KeyboardHandler for App {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw_modifiers: &[u32],
        _keysyms: &[xkeysym::Keysym],
    ) {}

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {}

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        self.handle_key(event, clear_ui::widget::ElementState::Pressed);
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: smithay_client_toolkit::seat::keyboard::KeyEvent,
    ) {
        self.handle_key(event, clear_ui::widget::ElementState::Released);
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: smithay_client_toolkit::seat::keyboard::Modifiers,
        _layout: u32,
    ) {
        self.ctrl_pressed = modifiers.ctrl;
        self.shift_pressed = modifiers.shift;
    }
}

impl App {
    fn handle_key(&mut self, event: smithay_client_toolkit::seat::keyboard::KeyEvent, state: clear_ui::widget::ElementState) {
        use clear_ui::widget::{Key, KeyEvent, NamedKey};
        let logical_key = match event.keysym {
            xkeysym::Keysym::Escape => Key::Named(NamedKey::Escape),
            xkeysym::Keysym::Return => Key::Named(NamedKey::Enter),
            xkeysym::Keysym::BackSpace => Key::Named(NamedKey::Backspace),
            xkeysym::Keysym::Down => Key::Named(NamedKey::ArrowDown),
            xkeysym::Keysym::Up => Key::Named(NamedKey::ArrowUp),
            xkeysym::Keysym::Left => Key::Named(NamedKey::ArrowLeft),
            xkeysym::Keysym::Right => Key::Named(NamedKey::ArrowRight),
            xkeysym::Keysym::Tab => Key::Named(NamedKey::Tab),
            xkeysym::Keysym::Delete => Key::Named(NamedKey::Delete),
            xkeysym::Keysym::space => Key::Named(NamedKey::Space),
            xkeysym::Keysym::j | xkeysym::Keysym::J => Key::Character("j".to_string()),
            xkeysym::Keysym::k | xkeysym::Keysym::K => Key::Character("k".to_string()),
            xkeysym::Keysym::u | xkeysym::Keysym::U => Key::Character("u".to_string()),
            xkeysym::Keysym::i | xkeysym::Keysym::I => Key::Character("i".to_string()),
            xkeysym::Keysym::n | xkeysym::Keysym::N => Key::Character("n".to_string()),
            xkeysym::Keysym::p | xkeysym::Keysym::P => Key::Character("p".to_string()),
            _ => {
                if let Some(ref text) = event.utf8 {
                    Key::Character(text.clone())
                } else {
                    return;
                }
            }
        };

        let custom_event = KeyEvent {
            state,
            logical_key,
            text: event.utf8.clone(),
            repeat: false,
            ctrl: self.ctrl_pressed,
            shift: self.shift_pressed,
        };

        if state == clear_ui::widget::ElementState::Pressed {
            if is_repeatable_key(&custom_event.logical_key) {
                self.pressed_key = Some(PressedKey {
                    logical_key: custom_event.logical_key.clone(),
                    text: custom_event.text.clone(),
                    first_pressed: std::time::Instant::now(),
                    last_repeated: std::time::Instant::now(),
                });
            } else {
                self.pressed_key = None;
            }
        } else if state == clear_ui::widget::ElementState::Released {
            if let Some(ref pk) = self.pressed_key {
                if pk.logical_key == custom_event.logical_key {
                    self.pressed_key = None;
                }
            }
        }

        if let Some(st) = &mut self.state {
            if st.handle_key_input(&custom_event) {
                self.redraw = true;
            }
        }
    }
}

impl WindowHandler for App {
    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &XdgWindow,
        configure: WindowConfigure,
        _serial: u32,
    ) {
        let (w, h) = configure.new_size;
        if let (Some(w), Some(h)) = (w, h) {
            let width = w.get();
            let height = h.get();
            if let Some(state) = &mut self.state {
                let pw = (width as f64 * state.scale_factor) as u32;
                let ph = (height as f64 * state.scale_factor) as u32;
                state.resize(pw, ph);
            }
        }
        self.redraw = true;
    }

    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &XdgWindow) {
        self.exit = true;
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    
    fn runtime_add_global(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _name: u32,
        _interface: &str,
        _version: u32,
    ) {}
    
    fn runtime_remove_global(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _name: u32,
        _interface: &str,
    ) {}
}

delegate_compositor!(App);
delegate_xdg_shell!(App);
delegate_xdg_window!(App);
delegate_shm!(App);
delegate_seat!(App);
delegate_pointer!(App);
delegate_keyboard!(App);
delegate_registry!(App);
delegate_output!(App);

fn main() {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let _guard = rt.enter();

    let mut initial_page = Page::ALL[0];
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let arg = args.last().unwrap().to_lowercase();
        for page in Page::ALL {
            if page.label().to_lowercase() == arg {
                initial_page = page;
                break;
            }
        }
    }

    let conn = Connection::connect_to_env().unwrap();
    let (globals, mut event_queue) = registry_queue_init(&conn).unwrap();
    let qh = event_queue.handle();

    let compositor_state = CompositorState::bind(&globals, &qh).unwrap();
    let xdg_shell_state = XdgShell::bind(&globals, &qh).unwrap();
    let shm_state = Shm::bind(&globals, &qh).unwrap();
    let seat_state = SeatState::new(&globals, &qh);
    let output_state = OutputState::new(&globals, &qh);

    let mut app = App {
        registry_state: RegistryState::new(&globals),
        compositor_state,
        xdg_shell_state,
        shm_state,
        seat_state,
        output_state,
        seats: Vec::new(),
        pointer: None,
        keyboard: None,
        window: None,
        surface: None,
        state: None,
        exit: false,
        redraw: true,
        ctrl_pressed: false,
        shift_pressed: false,
        pressed_key: None,
    };

    // Perform a roundtrip to populate output_state with active output scales
    event_queue.roundtrip(&mut app).unwrap();

    let scale = clear_ui::wayland::detect_scale_factor(&app.output_state);

    let pw = (820.0 * scale) as u32;
    let ph = (680.0 * scale) as u32;

    let mut state = pollster::block_on(SystemInterface::new(
        &conn,
        &qh,
        &app.compositor_state,
        &app.xdg_shell_state,
        pw,
        ph,
        scale,
    ));
    state.app.current_page = initial_page;
    app.window = Some(state.window.clone());
    app.surface = Some(state.surface.clone());
    app.state = Some(state);

    let mut event_loop = EventLoop::try_new().unwrap();
    let loop_handle = event_loop.handle();
    WaylandSource::new(conn, event_queue).insert(loop_handle.clone()).unwrap();

    const KEY_REPEAT_DELAY: std::time::Duration = std::time::Duration::from_millis(500);
    const KEY_REPEAT_INTERVAL: std::time::Duration = std::time::Duration::from_millis(50);

    loop {
        event_loop
            .dispatch(std::time::Duration::from_millis(16), &mut app)
            .unwrap();
        if app.exit {
            break;
        }
        if let Some(state) = &mut app.state {
            state.poll_background_updates();
            if state.needs_rebuild {
                app.redraw = true;
            }
        }

        if let Some(ref mut pk) = app.pressed_key {
            let now = std::time::Instant::now();
            if now.duration_since(pk.first_pressed) >= KEY_REPEAT_DELAY {
                if now.duration_since(pk.last_repeated) >= KEY_REPEAT_INTERVAL {
                    pk.last_repeated = now;
                    let custom_event = clear_ui::widget::KeyEvent {
                        state: clear_ui::widget::ElementState::Pressed,
                        logical_key: pk.logical_key.clone(),
                        text: pk.text.clone(),
                        repeat: true,
                        ctrl: app.ctrl_pressed,
                        shift: app.shift_pressed,
                    };
                    if let Some(st) = &mut app.state {
                        if st.handle_key_input(&custom_event) {
                            app.redraw = true;
                        }
                    }
                }
            }
        }

        if app.redraw {
            app.redraw = false;
            if let Some(state) = &mut app.state {
                state.render();
            }
        }
    }
}

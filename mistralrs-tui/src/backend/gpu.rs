//! GPU accelerated renderer built on top of wgpu + winit.
//!
//! The current implementation renders the Ratatui buffer into a monochrome text surface using
//! [`glyphon`]. Colour styling is intentionally coarse for now (default foreground on opaque
//! background) but the event loop, redraw pipeline and adapter negotiation are in place so future
//! visual upgrades can focus purely on richer painting logic.
//!
//! ### Follow-up work
//! - Wire this backend into a palette-aware renderer so Ratatui colours and styling survive the
//!   text-to-texture conversion.
//! - Harden capability negotiation: the current `unwrap_or` fallbacks assume the adapter reports at
//!   least one surface format/present mode/alpha mode.
//! - Replace the raw pointer bridge between the winit loop and the [`App`] with a safe interior
//!   mutability strategy (e.g. `Rc<RefCell<_>>` or a purpose-built channel).
//! - Share glyph caches and staging buffers across window resizes to avoid stutter once richer
//!   painting is added.

#![cfg(feature = "gpu")]

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{anyhow, Context, Result};
use glyphon::{
    Attrs, Buffer as GlyphBuffer, Color as GlyphColor, Family, FontSystem, Metrics, Resolution,
    Shaping, SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Wrap,
};
use pollster::block_on;
use ratatui::{backend::TestBackend, layout::Rect, Terminal, TerminalOptions, Viewport};
use tokio::runtime::Runtime;
use tracing::{debug, error, info, warn};
use wgpu::{
    CommandEncoderDescriptor, CompositeAlphaMode, DeviceDescriptor, Features, Instance,
    InstanceDescriptor, Limits, LoadOp, MultisampleState, Operations, PresentMode, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, RequestAdapterOptions, StoreOp, Surface,
    SurfaceConfiguration, SurfaceError, TextureUsages, TextureViewDescriptor,
};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    platform::run_on_demand::EventLoopExtRunOnDemand,
    window::{Window, WindowBuilder, WindowId},
};

use crate::{
    app::App,
    backend::Options,
    input::{self, InputEvent, KeyCode, KeyEvent, Modifiers},
};

/// Width of a single text cell in pixels used to derive the Ratatui grid dimensions.
const CELL_WIDTH: u32 = 10;
/// Height of a single text cell in pixels used to derive the Ratatui grid dimensions.
const CELL_HEIGHT: u32 = 20;
/// Font size used for glyphon layout (monospace).
const FONT_SIZE: f32 = 16.0;
/// Line height passed to glyphon so lines align with the Ratatui grid.
const FONT_LINE_HEIGHT: f32 = 22.0;

/// Launch the GPU backend.
pub fn run(runtime: &Runtime, app: &mut App, options: &Options) -> Result<()> {
    let instance = Instance::new(InstanceDescriptor::default());
    let mut event_loop = EventLoop::new().context("creating winit event loop")?;

    let runtime_ptr: *const Runtime = runtime;
    let app_ptr: *mut App = app;
    // SAFETY: the raw pointer bridge is temporary; the event loop never outlives the runtime nor
    // the app reference. A follow-up should wrap `App` inside a thread-safe container to avoid
    // direct pointer manipulation here.

    let mut state = RenderState::new(instance, options.tick_rate);

    event_loop
        .run_on_demand(move |event, elwt| {
            if let Err(err) = state.process_event(event, elwt, runtime_ptr, app_ptr) {
                error!(?err, "GPU backend crashed; exiting event loop");
                elwt.exit();
            }
        })
        .map_err(|err| anyhow!("winit event loop error: {err}"))?;

    Ok(())
}

struct RenderState {
    instance: Instance,
    tick_rate: Duration,
    last_tick: Instant,
    needs_redraw: bool,
    keyboard_modifiers: Modifiers,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    renderer: Option<Renderer>,
    terminal: Option<Terminal<TestBackend>>,
}

impl RenderState {
    fn new(instance: Instance, tick_rate: Duration) -> Self {
        Self {
            instance,
            tick_rate,
            last_tick: Instant::now(),
            needs_redraw: false,
            keyboard_modifiers: Modifiers::NONE,
            window: None,
            window_id: None,
            renderer: None,
            terminal: None,
        }
    }

    fn process_event(
        &mut self,
        event: Event<()>,
        elwt: &winit::event_loop::EventLoopWindowTarget<()>,
        runtime: *const Runtime,
        app: *mut App,
    ) -> Result<()> {
        match event {
            Event::Resumed => {
                self.ensure_window(elwt)
                    .context("initialising GPU window and surface")?;
                self.needs_redraw = true;
            }
            Event::Suspended => {
                debug!("Event loop suspended");
            }
            Event::WindowEvent { window_id, event } if Some(window_id) == self.window_id => {
                self.handle_window_event(event, elwt, runtime, app)?;
            }
            Event::AboutToWait => {
                if self.maybe_tick(runtime, app)? {
                    self.needs_redraw = true;
                }
                if self.needs_redraw {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                    self.needs_redraw = false;
                }
                if unsafe { (&*app).should_quit() } {
                    elwt.exit();
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn ensure_window(&mut self, elwt: &winit::event_loop::EventLoopWindowTarget<()>) -> Result<()> {
        if self.window.is_some() {
            return Ok(());
        }

        let window = Arc::new(
            WindowBuilder::new()
                .with_title("mistralrs-tui (GPU preview)")
                .with_visible(true)
                .build(elwt)?,
        );
        let window_id = window.id();
        let size = window.inner_size();

        let renderer = Renderer::new(&self.instance, window.clone(), size)
            .context("initialising wgpu renderer")?;

        let (cols, rows) = grid_from_size(size);
        let backend = TestBackend::new(cols, rows);
        let viewport = Viewport::Fixed(Rect::new(0, 0, cols, rows));
        let terminal = Terminal::with_options(backend, TerminalOptions { viewport })
            .context("building Ratatui terminal for GPU backend")?;

        info!(
            width = size.width,
            height = size.height,
            cols,
            rows,
            "GPU backend initialised"
        );

        self.window = Some(window);
        self.window_id = Some(window_id);
        self.renderer = Some(renderer);
        self.terminal = Some(terminal);
        self.last_tick = Instant::now();

        Ok(())
    }

    fn handle_window_event(
        &mut self,
        event: WindowEvent,
        elwt: &winit::event_loop::EventLoopWindowTarget<()>,
        runtime: *const Runtime,
        app: *mut App,
    ) -> Result<()> {
        match event {
            WindowEvent::CloseRequested => {
                self.dispatch_input(
                    InputEvent::Key(KeyEvent::new(KeyCode::Char('q'), Modifiers::default())),
                    runtime,
                    app,
                )?;
                self.needs_redraw = true;
                elwt.exit();
            }
            WindowEvent::Resized(size) => {
                self.resize(size)?;
                self.needs_redraw = true;
            }
            WindowEvent::ScaleFactorChanged {
                mut inner_size_writer,
                ..
            } => {
                if let Some(window) = &self.window {
                    let size = window.inner_size();
                    if let Err(err) = inner_size_writer.request_inner_size(size) {
                        warn!(?err, "failed to commit new inner size after scale change");
                    }
                    self.resize(size)?;
                    self.needs_redraw = true;
                }
            }
            WindowEvent::ModifiersChanged(state) => {
                self.keyboard_modifiers = map_modifiers(&state);
            }
            WindowEvent::RedrawRequested => {
                self.render(app)?;
            }
            _ => {
                if let Some(evt) = input::from_winit(&event, self.keyboard_modifiers) {
                    self.dispatch_input(evt, runtime, app)?;
                    self.needs_redraw = true;
                }
            }
        }

        if unsafe { (&*app).should_quit() } {
            elwt.exit();
        }

        Ok(())
    }

    fn dispatch_input(
        &mut self,
        event: InputEvent,
        runtime: *const Runtime,
        app: *mut App,
    ) -> Result<()> {
        unsafe {
            let runtime = &*runtime;
            let app = &mut *app;
            app.handle_event(event, runtime)
                .context("processing input event in GPU backend")?
        }
        Ok(())
    }

    fn maybe_tick(&mut self, runtime: *const Runtime, app: *mut App) -> Result<bool> {
        if self.last_tick.elapsed() >= self.tick_rate {
            unsafe {
                let runtime = &*runtime;
                let app = &mut *app;
                app.tick(runtime)
                    .context("processing tick in GPU backend")?;
            }
            self.last_tick = Instant::now();
            return Ok(true);
        }
        Ok(false)
    }

    fn resize(&mut self, size: PhysicalSize<u32>) -> Result<()> {
        let Some(renderer) = self.renderer.as_mut() else {
            return Ok(());
        };
        renderer.resize(size);

        if let Some(terminal) = self.terminal.as_mut() {
            let (cols, rows) = grid_from_size(size);
            terminal.backend_mut().resize(cols, rows);
        }

        Ok(())
    }

    fn render(&mut self, app: *mut App) -> Result<()> {
        let renderer = self
            .renderer
            .as_mut()
            .context("renderer not initialised before redraw")?;
        let terminal = self
            .terminal
            .as_mut()
            .context("terminal not initialised before redraw")?;

        unsafe {
            let app = &mut *app;
            terminal
                .draw(|frame| crate::ui::render(frame, app))
                .context("drawing Ratatui buffer in GPU backend")?;
        }

        let buffer_text = buffer_to_string(terminal.backend().buffer());

        renderer
            .draw(&buffer_text)
            .context("submitting GPU frame")?;

        if let Some(window) = &self.window {
            unsafe {
                let app = &*app;
                window.set_title(&format!(
                    "mistralrs-tui (GPU preview) — {}",
                    app.status_line()
                ));
            }
        }

        Ok(())
    }
}

fn map_modifiers(state: &winit::keyboard::ModifiersState) -> Modifiers {
    Modifiers {
        control: state.control_key(),
        alt: state.alt_key(),
        shift: state.shift_key(),
    }
}

struct Renderer {
    surface: Surface<'static>,
    device: wgpu::Device,
    queue: Queue,
    config: SurfaceConfiguration,
    font_system: FontSystem,
    cache: SwashCache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    text_buffer: GlyphBuffer,
}

impl Renderer {
    fn new(instance: &Instance, window: Arc<Window>, size: PhysicalSize<u32>) -> Result<Self> {
        let surface = instance
            .create_surface(window.clone())
            .context("creating wgpu surface")?;

        let adapter = block_on(instance.request_adapter(&RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }))
        .context("requesting wgpu adapter")?;

        let (device, queue) = block_on(adapter.request_device(
            &DeviceDescriptor {
                label: Some("mistralrs-tui-gpu"),
                required_features: Features::empty(),
                required_limits: Limits::downlevel_defaults(),
            },
            None,
        ))
        .context("requesting wgpu device")?;

        let surface_caps = surface.get_capabilities(&adapter);
        // ROBUSTNESS: guard against drivers that expose zero capabilities and return a clear
        // user-facing error instead of panicking via `unwrap_or` defaults below.
        let format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let present_mode = surface_caps
            .present_modes
            .iter()
            .copied()
            .find(|mode| matches!(mode, PresentMode::AutoVsync | PresentMode::Fifo))
            .unwrap_or(PresentMode::Fifo);
        let alpha_mode = surface_caps
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(CompositeAlphaMode::Opaque);

        let config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![format],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        let mut font_system = FontSystem::new();
        let cache = SwashCache::new();
        let mut atlas = TextAtlas::new(&device, &queue, format);
        let text_renderer =
            TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);

        let mut text_buffer =
            GlyphBuffer::new(&mut font_system, Metrics::new(FONT_SIZE, FONT_LINE_HEIGHT));
        text_buffer.set_wrap(&mut font_system, Wrap::Glyph);
        text_buffer.set_size(&mut font_system, config.width as f32, config.height as f32);

        Ok(Self {
            surface,
            device,
            queue,
            config,
            font_system,
            cache,
            atlas,
            text_renderer,
            text_buffer,
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }
        self.config.width = size.width;
        self.config.height = size.height;
        self.text_buffer
            .set_size(&mut self.font_system, size.width as f32, size.height as f32);
        self.surface.configure(&self.device, &self.config);
    }

    fn draw(&mut self, text: &str) -> Result<()> {
        self.text_buffer.set_text(
            &mut self.font_system,
            text,
            Attrs::new().family(Family::Monospace),
            Shaping::Advanced,
        );
        self.text_buffer.shape_until_scroll(&mut self.font_system);

        self.text_renderer
            .prepare(
                &self.device,
                &self.queue,
                &mut self.font_system,
                &mut self.atlas,
                Resolution {
                    width: self.config.width,
                    height: self.config.height,
                },
                [TextArea {
                    buffer: &self.text_buffer,
                    left: 10.0,
                    top: 10.0,
                    scale: 1.0,
                    bounds: TextBounds::default(),
                    default_color: GlyphColor::rgb(240, 240, 240),
                }],
                &mut self.cache,
            )
            .context("preparing glyphon text")?;

        let frame = match self.surface.get_current_texture() {
            Ok(frame) => frame,
            Err(SurfaceError::Lost) => {
                warn!("surface lost; reconfiguring");
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            Err(SurfaceError::OutOfMemory) => return Err(anyhow!("GPU surface out of memory")),
            Err(err) => {
                warn!(?err, "surface error when fetching frame");
                return Ok(());
            }
        };

        let view = frame.texture.create_view(&TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("mistralrs-tui-gpu-encoder"),
            });

        {
            let mut pass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("mistralrs-tui-gpu-pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.text_renderer
                .render(&self.atlas, &mut pass)
                .context("rendering glyphon text")?;
        }

        self.queue.submit(Some(encoder.finish()));
        frame.present();
        self.atlas.trim();
        Ok(())
    }
}

fn grid_from_size(size: PhysicalSize<u32>) -> (u16, u16) {
    let cols = (size.width / CELL_WIDTH).max(1) as u16;
    let rows = (size.height / CELL_HEIGHT).max(1) as u16;
    (cols, rows)
}

fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let width = buffer.area.width;
    let height = buffer.area.height;
    let mut output = String::with_capacity((width as usize + 1) * height as usize);
    for y in 0..height {
        for x in 0..width {
            let cell = &buffer[(x, y)];
            output.push_str(cell.symbol());
        }
        if y + 1 < height {
            output.push('\n');
        }
    }
    output
}

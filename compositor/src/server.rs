//! A minimal Wayland server for `sphatik-comp` (WP 2.8).
//!
//! Brings up the globals a client needs to show a window — `wl_compositor`,
//! `wl_shm`, `xdg_wm_base` (xdg-shell), `wl_seat`, `wl_output` and
//! `zwp_linux_dmabuf_v1` — on a calloop event loop, hosts the output through
//! Smithay's winit backend, and composites a mapped toplevel full-screen with
//! the GLES renderer. A single toplevel is supported, which is all WP 2.8
//! needs ("a terminal or test app renders inside"); the shell and multiple
//! windows come later.
//!
//! `run` runs until the window closes. `shot` renders the first client frame,
//! writes it to a PNG and exits, which the golden test drives under Xvfb.
//! dmabuf is advertised with the renderer's formats; importing real dmabufs is
//! deferred to the phone (Stage 3), so for now a shm client is the path.
//!
//! Linux + `winit-backend` only (ADR 0008).

use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::damage::OutputDamageTracker;
use smithay::backend::renderer::element::surface::WaylandSurfaceRenderElement;
use smithay::backend::renderer::gles::GlesRenderer;
use smithay::backend::renderer::utils::on_commit_buffer_handler;
use smithay::backend::renderer::{Color32F, ExportMem, ImportDma};
use smithay::backend::winit::{self, WinitEvent};
use smithay::desktop::space::render_output;
use smithay::desktop::{Space, Window};
use smithay::input::pointer::CursorImageStatus;
use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::output::{Mode as OutputMode, Output, PhysicalProperties, Subpixel};
use smithay::reexports::calloop::generic::Generic;
use smithay::reexports::calloop::timer::{TimeoutAction, Timer};
use smithay::reexports::calloop::{
    EventLoop, Interest, LoopSignal, Mode as CalloopMode, PostAction,
};
use smithay::reexports::wayland_server::backend::{ClientData, ClientId, DisconnectReason};
use smithay::reexports::wayland_server::protocol::wl_buffer::WlBuffer;
use smithay::reexports::wayland_server::protocol::wl_seat::WlSeat;
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::{Client, Display, DisplayHandle, Resource};
use smithay::utils::{Buffer as BufferCoords, Rectangle, Serial, Transform};
use smithay::wayland::buffer::BufferHandler;
use smithay::wayland::compositor::{
    with_states, BufferAssignment, CompositorClientState, CompositorHandler, CompositorState,
    SurfaceAttributes,
};
use smithay::wayland::dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportNotifier};
use smithay::wayland::output::OutputManagerState;
use smithay::wayland::shell::xdg::{
    PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
    XdgToplevelSurfaceData,
};
use smithay::wayland::shm::{ShmHandler, ShmState};
use smithay::wayland::socket::ListeningSocketSource;

/// Per-client state Smithay stores on each connection.
#[derive(Default)]
struct ClientState {
    compositor: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

/// The compositor's Wayland state.
struct State {
    dh: DisplayHandle,
    compositor: CompositorState,
    shm: ShmState,
    xdg: XdgShellState,
    output_manager: OutputManagerState,
    seat_state: SeatState<Self>,
    #[allow(dead_code)]
    seat: Seat<Self>,
    dmabuf: DmabufState,
    space: Space<Window>,
    output: Output,
    loop_signal: LoopSignal,
    window: Option<Window>,
    committed: bool,
    start: Instant,
}

impl CompositorHandler for State {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client
            .get_data::<ClientState>()
            .expect("client has ClientState")
            .compositor
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);

        // Send the initial configure once the toplevel has made its first
        // commit, per the xdg-shell handshake.
        if let Some(window) = self.window.clone() {
            if let Some(toplevel) = window.toplevel() {
                if toplevel.wl_surface() == surface {
                    let configured = with_states(surface, |states| {
                        states
                            .data_map
                            .get::<XdgToplevelSurfaceData>()
                            .map(|d| d.lock().unwrap().initial_configure_sent)
                            .unwrap_or(false)
                    });
                    if !configured {
                        self.configure_fullscreen(toplevel);
                    }
                }
            }
        }

        let has_buffer = with_states(surface, |states| {
            matches!(
                states
                    .cached_state
                    .get::<SurfaceAttributes>()
                    .current()
                    .buffer,
                Some(BufferAssignment::NewBuffer(_))
            )
        });
        if has_buffer {
            self.committed = true;
        }

        self.space.refresh();
    }
}

impl State {
    /// Sizes a toplevel to the output and sends it a configure.
    fn configure_fullscreen(&self, toplevel: &ToplevelSurface) {
        let size = self
            .output
            .current_mode()
            .map(|m| (m.size.w, m.size.h))
            .unwrap_or((800, 600));
        toplevel.with_pending_state(|state| {
            state.size = Some((size.0, size.1).into());
        });
        toplevel.send_configure();
    }
}

impl BufferHandler for State {
    fn buffer_destroyed(&mut self, _buffer: &WlBuffer) {}
}

impl ShmHandler for State {
    fn shm_state(&self) -> &ShmState {
        &self.shm
    }
}

impl XdgShellHandler for State {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = Window::new_wayland_window(surface);
        self.space.map_element(window.clone(), (0, 0), false);
        self.window = Some(window);
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {}

    fn grab(&mut self, _surface: PopupSurface, _seat: WlSeat, _serial: Serial) {}

    fn reposition_request(
        &mut self,
        _surface: PopupSurface,
        _positioner: PositionerState,
        _token: u32,
    ) {
    }
}

impl SeatHandler for State {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(&mut self, _seat: &Seat<Self>, _image: CursorImageStatus) {}

    fn focus_changed(&mut self, _seat: &Seat<Self>, _focused: Option<&WlSurface>) {}
}

impl DmabufHandler for State {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.dmabuf
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        _dmabuf: smithay::backend::allocator::dmabuf::Dmabuf,
        notifier: ImportNotifier,
    ) {
        // The global is advertised with the renderer's formats; live import is
        // deferred to the phone (Stage 3). No dmabuf client is exercised yet.
        let _ = notifier.successful::<State>();
    }
}

smithay::delegate_compositor!(State);
smithay::delegate_shm!(State);
smithay::delegate_xdg_shell!(State);
smithay::delegate_seat!(State);
smithay::delegate_output!(State);
smithay::delegate_dmabuf!(State);

/// Runs the compositor until its window closes.
pub fn run() -> Result<(), Box<dyn Error>> {
    run_compositor(None, None)
}

/// Renders the first client frame to `path` (PNG) and exits. Used by the
/// golden test under Xvfb. `socket` names the Wayland socket for the client.
pub fn shot(path: PathBuf, socket: Option<String>) -> Result<(), Box<dyn Error>> {
    run_compositor(socket, Some(path))
}

/// Builds the globals, backend and event loop, then runs.
fn run_compositor(socket: Option<String>, shot: Option<PathBuf>) -> Result<(), Box<dyn Error>> {
    let mut event_loop: EventLoop<State> = EventLoop::try_new()?;
    let display: Display<State> = Display::new()?;
    let dh = display.handle();

    let compositor = CompositorState::new::<State>(&dh);
    let shm = ShmState::new::<State>(&dh, vec![]);
    let xdg = XdgShellState::new::<State>(&dh);
    let output_manager = OutputManagerState::new_with_xdg_output::<State>(&dh);
    let mut seat_state = SeatState::<State>::new();
    let seat = seat_state.new_wl_seat(&dh, "seat0");

    let (mut backend, mut winit_loop) =
        winit::init::<GlesRenderer>().map_err(|e| format!("winit backend init failed: {e}"))?;
    let size = backend.window_size();

    let output = Output::new(
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Sphatik".to_string(),
            model: "winit".to_string(),
        },
    );
    let _output_global = output.create_global::<State>(&dh);
    let mode = OutputMode {
        size,
        refresh: 60_000,
    };
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);

    let mut space = Space::<Window>::default();
    space.map_output(&output, (0, 0));

    let dmabuf_formats = backend.renderer().dmabuf_formats();
    let mut dmabuf = DmabufState::new();
    let _dmabuf_global = dmabuf.create_global::<State>(&dh, dmabuf_formats);

    let mut damage_tracker = OutputDamageTracker::from_output(&output);

    let mut state = State {
        dh: dh.clone(),
        compositor,
        shm,
        xdg,
        output_manager,
        seat_state,
        seat,
        dmabuf,
        space,
        output: output.clone(),
        loop_signal: event_loop.get_signal(),
        window: None,
        committed: false,
        start: Instant::now(),
    };

    // Listening socket: clients connect here (WAYLAND_DISPLAY).
    let socket_source = match &socket {
        Some(name) => ListeningSocketSource::with_name(name)?,
        None => ListeningSocketSource::new_auto()?,
    };
    let socket_name = socket_source.socket_name().to_string_lossy().into_owned();
    println!("wayland socket: {socket_name}");

    let handle = event_loop.handle();
    handle.insert_source(socket_source, move |stream, _, state: &mut State| {
        let _ = state
            .dh
            .insert_client(stream, Arc::new(ClientState::default()));
    })?;
    handle.insert_source(
        Generic::new(display, Interest::READ, CalloopMode::Level),
        |_, display, state: &mut State| {
            // SAFETY: calloop gives exclusive access to the display here, and
            // the borrow does not outlive this closure.
            unsafe {
                display.get_mut().dispatch_clients(state)?;
            }
            Ok(PostAction::Continue)
        },
    )?;

    // A safety deadline so a client-less --shot run cannot hang CI.
    let deadline = Timer::from_duration(Duration::from_secs(20));
    handle.insert_source(deadline, |_, _, state: &mut State| {
        state.loop_signal.stop();
        TimeoutAction::Drop
    })?;

    // The frame timer: pump winit, composite, present, and (for --shot)
    // capture the first client frame.
    let frame = Timer::immediate();
    handle.insert_source(frame, move |_, _, state: &mut State| {
        let mut close = false;
        let _ = winit_loop.dispatch_new_events(|event| {
            if let WinitEvent::CloseRequested = event {
                close = true;
            }
        });
        if close {
            state.loop_signal.stop();
            return TimeoutAction::Drop;
        }

        if let Err(e) = render_frame(state, &mut backend, &mut damage_tracker, shot.as_deref()) {
            eprintln!("sphatik-comp: render error: {e}");
            state.loop_signal.stop();
            return TimeoutAction::Drop;
        }
        let _ = state.dh.flush_clients();
        TimeoutAction::ToDuration(Duration::from_millis(16))
    })?;

    event_loop.run(None, &mut state, |_| {})?;

    if shot.is_some() && !state.committed {
        return Err("no client committed a frame before the deadline".into());
    }
    Ok(())
}

/// Composites the space onto the winit output, and on a `--shot` run captures
/// the first frame that contains client content, writes it, and stops.
fn render_frame(
    state: &mut State,
    backend: &mut winit::WinitGraphicsBackend<GlesRenderer>,
    damage_tracker: &mut OutputDamageTracker,
    shot: Option<&Path>,
) -> Result<(), Box<dyn Error>> {
    let size = backend.window_size();
    let (renderer, mut fb) = backend.bind()?;

    render_output::<_, WaylandSurfaceRenderElement<GlesRenderer>, _, _>(
        &state.output,
        renderer,
        &mut fb,
        1.0,
        0,
        [&state.space],
        &[],
        damage_tracker,
        Color32F::new(0.02, 0.02, 0.06, 1.0),
    )?;

    let mut done = false;
    if let Some(path) = shot {
        if state.committed {
            let region = Rectangle::<i32, BufferCoords>::from_size((size.w, size.h).into());
            let mapping = renderer.copy_framebuffer(&fb, region, Fourcc::Abgr8888)?;
            let bytes = renderer.map_texture(&mapping)?.to_vec();
            write_png(path, size.w as u32, size.h as u32, &bytes);
            done = true;
        }
    }

    drop(fb);
    backend.submit(Some(&[Rectangle::from_size(size)]))?;

    // Keep clients animating by acknowledging frame callbacks.
    let time = state.start.elapsed();
    state.space.elements().for_each(|w| {
        w.send_frame(&state.output, time, Some(Duration::ZERO), |_, _| {
            Some(state.output.clone())
        })
    });

    if done {
        state.loop_signal.stop();
    }
    Ok(())
}

/// Writes an RGBA8 buffer to a PNG.
fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    use std::fs::File;
    use std::io::BufWriter;

    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let file = match File::create(path) {
        Ok(f) => BufWriter::new(f),
        Err(e) => {
            eprintln!("sphatik-comp: cannot write {}: {e}", path.display());
            return;
        }
    };
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    if let Err(e) = encoder
        .write_header()
        .and_then(|mut w| w.write_image_data(rgba))
    {
        eprintln!("sphatik-comp: PNG encode failed: {e}");
    }
}

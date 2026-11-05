//! WP 2.8 compositing test: drive the compositor as a real Wayland client and
//! check the client's window is composited onto the output.
//!
//! The test launches the compositor binary in `--shot` mode (which opens its
//! window via the winit backend — under Xvfb in CI — and writes the first
//! client frame to a PNG), connects to it as a Wayland client, maps a
//! full-screen `xdg_toplevel` backed by a solid-colour `wl_shm` buffer, and
//! then asserts the captured frame is that colour. Proves "a test app renders
//! inside" (WP 2.8) without a committed golden image.
//!
//! Linux + `winit-backend` only; a no-op elsewhere.
#![cfg(all(target_os = "linux", feature = "winit-backend"))]

use std::io::Write;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use wayland_client::globals::{registry_queue_init, GlobalListContents};
use wayland_client::protocol::{
    wl_buffer::WlBuffer, wl_compositor::WlCompositor, wl_registry, wl_registry::WlRegistry, wl_shm,
    wl_shm::WlShm, wl_shm_pool::WlShmPool, wl_surface::WlSurface,
};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::xdg::shell::client::{
    xdg_surface::{self, XdgSurface},
    xdg_toplevel::{self, XdgToplevel},
    xdg_wm_base::{self, XdgWmBase},
};

/// The opaque colour the client paints, as (R, G, B) — Crystal Cyan.
const COLOR: (u8, u8, u8) = (138, 240, 240);
/// A socket name under the test's private `XDG_RUNTIME_DIR`.
const SOCKET: &str = "sphatik-wp28";

/// Client-side Wayland state.
struct App {
    compositor: WlCompositor,
    shm: WlShm,
    wm_base: XdgWmBase,
    surface: WlSurface,
    width: i32,
    height: i32,
    configured: bool,
    attached: bool,
    closed: bool,
}

#[test]
fn client_window_is_composited_fullscreen() {
    // A private runtime dir (0700) for the compositor's socket.
    let runtime = tempfile::tempdir().expect("runtime dir");
    set_mode_700(runtime.path());
    let shot = runtime.path().join("frame.png");

    let bin = env!("CARGO_BIN_EXE_sphatik-comp");
    let mut child = Command::new(bin)
        .args(["--shot", shot.to_str().unwrap(), "--socket", SOCKET])
        .env("XDG_RUNTIME_DIR", runtime.path())
        .spawn()
        .expect("spawn compositor");

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_client(runtime.path(), &mut child, &shot)
    }));

    let _ = child.kill();
    let _ = child.wait();
    if let Err(e) = result {
        std::panic::resume_unwind(e);
    }
}

/// Connects, maps a full-screen solid toplevel, waits for the capture, and
/// checks it.
fn run_client(runtime: &Path, child: &mut Child, shot: &Path) {
    let socket_path = runtime.join(SOCKET);
    let stream = wait_for_socket(&socket_path, child);
    let conn = Connection::from_socket(stream).expect("connect to compositor");

    let (globals, mut queue) = registry_queue_init::<App>(&conn).expect("registry init");
    let qh = queue.handle();

    let compositor: WlCompositor = globals.bind(&qh, 1..=6, ()).expect("wl_compositor");
    let shm: WlShm = globals.bind(&qh, 1..=1, ()).expect("wl_shm");
    let wm_base: XdgWmBase = globals.bind(&qh, 1..=6, ()).expect("xdg_wm_base");

    let surface = compositor.create_surface(&qh, ());
    let xdg_surface = wm_base.get_xdg_surface(&surface, &qh, ());
    let toplevel = xdg_surface.get_toplevel(&qh, ());
    toplevel.set_title("sphatik-wp28".to_string());
    surface.commit();

    let mut app = App {
        compositor,
        shm,
        wm_base,
        surface: surface.clone(),
        width: 0,
        height: 0,
        configured: false,
        attached: false,
        closed: false,
    };

    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        // A dispatch error means the compositor closed the socket (either after
        // it captured the frame, or at its own deadline). Stop and check the
        // PNG rather than panicking, so the assertion reports the real cause.
        if queue.blocking_dispatch(&mut app).is_err() {
            break;
        }

        if app.configured && !app.attached {
            let (w, h) = (app.width.max(1), app.height.max(1));
            let buffer = solid_buffer(&app.shm, &qh, w, h);
            app.surface.attach(Some(&buffer), 0, 0);
            app.surface.damage(0, 0, w, h);
            app.surface.commit();
            conn.flush().expect("flush");
            app.attached = true;
        }

        // The compositor exits once it has captured the frame.
        if let Ok(Some(_status)) = child.try_wait() {
            break;
        }
        if app.closed {
            break;
        }
    }

    // Wait briefly for the child to finish writing the PNG.
    let _ = child.wait();
    assert!(shot.exists(), "compositor did not write {}", shot.display());
    assert_center_is_color(shot, COLOR);
}

/// Polls for the compositor's socket, failing if the child dies first.
fn wait_for_socket(path: &Path, child: &mut Child) -> UnixStream {
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if let Ok(stream) = UnixStream::connect(path) {
            return stream;
        }
        if let Ok(Some(status)) = child.try_wait() {
            panic!("compositor exited early with {status} before the socket appeared");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("compositor socket {} never appeared", path.display());
}

/// Builds a solid-colour XRGB8888 `wl_shm` buffer `w`x`h`.
fn solid_buffer(shm: &WlShm, qh: &QueueHandle<App>, w: i32, h: i32) -> WlBuffer {
    let stride = w * 4;
    let size = (stride * h) as usize;
    // XRGB8888 is little-endian 0xXXRRGGBB -> bytes B, G, R, X in memory.
    let px = [COLOR.2, COLOR.1, COLOR.0, 0xFF];
    let mut file = tempfile::tempfile().expect("shm file");
    let data: Vec<u8> = px.iter().copied().cycle().take(size).collect();
    file.write_all(&data).expect("write shm");
    file.flush().expect("flush shm");

    use std::os::fd::AsFd;
    let pool = shm.create_pool(file.as_fd(), size as i32, qh, ());
    pool.create_buffer(0, w, h, stride, wl_shm::Format::Xrgb8888, qh, ())
}

/// Sets 0700 permissions, as Wayland expects of `XDG_RUNTIME_DIR`.
fn set_mode_700(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
}

/// Asserts the centre pixel of the captured RGBA PNG matches `color`.
fn assert_center_is_color(path: &Path, color: (u8, u8, u8)) {
    let file = std::io::BufReader::new(std::fs::File::open(path).expect("open png"));
    let decoder = png::Decoder::new(file);
    let mut reader = decoder.read_info().expect("png info");
    let mut buf = vec![0; reader.output_buffer_size().expect("png size")];
    let info = reader.next_frame(&mut buf).expect("png frame");
    assert_eq!(
        info.color_type,
        png::ColorType::Rgba,
        "expected RGBA output"
    );

    let (w, h) = (info.width as usize, info.height as usize);
    let (cx, cy) = (w / 2, h / 2);
    let i = (cy * w + cx) * 4;
    let (r, g, b) = (buf[i], buf[i + 1], buf[i + 2]);
    let tol = 48i32;
    let close = (r as i32 - color.0 as i32).abs() <= tol
        && (g as i32 - color.1 as i32).abs() <= tol
        && (b as i32 - color.2 as i32).abs() <= tol;
    assert!(
        close,
        "centre pixel ({r},{g},{b}) is not close to the client colour {color:?}"
    );
}

// --- Wayland event plumbing -------------------------------------------------

impl Dispatch<WlRegistry, GlobalListContents> for App {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<XdgWmBase, ()> for App {
    fn event(
        _: &mut Self,
        wm_base: &XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<XdgSurface, ()> for App {
    fn event(
        state: &mut Self,
        xdg_surface: &XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            xdg_surface.ack_configure(serial);
            state.configured = true;
        }
    }
}

impl Dispatch<XdgToplevel, ()> for App {
    fn event(
        state: &mut Self,
        _: &XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } => {
                if width > 0 && height > 0 {
                    state.width = width;
                    state.height = height;
                } else if state.width == 0 {
                    state.width = 256;
                    state.height = 256;
                }
            }
            xdg_toplevel::Event::Close => state.closed = true,
            _ => {}
        }
    }
}

// Interfaces whose events we don't need.
macro_rules! ignore_events {
    ($($iface:ty),* $(,)?) => {$(
        impl Dispatch<$iface, ()> for App {
            fn event(
                _: &mut Self,
                _: &$iface,
                _: <$iface as Proxy>::Event,
                _: &(),
                _: &Connection,
                _: &QueueHandle<Self>,
            ) {
            }
        }
    )*};
}
ignore_events!(WlCompositor, WlShm, WlShmPool, WlSurface, WlBuffer);

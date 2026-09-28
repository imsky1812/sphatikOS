//! The shell's gesture recogniser: pure logic, no platform code.
//!
//! A port of the prototype's gesture manager (`prototype/index.html`,
//! `/* Gesture manager */`; values in `docs/design/prototype-reference.md`,
//! section 7). A host (the compositor or `sphatik-preview`) feeds touch
//! samples into a [`TouchTracker`], asks [`classify`] what the drag is once it
//! passes the slop, maps movement to progress with [`progress`], and asks
//! [`release`] what to do when the finger lifts.
//!
//! Positions are logical points on the [`Canvas`]; times are milliseconds on
//! any monotonic clock; velocities are points per millisecond, as in the
//! prototype. Nothing here allocates.

use sphatik_motion::Preset;

/// Movement before a touch becomes a gesture, in points.
pub const TOUCH_SLOP: f32 = 8.0;
/// Weight of the previous velocity in the smoothing average.
pub const VELOCITY_KEEP: f32 = 0.7;
/// A finger that moves less than this between samples is "still", in points.
pub const STILL_DISTANCE: f32 = 3.0;
/// A finger still for longer than this is paused (used for the switcher), in ms.
pub const PAUSE_MS: f64 = 140.0;
/// Hold time for long-press (home edit mode, lock-screen customisation), in ms.
pub const LONG_PRESS_MS: f64 = 520.0;
/// Top strip where a downward pull opens Notification or Control Center.
pub const TOP_EDGE: f32 = 50.0;
/// Bottom strip where an upward swipe goes home or to the switcher.
pub const BOTTOM_EDGE: f32 = 56.0;
/// Side strips for the Back gesture (design spec: 20 pt edge zones).
pub const SIDE_EDGE: f32 = 20.0;
/// Resistance past the end of a panel pull.
pub const RUBBER_BAND: f32 = 0.18;

/// The logical screen the gestures happen on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Canvas {
    /// Width in points.
    pub width: f32,
    /// Height in points.
    pub height: f32,
}

impl Canvas {
    /// The prototype's 393 x 852 pt screen.
    pub const PROTOTYPE: Canvas = Canvas {
        width: 393.0,
        height: 852.0,
    };
    /// The Redmi Note 9 Pro: 1080 x 2400 px at scale 2.75.
    pub const CURTANA: Canvas = Canvas {
        width: 1080.0 / 2.75,
        height: 2400.0 / 2.75,
    };
}

/// A position in logical points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// Horizontal, from the left edge.
    pub x: f32,
    /// Vertical, from the top edge.
    pub y: f32,
}

impl Point {
    /// A point at `(x, y)`.
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Follows one finger: start, position, smoothed velocity, stillness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchTracker {
    start: Point,
    start_ms: f64,
    pos: Point,
    last_ms: f64,
    velocity: Point,
    still_since_ms: f64,
    past_slop: bool,
}

impl TouchTracker {
    /// A finger touching down at `at` on time `now_ms`.
    pub fn begin(at: Point, now_ms: f64) -> Self {
        Self {
            start: at,
            start_ms: now_ms,
            pos: at,
            last_ms: now_ms,
            velocity: Point::default(),
            still_since_ms: now_ms,
            past_slop: false,
        }
    }

    /// A new sample. Velocity is an exponential average of the instantaneous
    /// speed, with sample gaps floored at 1 ms.
    pub fn move_to(&mut self, to: Point, now_ms: f64) {
        let dt = (now_ms - self.last_ms).max(1.0) as f32;
        let (dx, dy) = (to.x - self.pos.x, to.y - self.pos.y);
        let keep = VELOCITY_KEEP;
        self.velocity.x = keep * self.velocity.x + (1.0 - keep) * dx / dt;
        self.velocity.y = keep * self.velocity.y + (1.0 - keep) * dy / dt;
        if dx.hypot(dy) > STILL_DISTANCE {
            self.still_since_ms = now_ms;
        }
        self.pos = to;
        self.last_ms = now_ms;
        if !self.past_slop {
            let d = self.delta();
            self.past_slop = d.x.hypot(d.y) >= TOUCH_SLOP;
        }
    }

    /// Where the finger touched down.
    pub fn start(&self) -> Point {
        self.start
    }

    /// Where the finger is now.
    pub fn position(&self) -> Point {
        self.pos
    }

    /// Movement since touch-down.
    pub fn delta(&self) -> Point {
        Point::new(self.pos.x - self.start.x, self.pos.y - self.start.y)
    }

    /// Smoothed velocity in points per millisecond.
    pub fn velocity(&self) -> Point {
        self.velocity
    }

    /// True once the finger has moved at least [`TOUCH_SLOP`] from where it
    /// started; from then on the touch is a drag, not a tap.
    pub fn past_slop(&self) -> bool {
        self.past_slop
    }

    /// True if the finger has been still for more than [`PAUSE_MS`].
    pub fn is_paused(&self, now_ms: f64) -> bool {
        now_ms - self.still_since_ms > PAUSE_MS
    }

    /// True if this touch has become a long-press: held for
    /// [`LONG_PRESS_MS`] without leaving the slop.
    pub fn is_long_press(&self, now_ms: f64) -> bool {
        !self.past_slop && now_ms - self.start_ms >= LONG_PRESS_MS
    }
}

/// A shell panel that slides over everything.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Panel {
    /// Notification Center (top-left pull).
    Notifications,
    /// Control Center (top-right pull).
    Control,
    /// Intent Bar (pull down on home).
    Intent,
}

/// Which side a Back swipe started from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    /// Left edge, swiping right.
    Left,
    /// Right edge, swiping left.
    Right,
}

/// What the finger went down on, as far as gestures care.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Hit {
    /// Anything else.
    #[default]
    Other,
    /// A home-screen icon while in edit mode.
    EditableIcon,
    /// A card in the app switcher.
    SwitcherCard,
    /// A notification card in Notification Center.
    NotificationCard,
    /// Scrollable app content (a downward drag scrolls rather than opening
    /// the Intent Bar).
    ScrollArea,
}

/// The shell state that decides what a drag means.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ShellContext {
    /// The lock screen covers the screen (unlock progress below 0.5).
    pub locked: bool,
    /// The lock screen is in customise mode.
    pub lock_editing: bool,
    /// An app is open.
    pub app_open: bool,
    /// The panel that is open (progress above 0.5), if any.
    pub open_panel: Option<Panel>,
    /// The app switcher is showing.
    pub switcher_open: bool,
    /// Home is in edit mode.
    pub edit_mode: bool,
    /// The Halo is expanded.
    pub halo_expanded: bool,
}

/// What a drag turned out to be.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Gesture {
    /// Pull a panel down from the top edge.
    OpenPanel(Panel),
    /// Push an open panel back up.
    ClosePanel(Panel),
    /// Swipe the lock screen up.
    Unlock,
    /// Swipe up from the bottom edge inside an app.
    Home,
    /// Swipe up from the bottom edge on home, towards the switcher.
    HomeSwitcher,
    /// Horizontal drag between home and the App Library.
    Page,
    /// Inward swipe from a side edge inside an app.
    Back(Side),
    /// Drag a home icon in edit mode.
    MoveIcon,
    /// Swipe a notification card sideways.
    SwipeNotification,
    /// Flick a switcher card up to close it.
    FlickSwitcherCard,
    /// Scroll the switcher sideways.
    ScrollSwitcher,
}

/// Classifies a drag once it has passed the slop. The order of the checks is
/// the prototype's (first match wins), with the design spec's Back gesture
/// (conflict C10) inserted where it cannot shadow another gesture. Returns
/// `None` when the drag should go to whatever is under the finger.
pub fn classify(
    ctx: &ShellContext,
    canvas: Canvas,
    start: Point,
    delta: Point,
    hit: Hit,
) -> Option<Gesture> {
    let vertical = delta.y.abs() > delta.x.abs();
    let down = vertical && delta.y > 0.0;
    let up = vertical && delta.y < 0.0;
    let top_panel = || {
        if start.x < canvas.width / 2.0 {
            Panel::Notifications
        } else {
            Panel::Control
        }
    };

    if ctx.edit_mode {
        return (hit == Hit::EditableIcon).then_some(Gesture::MoveIcon);
    }
    if ctx.switcher_open {
        return if start.y < TOP_EDGE && down {
            Some(Gesture::OpenPanel(top_panel()))
        } else if hit == Hit::SwitcherCard && up {
            Some(Gesture::FlickSwitcherCard)
        } else if !vertical {
            Some(Gesture::ScrollSwitcher)
        } else {
            None
        };
    }
    if ctx.open_panel == Some(Panel::Notifications) && hit == Hit::NotificationCard && !vertical {
        return Some(Gesture::SwipeNotification);
    }
    if start.y < TOP_EDGE && down && ctx.open_panel.is_none() && !ctx.halo_expanded {
        return Some(Gesture::OpenPanel(top_panel()));
    }
    if let (Some(panel), true) = (ctx.open_panel, up) {
        return Some(Gesture::ClosePanel(panel));
    }
    if ctx.app_open && ctx.open_panel.is_none() && !vertical {
        if start.x < SIDE_EDGE && delta.x > 0.0 {
            return Some(Gesture::Back(Side::Left));
        }
        if start.x > canvas.width - SIDE_EDGE && delta.x < 0.0 {
            return Some(Gesture::Back(Side::Right));
        }
    }
    let on_home = !ctx.locked && !ctx.app_open && ctx.open_panel.is_none();
    if !vertical && on_home && !ctx.halo_expanded {
        return Some(Gesture::Page);
    }
    if ctx.locked && !ctx.lock_editing && up {
        return Some(Gesture::Unlock);
    }
    let from_bottom = start.y > canvas.height - BOTTOM_EDGE && up;
    if ctx.app_open && from_bottom {
        return Some(Gesture::Home);
    }
    if on_home && from_bottom {
        return Some(Gesture::HomeSwitcher);
    }
    if on_home && down && hit != Hit::ScrollArea {
        return Some(Gesture::OpenPanel(Panel::Intent));
    }
    None
}

/// Resistance past full travel: 1:1 up to 1.0, then [`RUBBER_BAND`].
pub fn rubber_band(p: f32) -> f32 {
    if p <= 1.0 {
        p.max(0.0)
    } else {
        1.0 + (p - 1.0) * RUBBER_BAND
    }
}

/// Finger travel (points) for a full panel pull: 0.42 of the screen height
/// for Notification and Control Center, 0.28 for the Intent Bar.
fn panel_travel(canvas: Canvas, panel: Panel) -> f32 {
    match panel {
        Panel::Notifications | Panel::Control => 0.42 * canvas.height,
        Panel::Intent => 0.28 * canvas.height,
    }
}

/// Finger travel for a full Back swipe (provisional: the spec gives none).
pub fn back_travel(canvas: Canvas) -> f32 {
    0.5 * canvas.width
}

/// Progress of a drag, for the gestures that drive a single value.
///
/// | Gesture | Progress |
/// | --- | --- |
/// | `Unlock` | lock progress, 0 → 1 over half the screen height |
/// | `Home` | app window progress, 1 → 0 over 0.55 of the height |
/// | `OpenPanel` | 0 → 1 over the panel travel, rubber-banded past 1 |
/// | `ClosePanel` | 1 → 0 over the panel travel |
/// | `Page` | `from` minus the horizontal drag over the screen width |
/// | `Back` | 0 → 1 over [`back_travel`] |
///
/// `from` is the value when the drag started (used by `Page`). Gestures that
/// move objects directly (cards, icons, the switcher track) return `None`.
pub fn progress(gesture: Gesture, canvas: Canvas, delta: Point, from: f32) -> Option<f32> {
    let clamp01 = |v: f32| v.clamp(0.0, 1.0);
    let h = canvas.height;
    Some(match gesture {
        Gesture::Unlock => clamp01(-delta.y / (0.5 * h)),
        Gesture::Home => 1.0 - clamp01(-delta.y / (0.55 * h)),
        Gesture::OpenPanel(p) => rubber_band(delta.y / panel_travel(canvas, p)),
        Gesture::ClosePanel(p) => 1.0 - clamp01(-delta.y / panel_travel(canvas, p)),
        Gesture::Page => clamp01(from - delta.x / canvas.width),
        Gesture::Back(_) => clamp01(delta.x.abs() / back_travel(canvas)),
        Gesture::HomeSwitcher
        | Gesture::MoveIcon
        | Gesture::SwipeNotification
        | Gesture::FlickSwitcherCard
        | Gesture::ScrollSwitcher => return None,
    })
}

/// What the shell does when the finger lifts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Release {
    /// Spring the gesture's progress to `to` with `preset`, starting at
    /// `velocity` (progress per second).
    Spring {
        /// Final progress.
        to: f32,
        /// Initial velocity handed over from the finger.
        velocity: f32,
        /// Spring to use.
        preset: Preset,
    },
    /// Close the app into the app switcher.
    OpenSwitcher,
    /// Go back one level in the app.
    Back,
    /// Throw a notification card off-screen, left (-1) or right (+1).
    DismissNotification {
        /// Direction of travel.
        direction: f32,
    },
    /// Close the app whose switcher card was flicked up.
    CloseSwitcherCard,
    /// Spring the dragged object back to where it was.
    Restore,
    /// Nothing special: the host finishes the gesture itself (icon drop,
    /// switcher snap via [`switcher_snap`]).
    Finish,
}

/// Everything [`release`] needs about the finger when it lifts.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LiftOff {
    /// Movement since touch-down.
    pub delta: Point,
    /// Smoothed velocity, points per millisecond.
    pub velocity: Point,
    /// The finger was still for more than [`PAUSE_MS`] before lifting.
    pub paused: bool,
    /// Progress from [`progress`] at lift-off (ignored by object gestures).
    pub progress: f32,
    /// True if there are recent apps to show in the switcher.
    pub has_recents: bool,
    /// The screen the gesture happened on.
    pub canvas: Canvas,
}

/// Decides what happens when the finger lifts, with the prototype's
/// thresholds and velocity hand-off factors (reference section 7.3) and the
/// spring presets settled in conflicts C1 and C7.
pub fn release(gesture: Gesture, lift: LiftOff) -> Release {
    let LiftOff {
        delta,
        velocity: v,
        paused,
        progress: p,
        has_recents,
        canvas,
    } = lift;
    let spring = |to: f32, velocity: f32, preset: Preset| Release::Spring {
        to,
        velocity,
        preset,
    };
    match gesture {
        // Spec, lock screen states: "Swipe up; Bouncy spring".
        Gesture::Unlock if p > 0.25 || v.y < -0.45 => spring(1.0, -v.y * 2.0, Preset::Bouncy),
        Gesture::Unlock => spring(0.0, 0.0, Preset::Smooth),
        Gesture::Home => {
            let k = 1.0 - p;
            if k > 0.14 && (paused || (v.y.abs() < 0.25 && k < 0.6)) {
                Release::OpenSwitcher
            } else if k > 0.18 || v.y < -0.4 {
                // Conflict C7: app open and close use Bouncy.
                spring(0.0, v.y * 2.0, Preset::Bouncy)
            } else {
                spring(1.0, 0.0, Preset::Bouncy)
            }
        }
        Gesture::HomeSwitcher if -delta.y > 70.0 && has_recents => Release::OpenSwitcher,
        Gesture::HomeSwitcher => Release::Restore,
        Gesture::OpenPanel(panel) => {
            let (threshold, factor) = match panel {
                Panel::Intent => (0.35, 3.0),
                Panel::Notifications | Panel::Control => (0.3, 2.0),
            };
            if p > threshold || v.y > 0.4 {
                spring(1.0, v.y * factor, Preset::Bouncy)
            } else {
                spring(0.0, 0.0, Preset::Smooth)
            }
        }
        Gesture::ClosePanel(_) if p < 0.75 || v.y < -0.4 => spring(0.0, v.y * 2.0, Preset::Smooth),
        Gesture::ClosePanel(_) => spring(1.0, 0.0, Preset::Bouncy),
        Gesture::Page => {
            let to = if v.x < -0.35 {
                1.0
            } else if v.x > 0.35 {
                0.0
            } else if p > 0.5 {
                1.0
            } else {
                0.0
            };
            spring(to, -v.x * 1000.0 / canvas.width, Preset::Smooth)
        }
        Gesture::Back(_) if p > 0.35 || v.x.abs() > 0.35 => Release::Back,
        Gesture::Back(_) => Release::Restore,
        Gesture::SwipeNotification if delta.x.abs() > 110.0 || v.x.abs() > 0.6 => {
            let dir = if delta.x != 0.0 { delta.x } else { v.x };
            Release::DismissNotification {
                direction: dir.signum(),
            }
        }
        Gesture::SwipeNotification => Release::Restore,
        Gesture::FlickSwitcherCard if -delta.y > 140.0 || v.y < -0.6 => Release::CloseSwitcherCard,
        Gesture::FlickSwitcherCard => Release::Restore,
        Gesture::MoveIcon | Gesture::ScrollSwitcher => Release::Finish,
    }
}

/// Switcher card pitch: a 250 pt card plus an 18 pt gap.
pub const SWITCHER_STEP: f32 = 268.0;

/// Where the switcher track should settle after a horizontal drag: the
/// nearest card to the current offset plus momentum (`vx * 180`).
pub fn switcher_snap(scroll: f32, velocity_x: f32) -> f32 {
    ((scroll - velocity_x * 180.0) / SWITCHER_STEP).round() * SWITCHER_STEP
}

#[cfg(test)]
mod tests {
    use super::*;

    const C: Canvas = Canvas::PROTOTYPE;

    fn home() -> ShellContext {
        ShellContext::default()
    }
    fn locked() -> ShellContext {
        ShellContext {
            locked: true,
            ..ShellContext::default()
        }
    }
    fn in_app() -> ShellContext {
        ShellContext {
            app_open: true,
            ..ShellContext::default()
        }
    }
    fn p(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }
    fn lift(delta: Point, velocity: Point, progress: f32) -> LiftOff {
        LiftOff {
            delta,
            velocity,
            paused: false,
            progress,
            has_recents: true,
            canvas: C,
        }
    }

    // ---------- tracker ----------

    #[test]
    fn slop_is_eight_points() {
        let mut t = TouchTracker::begin(p(100.0, 100.0), 0.0);
        t.move_to(p(105.0, 105.0), 16.0); // 7.07 pt
        assert!(!t.past_slop());
        t.move_to(p(106.0, 106.0), 32.0); // 8.49 pt
        assert!(t.past_slop());
        t.move_to(p(100.0, 100.0), 48.0); // coming back does not undo it
        assert!(t.past_slop());
    }

    #[test]
    fn velocity_is_an_exponential_average_in_points_per_ms() {
        let mut t = TouchTracker::begin(p(0.0, 0.0), 0.0);
        t.move_to(p(0.0, -10.0), 10.0); // -1 pt/ms
        assert!((t.velocity().y - -0.3).abs() < 1e-6);
        t.move_to(p(0.0, -20.0), 20.0);
        assert!((t.velocity().y - -0.51).abs() < 1e-6);
    }

    #[test]
    fn sample_gaps_are_floored_at_one_millisecond() {
        let mut t = TouchTracker::begin(p(0.0, 0.0), 0.0);
        t.move_to(p(10.0, 0.0), 0.0);
        assert!((t.velocity().x - 3.0).abs() < 1e-6);
    }

    #[test]
    fn pause_needs_140_ms_without_moving_more_than_3_points() {
        let mut t = TouchTracker::begin(p(0.0, 0.0), 0.0);
        t.move_to(p(0.0, -50.0), 100.0);
        t.move_to(p(0.0, -52.0), 200.0); // 2 pt: still
        assert!(!t.is_paused(240.0));
        assert!(t.is_paused(241.0));
        t.move_to(p(0.0, -60.0), 250.0); // 8 pt: moving again
        assert!(!t.is_paused(300.0));
    }

    #[test]
    fn long_press_needs_520_ms_inside_the_slop() {
        let mut t = TouchTracker::begin(p(50.0, 50.0), 1000.0);
        t.move_to(p(53.0, 52.0), 1300.0);
        assert!(!t.is_long_press(1519.0));
        assert!(t.is_long_press(1520.0));
        t.move_to(p(70.0, 50.0), 1600.0);
        assert!(!t.is_long_press(2000.0));
    }

    // ---------- classification ----------

    #[test]
    fn top_edge_pull_opens_notifications_left_and_control_right() {
        let down = p(0.0, 40.0);
        assert_eq!(
            classify(&home(), C, p(100.0, 20.0), down, Hit::Other),
            Some(Gesture::OpenPanel(Panel::Notifications))
        );
        assert_eq!(
            classify(&in_app(), C, p(300.0, 49.0), down, Hit::Other),
            Some(Gesture::OpenPanel(Panel::Control))
        );
        assert_eq!(
            classify(&locked(), C, p(300.0, 10.0), down, Hit::Other),
            Some(Gesture::OpenPanel(Panel::Control)),
            "panels open over the lock screen too"
        );
    }

    #[test]
    fn top_edge_ends_at_50_points() {
        assert_eq!(
            classify(&home(), C, p(100.0, 50.0), p(0.0, 40.0), Hit::Other),
            Some(Gesture::OpenPanel(Panel::Intent)),
            "below the edge, a pull on home is the Intent Bar"
        );
    }

    #[test]
    fn expanded_halo_blocks_top_pulls() {
        let ctx = ShellContext {
            halo_expanded: true,
            ..home()
        };
        assert_ne!(
            classify(&ctx, C, p(100.0, 20.0), p(0.0, 40.0), Hit::Other),
            Some(Gesture::OpenPanel(Panel::Notifications))
        );
    }

    #[test]
    fn upward_drag_closes_the_open_panel() {
        let ctx = ShellContext {
            open_panel: Some(Panel::Control),
            ..home()
        };
        assert_eq!(
            classify(&ctx, C, p(200.0, 500.0), p(3.0, -30.0), Hit::Other),
            Some(Gesture::ClosePanel(Panel::Control))
        );
    }

    #[test]
    fn notification_cards_swipe_sideways() {
        let ctx = ShellContext {
            open_panel: Some(Panel::Notifications),
            ..home()
        };
        assert_eq!(
            classify(
                &ctx,
                C,
                p(200.0, 200.0),
                p(30.0, 5.0),
                Hit::NotificationCard
            ),
            Some(Gesture::SwipeNotification)
        );
        assert_eq!(
            classify(&ctx, C, p(200.0, 200.0), p(30.0, 5.0), Hit::Other),
            None
        );
    }

    #[test]
    fn swipe_up_on_the_lock_screen_unlocks() {
        assert_eq!(
            classify(&locked(), C, p(200.0, 600.0), p(2.0, -20.0), Hit::Other),
            Some(Gesture::Unlock)
        );
        let editing = ShellContext {
            lock_editing: true,
            ..locked()
        };
        assert_eq!(
            classify(&editing, C, p(200.0, 600.0), p(2.0, -20.0), Hit::Other),
            None
        );
    }

    #[test]
    fn bottom_edge_swipe_goes_home_from_an_app() {
        assert_eq!(
            classify(&in_app(), C, p(200.0, 800.0), p(0.0, -20.0), Hit::Other),
            Some(Gesture::Home)
        );
        assert_eq!(
            classify(&in_app(), C, p(200.0, 795.0), p(0.0, -20.0), Hit::Other),
            None,
            "the bottom zone is 56 pt"
        );
    }

    #[test]
    fn bottom_edge_uses_the_device_canvas_height() {
        let y = Canvas::CURTANA.height - 30.0;
        assert_eq!(
            classify(
                &in_app(),
                Canvas::CURTANA,
                p(200.0, y),
                p(0.0, -20.0),
                Hit::Other
            ),
            Some(Gesture::Home)
        );
    }

    #[test]
    fn bottom_edge_swipe_on_home_heads_for_the_switcher() {
        assert_eq!(
            classify(&home(), C, p(200.0, 820.0), p(0.0, -20.0), Hit::Other),
            Some(Gesture::HomeSwitcher)
        );
    }

    #[test]
    fn horizontal_drag_on_home_pages() {
        assert_eq!(
            classify(&home(), C, p(200.0, 400.0), p(-30.0, 4.0), Hit::Other),
            Some(Gesture::Page)
        );
        assert_eq!(
            classify(&locked(), C, p(200.0, 400.0), p(-30.0, 4.0), Hit::Other),
            None
        );
    }

    #[test]
    fn downward_drag_on_home_opens_the_intent_bar_but_not_over_scroll_areas() {
        assert_eq!(
            classify(&home(), C, p(200.0, 400.0), p(1.0, 30.0), Hit::Other),
            Some(Gesture::OpenPanel(Panel::Intent))
        );
        assert_eq!(
            classify(&home(), C, p(200.0, 400.0), p(1.0, 30.0), Hit::ScrollArea),
            None
        );
    }

    #[test]
    fn inward_side_swipe_in_an_app_is_back() {
        assert_eq!(
            classify(&in_app(), C, p(10.0, 400.0), p(30.0, 3.0), Hit::Other),
            Some(Gesture::Back(Side::Left))
        );
        assert_eq!(
            classify(&in_app(), C, p(380.0, 400.0), p(-30.0, 3.0), Hit::Other),
            Some(Gesture::Back(Side::Right))
        );
        assert_eq!(
            classify(&in_app(), C, p(10.0, 400.0), p(-30.0, 3.0), Hit::Other),
            None,
            "outward swipes are not Back"
        );
        assert_eq!(
            classify(&in_app(), C, p(30.0, 400.0), p(30.0, 3.0), Hit::Other),
            None,
            "the edge zone is 20 pt"
        );
    }

    #[test]
    fn edit_mode_only_drags_icons() {
        let ctx = ShellContext {
            edit_mode: true,
            ..home()
        };
        assert_eq!(
            classify(&ctx, C, p(100.0, 300.0), p(20.0, 20.0), Hit::EditableIcon),
            Some(Gesture::MoveIcon)
        );
        assert_eq!(
            classify(&ctx, C, p(100.0, 20.0), p(0.0, 40.0), Hit::Other),
            None,
            "not even the top edge"
        );
    }

    #[test]
    fn switcher_gestures() {
        let ctx = ShellContext {
            switcher_open: true,
            ..home()
        };
        assert_eq!(
            classify(&ctx, C, p(200.0, 400.0), p(0.0, -30.0), Hit::SwitcherCard),
            Some(Gesture::FlickSwitcherCard)
        );
        assert_eq!(
            classify(&ctx, C, p(200.0, 400.0), p(-40.0, 5.0), Hit::SwitcherCard),
            Some(Gesture::ScrollSwitcher)
        );
        assert_eq!(
            classify(&ctx, C, p(50.0, 20.0), p(0.0, 30.0), Hit::Other),
            Some(Gesture::OpenPanel(Panel::Notifications))
        );
        assert_eq!(
            classify(&ctx, C, p(200.0, 400.0), p(0.0, 30.0), Hit::Other),
            None
        );
    }

    // ---------- progress ----------

    #[test]
    fn progress_maps_travel_like_the_prototype() {
        let h = C.height;
        let pr = |g, d| progress(g, C, d, 0.0).expect("single-value gesture");
        assert!((pr(Gesture::Unlock, p(0.0, -0.25 * h)) - 0.5).abs() < 1e-6);
        assert_eq!(pr(Gesture::Unlock, p(0.0, 50.0)), 0.0);
        assert!((pr(Gesture::Home, p(0.0, -0.275 * h)) - 0.5).abs() < 1e-6);
        let nc = Gesture::OpenPanel(Panel::Notifications);
        assert!((pr(nc, p(0.0, 0.21 * h)) - 0.5).abs() < 1e-6);
        let intent = Gesture::OpenPanel(Panel::Intent);
        assert!((pr(intent, p(0.0, 0.14 * h)) - 0.5).abs() < 1e-6);
        let close = Gesture::ClosePanel(Panel::Control);
        assert!((pr(close, p(0.0, -0.21 * h)) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn panel_pulls_rubber_band_past_full_travel() {
        let travel = 0.42 * C.height;
        let over = progress(
            Gesture::OpenPanel(Panel::Control),
            C,
            p(0.0, 2.0 * travel),
            0.0,
        );
        let over = over.expect("single-value gesture");
        assert!((over - 1.18).abs() < 1e-6, "{over}");
        assert_eq!(rubber_band(-0.5), 0.0);
    }

    #[test]
    fn paging_starts_from_the_current_page() {
        assert_eq!(progress(Gesture::Page, C, p(-196.5, 0.0), 0.0), Some(0.5));
        assert_eq!(progress(Gesture::Page, C, p(196.5, 0.0), 1.0), Some(0.5));
        assert_eq!(progress(Gesture::Page, C, p(500.0, 0.0), 0.0), Some(0.0));
    }

    #[test]
    fn object_gestures_have_no_single_progress() {
        for g in [
            Gesture::MoveIcon,
            Gesture::SwipeNotification,
            Gesture::FlickSwitcherCard,
            Gesture::ScrollSwitcher,
            Gesture::HomeSwitcher,
        ] {
            assert_eq!(progress(g, C, p(10.0, 10.0), 0.0), None);
        }
    }

    // ---------- release ----------

    #[test]
    fn unlock_commits_past_a_quarter_or_on_a_flick() {
        let slow = p(0.0, -0.1);
        assert_eq!(
            release(Gesture::Unlock, lift(p(0.0, -250.0), slow, 0.26)),
            Release::Spring {
                to: 1.0,
                velocity: 0.2,
                preset: Preset::Bouncy
            }
        );
        assert!(matches!(
            release(Gesture::Unlock, lift(p(0.0, -30.0), p(0.0, -0.5), 0.07)),
            Release::Spring { to, .. } if to == 1.0
        ));
        assert_eq!(
            release(Gesture::Unlock, lift(p(0.0, -100.0), slow, 0.2)),
            Release::Spring {
                to: 0.0,
                velocity: 0.0,
                preset: Preset::Smooth
            }
        );
    }

    #[test]
    fn home_swipe_closes_the_app_or_opens_the_switcher() {
        let fast = p(0.0, -1.0);
        assert!(matches!(
            release(Gesture::Home, lift(p(0.0, -200.0), fast, 0.6)),
            Release::Spring { to, preset: Preset::Bouncy, .. } if to == 0.0
        ));
        let paused = LiftOff {
            paused: true,
            ..lift(p(0.0, -150.0), p(0.0, 0.0), 0.8)
        };
        assert_eq!(release(Gesture::Home, paused), Release::OpenSwitcher);
        // Slow and short: the prototype also treats this as a pause.
        assert_eq!(
            release(Gesture::Home, lift(p(0.0, -150.0), p(0.0, -0.1), 0.8)),
            Release::OpenSwitcher
        );
        assert!(matches!(
            release(Gesture::Home, lift(p(0.0, -40.0), p(0.0, -0.1), 0.9)),
            Release::Spring { to, .. } if to == 1.0
        ));
    }

    #[test]
    fn panels_open_past_their_thresholds() {
        let still = p(0.0, 0.0);
        let cc = Gesture::OpenPanel(Panel::Control);
        assert!(matches!(
            release(cc, lift(p(0.0, 120.0), still, 0.31)),
            Release::Spring { to, preset: Preset::Bouncy, .. } if to == 1.0
        ));
        assert!(matches!(
            release(cc, lift(p(0.0, 100.0), still, 0.29)),
            Release::Spring { to, .. } if to == 0.0
        ));
        let intent = Gesture::OpenPanel(Panel::Intent);
        assert!(matches!(
            release(intent, lift(p(0.0, 80.0), still, 0.34)),
            Release::Spring { to, .. } if to == 0.0
        ));
        assert_eq!(
            release(intent, lift(p(0.0, 30.0), p(0.0, 0.5), 0.1)),
            Release::Spring {
                to: 1.0,
                velocity: 1.5,
                preset: Preset::Bouncy
            }
        );
    }

    #[test]
    fn open_panel_closes_below_three_quarters() {
        let g = Gesture::ClosePanel(Panel::Notifications);
        assert!(matches!(
            release(g, lift(p(0.0, -100.0), p(0.0, 0.0), 0.74)),
            Release::Spring { to, .. } if to == 0.0
        ));
        assert!(matches!(
            release(g, lift(p(0.0, -30.0), p(0.0, 0.0), 0.9)),
            Release::Spring { to, .. } if to == 1.0
        ));
    }

    #[test]
    fn home_to_switcher_needs_70_points_and_recents() {
        assert_eq!(
            release(Gesture::HomeSwitcher, lift(p(0.0, -71.0), p(0.0, 0.0), 0.0)),
            Release::OpenSwitcher
        );
        let none = LiftOff {
            has_recents: false,
            ..lift(p(0.0, -200.0), p(0.0, 0.0), 0.0)
        };
        assert_eq!(release(Gesture::HomeSwitcher, none), Release::Restore);
    }

    #[test]
    fn paging_follows_a_flick_or_the_nearest_page() {
        let to = |vx: f32, prog: f32| match release(
            Gesture::Page,
            lift(p(0.0, 0.0), p(vx, 0.0), prog),
        ) {
            Release::Spring { to, .. } => to,
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(to(-0.4, 0.1), 1.0);
        assert_eq!(to(0.4, 0.9), 0.0);
        assert_eq!(to(0.0, 0.6), 1.0);
        assert_eq!(to(0.0, 0.4), 0.0);
    }

    #[test]
    fn notification_cards_dismiss_past_110_points_or_on_a_flick() {
        let g = Gesture::SwipeNotification;
        assert_eq!(
            release(g, lift(p(-111.0, 0.0), p(0.0, 0.0), 0.0)),
            Release::DismissNotification { direction: -1.0 }
        );
        assert_eq!(
            release(g, lift(p(0.0, 0.0), p(0.7, 0.0), 0.0)),
            Release::DismissNotification { direction: 1.0 }
        );
        assert_eq!(
            release(g, lift(p(60.0, 0.0), p(0.2, 0.0), 0.0)),
            Release::Restore
        );
    }

    #[test]
    fn switcher_cards_close_past_140_points_or_on_a_flick() {
        let g = Gesture::FlickSwitcherCard;
        assert_eq!(
            release(g, lift(p(0.0, -141.0), p(0.0, 0.0), 0.0)),
            Release::CloseSwitcherCard
        );
        assert_eq!(
            release(g, lift(p(0.0, -50.0), p(0.0, -0.7), 0.0)),
            Release::CloseSwitcherCard
        );
        assert_eq!(
            release(g, lift(p(0.0, -50.0), p(0.0, -0.2), 0.0)),
            Release::Restore
        );
    }

    #[test]
    fn back_commits_past_its_threshold() {
        let g = Gesture::Back(Side::Left);
        assert_eq!(
            release(g, lift(p(80.0, 0.0), p(0.1, 0.0), 0.4)),
            Release::Back
        );
        assert_eq!(
            release(g, lift(p(20.0, 0.0), p(0.5, 0.0), 0.1)),
            Release::Back
        );
        assert_eq!(
            release(g, lift(p(40.0, 0.0), p(0.1, 0.0), 0.2)),
            Release::Restore
        );
    }

    #[test]
    fn switcher_snaps_to_the_nearest_card_with_momentum() {
        assert_eq!(switcher_snap(300.0, 0.0), 268.0);
        assert_eq!(switcher_snap(300.0, -1.0), 536.0);
        assert_eq!(switcher_snap(100.0, 0.0), 0.0);
    }

    #[test]
    fn canvases_match_the_reference() {
        assert!((Canvas::CURTANA.width - 392.727).abs() < 1e-3);
        assert!((Canvas::CURTANA.height - 872.727).abs() < 1e-3);
    }
}

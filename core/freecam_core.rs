//! Freecam core: the camera state machine, with no dependency on the game.
//!
//! One file, std only, so each adapter can compile it as a plain module of the game crate
//! (legacy mixin bundle) or of the mod crate (trait mod). Everything here is ordinary Rust
//! and is unit tested on the host with `cargo test`.
//!
//! Conventions follow Minecraft: yaw 0 faces +Z (south), yaw 90 faces -X (west), positive
//! pitch looks down, positions are in blocks and speeds are in blocks per game tick (20/s).

// Each adapter uses a different part of this API.
#![allow(dead_code)]

use std::sync::Mutex;

pub const ID: &str = "uni_freecam";
pub const VERSION: &str = "0.1.0";
pub const TITLE: &str = "Universal Freecam";
pub const DESCRIPTION: &str = "Detach the camera and fly it anywhere while your player stays where it is. \
WASD fly, Space and Shift go up and down, Ctrl is faster, the mouse wheel changes the speed. \
Number keys 1-9 with Alt save and recall camera spots.";

/// Name of the mod's toggle key and its default.
pub const KEY_TOGGLE: &str = "toggle";
pub const DEFAULT_TOGGLE: &str = "key.keyboard.f4";

pub const FORWARD: usize = 0;
pub const BACK: usize = 1;
pub const LEFT: usize = 2;
pub const RIGHT: usize = 3;
pub const UP: usize = 4;
pub const DOWN: usize = 5;
pub const SPRINT: usize = 6;
pub const INPUTS: usize = 7;

/// Vanilla key names for the seven movement inputs, in the order of the constants above.
pub const INPUT_KEYS: [&str; INPUTS] = [
    "key.keyboard.w",
    "key.keyboard.s",
    "key.keyboard.a",
    "key.keyboard.d",
    "key.keyboard.space",
    "key.keyboard.left.shift",
    "key.keyboard.left.control",
];

/// Vanilla key names of the digits 1 to 9, for the camera spots.
pub const DIGIT_KEYS: [&str; TRIPODS] = [
    "key.keyboard.1",
    "key.keyboard.2",
    "key.keyboard.3",
    "key.keyboard.4",
    "key.keyboard.5",
    "key.keyboard.6",
    "key.keyboard.7",
    "key.keyboard.8",
    "key.keyboard.9",
];

pub const TRIPODS: usize = 9;

pub const SPEED_STEPS: [f64; 7] = [0.1, 0.25, 0.5, 1.0, 1.5, 2.5, 4.0];
pub const SPRINT_STEPS: [f64; 4] = [1.5, 2.0, 3.0, 5.0];
pub const MIN_SPEED: f64 = 0.02;
pub const MAX_SPEED: f64 = 8.0;
const DEFAULT_SPEED: f64 = 0.6;
const DEFAULT_SPRINT: f64 = 2.0;
const WHEEL_STEP: f64 = 1.2;
/// Degrees a mouse count turns the view: `Entity.turn` multiplies by 0.15.
const TURN: f64 = 0.15;
const MAX_PITCH: f32 = 90.0;
/// Half-width of the camera's collision box when "through walls" is off.
const RADIUS: f64 = 0.2;
/// Share of the gap to the wanted velocity closed each tick when smoothing is on.
const ACCEL: f64 = 0.35;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Settings {
    /// Blocks per tick before the sprint multiplier.
    pub speed: f64,
    /// Multiplier while the sprint input is held.
    pub sprint: f64,
    /// Fly through blocks. When false the camera stops at full blocks and slides along them.
    pub noclip: bool,
    /// Ease into and out of motion instead of moving at full speed at once.
    pub smooth: bool,
    /// Forward follows the look direction, up and down included. When false it stays level.
    pub fly_look: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self::new()
    }
}

impl Settings {
    pub const fn new() -> Self {
        Self { speed: DEFAULT_SPEED, sprint: DEFAULT_SPRINT, noclip: true, smooth: true, fly_look: true }
    }

    /// Reads the stored values; anything missing, unparsable or out of range falls back.
    pub fn read(get: impl Fn(&str) -> Option<String>) -> Self {
        let number = |key: &str, fallback: f64, lo: f64, hi: f64| {
            get(key)
                .and_then(|v| v.trim().parse::<f64>().ok())
                .filter(|v| v.is_finite())
                .map_or(fallback, |v| v.clamp(lo, hi))
        };
        let flag = |key: &str, fallback: bool| match get(key).as_deref().map(str::trim) {
            Some("true") => true,
            Some("false") => false,
            _ => fallback,
        };
        let d = Self::new();
        Self {
            speed: number("speed", d.speed, MIN_SPEED, MAX_SPEED),
            sprint: number("sprint", d.sprint, 1.0, 10.0),
            noclip: flag("noclip", d.noclip),
            smooth: flag("smooth", d.smooth),
            fly_look: flag("fly_look", d.fly_look),
        }
    }

    pub fn stored(&self) -> [(&'static str, String); 5] {
        [
            ("speed", self.speed.to_string()),
            ("sprint", self.sprint.to_string()),
            ("noclip", self.noclip.to_string()),
            ("smooth", self.smooth.to_string()),
            ("fly_look", self.fly_look.to_string()),
        ]
    }

    /// Labels of the settings page buttons, in the order `click` takes them.
    pub fn labels(&self) -> Vec<String> {
        let on = |b: bool| if b { "ON" } else { "OFF" };
        vec![
            format!("Through walls: {}", on(self.noclip)),
            format!("Speed: {:.2} blocks per tick", self.speed),
            format!("Sprint multiplier: x{:.1}", self.sprint),
            format!("Smooth motion: {}", on(self.smooth)),
            format!("Fly where you look: {}", on(self.fly_look)),
        ]
    }

    pub const BUTTONS: usize = 5;

    pub fn click(&mut self, button: usize) {
        match button {
            0 => self.noclip = !self.noclip,
            1 => self.speed = next_step(&SPEED_STEPS, self.speed),
            2 => self.sprint = next_step(&SPRINT_STEPS, self.sprint),
            3 => self.smooth = !self.smooth,
            4 => self.fly_look = !self.fly_look,
            _ => {}
        }
    }
}

/// The first step above `current`, wrapping to the first one.
fn next_step(steps: &[f64], current: f64) -> f64 {
    steps.iter().copied().find(|s| *s > current + 1e-9).unwrap_or(steps[0])
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pose {
    pub at: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
}

impl Pose {
    pub fn is_finite(&self) -> bool {
        self.at.iter().all(|v| v.is_finite()) && self.yaw.is_finite() && self.pitch.is_finite()
    }

    /// `x,y,z,yaw,pitch`, for keeping a camera spot in a setting.
    pub fn encode(&self) -> String {
        format!("{},{},{},{},{}", self.at[0], self.at[1], self.at[2], self.yaw, self.pitch)
    }

    pub fn decode(text: &str) -> Option<Self> {
        let mut parts = text.trim().split(',').map(|p| p.trim().parse::<f64>().ok());
        let mut next = || parts.next().flatten();
        let pose = Self { at: [next()?, next()?, next()?], yaw: next()? as f32, pitch: next()? as f32 };
        pose.is_finite().then_some(pose)
    }
}

#[derive(Clone, Copy, Debug)]
struct Camera {
    at: [f64; 3],
    previous: [f64; 3],
    velocity: [f64; 3],
    yaw: f32,
    pitch: f32,
}

#[derive(Debug)]
pub struct Freecam {
    pub settings: Settings,
    camera: Option<Camera>,
    held: [bool; INPUTS],
    tripods: [Option<Pose>; TRIPODS],
}

impl Default for Freecam {
    fn default() -> Self {
        Self::new()
    }
}

impl Freecam {
    pub const fn new() -> Self {
        Self { settings: Settings::new(), camera: None, held: [false; INPUTS], tripods: [None; TRIPODS] }
    }

    pub fn active(&self) -> bool {
        self.camera.is_some()
    }

    /// Detaches the camera at `pose`. A pose that is not finite is refused.
    pub fn start(&mut self, pose: Pose) -> bool {
        if !pose.is_finite() {
            return false;
        }
        self.camera = Some(Camera {
            at: pose.at,
            previous: pose.at,
            velocity: [0.0; 3],
            yaw: wrap_yaw(pose.yaw),
            pitch: pose.pitch.clamp(-MAX_PITCH, MAX_PITCH),
        });
        self.held = [false; INPUTS];
        true
    }

    /// Returns the camera to the player. Gives back the pose it was left at.
    pub fn stop(&mut self) -> Option<Pose> {
        self.held = [false; INPUTS];
        self.camera.take().map(|c| Pose { at: c.at, yaw: c.yaw, pitch: c.pitch })
    }

    pub fn hold(&mut self, input: usize, down: bool) {
        if let Some(held) = self.held.get_mut(input) {
            *held = down;
        }
    }

    pub fn release_all(&mut self) {
        self.held = [false; INPUTS];
    }

    /// Mouse look: `xo` and `yo` are the turn the game would give the player.
    /// Returns whether the camera took it.
    pub fn turn(&mut self, xo: f64, yo: f64) -> bool {
        let Some(c) = self.camera.as_mut() else { return false };
        if xo.is_finite() && yo.is_finite() {
            c.yaw = wrap_yaw(c.yaw + (xo * TURN) as f32);
            c.pitch = (c.pitch + (yo * TURN) as f32).clamp(-MAX_PITCH, MAX_PITCH);
        }
        true
    }

    /// Sets the look direction outright (an adapter whose game gives the angles, not the mouse).
    pub fn set_look(&mut self, yaw: f32, pitch: f32) {
        if let Some(c) = self.camera.as_mut()
            && yaw.is_finite()
            && pitch.is_finite()
        {
            c.yaw = wrap_yaw(yaw);
            c.pitch = pitch.clamp(-MAX_PITCH, MAX_PITCH);
        }
    }

    /// One game tick. `solid` says whether a block is a full collision block.
    pub fn tick(&mut self, solid: impl Fn([i32; 3]) -> bool) {
        let Settings { speed, sprint, noclip, smooth, fly_look } = self.settings;
        let held = self.held;
        let Some(c) = self.camera.as_mut() else { return };
        let wanted = motion(c.yaw, c.pitch, held, speed, sprint, fly_look);
        for axis in 0..3 {
            c.velocity[axis] = if smooth { c.velocity[axis] + (wanted[axis] - c.velocity[axis]) * ACCEL } else { wanted[axis] };
            if c.velocity[axis].abs() < 1e-6 {
                c.velocity[axis] = 0.0;
            }
        }
        c.previous = c.at;
        c.at = if noclip { add(c.at, c.velocity) } else { slide(c.at, c.velocity, &solid) };
    }

    /// The camera `partial` ticks after the last tick, for the frame being drawn.
    pub fn view(&self, partial: f32) -> Option<Pose> {
        let c = self.camera?;
        let t = f64::from(partial).clamp(0.0, 1.0);
        Some(Pose { at: std::array::from_fn(|i| c.previous[i] + (c.at[i] - c.previous[i]) * t), yaw: c.yaw, pitch: c.pitch })
    }

    /// Mouse wheel: scales the speed. `None` while the camera is attached to the player.
    pub fn scroll(&mut self, steps: f32) -> Option<f64> {
        self.camera?;
        if steps.is_finite() {
            self.settings.speed = (self.settings.speed * WHEEL_STEP.powf(f64::from(steps))).clamp(MIN_SPEED, MAX_SPEED);
        }
        Some(self.settings.speed)
    }

    /// Remembers the camera's place in spot `slot` (0 to 8).
    pub fn save_spot(&mut self, slot: usize) -> bool {
        let (Some(c), Some(spot)) = (self.camera, self.tripods.get_mut(slot)) else { return false };
        *spot = Some(Pose { at: c.at, yaw: c.yaw, pitch: c.pitch });
        true
    }

    /// Moves the camera to spot `slot`. False when it is empty or the camera is attached.
    pub fn recall_spot(&mut self, slot: usize) -> bool {
        let Some(Some(pose)) = self.tripods.get(slot).copied() else { return false };
        let Some(c) = self.camera.as_mut() else { return false };
        c.at = pose.at;
        c.previous = pose.at;
        c.velocity = [0.0; 3];
        c.yaw = pose.yaw;
        c.pitch = pose.pitch;
        true
    }

    pub fn spot(&self, slot: usize) -> Option<Pose> {
        self.tripods.get(slot).copied().flatten()
    }

    pub fn set_spot(&mut self, slot: usize, pose: Option<Pose>) {
        if let Some(spot) = self.tripods.get_mut(slot) {
            *spot = pose.filter(Pose::is_finite);
        }
    }

    /// Back to the player and forget every spot (a disconnect: coordinates mean nothing in the
    /// next world). The settings stay.
    pub fn reset(&mut self) {
        self.camera = None;
        self.held = [false; INPUTS];
        self.tripods = [None; TRIPODS];
    }
}

static STATE: Mutex<Freecam> = Mutex::new(Freecam::new());

/// Runs `f` on the one shared camera. Hooks may arrive from different threads of the game, so it
/// sits behind a lock; a hook that panicked earlier does not poison later ones.
pub fn with<R>(f: impl FnOnce(&mut Freecam) -> R) -> R {
    let mut guard = STATE.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    f(&mut guard)
}

fn wrap_yaw(yaw: f32) -> f32 {
    if !yaw.is_finite() {
        return 0.0;
    }
    let wrapped = (yaw + 180.0).rem_euclid(360.0) - 180.0;
    if wrapped == -180.0 { 180.0 } else { wrapped }
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

/// Velocity wanted by the held inputs: blocks per tick along x, y, z.
pub fn motion(yaw: f32, pitch: f32, held: [bool; INPUTS], speed: f64, sprint: f64, fly_look: bool) -> [f64; 3] {
    let axis = |plus: usize, minus: usize| f64::from(u8::from(held[plus])) - f64::from(u8::from(held[minus]));
    let (forward, strafe, rise) = (axis(FORWARD, BACK), axis(LEFT, RIGHT), axis(UP, DOWN));
    let speed = if held[SPRINT] { speed * sprint } else { speed };
    let yaw = f64::from(yaw).to_radians();
    let pitch = if fly_look { f64::from(pitch).to_radians() } else { 0.0 };
    // Facing +Z at yaw 0; left is +X there.
    let look = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
    let left = [yaw.cos(), 0.0, yaw.sin()];
    let mut v = [
        look[0] * forward + left[0] * strafe,
        look[1] * forward + rise,
        look[2] * forward + left[2] * strafe,
    ];
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length < 1e-9 {
        return [0.0; 3];
    }
    // Opposing keys have already cancelled; keep diagonals as fast as straight lines.
    let scale = speed / length.max(1.0);
    for part in &mut v {
        *part *= scale;
    }
    v
}

/// Moves `at` by `delta` one axis at a time, so the camera slides along walls.
pub fn slide(at: [f64; 3], delta: [f64; 3], solid: &impl Fn([i32; 3]) -> bool) -> [f64; 3] {
    let mut next = at;
    for axis in 0..3 {
        let mut moved = next;
        moved[axis] += delta[axis];
        if !blocked(moved, solid) {
            next = moved;
        }
    }
    next
}

fn blocked(at: [f64; 3], solid: &impl Fn([i32; 3]) -> bool) -> bool {
    let low = at.map(|v| (v - RADIUS).floor() as i32);
    let high = at.map(|v| (v + RADIUS).floor() as i32);
    (low[0]..=high[0]).any(|x| (low[1]..=high[1]).any(|y| (low[2]..=high[2]).any(|z| solid([x, y, z]))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f64; 3], b: [f64; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-9)
    }

    fn held(inputs: &[usize]) -> [bool; INPUTS] {
        std::array::from_fn(|i| inputs.contains(&i))
    }

    fn pose(at: [f64; 3]) -> Pose {
        Pose { at, yaw: 0.0, pitch: 0.0 }
    }

    fn stored<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        |key| pairs.iter().find(|p| p.0 == key).map(|p| p.1.to_owned())
    }

    #[test]
    fn motion_follows_where_the_camera_faces() {
        let m = |yaw, pitch, keys: &[usize], fly| motion(yaw, pitch, held(keys), 1.0, 2.0, fly);
        assert!(close(m(0.0, 0.0, &[FORWARD], true), [0.0, 0.0, 1.0]));
        assert!(close(m(90.0, 0.0, &[FORWARD], true), [-1.0, 0.0, 0.0]));
        assert!(close(m(-90.0, 0.0, &[FORWARD], true), [1.0, 0.0, 0.0]));
        assert!(close(m(0.0, 0.0, &[LEFT], true), [1.0, 0.0, 0.0]));
        assert!(close(m(0.0, 0.0, &[RIGHT], true), [-1.0, 0.0, 0.0]));
        assert!(close(m(0.0, 0.0, &[BACK], true), [0.0, 0.0, -1.0]));
        assert!(close(m(0.0, 0.0, &[UP], true), [0.0, 1.0, 0.0]));
        assert!(close(m(0.0, 0.0, &[DOWN], true), [0.0, -1.0, 0.0]));
        // Looking straight down and flying forward goes down; level flight ignores the pitch.
        assert!(close(m(0.0, 90.0, &[FORWARD], true), [0.0, -1.0, 0.0]));
        assert!(close(m(0.0, 90.0, &[FORWARD], false), [0.0, 0.0, 1.0]));
    }

    #[test]
    fn diagonals_are_not_faster_and_opposites_cancel() {
        let diagonal = motion(0.0, 0.0, held(&[FORWARD, LEFT]), 1.0, 2.0, true);
        let length = (diagonal[0].powi(2) + diagonal[1].powi(2) + diagonal[2].powi(2)).sqrt();
        assert!((length - 1.0).abs() < 1e-9);
        let all = motion(35.0, 20.0, held(&[FORWARD, RIGHT, UP]), 1.0, 2.0, true);
        assert!(((all[0].powi(2) + all[1].powi(2) + all[2].powi(2)).sqrt() - 1.0).abs() < 1e-9);
        assert!(close(motion(0.0, 0.0, held(&[FORWARD, BACK]), 1.0, 2.0, true), [0.0; 3]));
        assert!(close(motion(0.0, 0.0, held(&[]), 1.0, 2.0, true), [0.0; 3]));
    }

    #[test]
    fn sprint_multiplies_the_speed() {
        assert!(close(motion(0.0, 0.0, held(&[UP, SPRINT]), 0.5, 3.0, true), [0.0, 1.5, 0.0]));
        // The sprint input alone moves nothing.
        assert!(close(motion(0.0, 0.0, held(&[SPRINT]), 0.5, 3.0, true), [0.0; 3]));
    }

    #[test]
    fn slides_along_walls_and_through_air() {
        let wall = |p: [i32; 3]| p[0] == 1;
        assert!(close(slide([0.5, 0.5, 0.5], [0.6, 0.0, 0.4], &wall), [0.5, 0.5, 0.9]));
        assert!(close(slide([0.5, 0.5, 0.5], [0.6, 0.0, 0.4], &|_| false), [1.1, 0.5, 0.9]));
    }

    #[test]
    fn smooth_motion_eases_in_and_out() {
        let mut cam = Freecam::new();
        cam.settings = Settings { speed: 1.0, noclip: true, smooth: true, ..Settings::new() };
        assert!(cam.start(pose([0.0, 64.0, 0.0])));
        cam.hold(UP, true);
        cam.tick(|_| false);
        let first = cam.view(1.0).unwrap().at[1] - 64.0;
        assert!(first > 0.0 && first < 0.5, "eased in, got {first}");
        for _ in 0..60 {
            cam.tick(|_| false);
        }
        let before = cam.view(1.0).unwrap().at[1];
        cam.tick(|_| false);
        assert!((cam.view(1.0).unwrap().at[1] - before - 1.0).abs() < 1e-3, "reaches full speed");
        cam.hold(UP, false);
        let mut last = cam.view(1.0).unwrap().at[1];
        for _ in 0..200 {
            cam.tick(|_| false);
        }
        let y = cam.view(1.0).unwrap().at[1];
        assert!(y > last, "coasts a little after release");
        last = y;
        cam.tick(|_| false);
        assert_eq!(cam.view(1.0).unwrap().at[1], last, "and comes to rest");
    }

    #[test]
    fn instant_motion_without_smoothing_and_interpolation_between_ticks() {
        let mut cam = Freecam::new();
        cam.settings = Settings { speed: 1.0, smooth: false, ..Settings::new() };
        cam.start(pose([0.0, 64.0, 0.0]));
        cam.hold(UP, true);
        cam.tick(|_| false);
        assert!(close(cam.view(0.0).unwrap().at, [0.0, 64.0, 0.0]));
        assert!(close(cam.view(0.5).unwrap().at, [0.0, 64.5, 0.0]));
        assert!(close(cam.view(1.0).unwrap().at, [0.0, 65.0, 0.0]));
        // A partial tick outside 0..=1 is clamped, not extrapolated.
        assert!(close(cam.view(7.0).unwrap().at, [0.0, 65.0, 0.0]));
    }

    #[test]
    fn collision_applies_only_without_noclip() {
        let floor = |p: [i32; 3]| p[1] <= 63;
        let mut cam = Freecam::new();
        cam.settings = Settings { speed: 1.0, noclip: false, smooth: false, ..Settings::new() };
        cam.start(pose([0.5, 64.5, 0.5]));
        cam.hold(DOWN, true);
        cam.tick(floor);
        assert!(close(cam.view(1.0).unwrap().at, [0.5, 64.5, 0.5]), "stopped by the floor");
        cam.settings.noclip = true;
        cam.tick(floor);
        assert!(close(cam.view(1.0).unwrap().at, [0.5, 63.5, 0.5]), "passes through it");
    }

    #[test]
    fn turning_wraps_yaw_and_clamps_pitch() {
        let mut cam = Freecam::new();
        assert!(!cam.turn(10.0, 10.0), "no camera, the player keeps the mouse");
        cam.start(Pose { at: [0.0; 3], yaw: 170.0, pitch: 0.0 });
        assert!(cam.turn(200.0, 1000.0));
        let v = cam.view(0.0).unwrap();
        assert!((v.yaw - (-160.0)).abs() < 1e-4, "yaw wrapped, got {}", v.yaw);
        assert_eq!(v.pitch, 90.0);
        assert!(cam.turn(f64::NAN, 1.0), "the camera still owns the mouse");
        assert_eq!(cam.view(0.0).unwrap().yaw, v.yaw, "a bad delta changes nothing");
        cam.set_look(f32::NAN, 0.0);
        assert_eq!(cam.view(0.0).unwrap().yaw, v.yaw);
        cam.set_look(90.0, -200.0);
        let v = cam.view(0.0).unwrap();
        assert_eq!((v.yaw, v.pitch), (90.0, -90.0));
    }

    #[test]
    fn start_and_stop_reset_input() {
        let mut cam = Freecam::new();
        assert!(!cam.start(Pose { at: [f64::NAN, 0.0, 0.0], yaw: 0.0, pitch: 0.0 }));
        assert!(!cam.active());
        cam.start(pose([1.0, 2.0, 3.0]));
        cam.hold(FORWARD, true);
        cam.hold(99, true); // ignored
        let left = cam.stop().unwrap();
        assert_eq!(left.at, [1.0, 2.0, 3.0]);
        assert!(!cam.active());
        assert_eq!(cam.stop(), None);
        cam.start(pose([0.0; 3]));
        cam.settings.smooth = false;
        cam.tick(|_| false);
        assert!(close(cam.view(1.0).unwrap().at, [0.0; 3]), "keys held before stop do not carry over");
    }

    #[test]
    fn wheel_scales_the_speed_within_limits() {
        let mut cam = Freecam::new();
        assert_eq!(cam.scroll(1.0), None, "the wheel belongs to the hotbar while attached");
        cam.start(pose([0.0; 3]));
        cam.settings.speed = 1.0;
        assert!((cam.scroll(1.0).unwrap() - 1.2).abs() < 1e-12);
        assert_eq!(cam.scroll(100.0), Some(MAX_SPEED));
        assert_eq!(cam.scroll(-100.0), Some(MIN_SPEED));
        let kept = cam.settings.speed;
        assert_eq!(cam.scroll(f32::NAN), Some(kept));
    }

    #[test]
    fn spots_save_recall_and_round_trip_through_text() {
        let mut cam = Freecam::new();
        assert!(!cam.save_spot(0), "needs a camera");
        cam.start(Pose { at: [10.0, 70.0, -4.5], yaw: 45.0, pitch: 12.0 });
        assert!(cam.save_spot(2));
        assert!(!cam.save_spot(TRIPODS));
        cam.settings.smooth = false;
        cam.hold(UP, true);
        cam.tick(|_| false);
        assert!(!close(cam.view(1.0).unwrap().at, [10.0, 70.0, -4.5]));
        assert!(cam.recall_spot(2));
        assert_eq!(cam.view(0.0).unwrap(), Pose { at: [10.0, 70.0, -4.5], yaw: 45.0, pitch: 12.0 });
        assert!(!cam.recall_spot(3), "empty spot");
        let text = cam.spot(2).unwrap().encode();
        assert_eq!(Pose::decode(&text), cam.spot(2));
        assert_eq!(Pose::decode("1,2,3"), None);
        assert_eq!(Pose::decode("1,2,3,x,5"), None);
        assert_eq!(Pose::decode("NaN,2,3,4,5"), None);
        let mut other = Freecam::new();
        other.set_spot(0, Pose::decode(&text));
        other.set_spot(1, Some(Pose { at: [f64::INFINITY, 0.0, 0.0], yaw: 0.0, pitch: 0.0 }));
        assert_eq!(other.spot(0), cam.spot(2));
        assert_eq!(other.spot(1), None);
        other.start(pose([0.0; 3]));
        assert!(other.recall_spot(0));
    }

    #[test]
    fn settings_read_clamp_round_trip_and_click() {
        assert_eq!(Settings::read(stored(&[])), Settings::new());
        let s = Settings::read(stored(&[("speed", "99"), ("sprint", "0"), ("noclip", "false"), ("smooth", "maybe"), ("fly_look", " false ")]));
        assert_eq!(s.speed, MAX_SPEED);
        assert_eq!(s.sprint, 1.0);
        assert!(!s.noclip && s.smooth && !s.fly_look);
        assert_eq!(Settings::read(stored(&[("speed", "NaN")])).speed, Settings::new().speed);
        assert_eq!(Settings::read(stored(&[("speed", "inf")])).speed, Settings::new().speed);
        let pairs: Vec<(&str, String)> = s.stored().into();
        let borrowed: Vec<(&str, &str)> = pairs.iter().map(|(k, v)| (*k, v.as_str())).collect();
        assert_eq!(Settings::read(stored(&borrowed)), s);

        let mut c = Settings { speed: 1.0, ..Settings::new() };
        c.click(1);
        assert_eq!(c.speed, 1.5);
        c.speed = 4.0;
        c.click(1);
        assert_eq!(c.speed, SPEED_STEPS[0], "wraps around");
        c.click(2);
        assert_eq!(c.sprint, 3.0);
        c.click(0);
        c.click(3);
        c.click(4);
        c.click(99);
        assert!(!c.noclip && !c.smooth && !c.fly_look);
        assert_eq!(c.labels().len(), Settings::BUTTONS);
        assert_eq!(c.labels()[0], "Through walls: OFF");
    }

    #[test]
    fn shared_state_survives_a_poisoned_lock() {
        with(|f| f.stop());
        let result = std::panic::catch_unwind(|| with(|_| panic!("a hook failed")));
        assert!(result.is_err());
        assert!(with(|f| f.start(pose([0.0; 3]))), "later hooks still work");
        assert!(with(|f| f.active()));
        with(|f| f.stop());
    }

    #[test]
    fn reset_detaches_and_forgets_spots_but_keeps_settings() {
        let mut cam = Freecam::new();
        cam.settings.speed = 3.0;
        cam.start(pose([1.0, 2.0, 3.0]));
        cam.save_spot(4);
        cam.reset();
        assert!(!cam.active());
        assert_eq!(cam.spot(4), None);
        assert_eq!(cam.settings.speed, 3.0);
    }

    #[test]
    fn yaw_wrapping_keeps_the_range_stable() {
        assert_eq!(wrap_yaw(0.0), 0.0);
        assert_eq!(wrap_yaw(180.0), 180.0);
        assert_eq!(wrap_yaw(-180.0), 180.0);
        assert_eq!(wrap_yaw(540.0), 180.0);
        assert_eq!(wrap_yaw(f32::INFINITY), 0.0);
        assert!((wrap_yaw(190.0) - (-170.0)).abs() < 1e-4);
    }
}

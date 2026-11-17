// Glue between the freecam core and Horizon's native hook points (Nightly 9f42064 and the builds
// that keep these hooks). Compiled into `horizon-client` as `crate::game::uni_freecam_glue`.
// A support module body: items only, no file attributes.

use super::perspective::CameraType;
use super::uni_freecam_core as fc;
use super::Game;
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

const VIEWS: [CameraType; 3] = [CameraType::FirstPerson, CameraType::ThirdPersonBack, CameraType::ThirdPersonFront];
const ALT_KEYS: [&str; 2] = ["key.keyboard.left.alt", "key.keyboard.right.alt"];
const SHIFT_KEYS: [&str; 2] = ["key.keyboard.left.shift", "key.keyboard.right.shift"];

/// The view the player had before the camera left, as an index into `VIEWS`.
static VIEW: AtomicU8 = AtomicU8::new(0);
static ALT: AtomicBool = AtomicBool::new(false);
static SHIFT: AtomicBool = AtomicBool::new(false);
static SETTINGS_LOADED: AtomicBool = AtomicBool::new(false);

/// Reads the stored settings once, then keeps the live copy in the core.
fn settings(game: &Game<'_>) -> fc::Settings {
    if !SETTINGS_LOADED.swap(true, Ordering::Relaxed) {
        let read = fc::Settings::read(|key| game.native_setting(fc::ID, key));
        fc::with(|f| f.settings = read);
    }
    fc::with(|f| f.settings)
}

fn store(game: &Game<'_>) {
    let settings = fc::with(|f| f.settings);
    for (key, value) in settings.stored() {
        game.native_set_setting(fc::ID, key, &value);
    }
}

fn active() -> bool {
    fc::with(|f| f.active())
}

pub(super) fn key(game: &mut Game<'_>, key: &str, pressed: bool) {
    if ALT_KEYS.contains(&key) {
        ALT.store(pressed, Ordering::Relaxed);
    }
    if SHIFT_KEYS.contains(&key) {
        SHIFT.store(pressed, Ordering::Relaxed);
    }
    let playing = game.native_playing();
    if active() {
        if let Some(input) = fc::INPUT_KEYS.iter().position(|k| *k == key)
            && (!pressed || playing)
        {
            fc::with(|f| f.hold(input, pressed));
        }
        if pressed
            && playing
            && ALT.load(Ordering::Relaxed)
            && let Some(slot) = fc::DIGIT_KEYS.iter().position(|k| *k == key)
        {
            // Alt+digit: go to the spot, or save one there when it is empty. Alt+Shift+digit: save here.
            let shift = SHIFT.load(Ordering::Relaxed);
            fc::with(|f| {
                if shift || f.spot(slot).is_none() {
                    f.save_spot(slot);
                } else {
                    f.recall_spot(slot);
                }
            });
        }
    }
    if !pressed || !playing || !game.native_on(fc::ID) {
        return;
    }
    if key != game.native_mod_key(fc::ID, fc::KEY_TOGGLE, fc::DEFAULT_TOGGLE) {
        return;
    }
    if active() { exit(game) } else { enter(game) }
}

fn enter(game: &mut Game<'_>) {
    settings(game);
    let Some(player) = game.player.as_ref() else { return };
    let at = [player.position.x, player.position.y + f64::from(game.eye), player.position.z];
    let view = VIEWS.iter().position(|v| *v == game.camera_type).unwrap_or(0) as u8;
    let pose = fc::Pose { at, yaw: game.yaw, pitch: game.pitch };
    if fc::with(|f| f.start(pose)) {
        VIEW.store(view, Ordering::Relaxed);
        // The player's body shows only in a third person view.
        game.camera_type = CameraType::ThirdPersonBack;
    }
}

fn exit(game: &mut Game<'_>) {
    if fc::with(|f| f.stop()).is_some() {
        let view = usize::from(VIEW.load(Ordering::Relaxed)).min(VIEWS.len() - 1);
        game.camera_type = VIEWS[view];
    }
}

pub(super) fn movement(forward: &mut f32, strafe: &mut f32, jump: &mut bool, sneak: &mut bool) {
    if active() {
        *forward = 0.0;
        *strafe = 0.0;
        *jump = false;
        *sneak = false;
    }
}

pub(super) fn interaction(swing_hand: &mut bool, canceled: &mut bool) {
    if active() {
        *canceled = true;
        *swing_hand = false;
    }
}

pub(super) fn turn(xo: f64, yo: f64) -> ControlFlow<()> {
    if fc::with(|f| f.turn(xo, yo)) { ControlFlow::Break(()) } else { ControlFlow::Continue(()) }
}

pub(super) fn camera(partial: f32, position: &mut [f64; 3], yaw: &mut f32, pitch: &mut f32) {
    if let Some(pose) = fc::with(|f| f.view(partial)) {
        *position = pose.at;
        *yaw = pose.yaw;
        *pitch = pose.pitch;
    }
}

pub(super) fn tick(game: &mut Game<'_>) {
    if !active() {
        return;
    }
    if game.player.is_none() || !game.native_on(fc::ID) {
        exit(game);
        return;
    }
    game.camera_type = CameraType::ThirdPersonBack;
    let game = &*game;
    fc::with(|f| f.tick(|[x, y, z]| game.res.facts().get(game.world.block(x, y, z)).full_collision));
}

pub(super) fn wheel(game: &mut Game<'_>, steps: f32, claimed: &mut bool) {
    if !game.native_on(fc::ID) {
        return;
    }
    if fc::with(|f| f.scroll(steps)).is_some() {
        store(game);
        *claimed = true;
    }
}

pub(super) fn disconnect(_game: &Game<'_>) {
    fc::with(|f| f.reset());
    ALT.store(false, Ordering::Relaxed);
    SHIFT.store(false, Ordering::Relaxed);
}

pub(super) fn listed(mods: &mut Vec<super::native_hooks::NativeMod>) {
    mods.push(super::native_hooks::NativeMod {
        id: fc::ID.into(),
        title: format!("{} {}", fc::TITLE, fc::VERSION),
        description: fc::DESCRIPTION.into(),
        keys: vec![[fc::KEY_TOGGLE.into(), "Toggle freecam".into(), fc::DEFAULT_TOGGLE.into()]],
    });
}

pub(super) fn page(game: &Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    if mod_id == fc::ID {
        buttons.extend(settings(game).labels());
    }
}

pub(super) fn click(game: &mut Game<'_>, mod_id: &str, button: usize) {
    if mod_id != fc::ID {
        return;
    }
    settings(game);
    fc::with(|f| f.settings.click(button));
    store(game);
}

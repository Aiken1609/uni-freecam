//! Universal Freecam for Horizon builds that have the typed mod API (`mod_api::Hooks`).
//!
//! The camera logic lives in `freecam_core` (shared with the legacy adapter); this file only
//! connects it to the game's hooks.
//!
//! Limits of the public API, stated so they are not a surprise: there is no hook for raw mouse
//! movement, so the camera looks where the player looks and the player's body turns with it (the
//! server sees the head turn; the position does not change). Moving the camera itself is free.

use mod_api::{
    CameraPoseEvent, CameraType, ClientDisconnectingEvent, ClientHost, ClientTickPreEvent, Flow, Hooks,
    InputKeyEvent, KeyAction, KeyMappingPreEvent, KeyRegistration, MouseScrollingEvent, Registrar,
    RegistrationContext, SettingChangedEvent, SettingSpec,
};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

mod freecam_core;
use freecam_core as fc;

/// GLFW's modifier bits, as the key hooks give them.
const SHIFT: u32 = 1;
const ALT: u32 = 4;

/// The vanilla mappings the camera takes over while it flies, in the order of `freecam_core`'s inputs.
const FLY_MAPPINGS: [&str; fc::INPUTS] = ["key.forward", "key.back", "key.left", "key.right", "key.jump", "key.sneak", "key.sprint"];
/// Mappings that must not reach the player while the camera is away: moving, and acting on the
/// world from the player's own view (attack, use, pick, drop, swap, hotbar). They are read by
/// mapping name, never by key, so whatever the player rebound them to keeps working.
const BLOCKED_MAPPINGS: [&str; 21] = [
    "key.forward", "key.back", "key.left", "key.right", "key.jump", "key.sneak", "key.sprint",
    "key.attack", "key.use", "key.pickItem", "key.drop", "key.swapOffhand",
    "key.hotbar.1", "key.hotbar.2", "key.hotbar.3", "key.hotbar.4", "key.hotbar.5",
    "key.hotbar.6", "key.hotbar.7", "key.hotbar.8", "key.hotbar.9",
];

#[derive(Default)]
struct Mod {
    /// The toggle was pressed to start; the camera starts at the next frame's pose.
    start_pending: AtomicBool,
    /// The perspective to give back, as 0 first person, 1 third back, 2 third front.
    view: AtomicU8,
}

fn view_index(view: CameraType) -> u8 {
    match view {
        CameraType::FirstPerson => 0,
        CameraType::ThirdPersonBack => 1,
        CameraType::ThirdPersonFront => 2,
    }
}

fn view_from(index: u8) -> CameraType {
    match index {
        1 => CameraType::ThirdPersonBack,
        2 => CameraType::ThirdPersonFront,
        _ => CameraType::FirstPerson,
    }
}

/// Copies the player's saved settings into the camera.
fn load_settings(host: &dyn ClientHost) {
    let d = fc::Settings::new();
    let settings = fc::Settings {
        speed: host.setting_f64("speed").unwrap_or(d.speed).clamp(fc::MIN_SPEED, fc::MAX_SPEED),
        sprint: host.setting_f64("sprint").unwrap_or(d.sprint).clamp(1.0, 10.0),
        noclip: host.setting_bool("noclip").unwrap_or(d.noclip),
        smooth: host.setting_bool("smooth").unwrap_or(d.smooth),
        fly_look: host.setting_bool("fly_look").unwrap_or(d.fly_look),
    };
    fc::with(|f| f.settings = settings);
}

impl Mod {
    fn leave(&self, host: &mut dyn ClientHost) {
        self.start_pending.store(false, Ordering::Relaxed);
        if fc::with(|f| f.stop()).is_some() {
            host.set_camera_type(view_from(self.view.load(Ordering::Relaxed)));
        }
    }
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        r.register_key_mapping(
            KeyRegistration::new("toggle", fc::DEFAULT_TOGGLE)
                .label("key.uni_freecam.toggle")
                .category("key.uni_freecam.category"),
        );
        let settings = [
            SettingSpec::range("speed", "uni_freecam.setting.speed", 0.05, 4.0, 0.05, fc::Settings::new().speed)
                .tooltip("uni_freecam.setting.speed.tooltip"),
            SettingSpec::range("sprint", "uni_freecam.setting.sprint", 1.0, 6.0, 0.5, fc::Settings::new().sprint)
                .tooltip("uni_freecam.setting.sprint.tooltip"),
            SettingSpec::toggle("noclip", "uni_freecam.setting.noclip", true).tooltip("uni_freecam.setting.noclip.tooltip"),
            SettingSpec::toggle("smooth", "uni_freecam.setting.smooth", true).tooltip("uni_freecam.setting.smooth.tooltip"),
            SettingSpec::toggle("fly_look", "uni_freecam.setting.fly_look", true).tooltip("uni_freecam.setting.fly_look.tooltip"),
        ];
        for setting in settings {
            // A refused setting only loses its row; the camera falls back to the defaults.
            let _ = r.register_setting(setting);
        }
    }

    fn setting_changed(&self, host: &mut dyn ClientHost, event: &mut SettingChangedEvent<'_>) {
        if event.mod_id == fc::ID {
            load_settings(host);
        }
    }

    fn client_tick_pre(&self, host: &mut dyn ClientHost, _event: &mut ClientTickPreEvent) {
        let pressed = host.consume_click(fc::ID, "toggle");
        let playing = host.playing();
        if pressed && playing {
            if fc::with(|f| f.active()) || self.start_pending.load(Ordering::Relaxed) {
                self.leave(host);
            } else if host.camera_entity().is_some() {
                load_settings(host);
                // Let go of anything held, so the player does not keep walking while the camera flies.
                for mapping in FLY_MAPPINGS {
                    let _ = host.set_mapping_down(mapping, false);
                }
                self.start_pending.store(true, Ordering::Relaxed);
            }
        }
        if !fc::with(|f| f.active()) {
            return;
        }
        if host.camera_entity().is_none() {
            // No world any more (a disconnect, a death screen that dropped the player).
            fc::with(|f| f.reset());
            return;
        }
        if !playing {
            // A screen is open: the keys are not read, so nothing is still held.
            fc::with(|f| f.release_all());
        }
        let host = &*host;
        fc::with(|f| f.tick(|pos| !host.block_collision(pos).is_empty()));
    }

    fn key_mapping_pre(&self, _host: &mut dyn ClientHost, event: &mut KeyMappingPreEvent<'_>) -> Flow {
        if !fc::with(|f| f.active()) {
            return Flow::Continue;
        }
        if let Some(input) = FLY_MAPPINGS.iter().position(|m| *m == event.mapping) {
            fc::with(|f| f.hold(input, event.pressed));
        }
        // A release always goes through (the game holds nothing anyway); only presses are dropped.
        if event.pressed && BLOCKED_MAPPINGS.contains(&event.mapping) { Flow::Cancel } else { Flow::Continue }
    }

    fn input_key(&self, host: &mut dyn ClientHost, event: &mut InputKeyEvent<'_>) {
        if event.action != KeyAction::Press || !host.playing() || !fc::with(|f| f.active()) {
            return;
        }
        if event.modifiers & ALT == 0 {
            return;
        }
        let Some(slot) = fc::DIGIT_KEYS.iter().position(|k| *k == event.key) else { return };
        // Alt+digit: go to the spot, or save one there when it is empty. Alt+Shift+digit: save here.
        let shift = event.modifiers & SHIFT != 0;
        fc::with(|f| {
            if shift || f.spot(slot).is_none() {
                f.save_spot(slot);
            } else {
                f.recall_spot(slot);
            }
        });
    }

    fn mouse_scrolling(&self, host: &mut dyn ClientHost, event: &mut MouseScrollingEvent) -> Flow {
        if !host.playing() {
            return Flow::Continue;
        }
        let steps = event.accumulated[1] as f32;
        if fc::with(|f| f.scroll(steps)).is_some() { Flow::Cancel } else { Flow::Continue }
    }

    fn camera_pose(&self, host: &dyn ClientHost, event: &mut CameraPoseEvent<'_>) {
        if self.start_pending.swap(false, Ordering::Relaxed) {
            let pose = fc::Pose { at: *event.position, yaw: event.yaw, pitch: event.pitch };
            if fc::with(|f| f.start(pose)) {
                self.view.store(view_index(host.camera_type()), Ordering::Relaxed);
                // The player's body shows only in a third person view.
                host.set_camera_type(CameraType::ThirdPersonBack);
            }
        }
        // The camera looks where the player looks (see the module docs), and flies from its own spot.
        let Some(pose) = fc::with(|f| {
            f.set_look(event.yaw, event.pitch);
            f.view(event.partial)
        }) else {
            return;
        };
        *event.position = pose.at;
    }

    fn client_disconnecting(&self, _host: &dyn ClientHost, _event: &mut ClientDisconnectingEvent) {
        self.start_pending.store(false, Ordering::Relaxed);
        fc::with(|f| f.reset());
    }
}

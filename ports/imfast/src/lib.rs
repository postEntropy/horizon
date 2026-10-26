//! I'm Fast: lifts vanilla's movement speed cap.
//!
//! The original raises the hard-clamped entity speed with a mixin; a sandboxed
//! mod cannot touch the motion code, so the port moves the player itself while
//! a boost key is held, at a speed multiplier read from the mod's config.
use horizon_mod_sdk::{
    Mod, Registration, config, dimensions, game,
    keys::{self, KeyBinding},
};

const BOOST_KEY: &str = "imfast_boost";

struct ImFast {
    last_multiplier: f64,
}

fn multiplier() -> f64 {
    config::get_float("boost")
        .unwrap_or(3.0)
        .clamp(0.5, 20.0)
}

impl Mod for ImFast {
    fn init() -> Result<(Self, Registration), String> {
        let _ = keys::register(&KeyBinding {
            id: BOOST_KEY.into(),
            label: "I'm Fast: Speed Boost".into(),
            default_key: "KeyV".into(),
        });
        Ok((
            ImFast {
                last_multiplier: multiplier(),
            },
            Registration::new().on_tick().command("imfast"),
        ))
    }

    fn tick(&mut self, _tick: u64) {
        if !keys::is_down(BOOST_KEY) {
            return;
        }
        let Some(player) = game::get_player() else {
            return;
        };
        let speed = multiplier();
        // 0.02 blocks per tick is walking pace; the multiplier lifts the cap.
        let step = 0.02 * speed;
        let yaw = (player.yaw as f64).to_radians();
        let dx = -yaw.sin() * step;
        let dz = yaw.cos() * step;
        let _ = dimensions::teleport(
            "Player",
            &player.dimension,
            player.x + dx,
            player.y,
            player.z + dz,
        );
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "imfast" {
            return Err(format!("Unknown command /{name}"));
        }
        let arg = args.trim();
        if !arg.is_empty() {
            let Ok(value) = arg.parse::<f64>() else {
                return Err("Usage: /imfast [multiplier]".into());
            };
            let value = value.clamp(0.5, 20.0);
            if let Err(error) = config::set_float("boost", value) {
                return Err(format!("Cannot set boost: {error}"));
            }
            self.last_multiplier = value;
        }
        let speed = multiplier();
        self.last_multiplier = speed;
        Ok(Some(format!(
            "💨 I'm Fast: hold V to move at {speed}× walking speed ({:.2} blocks/tick).",
            0.02 * speed
        )))
    }
}

horizon_mod_sdk::export_mod!(ImFast);

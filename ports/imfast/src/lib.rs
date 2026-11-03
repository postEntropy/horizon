//! I'm Fast: lifts vanilla's movement speed cap.
//!
//! The original raises the hard-clamped entity speed with a mixin; a sandboxed
//! mod cannot touch the motion code, so the port moves the player itself while
//! a boost key is held, at a speed multiplier read from the mod's config.
use horizon_mod_sdk::{
    Mod, Registration, config, dimensions, game,
    keys::{self, KeyBinding},
};

pub const BOOST_KEY: &str = "imfast_boost";

pub fn calculate_step(yaw_degrees: f32, speed: f64) -> (f64, f64) {
    let step = 0.02 * speed;
    let yaw = (yaw_degrees as f64).to_radians();
    let dx = -yaw.sin() * step;
    let dz = yaw.cos() * step;
    (dx, dz)
}

pub fn clamp_multiplier(val: f64) -> f64 {
    val.clamp(0.5, 20.0)
}

pub struct ImFast {
    pub last_multiplier: f64,
}

impl Default for ImFast {
    fn default() -> Self {
        Self {
            last_multiplier: 3.0,
        }
    }
}

fn multiplier() -> f64 {
    config::get_float("boost").unwrap_or(3.0).clamp(0.5, 20.0)
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
        let (dx, dz) = calculate_step(player.yaw, speed);
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
            let value = clamp_multiplier(value);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_step_vector_calculation() {
        // Yaw 0 (facing south in MC): sin(0)=0, cos(0)=1 -> dx=0, dz=step
        let (dx, dz) = calculate_step(0.0, 5.0);
        assert!((dx - 0.0).abs() < 1e-5);
        assert!((dz - 0.1).abs() < 1e-5);

        // Yaw 180 (facing north): sin(pi)=0, cos(pi)=-1 -> dx=0, dz=-step
        let (dx, dz) = calculate_step(180.0, 5.0);
        assert!((dx - 0.0).abs() < 1e-5);
        assert!((dz - -0.1).abs() < 1e-5);
    }

    #[test]
    fn test_multiplier_clamping() {
        assert_eq!(clamp_multiplier(0.1), 0.5);
        assert_eq!(clamp_multiplier(5.0), 5.0);
        assert_eq!(clamp_multiplier(100.0), 20.0);
    }
}

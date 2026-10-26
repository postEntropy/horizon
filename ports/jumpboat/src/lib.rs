//! Jumpy Boats: allows boats to jump when the player presses Space.
use horizon_mod_sdk::{
    Mod, Registration, entities, game, keys::{self, KeyBinding},
};

struct JumpBoat;

impl Mod for JumpBoat {
    fn init() -> Result<(Self, Registration), String> {
        let _ = keys::register(&KeyBinding {
            id: "boat_jump".into(),
            label: "Boat Jump".into(),
            default_key: "Space".into(),
        });
        Ok((JumpBoat, Registration::new().on_tick()))
    }

    fn tick(&mut self, _tick: u64) {
        if keys::was_pressed("boat_jump") {
            if let Some(player) = game::get_player() {
                // Find nearest boat entity within riding distance
                for ent in entities::nearby(&player.dimension, player.x, player.y, player.z, 1.5) {
                    if ent.kind.contains("boat") || ent.kind.contains("raft") {
                        // Apply upward jump velocity to boat
                        let _ = entities::set_velocity(ent.handle, ent.vx, 0.42, ent.vz);
                        break;
                    }
                }
            }
        }
    }
}

horizon_mod_sdk::export_mod!(JumpBoat);

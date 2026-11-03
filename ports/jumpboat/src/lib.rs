//! Jumpy Boats: allows boats and rafts to jump when the player presses Space.
use horizon_mod_sdk::{
    Mod, Registration, entities, game,
    keys::{self, KeyBinding},
};

pub struct JumpPhysics {
    pub jump_power: f64,
    pub cooldown_ticks: u32,
    pub current_cooldown: u32,
}

impl Default for JumpPhysics {
    fn default() -> Self {
        Self {
            jump_power: 0.42,
            cooldown_ticks: 10,
            current_cooldown: 0,
        }
    }
}

impl JumpPhysics {
    pub fn new(jump_power: f64) -> Self {
        Self {
            jump_power: jump_power.clamp(0.1, 2.0),
            cooldown_ticks: 10,
            current_cooldown: 0,
        }
    }

    pub fn tick(&mut self) {
        if self.current_cooldown > 0 {
            self.current_cooldown -= 1;
        }
    }

    pub fn can_jump(&self) -> bool {
        self.current_cooldown == 0
    }

    pub fn trigger_jump(&mut self) -> Option<f64> {
        if self.can_jump() {
            self.current_cooldown = self.cooldown_ticks;
            Some(self.jump_power)
        } else {
            None
        }
    }
}

pub fn is_boat_or_raft(kind: &str) -> bool {
    let name = kind.split(':').nth(1).unwrap_or(kind);
    name.contains("boat") || name.contains("raft")
}

pub struct JumpBoat {
    pub physics: JumpPhysics,
}

impl Default for JumpBoat {
    fn default() -> Self {
        Self {
            physics: JumpPhysics::default(),
        }
    }
}

impl Mod for JumpBoat {
    fn init() -> Result<(Self, Registration), String> {
        let _ = keys::register(&KeyBinding {
            id: "boat_jump".into(),
            label: "Boat Jump".into(),
            default_key: "Space".into(),
        });
        Ok((
            JumpBoat::default(),
            Registration::new().on_tick().command("jumpboat"),
        ))
    }

    fn tick(&mut self, _tick: u64) {
        self.physics.tick();
        if keys::was_pressed("boat_jump") && self.physics.can_jump() {
            if let Some(player) = game::get_player() {
                for ent in entities::nearby(&player.dimension, player.x, player.y, player.z, 1.5) {
                    if is_boat_or_raft(&ent.kind) {
                        if let Some(vy) = self.physics.trigger_jump() {
                            let _ = entities::set_velocity(ent.handle, ent.vx, vy, ent.vz);
                        }
                        break;
                    }
                }
            }
        }
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "jumpboat" {
            return Err(format!("Unknown command /{name}"));
        }
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] => Ok(Some(format!(
                "Jumpy Boats: jump_power = {:.2}, cooldown = {} ticks.",
                self.physics.jump_power, self.physics.cooldown_ticks
            ))),
            [power_str] => {
                if let Ok(val) = power_str.parse::<f64>() {
                    self.physics.jump_power = val.clamp(0.1, 2.0);
                    Ok(Some(format!(
                        "Jumpy Boats: jump_power set to {:.2}.",
                        self.physics.jump_power
                    )))
                } else {
                    Err("Invalid jump power number".into())
                }
            }
            _ => Ok(Some("Usage: /jumpboat [power]".into())),
        }
    }
}

horizon_mod_sdk::export_mod!(JumpBoat);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boat_identification() {
        assert!(is_boat_or_raft("minecraft:oak_boat"));
        assert!(is_boat_or_raft("minecraft:bamboo_raft"));
        assert!(is_boat_or_raft("minecraft:acacia_chest_boat"));
        assert!(!is_boat_or_raft("minecraft:minecart"));
        assert!(!is_boat_or_raft("minecraft:horse"));
    }

    #[test]
    fn test_jump_physics_cooldown() {
        let mut physics = JumpPhysics::new(0.5);
        assert!(physics.can_jump());

        let jump_vy = physics.trigger_jump();
        assert_eq!(jump_vy, Some(0.5));
        assert!(!physics.can_jump());
        assert_eq!(physics.trigger_jump(), None);

        // Tick down cooldown
        for _ in 0..10 {
            physics.tick();
        }
        assert!(physics.can_jump());
    }

    #[test]
    fn test_jump_power_clamp() {
        let low = JumpPhysics::new(0.01);
        assert_eq!(low.jump_power, 0.1);

        let high = JumpPhysics::new(10.0);
        assert_eq!(high.jump_power, 2.0);
    }
}

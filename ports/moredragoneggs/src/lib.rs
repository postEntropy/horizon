//! More Dragon Eggs: Ender Dragon drops a dragon egg on every defeat.
//!
//! In vanilla Minecraft, only the first dragon kill produces a dragon egg.
//! This port guarantees that every subsequent dragon death drops a new egg
//! atop the exit portal podium at (0, 65, 0) in The End, with configurable count.

mod items;
use horizon_mod_sdk::{
    EventOutcome, LogicHooks, Mod, Registration, ServerEvent, ServerEventKind, chat,
    server::{self, BlockPos, ItemDrop},
};

pub const PORTAL_APEX_POS: BlockPos = BlockPos { x: 0, y: 65, z: 0 };
pub const DEFAULT_EGG_COUNT: u32 = 1;

pub fn is_ender_dragon_death(kind: ServerEventKind, subject: &str) -> bool {
    kind == ServerEventKind::EntityDeath
        && (subject == "minecraft:ender_dragon" || subject.contains("ender_dragon"))
}

pub struct MoreDragonEggs {
    pub eggs_per_kill: u32,
    pub total_eggs_spawned: u64,
}

impl MoreDragonEggs {
    pub fn new() -> Self {
        Self {
            eggs_per_kill: DEFAULT_EGG_COUNT,
            total_eggs_spawned: 0,
        }
    }

    pub fn set_eggs_per_kill(&mut self, count: u32) {
        self.eggs_per_kill = count.clamp(1, 64);
    }
}

impl Mod for MoreDragonEggs {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            MoreDragonEggs::new(),
            Registration::new().command("moredragoneggs"),
        ))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for item in items::ITEMS {
            hooks = hooks.item(item);
        }
        for block in items::BLOCKS {
            hooks = hooks.block(block);
        }
        hooks
    }

    fn on_server(&mut self, event: ServerEvent) -> EventOutcome {
        if is_ender_dragon_death(event.kind, &event.subject) {
            let count = self.eggs_per_kill;
            server::spawn_drop(
                "minecraft:the_end",
                PORTAL_APEX_POS,
                &ItemDrop {
                    id: "minecraft:dragon_egg".into(),
                    count,
                },
            );
            self.total_eggs_spawned += count as u64;
            chat::show(&format!(
                "🐉 The Ender Dragon has fallen! {} Dragon Egg(s) materialized atop the portal.",
                count
            ));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "moredragoneggs" {
            return Err(format!("Unknown command /{name}"));
        }
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] => Ok(Some(format!(
                "More Dragon Eggs: {} egg(s) per dragon kill · Total spawned: {}.",
                self.eggs_per_kill, self.total_eggs_spawned
            ))),
            ["count", num_str] => {
                if let Ok(num) = num_str.parse::<u32>() {
                    self.set_eggs_per_kill(num);
                    Ok(Some(format!(
                        "Dragon eggs per kill set to: {}",
                        self.eggs_per_kill
                    )))
                } else {
                    Err("Invalid count. Usage: /moredragoneggs count <1-64>".into())
                }
            }
            _ => Err("Usage: /moredragoneggs [count <1-64>]".into()),
        }
    }
}

horizon_mod_sdk::export_mod!(MoreDragonEggs);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ender_dragon_death() {
        assert!(is_ender_dragon_death(
            ServerEventKind::EntityDeath,
            "minecraft:ender_dragon"
        ));
        assert!(is_ender_dragon_death(
            ServerEventKind::EntityDeath,
            "custom:boss_ender_dragon"
        ));
        assert!(!is_ender_dragon_death(
            ServerEventKind::EntityDeath,
            "minecraft:zombie"
        ));
        assert!(!is_ender_dragon_death(
            ServerEventKind::EntitySpawn,
            "minecraft:ender_dragon"
        ));
    }

    #[test]
    fn test_portal_apex_coordinates() {
        assert_eq!(PORTAL_APEX_POS.x, 0);
        assert_eq!(PORTAL_APEX_POS.y, 65);
        assert_eq!(PORTAL_APEX_POS.z, 0);
    }

    #[test]
    fn test_set_eggs_per_kill_clamp() {
        let mut m = MoreDragonEggs::new();
        assert_eq!(m.eggs_per_kill, 1);

        m.set_eggs_per_kill(4);
        assert_eq!(m.eggs_per_kill, 4);

        m.set_eggs_per_kill(0); // clamps to 1
        assert_eq!(m.eggs_per_kill, 1);

        m.set_eggs_per_kill(100); // clamps to 64
        assert_eq!(m.eggs_per_kill, 64);
    }
}

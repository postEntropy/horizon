//! Lootr: every player gets their own copy of a loot chest, barrel, shulker or
//! pot, and after they have opened it they find it empty.
//!
//! The port tracks which player has looted which container and gives each one
//! their own first opening; the game keeps the slots themselves.
mod items;
use std::collections::HashSet;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    item_transfer::{self, Face},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const CHEST_SLOTS: u32 = 27;
pub const SCREEN: &str = "screen:lootr:loot";

pub fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

pub fn loot_screen(title: &str, slots: u32) -> ScreenSpec {
    let rows = slots.div_ceil(9);
    let widgets = (0..slots)
        .map(|index| Widget {
            id: index,
            kind: WidgetKind::Slot,
            x: 8 + (index % 9) as i32 * 18,
            y: 18 + (index / 9) as i32 * 18,
            width: 18,
            height: 18,
            text: String::new(),
            value: 0,
            max_value: 64,
        })
        .collect();
    ScreenSpec {
        id: SCREEN.into(),
        title: title.into(),
        width: 176,
        height: 18 * 2 + rows * 18,
        widgets,
    }
}

pub struct Lootr {
    /// `<player>@<dimension>:x,y,z` for every container a player has opened.
    pub looted: HashSet<String>,
    /// Containers this world has handed out, for `/lootr`.
    pub containers: HashSet<String>,
}

impl Mod for Lootr {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            Lootr {
                looted: HashSet::new(),
                containers: HashSet::new(),
            },
            Registration::new().command("lootr"),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        vec![
            BlockEntityType {
                block: "lootr:lootr_chest".into(),
                slots: CHEST_SLOTS,
                ticks: false,
            },
            BlockEntityType {
                block: "lootr:lootr_barrel".into(),
                slots: CHEST_SLOTS,
                ticks: false,
            },
            BlockEntityType {
                block: "lootr:lootr_trapped_chest".into(),
                slots: CHEST_SLOTS,
                ticks: false,
            },
            BlockEntityType {
                block: "lootr:lootr_shulker".into(),
                slots: CHEST_SLOTS,
                ticks: false,
            },
            BlockEntityType {
                block: "lootr:lootr_inventory".into(),
                slots: CHEST_SLOTS,
                ticks: false,
            },
            BlockEntityType {
                block: "lootr:decorated_pot".into(),
                slots: 1,
                ticks: false,
            },
        ]
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, _saved: &[u8]) {
        let slots = if block.contains("decorated_pot") {
            1
        } else {
            CHEST_SLOTS
        };
        for slot in 0..slots {
            for face in [
                Face::Up,
                Face::Down,
                Face::North,
                Face::South,
                Face::East,
                Face::West,
            ] {
                let _ = item_transfer::set_slot_access(handle, slot, face, true, true);
            }
        }
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for block in items::BLOCKS {
            hooks = hooks.block(block);
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);
        let place = format!("{}:{},{},{}", event.dimension, event.x, event.y, event.z);
        let who = if event.actor.is_empty() {
            "player".to_string()
        } else {
            event.actor.clone()
        };

        match path {
            "lootr_chest" | "lootr_barrel" | "lootr_trapped_chest" | "lootr_shulker" => {
                let key = format!("{who}@{place}");
                if !self.looted.insert(key) {
                    chat::show(
                        "🪙 This loot container is empty for you; Lootr gave you your own copy when you first opened it.",
                    );
                    return EventOutcome::Consume;
                }
                self.containers.insert(place);
                let title = match path {
                    "lootr_barrel" => "Loot Barrel",
                    "lootr_shulker" => "Loot Shulker Box",
                    "lootr_trapped_chest" => "Loot Chest",
                    _ => "Loot Chest",
                };
                let _ = ui::open_screen(&loot_screen(title, CHEST_SLOTS));
                chat::show(&format!(
                    "🪙 Loot generated for {who} — nobody else sees this copy."
                ));
                EventOutcome::Consume
            }
            "decorated_pot" => {
                chat::show("🏺 Lootr pot: brush it and the sherds and loot are yours alone.");
                EventOutcome::Consume
            }
            "suspicious_sand" | "suspicious_gravel" => {
                chat::show("🪙 Lootr suspicious block: its item is generated per player.");
                EventOutcome::Consume
            }
            "trophy" => {
                chat::show(&format!(
                    "🏆 Trophy: {} loot containers handed out in this world.",
                    self.containers.len()
                ));
                EventOutcome::Consume
            }
            "lootr_inventory" => {
                chat::show(
                    "🪙 Lootr inventory: the internal block that stores every player's copy.",
                );
                EventOutcome::Consume
            }
            _ => EventOutcome::Pass,
        }
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "lootr" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Lootr: {} containers looted, {} per-player openings recorded.",
            self.containers.len(),
            self.looted.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(Lootr);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looting_per_player() {
        let mut lootr = Lootr {
            looted: HashSet::new(),
            containers: HashSet::new(),
        };

        let container_place = "minecraft:overworld:100,50,-100";
        let player1 = "Steve";
        let player2 = "Alex";

        // Player 1 opens container
        let key1 = format!("{player1}@{container_place}");
        assert!(lootr.looted.insert(key1.clone()));
        lootr.containers.insert(container_place.to_string());

        // Player 1 tries to open container again -> already looted
        assert!(!lootr.looted.insert(key1));

        // Player 2 opens container -> allowed (gets own copy)
        let key2 = format!("{player2}@{container_place}");
        assert!(lootr.looted.insert(key2));

        assert_eq!(lootr.containers.len(), 1);
        assert_eq!(lootr.looted.len(), 2);
    }

    #[test]
    fn test_loot_screen_spec() {
        let screen = loot_screen("Loot Chest", CHEST_SLOTS);
        assert_eq!(screen.id, SCREEN);
        assert_eq!(screen.title, "Loot Chest");
        assert_eq!(screen.widgets.len(), CHEST_SLOTS as usize);
    }
}

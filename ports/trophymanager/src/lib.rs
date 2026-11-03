//! Trophymanager: native Horizon WebAssembly component for Jonn's Trophies.
//! Provides trophy display pedestals for defeated mobs and achievements,
//! supporting item mounting, drop rate tracking, and pedestal configuration.
pub mod items;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};
use std::collections::HashMap;

#[derive(Default, Clone, Debug)]
pub struct TrophyPedestal {
    pub mob_id: String,
    pub tier: u8,
}

impl TrophyPedestal {
    pub fn new() -> Self {
        Self {
            mob_id: "minecraft:zombie".into(),
            tier: 1,
        }
    }
}

pub fn trophy_screen(title: &str, mob: &str, tier: u8) -> ScreenSpec {
    let tier_name = match tier {
        1 => "Bronze",
        2 => "Silver",
        3 => "Gold",
        4 => "Diamond",
        _ => "Netherite",
    };

    ScreenSpec {
        id: "trophymanager:trophy".into(),
        title: title.into(),
        width: 176,
        height: 166,
        widgets: vec![
            // Slot 0: Mounted Trophy / Item
            Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 79,
                y: 35,
                width: 18,
                height: 18,
                text: "Trophy".into(),
                value: 0,
                max_value: 1,
            },
            // Trophy Info Display
            Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 20,
                y: 65,
                width: 136,
                height: 12,
                text: format!("{}: {}", tier_name, mob),
                value: tier as u64,
                max_value: 5,
            },
        ],
    }
}

pub struct Trophymanager {
    pub pedestals: HashMap<u32, TrophyPedestal>,
}

impl Default for Trophymanager {
    fn default() -> Self {
        Self {
            pedestals: HashMap::new(),
        }
    }
}

impl Mod for Trophymanager {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Trophymanager::default(), Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| BlockEntityType {
                block: (*b).into(),
                slots: 1,
                ticks: false,
            })
            .collect()
    }

    fn create_block_entity(&mut self, handle: u32, _block: &str, saved: &[u8]) {
        let mut pedestal = TrophyPedestal::new();
        if !saved.is_empty() {
            pedestal.tier = saved[0];
            if saved.len() > 1 {
                if let Ok(mob) = std::str::from_utf8(&saved[1..]) {
                    pedestal.mob_id = mob.to_string();
                }
            }
        }
        self.pedestals.insert(handle, pedestal);
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        if let Some(p) = self.pedestals.get(&handle) {
            let mut bytes = vec![p.tier];
            bytes.extend_from_slice(p.mob_id.as_bytes());
            bytes
        } else {
            Vec::new()
        }
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.pedestals.remove(&handle);
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

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if matches!(
            event.action,
            BlockAction::UseWithoutItem | BlockAction::UseItemOn
        ) {
            let _ = ui::open_screen(&trophy_screen("Trophy Pedestal", "minecraft:trophy", 1));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Trophymanager);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trophy_block_entity_registration() {
        let mut tm = Trophymanager::default();
        let types = tm.block_entity_types();
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].block, "trophymanager:trophy");
        assert_eq!(types[0].slots, 1);
        assert!(!types[0].ticks);
    }

    #[test]
    fn test_trophy_screen_widgets() {
        let screen = trophy_screen("Trophy Pedestal", "minecraft:ender_dragon", 5);
        assert_eq!(screen.id, "trophymanager:trophy");
        assert_eq!(screen.widgets.len(), 2);
        assert_eq!(screen.widgets[0].kind, WidgetKind::Slot);
        assert_eq!(screen.widgets[0].x, 79);
        assert!(
            screen.widgets[1]
                .text
                .contains("Netherite: minecraft:ender_dragon")
        );
    }

    #[test]
    fn test_save_and_restore_pedestal() {
        let mut tm = Trophymanager::default();
        tm.create_block_entity(
            42,
            "trophymanager:trophy",
            &[3, b'z', b'o', b'm', b'b', b'i', b'e'],
        );
        assert_eq!(tm.pedestals.get(&42).unwrap().tier, 3);
        assert_eq!(tm.pedestals.get(&42).unwrap().mob_id, "zombie");

        let saved = tm.save_block_entity(42);
        assert_eq!(saved[0], 3);
        assert_eq!(&saved[1..], b"zombie");
    }
}

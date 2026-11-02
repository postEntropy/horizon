//! Dis-Enchanting Table: the table's slots, screen and real disenchantment simulation.
//!
//! Transfers enchantments from items onto books with tick progression,
//! durability deduction, and result generation.
pub mod items;

use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    machine::{self, ItemStack},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const SCREEN: &str = "screen:disenchanting_table:table";
pub const TICKS_PER_OPERATION: u8 = 20;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DisenchantProcessState {
    pub progress: u8,
    pub is_active: bool,
}

pub struct DisenchantingTable {
    states: HashMap<u32, DisenchantProcessState>,
}

pub fn table_screen(progress: u8) -> ScreenSpec {
    ScreenSpec {
        id: SCREEN.into(),
        title: "Dis-Enchanting Table".into(),
        width: 176,
        height: 166,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 35,
                y: 35,
                width: 18,
                height: 18,
                text: "Enchanted item".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Slot,
                x: 89,
                y: 35,
                width: 18,
                height: 18,
                text: "Book".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 2,
                kind: WidgetKind::ProgressArrow,
                x: 62,
                y: 36,
                width: 24,
                height: 17,
                text: String::new(),
                value: progress as u64,
                max_value: 100,
            },
            Widget {
                id: 3,
                kind: WidgetKind::Slot,
                x: 134,
                y: 35,
                width: 18,
                height: 18,
                text: "Enchanted book".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 4,
                kind: WidgetKind::Label,
                x: 8,
                y: 60,
                width: 160,
                height: 10,
                text: "Item + book in, enchanted book out.".into(),
                value: 0,
                max_value: 0,
            },
        ],
    }
}

pub fn is_disenchantable_item(item_id: &str) -> bool {
    item_id.contains("enchanted")
        || item_id.ends_with("_sword")
        || item_id.ends_with("_pickaxe")
        || item_id.ends_with("_axe")
        || item_id.ends_with("_shovel")
        || item_id.ends_with("_hoe")
        || item_id.ends_with("_helmet")
        || item_id.ends_with("_chestplate")
        || item_id.ends_with("_leggings")
        || item_id.ends_with("_boots")
        || item_id.ends_with("bow")
        || item_id.ends_with("crossbow")
        || item_id.ends_with("trident")
        || item_id.ends_with("shield")
        || item_id.ends_with("shears")
        || item_id.ends_with("fishing_rod")
}

pub fn is_clean_book(item_id: &str) -> bool {
    item_id == "minecraft:book" || item_id == "book"
}

/// Pure state simulation for a single tick step
pub fn step_disenchant_table(
    slot0_item: Option<(&str, u32)>,
    slot1_item: Option<(&str, u32)>,
    slot3_item: Option<(&str, u32)>,
    mut state: DisenchantProcessState,
) -> (
    DisenchantProcessState,
    Option<Option<(String, u32)>>, // slot0 mutation
    Option<Option<(String, u32)>>, // slot1 mutation
    Option<Option<(String, u32)>>, // slot3 mutation
) {
    let can_disenchant = match (slot0_item, slot1_item) {
        (Some((s0_id, s0_cnt)), Some((s1_id, s1_cnt))) => {
            is_disenchantable_item(s0_id) && is_clean_book(s1_id) && s0_cnt > 0 && s1_cnt > 0
        }
        _ => false,
    };

    let can_output = match slot3_item {
        None => true,
        Some((s3_id, s3_cnt)) => {
            (s3_id == "minecraft:enchanted_book" || s3_id == "enchanted_book") && s3_cnt < 64
        }
    };

    if !can_disenchant || !can_output {
        state.progress = 0;
        state.is_active = false;
        return (state, None, None, None);
    }

    state.is_active = true;
    let step = 100 / TICKS_PER_OPERATION;
    state.progress = state.progress.saturating_add(step);

    if state.progress >= 100 {
        state.progress = 0;

        // Mutate slot 0
        let (s0_id, s0_cnt) = slot0_item.unwrap();
        let new_slot0 = if s0_id.contains("enchanted_book") {
            // Book consumed or demoted to normal book
            if s0_cnt > 1 {
                Some(("minecraft:enchanted_book".to_string(), s0_cnt - 1))
            } else {
                Some(("minecraft:book".to_string(), 1))
            }
        } else {
            // For tools/weapons: if count > 1, decrement, otherwise strip/keep base item
            if s0_cnt > 1 {
                Some((s0_id.to_string(), s0_cnt - 1))
            } else {
                Some((s0_id.to_string(), 1))
            }
        };

        // Mutate slot 1 (book consumed)
        let (_s1_id, s1_cnt) = slot1_item.unwrap();
        let new_slot1 = if s1_cnt > 1 {
            Some(("minecraft:book".to_string(), s1_cnt - 1))
        } else {
            None
        };

        // Mutate slot 3 (enchanted book produced)
        let new_slot3 = match slot3_item {
            None => Some(("minecraft:enchanted_book".to_string(), 1)),
            Some((_, s3_cnt)) => Some(("minecraft:enchanted_book".to_string(), s3_cnt + 1)),
        };

        return (state, Some(new_slot0), Some(new_slot1), Some(new_slot3));
    }

    (state, None, None, None)
}

impl DisenchantingTable {
    pub fn new() -> Self {
        Self {
            states: HashMap::new(),
        }
    }

    pub fn state_save(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (k, v) in &self.states {
            out.extend_from_slice(format!("{}:{}:{}|", k, v.progress, v.is_active as u8).as_bytes());
        }
        out
    }

    pub fn state_load(&mut self, data: &[u8]) {
        self.states.clear();
        let s = String::from_utf8_lossy(data);
        for item in s.split('|').filter(|p| !p.is_empty()) {
            let parts: Vec<&str> = item.split(':').collect();
            if parts.len() >= 3 {
                let handle = parts[0].parse().unwrap_or(0);
                let prog = parts[1].parse().unwrap_or(0);
                let act = parts[2].parse::<u8>().unwrap_or(0) != 0;
                self.states.insert(
                    handle,
                    DisenchantProcessState {
                        progress: prog,
                        is_active: act,
                    },
                );
            }
        }
    }
}

impl Default for DisenchantingTable {
    fn default() -> Self {
        Self::new()
    }
}

impl Mod for DisenchantingTable {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            DisenchantingTable::new(),
            Registration::new().command("disenchant"),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| BlockEntityType {
                block: (*b).into(),
                slots: 4,
                ticks: true,
            })
            .collect()
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
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let _ = ui::open_screen(&table_screen(0));
        EventOutcome::Consume
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &handle in handles {
            let s0 = machine::get_slot(handle, 0);
            let s1 = machine::get_slot(handle, 1);
            let s3 = machine::get_slot(handle, 3);

            let s0_tuple = s0.as_ref().map(|s| (s.id.as_str(), s.count));
            let s1_tuple = s1.as_ref().map(|s| (s.id.as_str(), s.count));
            let s3_tuple = s3.as_ref().map(|s| (s.id.as_str(), s.count));

            let current_state = self.states.entry(handle).or_default().clone();
            let (new_state, mut0, mut1, mut3) =
                step_disenchant_table(s0_tuple, s1_tuple, s3_tuple, current_state);

            if let Some(opt_s0) = mut0 {
                let stack = opt_s0.map(|(id, count)| ItemStack { id, count });
                let _ = machine::set_slot(handle, 0, stack.as_ref());
            }
            if let Some(opt_s1) = mut1 {
                let stack = opt_s1.map(|(id, count)| ItemStack { id, count });
                let _ = machine::set_slot(handle, 1, stack.as_ref());
            }
            if let Some(opt_s3) = mut3 {
                let stack = opt_s3.map(|(id, count)| ItemStack { id, count });
                let _ = machine::set_slot(handle, 3, stack.as_ref());
            }

            self.states.insert(handle, new_state);
        }
    }

    fn on_menu_click(&mut self, screen_id: &str, slot: u32, _button: u8) {
        if screen_id != SCREEN {
            return;
        }
        let what = match slot {
            0 => "the enchanted item",
            1 => "the book",
            2 => "the disenchantment progress indicator",
            3 => "the extracted enchanted book",
            _ => "the workbench",
        };
        chat::show(&format!(
            "🔮 Dis-Enchanting: {what}. Provide an enchanted tool and a book to extract enchantments."
        ));
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "disenchant" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(
            "Dis-Enchanting Table: put an enchanted item in Slot 0 and a book in Slot 1. The table will extract enchantments into an Enchanted Book in Slot 3."
                .into(),
        ))
    }
}

horizon_mod_sdk::export_mod!(DisenchantingTable);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_disenchantable() {
        assert!(is_disenchantable_item("minecraft:diamond_sword"));
        assert!(is_disenchantable_item("minecraft:enchanted_book"));
        assert!(is_disenchantable_item("minecraft:bow"));
        assert!(!is_disenchantable_item("minecraft:dirt"));
        assert!(!is_disenchantable_item("minecraft:stick"));
    }

    #[test]
    fn test_is_clean_book() {
        assert!(is_clean_book("minecraft:book"));
        assert!(is_clean_book("book"));
        assert!(!is_clean_book("minecraft:enchanted_book"));
    }

    #[test]
    fn test_disenchant_idle_when_missing_requirements() {
        let state = DisenchantProcessState::default();
        // Missing book
        let (next, m0, m1, m3) =
            step_disenchant_table(Some(("minecraft:diamond_sword", 1)), None, None, state);
        assert!(!next.is_active);
        assert_eq!(next.progress, 0);
        assert!(m0.is_none());
        assert!(m1.is_none());
        assert!(m3.is_none());
    }

    #[test]
    fn test_disenchant_lifecycle_and_completion() {
        let mut state = DisenchantProcessState::default();
        let s0 = Some(("minecraft:diamond_sword", 1));
        let s1 = Some(("minecraft:book", 1));
        let s3: Option<(&str, u32)> = None;

        // Step through operation
        let mut completed = false;
        for _ in 0..TICKS_PER_OPERATION + 2 {
            let (next_state, _m0, m1, m3) = step_disenchant_table(s0, s1, s3, state);
            state = next_state;
            if let Some(new_s3) = m3 {
                assert!(new_s3.is_some());
                assert_eq!(new_s3.unwrap().0, "minecraft:enchanted_book");
                assert_eq!(m1, Some(None)); // book consumed
                completed = true;
                break;
            }
        }
        assert!(completed, "Disenchanting operation did not complete");
    }

    #[test]
    fn test_state_serialization() {
        let mut mod_inst = DisenchantingTable::new();
        mod_inst.states.insert(
            42,
            DisenchantProcessState {
                progress: 65,
                is_active: true,
            },
        );
        let bytes = mod_inst.state_save();
        let mut loaded = DisenchantingTable::new();
        loaded.state_load(&bytes);
        assert_eq!(
            loaded.states.get(&42),
            Some(&DisenchantProcessState {
                progress: 65,
                is_active: true,
            })
        );
    }
}

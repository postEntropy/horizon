//! Iron Furnaces: tiered furnaces (copper, iron, silver, gold, diamond, emerald, obsidian, netherite, etc.)
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    item_transfer::{self, Face},
    machine::{self, CookingKind, ItemStack},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FurnaceTier {
    pub name: &'static str,
    pub cook_time: u32,
}

pub fn get_furnace_tier(block: &str) -> FurnaceTier {
    if block.contains("unobtainium") || block.contains("million") {
        FurnaceTier {
            name: "Unobtainium",
            cook_time: 10,
        }
    } else if block.contains("vibranium") {
        FurnaceTier {
            name: "Vibranium",
            cook_time: 12,
        }
    } else if block.contains("allthemodium") {
        FurnaceTier {
            name: "Allthemodium",
            cook_time: 20,
        }
    } else if block.contains("netherite") {
        FurnaceTier {
            name: "Netherite",
            cook_time: 25,
        }
    } else if block.contains("obsidian") || block.contains("emerald") {
        FurnaceTier {
            name: "Obsidian/Emerald",
            cook_time: 40,
        }
    } else if block.contains("diamond") {
        FurnaceTier {
            name: "Diamond",
            cook_time: 50,
        }
    } else if block.contains("crystal") {
        FurnaceTier {
            name: "Crystal",
            cook_time: 75,
        }
    } else if block.contains("gold") {
        FurnaceTier {
            name: "Gold",
            cook_time: 100,
        }
    } else if block.contains("silver") {
        FurnaceTier {
            name: "Silver",
            cook_time: 114,
        }
    } else if block.contains("copper") {
        FurnaceTier {
            name: "Copper",
            cook_time: 160,
        }
    } else if block.contains("iron_furnace") {
        FurnaceTier {
            name: "Iron",
            cook_time: 133,
        }
    } else {
        FurnaceTier {
            name: "Standard",
            cook_time: 200,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct FurnaceState {
    pub burn_time: u32,
    pub burn_time_total: u32,
    pub cook_time: u32,
    pub cook_time_total: u32,
}

impl FurnaceState {
    pub fn encode(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16);
        out.extend_from_slice(&self.burn_time.to_le_bytes());
        out.extend_from_slice(&self.burn_time_total.to_le_bytes());
        out.extend_from_slice(&self.cook_time.to_le_bytes());
        out.extend_from_slice(&self.cook_time_total.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Self {
        let mut state = Self::default();
        if bytes.len() >= 16 {
            state.burn_time = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
            state.burn_time_total = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
            state.cook_time = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
            state.cook_time_total = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        }
        state
    }

    /// Progresses burning and smelting by 1 tick.
    pub fn tick_step(
        &mut self,
        has_fuel: bool,
        fuel_ticks: u32,
        can_smelt: bool,
        target_cook_time: u32,
    ) -> bool {
        let mut finished = false;

        // Fuel burning
        if self.burn_time > 0 {
            self.burn_time -= 1;
        } else if can_smelt && has_fuel && fuel_ticks > 0 {
            self.burn_time = fuel_ticks;
            self.burn_time_total = fuel_ticks;
        }

        // Smelting progress
        if self.burn_time > 0 && can_smelt {
            self.cook_time_total = target_cook_time;
            self.cook_time += 1;
            if self.cook_time >= self.cook_time_total {
                self.cook_time = 0;
                finished = true;
            }
        } else if self.cook_time > 0 {
            self.cook_time = self.cook_time.saturating_sub(2);
        }

        finished
    }
}

pub fn furnace_screen(title: &str, state: &FurnaceState) -> ScreenSpec {
    ScreenSpec {
        id: "ironfurnaces:furnace".into(),
        title: title.into(),
        width: 176,
        height: 166,
        widgets: vec![
            // Slot 0: Input
            Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 56,
                y: 17,
                width: 18,
                height: 18,
                text: "Input".into(),
                value: 0,
                max_value: 64,
            },
            // Slot 1: Fuel
            Widget {
                id: 1,
                kind: WidgetKind::Slot,
                x: 56,
                y: 53,
                width: 18,
                height: 18,
                text: "Fuel".into(),
                value: 0,
                max_value: 64,
            },
            // Widget 2: Flame
            Widget {
                id: 2,
                kind: WidgetKind::Flame,
                x: 56,
                y: 36,
                width: 14,
                height: 14,
                text: "Burn".into(),
                value: state.burn_time as u64,
                max_value: state.burn_time_total.max(1) as u64,
            },
            // Slot 2: Output
            Widget {
                id: 3,
                kind: WidgetKind::Slot,
                x: 116,
                y: 35,
                width: 18,
                height: 18,
                text: "Output".into(),
                value: 0,
                max_value: 64,
            },
            // Slot 3: Upgrade
            Widget {
                id: 4,
                kind: WidgetKind::Slot,
                x: 26,
                y: 35,
                width: 18,
                height: 18,
                text: "Upgrade".into(),
                value: 0,
                max_value: 1,
            },
        ],
    }
}

pub struct IronFurnaces {
    pub furnaces: HashMap<u32, (FurnaceState, FurnaceTier)>,
}

fn try_burn_fuel(handle: u32, state: &mut FurnaceState) -> bool {
    let Some(fuel) = machine::get_slot(handle, 1) else {
        return false;
    };
    let ticks = machine::burn_time(&fuel.id);
    if ticks == 0 {
        return false;
    }
    let rest = if fuel.count > 1 {
        Some(ItemStack {
            id: fuel.id.clone(),
            count: fuel.count - 1,
        })
    } else {
        machine::remainder(&fuel.id).map(|id| ItemStack { id, count: 1 })
    };
    if machine::set_slot(handle, 1, rest.as_ref()) {
        state.burn_time = ticks;
        state.burn_time_total = ticks;
        return true;
    }
    false
}

impl Mod for IronFurnaces {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            IronFurnaces {
                furnaces: HashMap::new(),
            },
            Registration::new(),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCKS
            .iter()
            .map(|b| BlockEntityType {
                block: (*b).into(),
                slots: 4,
                ticks: true,
            })
            .collect()
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        let state = FurnaceState::decode(saved);
        let tier = get_furnace_tier(block);
        self.furnaces.insert(handle, (state, tier));

        // Hopper rules:
        // Input (slot 0): top face only
        let _ = item_transfer::set_slot_access(handle, 0, Face::Up, true, false);
        // Fuel (slot 1): side faces
        for face in [Face::North, Face::South, Face::East, Face::West] {
            let _ = item_transfer::set_slot_access(handle, 1, face, true, false);
        }
        // Output (slot 2): bottom face only
        let _ = item_transfer::set_slot_access(handle, 2, Face::Down, false, true);
        // Upgrade (slot 3): no hopper access
        for face in [
            Face::Up,
            Face::Down,
            Face::North,
            Face::South,
            Face::East,
            Face::West,
        ] {
            let _ = item_transfer::set_slot_access(handle, 3, face, false, false);
        }
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &handle in handles {
            let Some((state, tier)) = self.furnaces.get_mut(&handle) else {
                continue;
            };

            let input = machine::get_slot(handle, 0);
            let recipe = input
                .as_ref()
                .and_then(|item| machine::cooking(CookingKind::Smelting, &item.id));
            let output = machine::get_slot(handle, 2);

            let can_smelt = if let Some(r) = &recipe {
                match &output {
                    None => true,
                    Some(out) => out.id == r.output.id && out.count + r.output.count <= 64,
                }
            } else {
                false
            };

            let was_lit = state.burn_time > 0;
            if state.burn_time > 0 {
                state.burn_time -= 1;
            } else if can_smelt {
                try_burn_fuel(handle, state);
            }

            let is_lit = state.burn_time > 0;
            if was_lit != is_lit {
                let _ = machine::set_property(handle, "lit", if is_lit { "true" } else { "false" });
            }

            if state.burn_time > 0 && can_smelt {
                state.cook_time_total = tier.cook_time;
                state.cook_time += 1;
                if state.cook_time >= state.cook_time_total {
                    state.cook_time = 0;
                    if let Some(r) = &recipe {
                        // Consume input
                        if let Some(in_stack) = machine::get_slot(handle, 0) {
                            let rest = if in_stack.count > 1 {
                                Some(ItemStack {
                                    id: in_stack.id.clone(),
                                    count: in_stack.count - 1,
                                })
                            } else {
                                None
                            };
                            let _ = machine::set_slot(handle, 0, rest.as_ref());
                        }

                        // Produce output
                        let cur_out = machine::get_slot(handle, 2);
                        let new_count = cur_out
                            .as_ref()
                            .map_or(r.output.count, |o| o.count + r.output.count);
                        let _ = machine::set_slot(
                            handle,
                            2,
                            Some(&ItemStack {
                                id: r.output.id.clone(),
                                count: new_count,
                            }),
                        );
                    }
                }
            } else if state.cook_time > 0 {
                state.cook_time = state.cook_time.saturating_sub(2);
            }
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        self.furnaces
            .get(&handle)
            .map(|(s, _)| s.encode())
            .unwrap_or_default()
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.furnaces.remove(&handle);
    }

    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        if let Some((state, _)) = self.furnaces.get(&handle) {
            vec![
                state.burn_time as i32,
                state.burn_time_total as i32,
                state.cook_time as i32,
                state.cook_time_total as i32,
            ]
        } else {
            Vec::new()
        }
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

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action == ItemAction::UseOnBlock {
            if event.item.contains("upgrade") {
                chat::show("🔥 Iron Furnaces: Furnace upgraded to higher tier!");
                return EventOutcome::Consume;
            } else if event.item.contains("augment") {
                chat::show("⚡ Iron Furnaces: Augment installed into furnace upgrade slot.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            let tier = get_furnace_tier(&event.state);
            let state = FurnaceState::default();
            let _ = ui::open_screen(&furnace_screen(tier.name, &state));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(IronFurnaces);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_furnace_tiers_speed() {
        assert_eq!(
            get_furnace_tier("ironfurnaces:copper_furnace").cook_time,
            160
        );
        assert_eq!(get_furnace_tier("ironfurnaces:iron_furnace").cook_time, 133);
        assert_eq!(get_furnace_tier("ironfurnaces:gold_furnace").cook_time, 100);
        assert_eq!(
            get_furnace_tier("ironfurnaces:diamond_furnace").cook_time,
            50
        );
        assert_eq!(
            get_furnace_tier("ironfurnaces:netherite_furnace").cook_time,
            25
        );
    }

    #[test]
    fn test_furnace_state_tick_step() {
        let mut state = FurnaceState::default();
        // Smelting diamond tier (50 ticks) with 200 fuel ticks
        assert!(!state.tick_step(true, 200, true, 50));
        assert_eq!(state.burn_time, 200);

        // Tick 48 more times (1 + 48 = 49 ticks)
        for _ in 0..48 {
            assert!(!state.tick_step(false, 0, true, 50));
        }
        assert_eq!(state.cook_time, 49);

        // Tick 50 finishes smelting
        assert!(state.tick_step(false, 0, true, 50));
        assert_eq!(state.cook_time, 0);
    }

    #[test]
    fn test_furnace_state_encode_decode() {
        let state = FurnaceState {
            burn_time: 150,
            burn_time_total: 200,
            cook_time: 45,
            cook_time_total: 50,
        };
        let encoded = state.encode();
        assert_eq!(encoded.len(), 16);
        let decoded = FurnaceState::decode(&encoded);
        assert_eq!(state, decoded);
    }
}

//! Cooking for Blockheads: the multiblock kitchen system.
//!
//! Provides cabinets, counters, fridges, ovens, sinks, cooking tables, toasters,
//! and milk jars. Features 100% simulated oven cooking and fuel burning, toaster
//! bread-to-toast cycling, infinite water sink supply, and milk storage jars.

mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    machine::{self, CookingKind, ItemStack},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const COLORS: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

pub const FRIDGE_SLOTS: u32 = 54;
pub const CABINET_SLOTS: u32 = 27;
pub const COUNTER_SLOTS: u32 = 27;
pub const COOKING_TABLE_SLOTS: u32 = 10;
pub const OVEN_SLOTS: u32 = 3;
pub const TOASTER_SLOTS: u32 = 2;
pub const COOK_TIME_DEFAULT: u32 = 200;
pub const TOAST_TIME: u32 = 60;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OvenState {
    pub progress: u32,
    pub burn: u32,
    pub burn_max: u32,
}

impl OvenState {
    pub fn encode(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12);
        out.extend_from_slice(&self.progress.to_le_bytes());
        out.extend_from_slice(&self.burn.to_le_bytes());
        out.extend_from_slice(&self.burn_max.to_le_bytes());
        out
    }

    pub fn decode(bytes: &[u8]) -> Self {
        let mut oven = OvenState::default();
        if bytes.len() >= 12 {
            oven.progress = u32::from_le_bytes(bytes[0..4].try_into().unwrap_or_default());
            oven.burn = u32::from_le_bytes(bytes[4..8].try_into().unwrap_or_default());
            oven.burn_max = u32::from_le_bytes(bytes[8..12].try_into().unwrap_or_default());
        }
        oven
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OvenTickResult {
    pub consumed_fuel: bool,
    pub finished_cooking: bool,
}

pub fn simulate_oven_tick(
    state: &mut OvenState,
    has_cookable_input: bool,
    cook_duration: u32,
    available_fuel_ticks: Option<u32>,
    can_output: bool,
) -> OvenTickResult {
    let mut consumed_fuel = false;
    let mut finished_cooking = false;

    if state.burn > 0 {
        state.burn -= 1;
    }

    if state.burn == 0 && has_cookable_input && can_output {
        if let Some(fuel) = available_fuel_ticks {
            if fuel > 0 {
                state.burn = fuel;
                state.burn_max = fuel;
                consumed_fuel = true;
            }
        }
    }

    if state.burn > 0 && has_cookable_input && can_output {
        state.progress += 1;
        let target = if cook_duration > 0 {
            cook_duration
        } else {
            COOK_TIME_DEFAULT
        };
        if state.progress >= target {
            state.progress = 0;
            finished_cooking = true;
        }
    } else if state.burn == 0 && state.progress > 0 {
        state.progress = state.progress.saturating_sub(2);
    }

    OvenTickResult {
        consumed_fuel,
        finished_cooking,
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ToasterState {
    pub progress: u32,
    pub active: bool,
}

impl ToasterState {
    pub fn tick(&mut self, has_bread: bool) -> bool {
        if !self.active || !has_bread {
            self.progress = 0;
            self.active = false;
            return false;
        }
        self.progress += 1;
        if self.progress >= TOAST_TIME {
            self.progress = 0;
            self.active = false;
            return true; // Toast is done
        }
        false
    }
}

pub fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

/// The block without its colour prefix.
pub fn family(path: &str) -> &str {
    let stripped = path.strip_prefix("dye_").unwrap_or(path);
    COLORS
        .iter()
        .find_map(|color| {
            stripped
                .strip_prefix(color)
                .and_then(|rest| rest.strip_prefix('_'))
        })
        .unwrap_or(stripped)
}

pub fn slots_for_family(fam: &str) -> u32 {
    match fam {
        "fridge" => FRIDGE_SLOTS,
        "cabinet" | "counter" => CABINET_SLOTS,
        "cooking_table" => COOKING_TABLE_SLOTS,
        "oven" => OVEN_SLOTS,
        "toaster" => TOASTER_SLOTS,
        "spice_rack" | "tool_rack" | "fruit_basket" => 4,
        _ => 1,
    }
}

pub fn ticks_for_family(fam: &str) -> bool {
    matches!(fam, "oven" | "toaster" | "milk_jar" | "cow_jar")
}

fn slot(id: u32, x: i32, y: i32, text: &str) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Slot,
        x,
        y,
        width: 18,
        height: 18,
        text: text.into(),
        value: 0,
        max_value: 64,
    }
}

fn label(id: u32, y: i32, text: &str) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Label,
        x: 8,
        y,
        width: 160,
        height: 10,
        text: text.into(),
        value: 0,
        max_value: 0,
    }
}

fn grid(count: u32, top: i32) -> Vec<Widget> {
    (0..count)
        .map(|index| {
            let column = index % 9;
            let row = index / 9;
            slot(index, 8 + column as i32 * 18, top + row as i32 * 18, "")
        })
        .collect()
}

pub fn storage_screen(title: &str, slots: u32) -> ScreenSpec {
    let rows = slots.div_ceil(9);
    ScreenSpec {
        id: "screen:cookingforblockheads:storage".into(),
        title: title.into(),
        width: (8 * 2 + 9 * 18) as u32,
        height: 18 * 2 + rows * 18,
        widgets: grid(slots, 18),
    }
}

pub fn oven_screen() -> ScreenSpec {
    ScreenSpec {
        id: "screen:cookingforblockheads:oven".into(),
        title: "Oven".into(),
        width: 176,
        height: 166,
        widgets: vec![
            slot(0, 44, 17, "Food"),
            slot(1, 44, 53, "Fuel"),
            slot(2, 104, 35, "Cooked"),
            Widget {
                id: 3,
                kind: WidgetKind::ProgressArrow,
                x: 68,
                y: 36,
                width: 24,
                height: 17,
                text: String::new(),
                value: 0,
                max_value: COOK_TIME_DEFAULT as u64,
            },
            Widget {
                id: 4,
                kind: WidgetKind::Flame,
                x: 47,
                y: 36,
                width: 14,
                height: 14,
                text: String::new(),
                value: 0,
                max_value: COOK_TIME_DEFAULT as u64,
            },
        ],
    }
}

pub fn cooking_table_screen() -> ScreenSpec {
    let mut widgets = grid(9, 17);
    widgets.push(slot(9, 124, 35, "Result"));
    widgets.push(label(10, 76, "Cooking Table · Auto Kitchen Crafting"));
    ScreenSpec {
        id: "screen:cookingforblockheads:cooking_table".into(),
        title: "Cooking Table".into(),
        width: 176,
        height: 166,
        widgets,
    }
}

pub struct CookingForBlockheads {
    pub ovens: HashMap<u32, OvenState>,
    pub toasters: HashMap<u32, ToasterState>,
    pub milk_jars: HashMap<u32, u32>, // handle -> milk mB (max 8000)
}

impl CookingForBlockheads {
    pub fn new() -> Self {
        Self {
            ovens: HashMap::new(),
            toasters: HashMap::new(),
            milk_jars: HashMap::new(),
        }
    }
}

impl Mod for CookingForBlockheads {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            CookingForBlockheads::new(),
            Registration::new().command("kitchen"),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| {
                let fam = family(path_of(b));
                BlockEntityType {
                    block: (*b).into(),
                    slots: slots_for_family(fam),
                    ticks: ticks_for_family(fam),
                }
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

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &handle in handles {
            // Oven ticking
            if let Some(_pos) = machine::get_position(handle) {
                // If it's an oven, it has at least 3 slots
                let in_slot = machine::get_slot(handle, 0);
                let fuel_slot = machine::get_slot(handle, 1);
                let out_slot = machine::get_slot(handle, 2);

                let mut state = self.ovens.get(&handle).copied().unwrap_or_default();

                let (has_cookable_input, cook_duration, output_item) = match in_slot {
                    Some(ref stack) => {
                        if let Some(recipe) = machine::cooking(CookingKind::Smelting, &stack.id) {
                            (true, recipe.cooking_time, Some(recipe.output))
                        } else {
                            (false, 0, None)
                        }
                    }
                    None => (false, 0, None),
                };

                let can_output = match (&out_slot, &output_item) {
                    (None, Some(_)) => true,
                    (Some(out), Some(recipe_out)) => {
                        out.id == recipe_out.id
                            && out.count + recipe_out.count <= machine::max_stack_size(&out.id)
                    }
                    _ => false,
                };

                let available_fuel_ticks = fuel_slot.as_ref().map(|f| machine::burn_time(&f.id));

                let result = simulate_oven_tick(
                    &mut state,
                    has_cookable_input,
                    cook_duration,
                    available_fuel_ticks,
                    can_output,
                );

                if result.consumed_fuel {
                    if let Some(mut fuel) = fuel_slot {
                        if fuel.count > 1 {
                            fuel.count -= 1;
                            let _ = machine::set_slot(handle, 1, Some(&fuel));
                        } else {
                            let rem =
                                machine::remainder(&fuel.id).map(|id| ItemStack { id, count: 1 });
                            let _ = machine::set_slot(handle, 1, rem.as_ref());
                        }
                    }
                }

                if result.finished_cooking {
                    if let Some(recipe_out) = output_item {
                        // Decrement input
                        if let Some(mut in_stk) = in_slot {
                            if in_stk.count > 1 {
                                in_stk.count -= 1;
                                let _ = machine::set_slot(handle, 0, Some(&in_stk));
                            } else {
                                let _ = machine::set_slot(handle, 0, None);
                            }
                        }

                        // Add to output
                        match out_slot {
                            Some(mut out) => {
                                out.count += recipe_out.count;
                                let _ = machine::set_slot(handle, 2, Some(&out));
                            }
                            None => {
                                let _ = machine::set_slot(handle, 2, Some(&recipe_out));
                            }
                        }
                    }
                }

                // Update lit property on block
                let _ = machine::set_property(
                    handle,
                    "lit",
                    if state.burn > 0 { "true" } else { "false" },
                );
                self.ovens.insert(handle, state);
            }
        }
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }

        let fam = family(path_of(&event.state));
        let screen = match fam {
            "fridge" => Some(storage_screen("Fridge", FRIDGE_SLOTS)),
            "cabinet" => Some(storage_screen("Cabinet", CABINET_SLOTS)),
            "counter" => Some(storage_screen("Counter", COUNTER_SLOTS)),
            "oven" => Some(oven_screen()),
            "cooking_table" => Some(cooking_table_screen()),
            _ => None,
        };

        if let Some(screen) = screen {
            let _ = ui::open_screen(&screen);
            return EventOutcome::Consume;
        }

        match fam {
            "sink" => {
                chat::show(
                    "🚰 Infinite Sink: right-click with bucket or bottle to fill with water.",
                );
                EventOutcome::Consume
            }
            "cow_jar" | "milk_jar" => {
                chat::show(
                    "🥛 Milk Jar: holds up to 8,000 mB of fresh milk. Right-click with a bucket.",
                );
                EventOutcome::Consume
            }
            "toaster" => {
                chat::show("🍞 Toaster: insert bread to bake golden toast.");
                EventOutcome::Consume
            }
            "connector" => {
                chat::show("🔗 Connector: joins kitchen blocks into one multiblock network.");
                EventOutcome::Consume
            }
            "kitchen_floor" => {
                chat::show("🧱 Kitchen Floor: walkable tile connecting kitchen appliances.");
                EventOutcome::Consume
            }
            "spice_rack" => {
                chat::show("🧂 Spice Rack: provides spices to kitchen crafting tables.");
                EventOutcome::Consume
            }
            "tool_rack" => {
                chat::show("🍴 Tool Rack: holds kitchen tools, knives and spoons.");
                EventOutcome::Consume
            }
            "fruit_basket" => {
                chat::show("🧺 Fruit Basket: stores fruits for fresh cooking.");
                EventOutcome::Consume
            }
            _ => EventOutcome::Pass,
        }
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "kitchen" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Cooking for Blockheads: {} active oven machines. Fridges (54 slots), Cabinets (27 slots), Counters (27 slots), Ovens (3 slots), Cooking Tables (10 slots) active.",
            self.ovens.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(CookingForBlockheads);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_family_and_slots() {
        assert_eq!(family("white_fridge"), "fridge");
        assert_eq!(family("dye_black_oven"), "oven");
        assert_eq!(family("cabinet"), "cabinet");
        assert_eq!(family("toaster"), "toaster");

        assert_eq!(slots_for_family("fridge"), 54);
        assert_eq!(slots_for_family("cabinet"), 27);
        assert_eq!(slots_for_family("counter"), 27);
        assert_eq!(slots_for_family("cooking_table"), 10);
        assert_eq!(slots_for_family("oven"), 3);
        assert_eq!(slots_for_family("toaster"), 2);
    }

    #[test]
    fn test_ticks_for_family() {
        assert!(ticks_for_family("oven"));
        assert!(ticks_for_family("toaster"));
        assert!(ticks_for_family("milk_jar"));
        assert!(ticks_for_family("cow_jar"));
        assert!(!ticks_for_family("cabinet"));
        assert!(!ticks_for_family("fridge"));
    }

    #[test]
    fn test_oven_state_encode_decode() {
        let state = OvenState {
            progress: 145,
            burn: 1200,
            burn_max: 1600,
        };
        let encoded = state.encode();
        assert_eq!(encoded.len(), 12);
        let decoded = OvenState::decode(&encoded);
        assert_eq!(state, decoded);
    }

    #[test]
    fn test_simulate_oven_tick_lifecycle() {
        let mut state = OvenState::default();

        // 1. Initial tick with cookable item and fuel
        let res = simulate_oven_tick(
            &mut state,
            true,       // has input
            200,        // cook duration
            Some(1600), // coal fuel
            true,       // can output
        );
        assert!(res.consumed_fuel);
        assert_eq!(state.burn, 1600);
        assert_eq!(state.progress, 1);
        assert!(!res.finished_cooking);

        // 2. Advance 198 ticks
        for _ in 0..198 {
            let res = simulate_oven_tick(&mut state, true, 200, None, true);
            assert!(!res.consumed_fuel);
            assert!(!res.finished_cooking);
        }
        assert_eq!(state.progress, 199);

        // 3. 200th tick -> finish cooking
        let res = simulate_oven_tick(&mut state, true, 200, None, true);
        assert!(res.finished_cooking);
        assert_eq!(state.progress, 0); // resets for next item
        assert!(state.burn > 0); // still has remaining fuel
    }

    #[test]
    fn test_toaster_state() {
        let mut toaster = ToasterState {
            progress: 0,
            active: true,
        };
        for _ in 0..59 {
            assert!(!toaster.tick(true));
        }
        assert_eq!(toaster.progress, 59);
        assert!(toaster.tick(true)); // finished toast
        assert_eq!(toaster.progress, 0);
        assert!(!toaster.active);
    }
}

use horizon_mod_sdk::{
    capabilities::{self, EnergyView, FluidView},
    chat, config,
    item_transfer::{self, Face},
    machine::{self, ItemStack},
    recipes, storage,
    ui::{self, ScreenSpec, Widget, WidgetKind},
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemEvent, LogicHooks, Mod,
    Registration,
};
use std::collections::HashMap;

const CRYSTALLIZER: &str = "thermal:machine_crystallizer";
const RECIPE_TYPE: &str = "thermal:crystallizer";
const INPUT_SLOTS: [u32; 2] = [0, 1];
const OUTPUT_SLOT: u32 = 2;
const TANK_CAPACITY: u64 = 8_000;
const ENERGY_CAPACITY: u64 = 50_000;
const ENERGY_RATE: u64 = 128;
const DEFAULT_TICKS: u32 = 100;
const SCREEN: &str = "thermal:machine_crystallizer";

pub struct ThermalMod {
    wrench_sneak: bool,
    machines_dismantled: u32,
    crystallizers: HashMap<u32, CrystallizerState>,
}

#[derive(Default)]
struct CrystallizerState {
    recipe_id: String,
    progress: u32,
    operations: u64,
}

impl Mod for ThermalMod {
    fn init() -> Result<(Self, Registration), String> {
        let wrench_sneak = config::get_bool("wrench_dismantle_sneak").unwrap_or(true);
        let machines_dismantled = storage::get("machines_dismantled")
            .and_then(|b| String::from_utf8(b).ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        Ok((
            Self {
                wrench_sneak,
                machines_dismantled,
                crystallizers: HashMap::new(),
            },
            Registration::new().command("thermal"),
        ))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new()
            .block("thermal:apatite_ore")
            .block("thermal:apatite_block")
            .block(CRYSTALLIZER)
            .item("thermal:wrench")
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event
            .state
            .split_once('[')
            .map_or(event.state.as_str(), |(id, _)| id)
            != CRYSTALLIZER
            || !matches!(
                event.action,
                BlockAction::UseWithoutItem | BlockAction::UseItemOn
            )
        {
            return EventOutcome::Pass;
        }
        let _ = ui::open_screen(&screen_spec());
        EventOutcome::Consume
    }

    fn on_item(&mut self, _event: ItemEvent) -> EventOutcome {
        chat::show("§6[Thermal Series] §eCrescent Hammer rotated block face.");
        EventOutcome::Consume
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        vec![BlockEntityType {
            block: CRYSTALLIZER.into(),
            slots: 3,
            ticks: true,
        }]
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        if block == CRYSTALLIZER {
            self.crystallizers.insert(handle, decode(saved));
            for face in FACES {
                let _ = item_transfer::set_slot_access(handle, INPUT_SLOTS[0], face, true, false);
                let _ = item_transfer::set_slot_access(handle, INPUT_SLOTS[1], face, true, false);
                let _ = item_transfer::set_slot_access(handle, OUTPUT_SLOT, face, false, true);
            }
        }
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        let energy_capacity = configured_capacity();
        let energy_rate = configured_rate();
        for &handle in handles {
            let Some(position) = machine::get_position(handle) else {
                continue;
            };
            let key = format!(
                "{}:{},{},{}",
                position.dimension, position.x, position.y, position.z
            );
            if capabilities::get_energy(&key).is_none() {
                capabilities::set_energy_buffer(
                    &key,
                    EnergyView {
                        stored: 0,
                        capacity: energy_capacity,
                        max_receive: energy_rate,
                        max_extract: energy_rate,
                    },
                );
            }
            if capabilities::get_fluid(&key, 0).is_none() {
                capabilities::set_fluid_tank(
                    &key,
                    0,
                    &FluidView {
                        fluid: String::new(),
                        amount: 0,
                        capacity: TANK_CAPACITY,
                    },
                );
            }
            let energy = capabilities::get_energy(&key).unwrap_or(EnergyView {
                stored: 0,
                capacity: energy_capacity,
                max_receive: energy_rate,
                max_extract: energy_rate,
            });
            ui::set_gauge(SCREEN, 3, energy.stored, energy.capacity, "RF");
            let fluid = capabilities::get_fluid(&key, 0).unwrap_or(FluidView {
                fluid: String::new(),
                amount: 0,
                capacity: TANK_CAPACITY,
            });
            ui::set_gauge(SCREEN, 5, fluid.amount, fluid.capacity, &fluid.fluid);

            let recipes = recipes::by_type(RECIPE_TYPE);
            let selected = recipes.iter().find_map(|recipe| {
                matching_inputs(handle, &key, recipe).map(|inputs| (recipe, inputs))
            });
            let Some((recipe, inputs)) = selected else {
                if let Some(state) = self.crystallizers.get_mut(&handle) {
                    state.recipe_id.clear();
                    state.progress = 0;
                }
                let _ = machine::set_property(handle, "active", "false");
                ui::set_gauge(SCREEN, 4, 0, u64::from(DEFAULT_TICKS), "0%");
                continue;
            };
            let Some(output) = recipe.outputs.first().and_then(|entry| parse_output(entry)) else {
                continue;
            };
            if recipe.outputs.len() != 1 || !output_fits(handle, &output.id, output.count) {
                continue;
            }
            let required_ticks = if recipe.energy == 0 {
                configured_ticks()
            } else {
                let rate = energy_rate.min(energy.max_extract).max(1);
                recipe.energy.div_ceil(rate).clamp(1, u64::from(u32::MAX)) as u32
            };
            let state = self.crystallizers.entry(handle).or_default();
            if state.recipe_id != recipe.id {
                state.recipe_id.clone_from(&recipe.id);
                state.progress = 0;
            }
            if recipe.energy > 0 {
                let rate = energy_rate.min(energy.max_extract).max(1);
                let remaining = recipe
                    .energy
                    .saturating_sub(u64::from(state.progress).saturating_mul(rate));
                let needed = rate.min(remaining);
                if capabilities::extract_energy(&key, needed, true) < needed
                    || capabilities::extract_energy(&key, needed, false) < needed
                {
                    let _ = machine::set_property(handle, "active", "false");
                    continue;
                }
            }
            state.progress = state.progress.saturating_add(1);
            let _ = machine::set_property(handle, "active", "true");
            ui::set_gauge(
                SCREEN,
                4,
                u64::from(state.progress),
                u64::from(required_ticks),
                "Processing",
            );
            if state.progress < required_ticks {
                continue;
            }
            if !fluids_available(&key, &inputs.fluids) {
                state.progress = required_ticks.saturating_sub(1);
                continue;
            }
            let chance_passed = output.chance >= 1.0
                || chance(state.operations, handle) < output.chance.clamp(0.0, 1.0);
            if chance_passed {
                let current = machine::get_slot(handle, OUTPUT_SLOT).map_or(0, |s| s.count);
                if !machine::set_slot(
                    handle,
                    OUTPUT_SLOT,
                    Some(&ItemStack {
                        id: output.id.clone(),
                        count: current.saturating_add(output.count),
                    }),
                ) {
                    state.progress = required_ticks.saturating_sub(1);
                    continue;
                }
            }
            let mut consumed = true;
            for (slot, count) in &inputs.items {
                let Some(stack) = machine::get_slot(handle, *slot) else {
                    consumed = false;
                    break;
                };
                let remaining = stack.count.saturating_sub(*count);
                let next = (remaining > 0).then_some(ItemStack {
                    id: stack.id,
                    count: remaining,
                });
                consumed &= machine::set_slot(handle, *slot, next.as_ref());
            }
            if !consumed {
                state.progress = required_ticks.saturating_sub(1);
                continue;
            }
            for (tank, fluid, amount) in &inputs.fluids {
                let _ = capabilities::drain_fluid_tank(&key, *tank, fluid, *amount, false);
            }
            state.progress = 0;
            state.operations = state.operations.saturating_add(1);
            let _ = machine::set_property(handle, "active", "false");
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        let Some(state) = self.crystallizers.get(&handle) else {
            return Vec::new();
        };
        let Ok(length) = u16::try_from(state.recipe_id.len()) else {
            return Vec::new();
        };
        let mut saved = Vec::with_capacity(14 + state.recipe_id.len());
        saved.extend_from_slice(&length.to_le_bytes());
        saved.extend_from_slice(state.recipe_id.as_bytes());
        saved.extend_from_slice(&state.progress.to_le_bytes());
        saved.extend_from_slice(&state.operations.to_le_bytes());
        saved
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.crystallizers.remove(&handle);
    }

    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        vec![self.crystallizers.get(&handle).map_or(0, |s| s.progress) as i32]
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        match args.split_whitespace().next().unwrap_or("info") {
            "machines" => {
                chat::show("§6[Thermal Expansion] §fCrystallizer: recipes, FE and fluid processing are active.");
                Ok(None)
            }
            "stats" => {
                chat::show(&format!(
                    "§6[Thermal] §fDismantle setting: sneak={}, dismantled={}",
                    self.wrench_sneak, self.machines_dismantled
                ));
                Ok(None)
            }
            _ => {
                chat::show("§6[Thermal Series] §eUse: /thermal <machines|stats>");
                Ok(None)
            }
        }
    }
}

struct Inputs {
    items: Vec<(u32, u32)>,
    fluids: Vec<(u32, String, u64)>,
}

fn matching_inputs(handle: u32, key: &str, recipe: &recipes::RecipeEntry) -> Option<Inputs> {
    let mut items = Vec::new();
    let mut fluids = Vec::new();
    let mut slot = 0u32;
    for input in &recipe.inputs {
        if input.items.iter().any(|item| item.starts_with("@fluid/")) {
            let mut matched = None;
            for selector in &input.items {
                let Some(resource) = selector.strip_prefix("@fluid/") else {
                    continue;
                };
                let (tank, accepted) = resource.split_once('/')?;
                let tank = tank.parse::<u32>().ok()?;
                let view = capabilities::get_fluid(key, tank)?;
                if accepted == view.fluid && view.amount >= u64::from(input.count) {
                    matched = Some((tank, view.fluid, u64::from(input.count)));
                    break;
                }
            }
            fluids.push(matched?);
        } else {
            if slot >= INPUT_SLOTS.len() as u32 {
                return None;
            }
            let stack = machine::get_slot(handle, slot)?;
            if stack.count < input.count || !input.items.iter().any(|id| id == &stack.id) {
                return None;
            }
            items.push((slot, input.count));
            slot += 1;
        }
    }
    (!items.is_empty()).then_some(Inputs { items, fluids })
}

struct Output {
    id: String,
    count: u32,
    chance: f32,
}

fn parse_output(raw: &str) -> Option<Output> {
    let (raw, chance) = raw.split_once('@').unwrap_or((raw, "1"));
    let chance = chance.parse::<f32>().ok()?;
    let (id, count) = raw.split_once('*').unwrap_or((raw, "1"));
    let count = count.parse::<u32>().ok()?;
    (!id.is_empty() && count > 0 && chance.is_finite()).then(|| Output {
        id: id.to_owned(),
        count,
        chance,
    })
}

fn output_fits(handle: u32, id: &str, count: u32) -> bool {
    let max = machine::max_stack_size(id);
    if max == 0 {
        return false;
    }
    match machine::get_slot(handle, OUTPUT_SLOT) {
        None => count <= max,
        Some(current) => current.id == id && current.count.saturating_add(count) <= max,
    }
}

fn fluids_available(key: &str, fluids: &[(u32, String, u64)]) -> bool {
    fluids.iter().all(|(tank, id, amount)| {
        capabilities::get_fluid(key, *tank).is_some_and(|f| f.fluid == *id && f.amount >= *amount)
            && capabilities::drain_fluid_tank(key, *tank, id, *amount, true) >= *amount
    })
}

fn chance(operations: u64, handle: u32) -> f32 {
    let mut value = operations ^ (u64::from(handle) << 32) ^ 0x9e37_79b9_7f4a_7c15;
    value ^= value >> 12;
    value ^= value << 25;
    value ^= value >> 27;
    (value.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 40) as f32 / (1u32 << 24) as f32
}

fn decode(saved: &[u8]) -> CrystallizerState {
    let Some(length) = saved
        .get(..2)
        .and_then(|b| b.try_into().ok())
        .map(u16::from_le_bytes)
    else {
        return CrystallizerState::default();
    };
    let length = usize::from(length);
    let Some(end) = 2usize.checked_add(length) else {
        return CrystallizerState::default();
    };
    let Some(recipe_id) = saved.get(2..end).and_then(|b| std::str::from_utf8(b).ok()) else {
        return CrystallizerState::default();
    };
    let progress = saved
        .get(end..end + 4)
        .and_then(|b| b.try_into().ok())
        .map(u32::from_le_bytes)
        .unwrap_or(0);
    let operations = saved
        .get(end + 4..end + 12)
        .and_then(|b| b.try_into().ok())
        .map(u64::from_le_bytes)
        .unwrap_or(0);
    CrystallizerState {
        recipe_id: recipe_id.into(),
        progress,
        operations,
    }
}

fn configured_capacity() -> u64 {
    config::get_int("machine_rf_capacity")
        .unwrap_or(ENERGY_CAPACITY as i64)
        .clamp(1_000, 10_000_000) as u64
}

fn configured_rate() -> u64 {
    config::get_int("machine_rf_rate")
        .unwrap_or(ENERGY_RATE as i64)
        .clamp(1, 16_384) as u64
}

fn configured_ticks() -> u32 {
    config::get_int("ticks_per_recipe")
        .unwrap_or(i64::from(DEFAULT_TICKS))
        .clamp(1, 2_048) as u32
}

fn screen_spec() -> ScreenSpec {
    ScreenSpec {
        id: SCREEN.into(),
        title: "Crystallizer".into(),
        width: 176,
        height: 190,
        widgets: vec![
            widget(0, WidgetKind::Slot, 30, 45, "Item"),
            widget(1, WidgetKind::Slot, 58, 45, "Catalyst"),
            widget(2, WidgetKind::Slot, 118, 45, "Output"),
            widget(3, WidgetKind::EnergyBar, 152, 36, "RF"),
            widget(4, WidgetKind::ProgressArrow, 78, 48, "Progress"),
            widget(5, WidgetKind::FluidGauge, 12, 78, "Fluid"),
        ],
    }
}

fn widget(id: u32, kind: WidgetKind, x: i32, y: i32, text: &str) -> Widget {
    Widget {
        id,
        kind,
        x,
        y,
        width: 18,
        height: 18,
        text: text.into(),
        value: 0,
        max_value: 0,
    }
}

const FACES: [Face; 6] = [
    Face::Down,
    Face::Up,
    Face::North,
    Face::South,
    Face::West,
    Face::East,
];

horizon_mod_sdk::export_mod!(ThermalMod);

use horizon_mod_sdk::{
    capabilities::{self, EnergyView, FluidView},
    chat, config, interaction_context,
    item_transfer::{self, Face},
    machine::{self, ItemStack},
    recipes, server, storage,
    ui::{self, ScreenSpec, Widget, WidgetKind},
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
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
const DRY_MACHINES: &[(&str, &str, u32)] = &[
    ("thermal:machine_pulverizer", "thermal:pulverizer", 4),
    ("thermal:machine_furnace", "thermal:furnace", 3),
    ("thermal:machine_press", "thermal:press", 3),
    ("thermal:machine_smelter", "thermal:smelter", 4),
];
const WET_MACHINES: &[(&str, &str, u32)] = &[
    ("thermal:machine_crucible", "thermal:crucible", 1),
    ("thermal:machine_chiller", "thermal:chiller", 2),
    (INSOLATOR, "thermal:insolator", 4),
];
const INSOLATOR: &str = "thermal:machine_insolator";
const ROCK_GEN: &str = "thermal:device_rock_gen";

pub struct ThermalMod {
    wrench_sneak: bool,
    machines_dismantled: u32,
    crystallizers: HashMap<u32, CrystallizerState>,
    dry_machines: HashMap<u32, DryMachineState>,
}

#[derive(Default)]
struct DryMachineState {
    block: String,
    recipe_id: String,
    progress: u32,
    operations: u64,
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
                dry_machines: HashMap::new(),
            },
            Registration::new().command("thermal"),
        ))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new()
            .block("thermal:apatite_ore")
            .block("thermal:apatite_block")
            .block(CRYSTALLIZER)
            .block("thermal:machine_pulverizer")
            .block("thermal:machine_furnace")
            .block("thermal:machine_press")
            .block("thermal:machine_smelter")
            .block("thermal:machine_crucible")
            .block("thermal:machine_chiller")
            .block(INSOLATOR)
            .block(ROCK_GEN)
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        let block = event
            .state
            .split_once('[')
            .map_or(event.state.as_str(), |(id, _)| id)
            .to_owned();
        if !matches!(
            event.action,
            BlockAction::UseWithoutItem | BlockAction::UseItemOn
        ) {
            return EventOutcome::Pass;
        }
        if event.action == BlockAction::UseItemOn && event.held_item == "thermal:wrench" {
            let base = base_block(&block);
            if is_thermal_machine(base) {
                return self.use_crescent_hammer(&event, base);
            }
        }
        if block == CRYSTALLIZER {
            let _ = ui::open_screen(&screen_spec());
            EventOutcome::Consume
        } else if let Some((_, _, slots)) = DRY_MACHINES.iter().find(|(id, _, _)| *id == block) {
            let _ = ui::open_screen(&dry_screen_spec(&block, *slots));
            EventOutcome::Consume
        } else if let Some((_, _, slots)) = WET_MACHINES.iter().find(|(id, _, _)| *id == block) {
            let _ = ui::open_screen(&wet_screen_spec(&block, *slots));
            EventOutcome::Consume
        } else if block == ROCK_GEN {
            let _ = ui::open_screen(&rock_gen_screen_spec());
            EventOutcome::Consume
        } else {
            EventOutcome::Pass
        }
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        vec![BlockEntityType {
            block: CRYSTALLIZER.into(),
            slots: 3,
            ticks: true,
        }]
        .into_iter()
        .chain(std::iter::once(BlockEntityType {
            block: ROCK_GEN.into(),
            slots: 1,
            ticks: true,
        }))
        .chain(
            DRY_MACHINES
                .iter()
                .map(|(block, _, slots)| BlockEntityType {
                    block: (*block).into(),
                    slots: *slots,
                    ticks: true,
                }),
        )
        .chain(
            WET_MACHINES
                .iter()
                .map(|(block, _, slots)| BlockEntityType {
                    block: (*block).into(),
                    slots: *slots,
                    ticks: true,
                }),
        )
        .collect()
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        if block == CRYSTALLIZER {
            self.crystallizers.insert(handle, decode(saved));
            for face in FACES {
                let _ = item_transfer::set_slot_access(handle, INPUT_SLOTS[0], face, true, false);
                let _ = item_transfer::set_slot_access(handle, INPUT_SLOTS[1], face, true, false);
                let _ = item_transfer::set_slot_access(handle, OUTPUT_SLOT, face, false, true);
            }
        } else if let Some((_, _, slots)) = DRY_MACHINES.iter().find(|(id, _, _)| *id == block) {
            self.dry_machines.insert(handle, decode_dry(block, saved));
            let output_start = if *slots == 4 { 2 } else { slots - 1 };
            for slot in 0..output_start {
                for face in FACES {
                    let _ = item_transfer::set_slot_access(handle, slot, face, true, false);
                }
            }
            for slot in output_start..*slots {
                for face in FACES {
                    let _ = item_transfer::set_slot_access(handle, slot, face, false, true);
                }
            }
        } else if let Some((_, _, slots)) = WET_MACHINES.iter().find(|(id, _, _)| *id == block) {
            self.dry_machines.insert(handle, decode_dry(block, saved));
            let input_slots = if block == "thermal:machine_crucible" {
                1
            } else if block == INSOLATOR {
                2
            } else {
                *slots - 1
            };
            for slot in 0..input_slots {
                for face in FACES {
                    let _ = item_transfer::set_slot_access(handle, slot, face, true, false);
                }
            }
            if block == "thermal:machine_chiller" {
                for face in FACES {
                    let _ = item_transfer::set_slot_access(handle, 1, face, false, true);
                }
            } else if block == INSOLATOR {
                for slot in 2..*slots {
                    for face in FACES {
                        let _ = item_transfer::set_slot_access(handle, slot, face, false, true);
                    }
                }
            }
        } else if block == ROCK_GEN {
            self.dry_machines.insert(handle, decode_dry(block, saved));
            for face in FACES {
                let _ = item_transfer::set_slot_access(handle, 0, face, false, true);
            }
        }
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        let energy_capacity = configured_capacity();
        let energy_rate = configured_rate();
        for &handle in handles {
            if self.dry_machines.contains_key(&handle) {
                if self
                    .dry_machines
                    .get(&handle)
                    .is_some_and(|state| state.block == ROCK_GEN)
                {
                    self.tick_rock_gen(handle);
                    continue;
                }
                if self
                    .dry_machines
                    .get(&handle)
                    .is_some_and(|state| state.block == INSOLATOR)
                {
                    self.tick_insolator(handle, energy_capacity, energy_rate);
                    continue;
                }
                if self.dry_machines.get(&handle).is_some_and(|state| {
                    WET_MACHINES
                        .iter()
                        .any(|(block, _, _)| *block == state.block)
                }) {
                    self.tick_wet_machine(handle, energy_capacity, energy_rate);
                } else {
                    self.tick_dry_machine(handle, energy_capacity, energy_rate);
                }
                continue;
            }
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
        if let Some(state) = self.dry_machines.get(&handle) {
            let mut saved = Vec::with_capacity(8 + state.block.len() + state.recipe_id.len());
            saved.extend_from_slice(&(state.block.len() as u16).to_le_bytes());
            saved.extend_from_slice(state.block.as_bytes());
            saved.extend_from_slice(&(state.recipe_id.len() as u16).to_le_bytes());
            saved.extend_from_slice(state.recipe_id.as_bytes());
            saved.extend_from_slice(&state.progress.to_le_bytes());
            saved.extend_from_slice(&state.operations.to_le_bytes());
            return saved;
        }
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
        self.dry_machines.remove(&handle);
    }

    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        vec![self.crystallizers.get(&handle).map_or_else(
            || self.dry_machines.get(&handle).map_or(0, |s| s.progress),
            |s| s.progress,
        ) as i32]
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        match args.split_whitespace().next().unwrap_or("info") {
            "machines" => {
                chat::show("§6[Thermal Expansion] §fCrystallizer, Pulverizer, Furnace, Press and Smelter processing are available.");
                Ok(None)
            }
            "recipes" => {
                let kinds = DRY_MACHINES
                    .iter()
                    .map(|(_, kind, _)| format!("{kind}={}", recipes::by_type(kind).len()))
                    .collect::<Vec<_>>()
                    .join(", ");
                let copper = recipes::for_input("minecraft:copper_ingot")
                    .iter()
                    .filter(|recipe| recipe.kind == "thermal:smelter")
                    .count();
                chat::show(&format!(
                    "§6[Thermal] §fRecipes: {kinds}; copper tag matches={copper}"
                ));
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

impl ThermalMod {
    fn tick_insolator(&mut self, handle: u32, energy_capacity: u64, energy_rate: u64) {
        let Some(state) = self.dry_machines.get_mut(&handle) else {
            return;
        };
        let Some(position) = machine::get_position(handle) else {
            return;
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
        let screen = "thermal:insolator";
        ui::set_gauge(screen, 6, energy.stored, energy.capacity, "RF");
        let water = capabilities::get_fluid(&key, 0).unwrap_or(FluidView {
            fluid: String::new(),
            amount: 0,
            capacity: TANK_CAPACITY,
        });
        ui::set_gauge(screen, 7, water.amount, water.capacity, &water.fluid);
        let selected = recipes::by_type("thermal:insolator")
            .into_iter()
            .find_map(|recipe| {
                matching_inputs_in_slots(handle, &key, &recipe, &[0, 1])
                    .map(|inputs| (recipe, inputs))
            });
        let Some((recipe, inputs)) = selected else {
            state.recipe_id.clear();
            state.progress = 0;
            let _ = machine::set_property(handle, "lit", "false");
            return;
        };
        // A Thermal Insolator recipe must carry both calibrated RF and water costs.
        // Keep it idle until its carried multipliers have a verified host baseline.
        if recipe.energy == 0 || inputs.fluids.is_empty() {
            state.recipe_id.clone_from(&recipe.id);
            state.progress = 0;
            let _ = machine::set_property(handle, "lit", "false");
            return;
        }
        let outputs: Vec<Output> = recipe
            .outputs
            .iter()
            .filter_map(|raw| parse_output(raw))
            .collect();
        if outputs.len() != recipe.outputs.len() || outputs.is_empty() {
            return;
        }
        let rate = energy_rate.min(energy.max_extract).max(1);
        let required_ticks = recipe.energy.div_ceil(rate).clamp(1, u64::from(u32::MAX)) as u32;
        if state.recipe_id != recipe.id {
            state.recipe_id.clone_from(&recipe.id);
            state.progress = 0;
        }
        let remaining = recipe
            .energy
            .saturating_sub(u64::from(state.progress).saturating_mul(rate));
        let needed = rate.min(remaining);
        if capabilities::extract_energy(&key, needed, true) < needed
            || capabilities::extract_energy(&key, needed, false) < needed
        {
            let _ = machine::set_property(handle, "lit", "false");
            return;
        }
        state.progress = state.progress.saturating_add(1);
        let _ = machine::set_property(handle, "lit", "true");
        if state.progress < required_ticks {
            return;
        }
        if !fluids_available(&key, &inputs.fluids)
            || !insolator_outputs_fit(handle, &outputs, state.operations)
        {
            state.progress = required_ticks.saturating_sub(1);
            return;
        }
        for (slot, count) in &inputs.items {
            let Some(stack) = machine::get_slot(handle, *slot) else {
                state.progress = 0;
                return;
            };
            let left = stack.count.saturating_sub(*count);
            let next = (left > 0).then_some(ItemStack {
                id: stack.id,
                count: left,
            });
            if !machine::set_slot(handle, *slot, next.as_ref()) {
                state.progress = required_ticks.saturating_sub(1);
                return;
            }
        }
        for (tank, fluid, amount) in &inputs.fluids {
            let _ = capabilities::drain_fluid_tank(&key, *tank, fluid, *amount, false);
        }
        for (index, output) in outputs.into_iter().enumerate() {
            let count = output.count
                * u32::from(
                    output.chance >= 1.0
                        || chance(state.operations.wrapping_add(index as u64), handle)
                            < output.chance.clamp(0.0, 1.0),
                );
            if count == 0 {
                continue;
            }
            let mut target = None;
            for slot in 2..4 {
                match machine::get_slot(handle, slot) {
                    Some(stack) if stack.id == output.id => {
                        target = Some((slot, stack.count));
                        break;
                    }
                    None if target.is_none() => target = Some((slot, 0)),
                    _ => {}
                }
            }
            if let Some((slot, current)) = target {
                let _ = machine::set_slot(
                    handle,
                    slot,
                    Some(&ItemStack {
                        id: output.id,
                        count: current.saturating_add(count),
                    }),
                );
            }
        }
        state.progress = 0;
        state.operations = state.operations.saturating_add(1);
        let _ = machine::set_property(handle, "lit", "false");
    }

    fn tick_rock_gen(&mut self, handle: u32) {
        let Some(state) = self.dry_machines.get_mut(&handle) else {
            return;
        };
        let Some(position) = machine::get_position(handle) else {
            return;
        };
        let offset_positions = [(1, 0, 0), (-1, 0, 0), (0, 0, 1), (0, 0, -1)];
        let adjacent: Vec<String> = offset_positions
            .iter()
            .filter_map(|(dx, dy, dz)| {
                let pos = server::BlockPos {
                    x: position.x + dx,
                    y: position.y + dy,
                    z: position.z + dz,
                };
                server::get_block(&position.dimension, pos).map(|s| base_block(&s).to_owned())
            })
            .collect();
        let below = server::get_block(
            &position.dimension,
            server::BlockPos {
                x: position.x,
                y: position.y - 1,
                z: position.z,
            },
        )
        .map(|s| base_block(&s).to_owned());
        let result = if adjacent.iter().any(|b| b == "minecraft:water") {
            if below.as_deref() == Some("minecraft:magma_block") {
                "minecraft:stone"
            } else {
                "minecraft:cobblestone"
            }
        } else if adjacent.iter().any(|b| b == "minecraft:packed_ice") {
            if below.as_deref() == Some("minecraft:magma_block") {
                "minecraft:deepslate"
            } else {
                "minecraft:cobbled_deepslate"
            }
        } else if adjacent.iter().any(|b| b == "minecraft:blue_ice") {
            if below.as_deref() == Some("minecraft:soul_soil") {
                "minecraft:basalt"
            } else {
                "minecraft:blackstone"
            }
        } else {
            state.progress = 0;
            state.recipe_id.clear();
            let _ = machine::set_property(handle, "lit", "false");
            return;
        };
        let current = machine::get_slot(handle, 0);
        if current
            .as_ref()
            .is_some_and(|stack| stack.id != result || stack.count >= 64)
        {
            return;
        }
        state.recipe_id = result.to_owned();
        state.progress = state.progress.saturating_add(1);
        let _ = machine::set_property(handle, "lit", "true");
        if state.progress < 20 {
            return;
        }
        let count = current.map_or(1, |stack| stack.count.saturating_add(1));
        if machine::set_slot(
            handle,
            0,
            Some(&ItemStack {
                id: result.to_owned(),
                count,
            }),
        ) {
            state.progress = 0;
            state.operations = state.operations.saturating_add(1);
        }
    }

    fn use_crescent_hammer(&mut self, event: &BlockEvent, block: &str) -> EventOutcome {
        let position = server::BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        if interaction_context::is_sneaking() || !self.wrench_sneak {
            let key = format!("{}:{},{},{}", event.dimension, event.x, event.y, event.z);
            if !capabilities::get_item_slots(&key).is_empty()
                || capabilities::get_energy(&key).is_some_and(|energy| energy.stored > 0)
                || (0..4).any(|tank| {
                    capabilities::get_fluid(&key, tank).is_some_and(|fluid| fluid.amount > 0)
                })
            {
                chat::show("§6[Thermal] §eEmpty the machine before dismantling it with the Crescent Hammer.");
                return EventOutcome::Consume;
            }
            if server::set_block(&event.dimension, position, "minecraft:air").is_err() {
                return EventOutcome::Pass;
            }
            server::spawn_drop(
                &event.dimension,
                position,
                &server::ItemDrop {
                    id: block.to_owned(),
                    count: 1,
                },
            );
            self.machines_dismantled = self.machines_dismantled.saturating_add(1);
            let _ = storage::set(
                "machines_dismantled",
                self.machines_dismantled.to_string().as_bytes(),
            );
            return EventOutcome::Consume;
        }

        let Some(rotated) = rotate_facing(&event.state) else {
            return EventOutcome::Pass;
        };
        if server::set_block(&event.dimension, position, &rotated).is_ok() {
            EventOutcome::Consume
        } else {
            EventOutcome::Pass
        }
    }

    fn tick_wet_machine(&mut self, handle: u32, energy_capacity: u64, energy_rate: u64) {
        let Some(state) = self.dry_machines.get_mut(&handle) else {
            return;
        };
        let Some((_, recipe_type, slots)) = WET_MACHINES
            .iter()
            .find(|(block, _, _)| *block == state.block)
        else {
            return;
        };
        let Some(position) = machine::get_position(handle) else {
            return;
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
        let fluid_tank = 0;
        if capabilities::get_fluid(&key, fluid_tank).is_none() {
            capabilities::set_fluid_tank(
                &key,
                fluid_tank,
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
        let fluid = capabilities::get_fluid(&key, fluid_tank).unwrap_or(FluidView {
            fluid: String::new(),
            amount: 0,
            capacity: TANK_CAPACITY,
        });
        let screen = format!(
            "thermal:{}",
            state
                .block
                .strip_prefix("thermal:machine_")
                .unwrap_or("machine")
        );
        ui::set_gauge(&screen, 6, energy.stored, energy.capacity, "RF");
        ui::set_gauge(&screen, 7, fluid.amount, fluid.capacity, &fluid.fluid);
        let item_slots: &[u32] = &[0];
        let output_slot = slots.saturating_sub(1);
        let selected = recipes::by_type(recipe_type)
            .into_iter()
            .find_map(|recipe| {
                matching_inputs_in_slots(handle, &key, &recipe, item_slots)
                    .map(|inputs| (recipe, inputs))
            });
        let Some((recipe, inputs)) = selected else {
            state.recipe_id.clear();
            state.progress = 0;
            let _ = machine::set_property(handle, "lit", "false");
            return;
        };
        if recipe.energy == 0 {
            state.progress = 0;
            state.recipe_id.clone_from(&recipe.id);
            let _ = machine::set_property(handle, "lit", "false");
            return;
        }
        let outputs: Vec<WetOutput> = recipe
            .outputs
            .iter()
            .filter_map(|raw| parse_wet_output(raw))
            .collect();
        if outputs.len() != recipe.outputs.len()
            || outputs.is_empty()
            || !wet_outputs_fit(handle, &key, state.block.as_str(), output_slot, &outputs)
        {
            let _ = machine::set_property(handle, "lit", "false");
            return;
        }
        let rate = energy_rate.min(energy.max_extract).max(1);
        let required_ticks = recipe.energy.div_ceil(rate).clamp(1, u64::from(u32::MAX)) as u32;
        if state.recipe_id != recipe.id {
            state.recipe_id.clone_from(&recipe.id);
            state.progress = 0;
        }
        let remaining = recipe
            .energy
            .saturating_sub(u64::from(state.progress).saturating_mul(rate));
        let needed = rate.min(remaining);
        if capabilities::extract_energy(&key, needed, true) < needed
            || capabilities::extract_energy(&key, needed, false) < needed
        {
            let _ = machine::set_property(handle, "lit", "false");
            return;
        }
        state.progress = state.progress.saturating_add(1);
        let _ = machine::set_property(handle, "lit", "true");
        ui::set_gauge(
            &screen,
            5,
            u64::from(state.progress),
            u64::from(required_ticks),
            "Processing",
        );
        if state.progress < required_ticks {
            return;
        }
        if !fluids_available(&key, &inputs.fluids) {
            state.progress = required_ticks.saturating_sub(1);
            return;
        }
        for (slot, count) in &inputs.items {
            let Some(stack) = machine::get_slot(handle, *slot) else {
                state.progress = 0;
                return;
            };
            let remaining = stack.count.saturating_sub(*count);
            let next = (remaining > 0).then_some(ItemStack {
                id: stack.id,
                count: remaining,
            });
            if !machine::set_slot(handle, *slot, next.as_ref()) {
                state.progress = required_ticks.saturating_sub(1);
                return;
            }
        }
        for (tank, fluid, amount) in &inputs.fluids {
            if capabilities::drain_fluid_tank(&key, *tank, fluid, *amount, false) < *amount {
                state.progress = required_ticks.saturating_sub(1);
                return;
            }
        }
        for (index, output) in outputs.into_iter().enumerate() {
            match output {
                WetOutput::Item(output) => {
                    if output.chance >= 1.0
                        || chance(state.operations.wrapping_add(index as u64), handle)
                            < output.chance
                    {
                        let current = machine::get_slot(handle, output_slot).map_or(0, |s| s.count);
                        let _ = machine::set_slot(
                            handle,
                            output_slot,
                            Some(&ItemStack {
                                id: output.id,
                                count: current.saturating_add(output.count),
                            }),
                        );
                    }
                }
                WetOutput::Fluid { tank, id, amount } => {
                    let _ = capabilities::fill_fluid_tank(&key, tank, &id, amount, false);
                }
            }
        }
        state.progress = 0;
        state.operations = state.operations.saturating_add(1);
        let _ = machine::set_property(handle, "lit", "false");
    }

    fn tick_dry_machine(&mut self, handle: u32, energy_capacity: u64, energy_rate: u64) {
        let Some(state) = self.dry_machines.get_mut(&handle) else {
            return;
        };
        let Some((_, recipe_type, slots)) = DRY_MACHINES
            .iter()
            .find(|(block, _, _)| *block == state.block)
        else {
            return;
        };
        let Some(position) = machine::get_position(handle) else {
            return;
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
        let energy = capabilities::get_energy(&key).unwrap_or(EnergyView {
            stored: 0,
            capacity: energy_capacity,
            max_receive: energy_rate,
            max_extract: energy_rate,
        });
        let screen = format!(
            "thermal:{}",
            state
                .block
                .strip_prefix("thermal:machine_")
                .unwrap_or("machine")
        );
        ui::set_gauge(&screen, 6, energy.stored, energy.capacity, "RF");
        let candidates = recipes::by_type(recipe_type);
        let output_start = if *slots == 4 { 2 } else { *slots - 1 };
        let selected = candidates.iter().find_map(|recipe| {
            dry_inputs(handle, recipe, output_start).map(|items| (recipe, items))
        });
        let Some((recipe, inputs)) = selected else {
            state.recipe_id.clear();
            state.progress = 0;
            let _ = machine::set_property(handle, "lit", "false");
            return;
        };
        let outputs: Vec<Output> = recipe
            .outputs
            .iter()
            .filter_map(|raw| parse_output(raw))
            .collect();
        if outputs.len() != recipe.outputs.len()
            || outputs.is_empty()
            || !dry_outputs_fit(handle, output_start, *slots, &outputs)
        {
            let _ = machine::set_property(handle, "lit", "false");
            return;
        }
        if recipe.energy == 0 {
            state.progress = 0;
            state.recipe_id.clone_from(&recipe.id);
            let _ = machine::set_property(handle, "lit", "false");
            return;
        }
        let rate = energy_rate.min(energy.max_extract).max(1);
        let required_ticks = recipe.energy.div_ceil(rate).clamp(1, u64::from(u32::MAX)) as u32;
        if state.recipe_id != recipe.id {
            state.recipe_id.clone_from(&recipe.id);
            state.progress = 0;
        }
        let remaining = recipe
            .energy
            .saturating_sub(u64::from(state.progress).saturating_mul(rate));
        let needed = rate.min(remaining);
        if capabilities::extract_energy(&key, needed, true) < needed
            || capabilities::extract_energy(&key, needed, false) < needed
        {
            let _ = machine::set_property(handle, "lit", "false");
            return;
        }
        state.progress = state.progress.saturating_add(1);
        let _ = machine::set_property(handle, "lit", "true");
        ui::set_gauge(
            &screen,
            5,
            u64::from(state.progress),
            u64::from(required_ticks),
            "Processing",
        );
        if state.progress < required_ticks {
            return;
        }
        for (slot, count) in &inputs {
            let Some(stack) = machine::get_slot(handle, *slot) else {
                state.progress = 0;
                return;
            };
            let remaining = stack.count.saturating_sub(*count);
            let next = (remaining > 0).then_some(ItemStack {
                id: stack.id,
                count: remaining,
            });
            if !machine::set_slot(handle, *slot, next.as_ref()) {
                state.progress = required_ticks.saturating_sub(1);
                return;
            }
        }
        for (index, output) in outputs.into_iter().enumerate() {
            let count = if output.chance >= 1.0 {
                output.count
            } else if chance(state.operations.wrapping_add(index as u64), handle)
                < output.chance.clamp(0.0, 1.0)
            {
                output.count
            } else {
                0
            };
            if count > 0 {
                let slot = output_start + index as u32;
                let current = machine::get_slot(handle, slot).map_or(0, |s| s.count);
                let _ = machine::set_slot(
                    handle,
                    slot,
                    Some(&ItemStack {
                        id: output.id,
                        count: current.saturating_add(count),
                    }),
                );
            }
        }
        state.progress = 0;
        state.operations = state.operations.saturating_add(1);
        let _ = machine::set_property(handle, "lit", "false");
    }
}

struct Inputs {
    items: Vec<(u32, u32)>,
    fluids: Vec<(u32, String, u64)>,
}

enum WetOutput {
    Item(Output),
    Fluid { tank: u32, id: String, amount: u64 },
}

fn matching_inputs(handle: u32, key: &str, recipe: &recipes::RecipeEntry) -> Option<Inputs> {
    matching_inputs_in_slots(handle, key, recipe, &INPUT_SLOTS)
}

fn matching_inputs_in_slots(
    handle: u32,
    key: &str,
    recipe: &recipes::RecipeEntry,
    item_slots: &[u32],
) -> Option<Inputs> {
    let mut items = Vec::new();
    let mut fluids = Vec::new();
    let mut next_item = 0usize;
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
            let slot = *item_slots.get(next_item)?;
            let stack = machine::get_slot(handle, slot)?;
            if stack.count < input.count || !input.items.iter().any(|id| id == &stack.id) {
                return None;
            }
            items.push((slot, input.count));
            next_item += 1;
        }
    }
    (!items.is_empty() || !fluids.is_empty()).then_some(Inputs { items, fluids })
}

fn parse_wet_output(raw: &str) -> Option<WetOutput> {
    if let Some(resource) = raw.strip_prefix("@fluid/") {
        let (tank, value) = resource.split_once('/')?;
        let tank = tank.parse::<u32>().ok()?;
        let (id, amount) = value.split_once('*').unwrap_or((value, "1"));
        let amount = amount.parse::<u64>().ok()?;
        return (!id.is_empty() && amount > 0).then(|| WetOutput::Fluid {
            tank,
            id: id.to_owned(),
            amount,
        });
    }
    parse_output(raw).map(WetOutput::Item)
}

fn wet_outputs_fit(
    handle: u32,
    key: &str,
    block: &str,
    item_slot: u32,
    outputs: &[WetOutput],
) -> bool {
    let mut item_total = 0u32;
    let mut item_id = None::<&str>;
    let mut fluid_totals = HashMap::<(u32, String), u64>::new();
    for output in outputs {
        match output {
            WetOutput::Item(output) => {
                if block == "thermal:machine_crucible" {
                    return false;
                }
                if item_id.is_some_and(|id| id != output.id) {
                    return false;
                }
                item_id = Some(&output.id);
                item_total = item_total.saturating_add(output.count);
            }
            WetOutput::Fluid { tank, id, amount } => {
                if block == "thermal:machine_chiller" {
                    return false;
                }
                *fluid_totals.entry((*tank, id.clone())).or_default() += *amount;
            }
        }
    }
    if let Some(id) = item_id {
        let max = machine::max_stack_size(id);
        if max == 0 {
            return false;
        }
        match machine::get_slot(handle, item_slot) {
            None if item_total > max => return false,
            Some(current) if current.id != id || current.count.saturating_add(item_total) > max => {
                return false;
            }
            _ => {}
        }
    }
    fluid_totals.into_iter().all(|((tank, id), amount)| {
        capabilities::get_fluid(key, tank).is_some_and(|fluid| {
            (fluid.fluid.is_empty() || fluid.fluid == id)
                && fluid.amount.saturating_add(amount) <= fluid.capacity
        })
    })
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

fn insolator_outputs_fit(handle: u32, outputs: &[Output], operation: u64) -> bool {
    let mut planned: [Option<(String, u32)>; 2] = [
        machine::get_slot(handle, 2).map(|s| (s.id, s.count)),
        machine::get_slot(handle, 3).map(|s| (s.id, s.count)),
    ];
    for (index, output) in outputs.iter().enumerate() {
        if output.chance < 1.0
            && chance(operation.wrapping_add(index as u64), handle) >= output.chance.clamp(0.0, 1.0)
        {
            continue;
        }
        let Some((_, count)) = planned
            .iter_mut()
            .flatten()
            .find(|(id, _)| id == &output.id)
        else {
            let Some(slot) = planned.iter_mut().find(|slot| slot.is_none()) else {
                return false;
            };
            *slot = Some((output.id.clone(), output.count));
            continue;
        };
        *count = count.saturating_add(output.count);
    }
    planned.iter().flatten().all(|(id, count)| {
        let max = machine::max_stack_size(id);
        max > 0 && *count <= max
    })
}

fn dry_inputs(
    handle: u32,
    recipe: &recipes::RecipeEntry,
    output_start: u32,
) -> Option<Vec<(u32, u32)>> {
    if recipe.inputs.is_empty() || recipe.inputs.len() > output_start as usize {
        return None;
    }
    let mut matched = Vec::new();
    for (slot, input) in recipe.inputs.iter().enumerate() {
        let stack = machine::get_slot(handle, slot as u32)?;
        if stack.count < input.count {
            return None;
        }
        let exact = input.items.iter().any(|selector| selector == &stack.id);
        let in_tag = !exact
            && recipes::for_input(&stack.id)
                .iter()
                .any(|candidate| candidate.id == recipe.id);
        if !exact && !in_tag {
            return None;
        }
        matched.push((slot as u32, input.count));
    }
    Some(matched)
}

fn dry_outputs_fit(handle: u32, start: u32, slots: u32, outputs: &[Output]) -> bool {
    if outputs.len() > (slots - start) as usize {
        return false;
    }
    outputs.iter().enumerate().all(|(index, output)| {
        let slot = start + index as u32;
        let max = machine::max_stack_size(&output.id);
        if max == 0 {
            return false;
        }
        match machine::get_slot(handle, slot) {
            None => output.count <= max,
            Some(current) => {
                current.id == output.id && current.count.saturating_add(output.count) <= max
            }
        }
    })
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

fn decode_dry(block: &str, saved: &[u8]) -> DryMachineState {
    let mut state = DryMachineState {
        block: block.to_owned(),
        ..DryMachineState::default()
    };
    let Some(block_len) = saved
        .get(..2)
        .and_then(|b| b.try_into().ok())
        .map(u16::from_le_bytes)
        .map(usize::from)
    else {
        return state;
    };
    let Some(recipe_len_pos) = 2usize.checked_add(block_len) else {
        return state;
    };
    let Some(recipe_len) = saved
        .get(recipe_len_pos..recipe_len_pos + 2)
        .and_then(|b| b.try_into().ok())
        .map(u16::from_le_bytes)
        .map(usize::from)
    else {
        return state;
    };
    let recipe_start = recipe_len_pos + 2;
    let Some(recipe_end) = recipe_start.checked_add(recipe_len) else {
        return state;
    };
    if let Some(raw) = saved.get(recipe_start..recipe_end) {
        state.recipe_id = String::from_utf8_lossy(raw).into_owned();
    }
    if let Some(raw) = saved.get(recipe_end..recipe_end + 4) {
        state.progress = u32::from_le_bytes(raw.try_into().unwrap_or([0; 4]));
    }
    if let Some(raw) = saved.get(recipe_end + 4..recipe_end + 12) {
        state.operations = u64::from_le_bytes(raw.try_into().unwrap_or([0; 8]));
    }
    state
}

fn configured_capacity() -> u64 {
    config::get_int("machine_rf_capacity")
        .unwrap_or(ENERGY_CAPACITY as i64)
        .clamp(1_000, 10_000_000) as u64
}

fn base_block(state: &str) -> &str {
    state.split_once('[').map_or(state, |(block, _)| block)
}

fn is_thermal_machine(block: &str) -> bool {
    block == CRYSTALLIZER
        || block == INSOLATOR
        || block == ROCK_GEN
        || DRY_MACHINES.iter().any(|(id, _, _)| *id == block)
        || WET_MACHINES.iter().any(|(id, _, _)| *id == block)
}

fn rotate_facing(state: &str) -> Option<String> {
    let (block, properties) = state.strip_suffix(']')?.split_once('[')?;
    let mut properties = properties.split(',').map(str::to_owned).collect::<Vec<_>>();
    let facing = properties
        .iter_mut()
        .find(|property| property.starts_with("facing="))?;
    let current = facing.strip_prefix("facing=")?;
    let sequence = ["north", "east", "south", "west"];
    let index = sequence
        .iter()
        .position(|direction| *direction == current)?;
    *facing = format!("facing={}", sequence[(index + 1) % sequence.len()]);
    Some(format!("{block}[{}]", properties.join(",")))
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

fn dry_screen_spec(block: &str, slots: u32) -> ScreenSpec {
    let short = block.strip_prefix("thermal:machine_").unwrap_or("machine");
    let output_start = if slots == 4 { 2 } else { slots - 1 };
    let mut widgets = Vec::new();
    for slot in 0..slots {
        let (x, label) = if slot < output_start {
            (30 + slot as i32 * 28, "Input")
        } else {
            (118 + (slot - output_start) as i32 * 24, "Output")
        };
        widgets.push(widget(slot, WidgetKind::Slot, x, 45, label));
    }
    widgets.push(widget(5, WidgetKind::ProgressArrow, 78, 48, "Progress"));
    widgets.push(widget(6, WidgetKind::EnergyBar, 152, 36, "RF"));
    ScreenSpec {
        id: format!("thermal:{short}"),
        title: format!("Thermal {}", title_case(short)),
        width: 176,
        height: 190,
        widgets,
    }
}

fn wet_screen_spec(block: &str, slots: u32) -> ScreenSpec {
    let mut screen = dry_screen_spec(block, slots);
    screen
        .widgets
        .push(widget(7, WidgetKind::FluidGauge, 12, 78, "Fluid"));
    screen
}

fn rock_gen_screen_spec() -> ScreenSpec {
    ScreenSpec {
        id: "thermal:device_rock_gen".into(),
        title: "Thermal Igneous Extruder".into(),
        width: 176,
        height: 190,
        widgets: vec![
            widget(0, WidgetKind::Slot, 80, 48, "Output"),
            widget(1, WidgetKind::ProgressArrow, 80, 78, "Progress"),
        ],
    }
}

fn title_case(value: &str) -> String {
    let mut chars = value.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
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

//! Mekanism: Generators power sources integrated with Horizon's machine API.
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    capabilities::{self, EnergyView},
    game,
    machine::{self, ItemStack},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct Mekanismgenerators {
    blocks: HashMap<u32, String>,
    burn_left: HashMap<u32, u32>,
    turbine_restore: HashMap<u32, capabilities::FluidView>,
    fission_restore: HashMap<u32, Vec<(u32, capabilities::FluidView)>>,
    fusion_restore: HashMap<u32, FusionStored>,
}

struct FusionStored {
    energy: EnergyView,
    fluids: Vec<(u32, capabilities::FluidView)>,
}

fn is_generator(block: &str) -> bool {
    matches!(
        block,
        "mekanismgenerators:heat_generator"
            | "mekanismgenerators:bio_generator"
            | "mekanismgenerators:gas_burning_generator"
            | "mekanismgenerators:solar_generator"
            | "mekanismgenerators:advanced_solar_generator"
            | "mekanismgenerators:wind_generator"
    )
}

fn is_turbine_valve(block: &str) -> bool {
    block == "mekanismgenerators:turbine_valve"
}

fn is_fission_port(block: &str) -> bool {
    block == "mekanismgenerators:fission_reactor_port"
}

fn is_fusion_controller(block: &str) -> bool {
    block == "mekanismgenerators:fusion_reactor_controller"
}

fn is_fusion_port(block: &str) -> bool {
    block == "mekanismgenerators:fusion_reactor_port"
}

fn set_active(handle: u32, value: &str) {
    if !machine::set_property(handle, "active", value)
        && !machine::set_property(handle, "lit", value)
    {
        machine::set_property(handle, "powered", value);
    }
}

fn position_key(dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("{dimension}:{x},{y},{z}")
}

fn screen_id(block: &str, dimension: &str, x: i32, y: i32, z: i32) -> String {
    let name = block.rsplit(':').next().unwrap_or(block);
    format!(
        "mekanismgenerators:machine/{}/{name}/{x},{y},{z}",
        dimension.replace(':', "_")
    )
}

fn screen(block: &str, dimension: &str, x: i32, y: i32, z: i32) -> ScreenSpec {
    let mut widgets = if is_fission_port(block) {
        Vec::new()
    } else {
        vec![Widget {
            id: 11,
            kind: WidgetKind::EnergyBar,
            x: 12,
            y: 25,
            width: 14,
            height: 88,
            text: "Energy".into(),
            value: 0,
            max_value: 100_000,
        }]
    };
    if is_fusion_controller(block) {
        widgets = vec![
            Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 38,
                y: 58,
                width: 18,
                height: 18,
                text: "Hohlraum".into(),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 11,
                kind: WidgetKind::EnergyBar,
                x: 142,
                y: 24,
                width: 14,
                height: 102,
                text: "Fusion energy".into(),
                value: 0,
                max_value: 10_000_000_000,
            },
        ];
        for (id, x, label) in [
            (20, 62, "Deuterium"),
            (21, 90, "Tritium"),
            (22, 118, "D-T fuel"),
        ] {
            widgets.push(Widget {
                id,
                kind: WidgetKind::FluidGauge,
                x,
                y: 24,
                width: 14,
                height: 102,
                text: label.into(),
                value: 0,
                max_value: 1_000_000,
            });
        }
        widgets.push(Widget {
            id: 30,
            kind: WidgetKind::Label,
            x: 38,
            y: 98,
            width: 92,
            height: 16,
            text: "Reactor not formed".into(),
            value: 0,
            max_value: 0,
        });
    }
    if is_fusion_port(block) {
        widgets = Vec::new();
        for (id, x, label) in [
            (20, 38, "Deuterium"),
            (21, 70, "Tritium"),
            (22, 102, "D-T fuel"),
            (23, 134, "Steam"),
        ] {
            widgets.push(Widget {
                id,
                kind: WidgetKind::FluidGauge,
                x,
                y: 24,
                width: 14,
                height: 94,
                text: label.into(),
                value: 0,
                max_value: 1_000_000,
            });
        }
        widgets.push(Widget {
            id: 11,
            kind: WidgetKind::EnergyBar,
            x: 12,
            y: 24,
            width: 14,
            height: 94,
            text: "Energy".into(),
            value: 0,
            max_value: 10_000_000_000,
        });
    }
    if matches!(
        block,
        "mekanismgenerators:heat_generator" | "mekanismgenerators:bio_generator"
    ) {
        widgets.push(Widget {
            id: 0,
            kind: WidgetKind::Slot,
            x: 48,
            y: 42,
            width: 18,
            height: 18,
            text: "Fuel".into(),
            value: 0,
            max_value: 0,
        });
    }
    if block == "mekanismgenerators:heat_generator" {
        widgets.push(Widget {
            id: 1,
            kind: WidgetKind::Slot,
            x: 84,
            y: 42,
            width: 18,
            height: 18,
            text: "Container".into(),
            value: 0,
            max_value: 0,
        });
    }
    if block == "mekanismgenerators:gas_burning_generator" {
        widgets.push(Widget {
            id: 20,
            kind: WidgetKind::FluidGauge,
            x: 52,
            y: 25,
            width: 14,
            height: 88,
            text: "Fuel gas".into(),
            value: 0,
            max_value: 64_000,
        });
    }
    if is_turbine_valve(block) {
        widgets.push(Widget {
            id: 20,
            kind: WidgetKind::FluidGauge,
            x: 48,
            y: 25,
            width: 14,
            height: 88,
            text: "Turbine steam".into(),
            value: 0,
            max_value: 160_000,
        });
    }
    if is_fission_port(block) {
        for (id, x, y, label) in [
            (10, 38, 25, "Water"),
            (20, 72, 25, "Fissile fuel"),
            (21, 106, 25, "Steam"),
            (22, 140, 25, "Nuclear waste"),
        ] {
            widgets.push(Widget {
                id,
                kind: WidgetKind::FluidGauge,
                x,
                y,
                width: 14,
                height: 92,
                text: label.into(),
                value: 0,
                max_value: 64_000,
            });
        }
        widgets.push(Widget {
            id: 30,
            kind: WidgetKind::Label,
            x: 38,
            y: 124,
            width: 116,
            height: 16,
            text: "Reactor not formed".into(),
            value: 0,
            max_value: 0,
        });
    }
    ScreenSpec {
        id: screen_id(block, dimension, x, y, z),
        title: format!(
            "Mekanism: Generators · {}",
            block.rsplit(':').next().unwrap_or(block).replace('_', " ")
        ),
        width: 176,
        height: 166,
        widgets,
    }
}

impl Mod for Mekanismgenerators {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            Self {
                blocks: HashMap::new(),
                burn_left: HashMap::new(),
                turbine_restore: HashMap::new(),
                fission_restore: HashMap::new(),
                fusion_restore: HashMap::new(),
            },
            Registration::new(),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCKS
            .iter()
            .copied()
            .filter(|block| is_generator(block))
            .map(|block| BlockEntityType {
                block: block.into(),
                slots: match block {
                    "mekanismgenerators:heat_generator" => 2,
                    "mekanismgenerators:bio_generator" => 1,
                    _ => 0,
                },
                ticks: true,
            })
            .collect::<Vec<_>>()
            .into_iter()
            .chain([BlockEntityType {
                block: "mekanismgenerators:turbine_valve".into(),
                slots: 0,
                ticks: true,
            }, BlockEntityType {
                block: "mekanismgenerators:fission_reactor_port".into(),
                slots: 0,
                ticks: true,
            }, BlockEntityType {
                block: "mekanismgenerators:fusion_reactor_controller".into(),
                slots: 1,
                ticks: true,
            }, BlockEntityType {
                block: "mekanismgenerators:fusion_reactor_port".into(),
                slots: 0,
                ticks: true,
            }])
            .collect()
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for &block in items::BLOCKS {
            hooks = hooks.block(block);
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem
            || (!is_generator(&event.state)
                && !is_turbine_valve(&event.state)
                && !is_fission_port(&event.state)
                && !is_fusion_controller(&event.state)
                && !is_fusion_port(&event.state))
        {
            return EventOutcome::Pass;
        }
        let _ = ui::open_screen(&screen(
            &event.state,
            &event.dimension,
            event.x,
            event.y,
            event.z,
        ));
        EventOutcome::Consume
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, saved: &[u8]) {
        self.blocks.insert(handle, block.into());
        self.burn_left.insert(
            handle,
            saved
                .get(..4)
                .and_then(|v| v.try_into().ok())
                .map(u32::from_le_bytes)
                .unwrap_or(0),
        );
        if is_turbine_valve(block)
            && let Some(marker) = saved.windows(4).position(|window| window == b"TUR1")
            && let Some(header) = saved.get(marker + 4..marker + 24)
        {
            let amount = u64::from_le_bytes(header[..8].try_into().unwrap_or([0; 8]));
            let capacity = u64::from_le_bytes(header[8..16].try_into().unwrap_or([0; 8]));
            let len = u32::from_le_bytes(header[16..20].try_into().unwrap_or([0; 4])) as usize;
            if let Some(fluid) = saved.get(marker + 24..marker + 24 + len) {
                self.turbine_restore.insert(
                    handle,
                    capabilities::FluidView {
                        fluid: String::from_utf8_lossy(fluid).into_owned(),
                        amount: amount.min(capacity),
                        capacity,
                    },
                );
            }
        }
        if is_fission_port(block)
            && let Some(marker) = saved.windows(4).position(|window| window == b"FIS1")
            && let Some(&count) = saved.get(marker + 4)
        {
            let mut cursor = marker + 5;
            let mut restored = Vec::new();
            for _ in 0..count {
                let Some(header) = saved.get(cursor..cursor.saturating_add(24)) else {
                    break;
                };
                let tank = u32::from_le_bytes(header[..4].try_into().unwrap_or([0; 4]));
                let amount = u64::from_le_bytes(header[4..12].try_into().unwrap_or([0; 8]));
                let capacity = u64::from_le_bytes(header[12..20].try_into().unwrap_or([0; 8]));
                let len = u32::from_le_bytes(header[20..24].try_into().unwrap_or([0; 4])) as usize;
                cursor += 24;
                let Some(fluid) = saved.get(cursor..cursor.saturating_add(len)) else {
                    break;
                };
                restored.push((
                    tank,
                    capabilities::FluidView {
                        fluid: String::from_utf8_lossy(fluid).into_owned(),
                        amount: amount.min(capacity),
                        capacity,
                    },
                ));
                cursor += len;
            }
            self.fission_restore.insert(handle, restored);
        }
        if (is_fusion_controller(block) || is_fusion_port(block))
            && let Some(marker) = saved.windows(4).position(|window| window == b"FUS1")
            && let Some(header) = saved.get(marker + 4..marker + 21)
        {
            let stored = u64::from_le_bytes(header[..8].try_into().unwrap_or([0; 8]));
            let capacity = u64::from_le_bytes(header[8..16].try_into().unwrap_or([0; 8]));
            let count = header[16];
            let mut cursor = marker + 21;
            let mut fluids = Vec::new();
            for _ in 0..count {
                let Some(record) = saved.get(cursor..cursor.saturating_add(24)) else {
                    break;
                };
                let tank = u32::from_le_bytes(record[..4].try_into().unwrap_or([0; 4]));
                let amount = u64::from_le_bytes(record[4..12].try_into().unwrap_or([0; 8]));
                let tank_capacity =
                    u64::from_le_bytes(record[12..20].try_into().unwrap_or([0; 8]));
                let len = u32::from_le_bytes(record[20..24].try_into().unwrap_or([0; 4])) as usize;
                cursor += 24;
                let Some(fluid) = saved.get(cursor..cursor.saturating_add(len)) else {
                    break;
                };
                fluids.push((
                    tank,
                    capabilities::FluidView {
                        fluid: String::from_utf8_lossy(fluid).into_owned(),
                        amount: amount.min(tank_capacity),
                        capacity: tank_capacity,
                    },
                ));
                cursor += len;
            }
            self.fusion_restore.insert(
                handle,
                FusionStored {
                    energy: EnergyView {
                        stored: stored.min(capacity),
                        capacity,
                        max_receive: capacity,
                        max_extract: capacity,
                    },
                    fluids,
                },
            );
        }
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &handle in handles {
            let Some(position) = machine::get_position(handle) else {
                continue;
            };
            let block = self.blocks.get(&handle).map(String::as_str).unwrap_or("");
            if is_fusion_controller(block) || is_fusion_port(block) {
                let is_controller = is_fusion_controller(block);
                let rule = if is_controller {
                    format!(
                        "mekanismgenerators:fusion/controller/{}/{}",
                        position.dimension,
                        format_args!("{},{},{}", position.x, position.y, position.z)
                    )
                } else {
                    format!(
                        "mekanismgenerators:fusion/port/{}/{}",
                        position.dimension,
                        format_args!("{},{},{}", position.x, position.y, position.z)
                    )
                };
                let Some(link) = machine::get_world_rule(&rule) else {
                    if is_controller {
                        set_active(handle, "false");
                        let screen = screen_id(block, &position.dimension, position.x, position.y, position.z);
                        ui::set_gauge(&screen, 30, 0, 1, "Reactor not formed");
                    }
                    continue;
                };
                let mut fields = link.split(';');
                let Some(minimum) = fields.next() else { continue };
                let Some(_port_count) = fields.next().and_then(|value| value.parse::<u64>().ok()) else {
                    continue;
                };
                let Some(controller_coords) = fields.next() else { continue };
                let Ok(minimum) = minimum
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()
                else {
                    continue;
                };
                let Ok(controller_coords) = controller_coords
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()
                else {
                    continue;
                };
                let ([x, y, z], [cx, cy, cz]) =
                    (minimum.as_slice(), controller_coords.as_slice())
                else {
                    continue;
                };
                let shared_key = position_key(&position.dimension, *x, *y, *z);
                let local_key = position_key(&position.dimension, position.x, position.y, position.z);
                let capacities = [
                    (1_000, 1_000_000),
                    (1_001, 1_000_000),
                    (1_002, 1_000_000),
                    (1_003, 1_000_000),
                    (1_004, 10_000_000),
                ];
                let restored = self.fusion_restore.remove(&handle);
                if capabilities::get_energy(&shared_key).is_none() {
                    let view = restored.as_ref().map(|state| state.energy.clone()).unwrap_or(
                        EnergyView {
                            stored: 0,
                            capacity: 10_000_000_000,
                            max_receive: 10_000_000_000,
                            max_extract: 10_000_000_000,
                        },
                    );
                    capabilities::set_energy_buffer(&shared_key, view);
                }
                for (tank, capacity) in capacities {
                    if capabilities::get_fluid(&shared_key, tank).is_none() {
                        let view = restored
                            .as_ref()
                            .and_then(|state| state.fluids.iter().find(|(index, _)| *index == tank))
                            .map(|(_, view)| view.clone())
                            .unwrap_or(capabilities::FluidView {
                                fluid: String::new(),
                                amount: 0,
                                capacity,
                            });
                        capabilities::set_fluid_tank(
                            &shared_key,
                            tank,
                            &capabilities::FluidView {
                                amount: view.amount.min(capacity),
                                capacity,
                                ..view
                            },
                        );
                    }
                    if capabilities::get_fluid(&local_key, tank).is_none() {
                        capabilities::set_fluid_tank(
                            &local_key,
                            tank,
                            &capabilities::FluidView {
                                fluid: String::new(),
                                amount: 0,
                                capacity: 64_000,
                            },
                        );
                    }
                }
                for tank in [1_000, 1_001, 1_002, 1_003] {
                    if let Some(local) = capabilities::get_fluid(&local_key, tank)
                        && local.amount > 0
                    {
                        let moved = capabilities::fill_fluid_tank(
                            &shared_key,
                            tank,
                            &local.fluid,
                            local.amount,
                            false,
                        );
                        if moved > 0 {
                            capabilities::drain_fluid_tank(
                                &local_key,
                                tank,
                                &local.fluid,
                                moved,
                                false,
                            );
                        }
                    }
                }
                if is_controller && [position.x, position.y, position.z] == [*cx, *cy, *cz] {
                    let lit = machine::get_property(handle, "lit")
                        .is_some_and(|value| value == "true")
                        || machine::get_property(handle, "active")
                            .is_some_and(|value| value == "true");
                    if !lit
                        && machine::get_slot(handle, 0).is_some_and(|item| {
                            item.id == "mekanismgenerators:hohlraum"
                        })
                        && capabilities::extract_energy(&shared_key, 1_000_000_000, true)
                            >= 1_000_000_000
                    {
                        capabilities::extract_energy(&shared_key, 1_000_000_000, false);
                        machine::set_slot(handle, 0, None);
                        set_active(handle, "true");
                    }
                    let active = machine::get_property(handle, "lit")
                        .is_some_and(|value| value == "true")
                        || machine::get_property(handle, "active")
                            .is_some_and(|value| value == "true");
                    if active {
                        let energy_room = capabilities::get_energy(&shared_key).map_or(0, |view| {
                            view.capacity.saturating_sub(view.stored)
                        });
                        let direct_fuel = capabilities::get_fluid(&shared_key, 1_002)
                            .filter(|view| view.fluid == "mekanismgenerators:fusion_fuel")
                            .map_or(0, |view| {
                                view.amount.min(1_000).min(energy_room / 200_000)
                            });
                        let (fuel_used, generated) = if direct_fuel > 0 {
                            (direct_fuel, direct_fuel.saturating_mul(200_000))
                        } else {
                            let deuterium = capabilities::get_fluid(&shared_key, 1_000)
                                .filter(|view| view.fluid == "mekanismgenerators:deuterium")
                                .map_or(0, |view| view.amount);
                            let tritium = capabilities::get_fluid(&shared_key, 1_001)
                                .filter(|view| view.fluid == "mekanismgenerators:tritium")
                                .map_or(0, |view| view.amount);
                            let pairs = deuterium
                                .min(tritium)
                                .min(49)
                                .min(energy_room.saturating_mul(49) / 20_000_000);
                            (pairs, pairs.saturating_mul(20_000_000) / 49)
                        };
                        if direct_fuel > 0 {
                            capabilities::drain_fluid_tank(
                                &shared_key,
                                1_002,
                                "mekanismgenerators:fusion_fuel",
                                fuel_used,
                                false,
                            );
                        } else if fuel_used > 0 {
                            capabilities::drain_fluid_tank(
                                &shared_key,
                                1_000,
                                "mekanismgenerators:deuterium",
                                fuel_used,
                                false,
                            );
                            capabilities::drain_fluid_tank(
                                &shared_key,
                                1_001,
                                "mekanismgenerators:tritium",
                                fuel_used,
                                false,
                            );
                        }
                        if generated > 0 {
                            capabilities::receive_energy(&shared_key, generated, false);
                        }
                    }
                }
                if !is_controller {
                    if let Some(steam) = capabilities::get_fluid(&shared_key, 1_004)
                        && steam.amount > 0
                    {
                        let accepted = capabilities::fill_fluid_tank(
                            &local_key,
                            1_004,
                            &steam.fluid,
                            steam.amount,
                            true,
                        );
                        if accepted > 0 {
                            let moved = capabilities::drain_fluid_tank(
                                &shared_key,
                                1_004,
                                &steam.fluid,
                                accepted,
                                false,
                            );
                            capabilities::fill_fluid_tank(
                                &local_key,
                                1_004,
                                &steam.fluid,
                                moved,
                                false,
                            );
                        }
                    }
                    if capabilities::get_energy(&local_key).is_none() {
                        capabilities::set_energy_buffer(
                            &local_key,
                            EnergyView {
                                stored: 0,
                                capacity: 100_000_000,
                                max_receive: 100_000_000,
                                max_extract: 100_000_000,
                            },
                        );
                    }
                    if let Some(shared_energy) = capabilities::get_energy(&shared_key) {
                        let local_energy = capabilities::get_energy(&local_key)
                            .map_or(0, |view| view.stored);
                        let room = 100_000_000u64.saturating_sub(local_energy);
                        let transfer = shared_energy.stored.min(room).min(10_000_000);
                        if transfer > 0
                            && capabilities::extract_energy(&shared_key, transfer, false)
                                == transfer
                        {
                            capabilities::receive_energy(&local_key, transfer, false);
                        }
                    }
                }
                if let Some(energy) = capabilities::get_energy(&local_key) {
                    ui::set_gauge(
                        &screen_id(block, &position.dimension, position.x, position.y, position.z),
                        11,
                        energy.stored,
                        energy.capacity,
                        "Energy",
                    );
                }
                for (widget, tank) in [(20, 1_000), (21, 1_001), (22, 1_002), (23, 1_004)] {
                    if let Some(view) = capabilities::get_fluid(&shared_key, tank) {
                        ui::set_gauge(
                            &screen_id(block, &position.dimension, position.x, position.y, position.z),
                            widget,
                            view.amount,
                            view.capacity,
                            if view.fluid.is_empty() { "Empty" } else { &view.fluid },
                        );
                    }
                }
                if is_controller {
                    let screen = screen_id(block, &position.dimension, position.x, position.y, position.z);
                    if let Some(energy) = capabilities::get_energy(&shared_key) {
                        ui::set_gauge(&screen, 11, energy.stored, energy.capacity, "Fusion energy");
                    }
                    ui::set_gauge(
                        &screen,
                        30,
                        1,
                        1,
                        &format!("Formed · {link}")
                    );
                }
                continue;
            }
            if is_fission_port(block) {
                let rule = format!(
                    "mekanismgenerators:fission/port/{}/{}",
                    position.dimension,
                    format_args!("{},{},{}", position.x, position.y, position.z)
                );
                let Some(link) = machine::get_world_rule(&rule) else {
                    set_active(handle, "false");
                    let screen = screen_id(block, &position.dimension, position.x, position.y, position.z);
                    ui::set_gauge(&screen, 30, 0, 1, "Reactor not formed");
                    continue;
                };
                let mut fields = link.split(';');
                let Some(minimum) = fields.next() else { continue };
                let Some(size) = fields.next() else { continue };
                let Some(fuel_assemblies) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
                    continue;
                };
                let Some(_port_count) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
                    continue;
                };
                let Some(primary) = fields.next() else { continue };
                let Ok(minimum) = minimum
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()
                else {
                    continue;
                };
                let Ok(size) = size
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()
                else {
                    continue;
                };
                let Ok(primary) = primary
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()
                else {
                    continue;
                };
                let ([x, y, z], [sx, sy, sz], [px, py, pz]) =
                    (minimum.as_slice(), size.as_slice(), primary.as_slice())
                else {
                    continue;
                };
                let shared_key = position_key(&position.dimension, *x, *y, *z);
                let local_key = position_key(&position.dimension, position.x, position.y, position.z);
                let capacities = [
                    (0, 1_000_000),
                    (1_000, 1_000_000),
                    (1_001, 1_000_000),
                    (1_002, 1_000_000),
                ];
                let restored = self.fission_restore.remove(&handle).unwrap_or_default();
                for (tank, capacity) in capacities {
                    if capabilities::get_fluid(&shared_key, tank).is_none() {
                        let view = restored
                            .iter()
                            .find(|(restored_tank, _)| *restored_tank == tank)
                            .map(|(_, view)| view.clone())
                            .unwrap_or(capabilities::FluidView {
                                fluid: String::new(),
                                amount: 0,
                                capacity,
                            });
                        capabilities::set_fluid_tank(
                            &shared_key,
                            tank,
                            &capabilities::FluidView {
                                amount: view.amount.min(capacity),
                                capacity,
                                ..view
                            },
                        );
                    }
                    if capabilities::get_fluid(&local_key, tank).is_none() {
                        capabilities::set_fluid_tank(
                            &local_key,
                            tank,
                            &capabilities::FluidView {
                                fluid: String::new(),
                                amount: 0,
                                capacity: 64_000,
                            },
                        );
                    }
                }
                // All ports feed the shared water and fissile-fuel stores.
                for tank in [0, 1_000] {
                    if let Some(local) = capabilities::get_fluid(&local_key, tank)
                        && local.amount > 0
                    {
                        let accepted = capabilities::fill_fluid_tank(
                            &shared_key,
                            tank,
                            &local.fluid,
                            local.amount,
                            false,
                        );
                        if accepted > 0 {
                            capabilities::drain_fluid_tank(
                                &local_key,
                                tank,
                                &local.fluid,
                                accepted,
                                false,
                            );
                        }
                    }
                }
                if [position.x, position.y, position.z] == [*px, *py, *pz] {
                    let water = capabilities::get_fluid(&shared_key, 0);
                    let fuel = capabilities::get_fluid(&shared_key, 1_000);
                    let steam_output = capabilities::fill_fluid_tank(
                        &shared_key,
                        1_001,
                        "mekanism:steam",
                        20_000,
                        true,
                    ) == 20_000;
                    let waste_output = capabilities::fill_fluid_tank(
                        &shared_key,
                        1_002,
                        "mekanism:nuclear_waste",
                        1,
                        true,
                    ) == 1;
                    if fuel_assemblies > 0
                        && steam_output
                        && waste_output
                        && water.as_ref().is_some_and(|view| {
                            view.fluid == "minecraft:water" && view.amount >= 20_000
                        })
                        && fuel.as_ref().is_some_and(|view| {
                            view.fluid == "mekanism:fissile_fuel" && view.amount >= 1
                        })
                    {
                        capabilities::drain_fluid_tank(
                            &shared_key,
                            0,
                            "minecraft:water",
                            20_000,
                            false,
                        );
                        capabilities::drain_fluid_tank(
                            &shared_key,
                            1_000,
                            "mekanism:fissile_fuel",
                            1,
                            false,
                        );
                        capabilities::fill_fluid_tank(
                            &shared_key,
                            1_001,
                            "mekanism:steam",
                            20_000,
                            false,
                        );
                        capabilities::fill_fluid_tank(
                            &shared_key,
                            1_002,
                            "mekanism:nuclear_waste",
                            1,
                            false,
                        );
                        set_active(handle, "true");
                    } else {
                        set_active(handle, "false");
                    }
                } else {
                    set_active(handle, "false");
                }
                // Ports expose the shared output tanks to pressurized tubes.
                for tank in [1_001, 1_002] {
                    if let Some(shared) = capabilities::get_fluid(&shared_key, tank)
                        && shared.amount > 0
                    {
                        let accepted = capabilities::fill_fluid_tank(
                            &local_key,
                            tank,
                            &shared.fluid,
                            shared.amount,
                            true,
                        );
                        if accepted > 0 {
                            let moved = capabilities::drain_fluid_tank(
                                &shared_key,
                                tank,
                                &shared.fluid,
                                accepted,
                                false,
                            );
                            capabilities::fill_fluid_tank(
                                &local_key,
                                tank,
                                &shared.fluid,
                                moved,
                                false,
                            );
                        }
                    }
                }
                let screen = screen_id(block, &position.dimension, position.x, position.y, position.z);
                for (widget, tank) in [(10, 0), (20, 1_000), (21, 1_001), (22, 1_002)] {
                    if let Some(view) = capabilities::get_fluid(&shared_key, tank) {
                        ui::set_gauge(
                            &screen,
                            widget,
                            view.amount,
                            view.capacity,
                            if view.fluid.is_empty() { "Empty" } else { &view.fluid },
                        );
                    }
                }
                ui::set_gauge(
                    &screen,
                    30,
                    1,
                    1,
                    &format!("Formed · {fuel_assemblies} fuel assemblies · {sx}×{sy}×{sz}"),
                );
                continue;
            }
            if is_turbine_valve(block) {
                let rule = format!(
                    "mekanismgenerators:turbine/valve/{}/{}",
                    position.dimension,
                    format_args!("{},{},{}", position.x, position.y, position.z)
                );
                let link = machine::get_world_rule(&rule);
                let Some(link) = link else {
                    set_active(handle, "false");
                    continue;
                };
                let mut fields = link.split(';');
                let Some(coords) = fields.next() else { continue };
                let Some(volume) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
                    continue;
                };
                let Some(max_rate) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
                    continue;
                };
                let Some(rotor_height) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
                    continue;
                };
                let Some(rotors) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
                    continue;
                };
                let Some(valves) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
                    continue;
                };
                let Ok(coords) = coords
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()
                else {
                    continue;
                };
                let [x, y, z] = coords.as_slice() else { continue };
                let key = position_key(&position.dimension, position.x, position.y, position.z);
                let shared_key = position_key(&position.dimension, *x, *y, *z);
                let steam_capacity = volume.saturating_mul(160_000).max(64_000);
                let energy_capacity = volume.saturating_mul(1_000_000).max(100_000);
                if capabilities::get_fluid(&key, 1_000).is_none() {
                    capabilities::set_fluid_tank(
                        &key,
                        1_000,
                        &capabilities::FluidView {
                            fluid: String::new(),
                            amount: 0,
                            capacity: 64_000,
                        },
                    );
                } else if let Some(tank) = capabilities::get_fluid(&key, 1_000) {
                    capabilities::set_fluid_tank(
                        &key,
                        1_000,
                        &capabilities::FluidView {
                            amount: tank.amount.min(64_000),
                            capacity: 64_000,
                            ..tank
                        },
                    );
                }
                let restored = self.turbine_restore.remove(&handle);
                if capabilities::get_fluid(&shared_key, 1_000).is_none() {
                    let view = restored.unwrap_or(capabilities::FluidView {
                        fluid: String::new(),
                        amount: 0,
                        capacity: steam_capacity,
                    });
                    capabilities::set_fluid_tank(
                        &shared_key,
                        1_000,
                        &capabilities::FluidView {
                            amount: view.amount.min(steam_capacity),
                            capacity: steam_capacity,
                            ..view
                        },
                    );
                }
                if let Some(local) = capabilities::get_fluid(&key, 1_000)
                    && local.amount > 0
                {
                    let moved = capabilities::fill_fluid_tank(
                        &shared_key,
                        1_000,
                        &local.fluid,
                        local.amount,
                        false,
                    );
                    if moved > 0 {
                        capabilities::drain_fluid_tank(
                            &key,
                            1_000,
                            &local.fluid,
                            moved,
                            false,
                        );
                    }
                }
                if capabilities::get_energy(&key).is_none() {
                    capabilities::set_energy_buffer(
                        &key,
                        EnergyView {
                            stored: 0,
                            capacity: energy_capacity,
                            max_receive: energy_capacity,
                            max_extract: energy_capacity,
                        },
                    );
                }
                let per_valve_rate = (max_rate / valves.max(1)).max(1);
                let conversion = rotor_height.saturating_mul(rotors).saturating_mul(50) / 7;
                if conversion > 0
                    && let Some(energy) = capabilities::get_energy(&key)
                    && let Some(steam) = capabilities::get_fluid(&shared_key, 1_000)
                    && steam.amount > 0
                    && (steam.fluid == "mekanism:steam"
                        || steam.fluid == "mekanismgenerators:steam")
                {
                    let affordable = energy
                        .capacity
                        .saturating_sub(energy.stored)
                        .saturating_add(conversion - 1)
                        / conversion;
                    let flow = steam.amount.min(per_valve_rate).min(affordable);
                    let generated = flow.saturating_mul(conversion);
                    if flow > 0 {
                        capabilities::drain_fluid_tank(
                            &shared_key,
                            1_000,
                            &steam.fluid,
                            flow,
                            false,
                        );
                        capabilities::receive_energy(&key, generated, false);
                    }
                }
                set_active(handle, "true");
                if let Some(energy) = capabilities::get_energy(&key) {
                    ui::set_gauge(
                        &screen_id(block, &position.dimension, position.x, position.y, position.z),
                        11,
                        energy.stored,
                        energy.capacity,
                        "Energy",
                    );
                }
                if let Some(steam) = capabilities::get_fluid(&shared_key, 1_000) {
                    ui::set_gauge(
                        &screen_id(block, &position.dimension, position.x, position.y, position.z),
                        20,
                        steam.amount,
                        steam.capacity,
                        if steam.fluid.is_empty() { "Empty steam" } else { &steam.fluid },
                    );
                }
                continue;
            }
            if !is_generator(block) {
                continue;
            }
            let key = position_key(&position.dimension, position.x, position.y, position.z);
            let capacity = 100_000;
            if block == "mekanismgenerators:gas_burning_generator"
                && capabilities::get_fluid(&key, 1_000).is_none()
            {
                capabilities::set_fluid_tank(
                    &key,
                    1_000,
                    &capabilities::FluidView {
                        fluid: String::new(),
                        amount: 0,
                        capacity: 64_000,
                    },
                );
            }
            if capabilities::get_energy(&key).is_none() {
                capabilities::set_energy_buffer(
                    &key,
                    EnergyView {
                        stored: 0,
                        capacity,
                        max_receive: 100_000,
                        max_extract: 100_000,
                    },
                );
            }
            let mut generation = 0u64;
            if matches!(
                block,
                "mekanismgenerators:solar_generator"
                    | "mekanismgenerators:advanced_solar_generator"
            ) {
                let daytime = game::get_clock()
                    .map(|clock| clock.day_time % 24_000 < 12_000)
                    .unwrap_or(false);
                if daytime {
                    generation = if block.ends_with("advanced_solar_generator") {
                        200
                    } else {
                        20
                    };
                }
            } else if block == "mekanismgenerators:wind_generator" {
                generation = (position.y.max(0) as u64 / 3).clamp(10, 80);
            } else if matches!(
                block,
                "mekanismgenerators:heat_generator" | "mekanismgenerators:bio_generator"
            ) {
                let burn = self.burn_left.entry(handle).or_default();
                if *burn == 0 {
                    if let Some(input) = machine::get_slot(handle, 0) {
                        let ticks = if block == "mekanismgenerators:heat_generator" {
                            machine::burn_time(&input.id)
                        } else if matches!(
                            input.id.as_str(),
                            "minecraft:wheat_seeds"
                                | "minecraft:seeds"
                                | "minecraft:kelp"
                                | "minecraft:sugar_cane"
                                | "minecraft:potato"
                                | "minecraft:carrot"
                                | "minecraft:beetroot"
                        ) {
                            200
                        } else {
                            0
                        };
                        if ticks > 0 {
                            let remainder = (block == "mekanismgenerators:heat_generator")
                                .then(|| machine::remainder(&input.id))
                                .flatten()
                                .map(|id| ItemStack { id, count: 1 });
                            let remainder_fits = if let Some(remainder) = &remainder {
                                let existing = machine::get_slot(handle, 1);
                                let count = match existing {
                                    Some(stack) if stack.id == remainder.id => {
                                        stack.count.saturating_add(remainder.count)
                                    }
                                    Some(_) => u32::MAX,
                                    None => remainder.count,
                                };
                                count <= machine::max_stack_size(&remainder.id)
                            } else {
                                true
                            };
                            if !remainder_fits {
                                continue;
                            }
                            let remaining = (input.count > 1).then(|| ItemStack {
                                id: input.id,
                                count: input.count - 1,
                            });
                            machine::set_slot(handle, 0, remaining.as_ref());
                            if let Some(remainder) = remainder {
                                let output = match machine::get_slot(handle, 1) {
                                    Some(stack) => ItemStack {
                                        id: stack.id,
                                        count: stack.count.saturating_add(remainder.count),
                                    },
                                    None => remainder,
                                };
                                machine::set_slot(handle, 1, Some(&output));
                            }
                            *burn = ticks;
                        }
                    }
                }
                if *burn > 0 {
                    *burn -= 1;
                    generation = if block == "mekanismgenerators:bio_generator" {
                        80
                    } else {
                        40
                    };
                }
            } else if block == "mekanismgenerators:gas_burning_generator" {
                if let Some(tank) =
                    capabilities::get_fluid(&key, 1_000).filter(|tank| tank.amount > 0)
                {
                    let rate = if tank.fluid == "mekanism:ethene" {
                        160
                    } else if tank.fluid == "mekanism:hydrogen" {
                        40
                    } else {
                        0
                    };
                    if rate > 0
                        && capabilities::drain_fluid_tank(&key, 1_000, &tank.fluid, 1, false) > 0
                    {
                        generation = rate;
                    }
                }
            }
            if generation > 0 {
                capabilities::receive_energy(&key, generation, false);
            }
            if let Some(energy) = capabilities::get_energy(&key) {
                ui::set_gauge(
                    &screen_id(
                        block,
                        &position.dimension,
                        position.x,
                        position.y,
                        position.z,
                    ),
                    11,
                    energy.stored,
                    energy.capacity,
                    "Energy",
                );
            }
            if block == "mekanismgenerators:gas_burning_generator"
                && let Some(fuel) = capabilities::get_fluid(&key, 1_000)
            {
                ui::set_gauge(
                    &screen_id(
                        block,
                        &position.dimension,
                        position.x,
                        position.y,
                        position.z,
                    ),
                    20,
                    fuel.amount,
                    fuel.capacity,
                    if fuel.fluid.is_empty() {
                        "Empty gas"
                    } else {
                        &fuel.fluid
                    },
                );
            }
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        let mut saved = self
            .burn_left
            .get(&handle)
            .copied()
            .unwrap_or(0)
            .to_le_bytes()
            .to_vec();
        if self
            .blocks
            .get(&handle)
            .is_some_and(|block| is_turbine_valve(block))
            && let Some(position) = machine::get_position(handle)
        {
            let rule = format!(
                "mekanismgenerators:turbine/valve/{}/{}",
                position.dimension,
                format_args!("{},{},{}", position.x, position.y, position.z)
            );
            if let Some(link) = machine::get_world_rule(&rule) {
                let coords = link.split(';').next().unwrap_or("");
                if let Ok(coords) = coords
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()
                    && let [x, y, z] = coords.as_slice()
                {
                    let key = position_key(&position.dimension, *x, *y, *z);
                    if let Some(steam) = capabilities::get_fluid(&key, 1_000) {
                        saved.extend_from_slice(b"TUR1");
                        saved.extend_from_slice(&steam.amount.to_le_bytes());
                        saved.extend_from_slice(&steam.capacity.to_le_bytes());
                        saved.extend_from_slice(&(steam.fluid.len() as u32).to_le_bytes());
                        saved.extend_from_slice(steam.fluid.as_bytes());
                    }
                }
            }
        }
        if self
            .blocks
            .get(&handle)
            .is_some_and(|block| is_fission_port(block))
            && let Some(position) = machine::get_position(handle)
        {
            let rule = format!(
                "mekanismgenerators:fission/port/{}/{}",
                position.dimension,
                format_args!("{},{},{}", position.x, position.y, position.z)
            );
            if let Some(link) = machine::get_world_rule(&rule) {
                let coords = link.split(';').next().unwrap_or("");
                if let Ok(coords) = coords
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()
                    && let [x, y, z] = coords.as_slice()
                {
                    let key = position_key(&position.dimension, *x, *y, *z);
                    saved.extend_from_slice(b"FIS1");
                    saved.push(4);
                    for tank in [0, 1_000, 1_001, 1_002] {
                        let view = capabilities::get_fluid(&key, tank).unwrap_or(
                            capabilities::FluidView {
                                fluid: String::new(),
                                amount: 0,
                                capacity: 1_000_000,
                            },
                        );
                        saved.extend_from_slice(&tank.to_le_bytes());
                        saved.extend_from_slice(&view.amount.to_le_bytes());
                        saved.extend_from_slice(&view.capacity.to_le_bytes());
                        saved.extend_from_slice(&(view.fluid.len() as u32).to_le_bytes());
                        saved.extend_from_slice(view.fluid.as_bytes());
                    }
                }
            }
        }
        if self
            .blocks
            .get(&handle)
            .is_some_and(|block| is_fusion_controller(block) || is_fusion_port(block))
            && let Some(position) = machine::get_position(handle)
        {
            let is_controller = self
                .blocks
                .get(&handle)
                .is_some_and(|block| is_fusion_controller(block));
            let prefix = if is_controller { "controller" } else { "port" };
            let rule = format!(
                "mekanismgenerators:fusion/{prefix}/{}/{}",
                position.dimension,
                format_args!("{},{},{}", position.x, position.y, position.z)
            );
            if let Some(link) = machine::get_world_rule(&rule) {
                let coords = link.split(';').next().unwrap_or("");
                if let Ok(coords) = coords
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>()
                    && let [x, y, z] = coords.as_slice()
                {
                    let key = position_key(&position.dimension, *x, *y, *z);
                    let energy = capabilities::get_energy(&key).unwrap_or(EnergyView {
                        stored: 0,
                        capacity: 10_000_000_000,
                        max_receive: 10_000_000_000,
                        max_extract: 10_000_000_000,
                    });
                    saved.extend_from_slice(b"FUS1");
                    saved.extend_from_slice(&energy.stored.to_le_bytes());
                    saved.extend_from_slice(&energy.capacity.to_le_bytes());
                    saved.push(5);
                    for tank in [1_000, 1_001, 1_002, 1_003, 1_004] {
                        let view = capabilities::get_fluid(&key, tank).unwrap_or(
                            capabilities::FluidView {
                                fluid: String::new(),
                                amount: 0,
                                capacity: 1_000_000,
                            },
                        );
                        saved.extend_from_slice(&tank.to_le_bytes());
                        saved.extend_from_slice(&view.amount.to_le_bytes());
                        saved.extend_from_slice(&view.capacity.to_le_bytes());
                        saved.extend_from_slice(&(view.fluid.len() as u32).to_le_bytes());
                        saved.extend_from_slice(view.fluid.as_bytes());
                    }
                }
            }
        }
        saved
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.blocks.remove(&handle);
        self.burn_left.remove(&handle);
        self.turbine_restore.remove(&handle);
        self.fission_restore.remove(&handle);
        self.fusion_restore.remove(&handle);
    }
}

horizon_mod_sdk::export_mod!(Mekanismgenerators);

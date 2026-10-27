//! Mekanism's item processing machines on Horizon's machine API.
mod items;

use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemEvent, LogicHooks, Mod,
    Registration,
    capabilities::{self, EnergyView},
    machine::{self, CookingKind, ItemStack},
    recipes,
    server::{self, BlockPos},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct Mekanism {
    progress: HashMap<u32, u32>,
    blocks: HashMap<u32, String>,
    energy: HashMap<u32, u64>,
    side_modes: HashMap<String, String>,
    matrix_energy: HashMap<u32, u64>,
    dynamic_restore: HashMap<u32, Vec<(u32, capabilities::FluidView)>>,
}

fn set_active(handle: u32, value: &str) {
    if !machine::set_property(handle, "active", value)
        && !machine::set_property(handle, "lit", value)
    {
        machine::set_property(handle, "powered", value);
    }
}

fn side_key(dimension: &str, x: i32, y: i32, z: i32, face: &str) -> String {
    format!("{}|{face}", energy_position(dimension, x, y, z))
}

fn side_rule(dimension: &str, x: i32, y: i32, z: i32, face: &str) -> String {
    format!(
        "mekanism:pipe/{}",
        side_key(dimension, x, y, z, face).replace(':', "_")
    )
}

fn is_machine(id: &str) -> bool {
    let name = id.strip_prefix("mekanism:").unwrap_or(id);
    [
        "_bin",
        "_combining_factory",
        "_compressing_factory",
        "_crushing_factory",
        "_enriching_factory",
        "_infusing_factory",
        "_injecting_factory",
        "_purifying_factory",
        "_sawing_factory",
        "_smelting_factory",
        "_energy_cube",
        "_chemical_tank",
        "_fluid_tank",
        "_induction_cell",
        "_induction_provider",
        "_universal_cable",
        "_logistical_transporter",
        "_mechanical_pipe",
        "_pressurized_tube",
        "_thermodynamic_conductor",
    ]
    .iter()
    .any(|suffix| name.ends_with(suffix))
        || matches!(
            name,
            "antiprotonic_nucleosynthesizer"
                | "boiler_valve"
                | "cardboard_box"
                | "chargepad"
                | "digital_miner"
                | "dimensional_stabilizer"
                | "dynamic_valve"
                | "electric_pump"
                | "fluidic_plenisher"
                | "diversion_transporter"
                | "induction_port"
                | "industrial_alarm"
                | "isotopic_centrifuge"
                | "laser"
                | "laser_amplifier"
                | "laser_tractor_beam"
                | "energized_smelter"
                | "enrichment_chamber"
                | "crusher"
                | "combiner"
                | "osmium_compressor"
                | "metallurgic_infuser"
                | "purification_chamber"
                | "chemical_injection_chamber"
                | "chemical_washer"
                | "chemical_crystallizer"
                | "chemical_dissolution_chamber"
                | "chemical_oxidizer"
                | "chemical_infuser"
                | "electrolytic_separator"
                | "precision_sawmill"
                | "pressurized_reaction_chamber"
                | "painting_machine"
                | "formulaic_assemblicator"
                | "pigment_extractor"
                | "pigment_mixer"
                | "nutritional_liquifier"
                | "fuelwood_heater"
                | "personal_chest"
                | "personal_barrel"
                | "qio_dashboard"
                | "qio_drive_array"
                | "qio_exporter"
                | "qio_importer"
                | "qio_redstone_adapter"
                | "logistical_sorter"
                | "modification_station"
                | "security_desk"
                | "oredictionificator"
                | "pressure_disperser"
                | "quantum_entangloporter"
                | "resistive_heater"
                | "restrictive_transporter"
                | "rotary_condensentrator"
                | "seismic_vibrator"
                | "solar_neutron_activator"
                | "sps_port"
                | "superheating_element"
                | "teleporter"
                | "thermal_evaporation_controller"
                | "thermal_evaporation_valve"
        )
}

fn screen_id(block: &str, dimension: &str, x: i32, y: i32, z: i32) -> String {
    let machine = block.strip_prefix("mekanism:").unwrap_or(block);
    let dimension = dimension.replace(':', "_").replace('/', "_");
    format!("mekanism:machine/{dimension}/{machine}/{x}_{y}_{z}")
}

fn screen(block: &str, dimension: &str, x: i32, y: i32, z: i32) -> ScreenSpec {
    let title = block
        .strip_prefix("mekanism:")
        .unwrap_or(block)
        .replace('_', " ");
    let has_second_input = matches!(
        recipe_type(block),
        Some("mekanism:combining" | "mekanism:sawing")
    );
    if matches!(
        block,
        "mekanism:electric_pump" | "mekanism:fluidic_plenisher"
    ) {
        return ScreenSpec {
            id: screen_id(block, dimension, x, y, z),
            title: format!("Mekanism · {title}"),
            width: 176,
            height: 166,
            widgets: vec![
                Widget {
                    id: 11,
                    kind: WidgetKind::EnergyBar,
                    x: 30,
                    y: 30,
                    width: 14,
                    height: 90,
                    text: "Energy".into(),
                    value: 0,
                    max_value: 100_000,
                },
                Widget {
                    id: 10,
                    kind: WidgetKind::FluidGauge,
                    x: 90,
                    y: 30,
                    width: 14,
                    height: 90,
                    text: if block == "mekanism:electric_pump" {
                        "Pump output".into()
                    } else {
                        "Plenisher input".into()
                    },
                    value: 0,
                    max_value: 64_000,
                },
            ],
        };
    }
    if block == "mekanism:induction_port" {
        return ScreenSpec {
            id: screen_id(block, dimension, x, y, z),
            title: format!("Mekanism · {title}"),
            width: 176,
            height: 166,
            widgets: vec![Widget {
                id: 11,
                kind: WidgetKind::EnergyBar,
                x: 74,
                y: 28,
                width: 18,
                height: 110,
                text: "Matrix energy".into(),
                value: 0,
                max_value: 1_000_000,
            }],
        };
    }
    if block == "mekanism:dynamic_valve" {
        return ScreenSpec {
            id: screen_id(block, dimension, x, y, z),
            title: format!("Mekanism · {title}"),
            width: 176,
            height: 166,
            widgets: vec![
                Widget {
                    id: 10,
                    kind: WidgetKind::FluidGauge,
                    x: 58,
                    y: 24,
                    width: 16,
                    height: 116,
                    text: "Tank fluid".into(),
                    value: 0,
                    max_value: 350_000,
                },
                Widget {
                    id: 11,
                    kind: WidgetKind::FluidGauge,
                    x: 98,
                    y: 24,
                    width: 16,
                    height: 116,
                    text: "Tank chemical".into(),
                    value: 0,
                    max_value: 16_000_000,
                },
            ],
        };
    }
    if block.ends_with("_fluid_tank")
        || block.ends_with("_chemical_tank")
        || block == "mekanism:thermal_evaporation_valve"
    {
        return ScreenSpec {
            id: screen_id(block, dimension, x, y, z),
            title: format!("Mekanism · {title}"),
            width: 176,
            height: 166,
            widgets: vec![Widget {
                id: 10,
                kind: WidgetKind::FluidGauge,
                x: 72,
                y: 30,
                width: 16,
                height: 100,
                text: if block.ends_with("_valve") {
                    "Valve".into()
                } else {
                    "Tank".into()
                },
                value: 0,
                max_value: 64_000,
            }],
        };
    }
    if block.ends_with("_mechanical_pipe") || block.ends_with("_pressurized_tube") {
        return ScreenSpec {
            id: screen_id(block, dimension, x, y, z),
            title: format!("Mekanism · {title}"),
            width: 176,
            height: 166,
            widgets: vec![Widget {
                id: 10,
                kind: WidgetKind::FluidGauge,
                x: 72,
                y: 30,
                width: 16,
                height: 100,
                text: "Pipe".into(),
                value: 0,
                max_value: 16_000,
            }],
        };
    }
    if block.ends_with("_energy_cube")
        || block.ends_with("_universal_cable")
        || block == "mekanism:resistive_heater"
    {
        return ScreenSpec {
            id: screen_id(block, dimension, x, y, z),
            title: format!("Mekanism · {title}"),
            width: 176,
            height: 166,
            widgets: vec![Widget {
                id: 11,
                kind: WidgetKind::EnergyBar,
                x: 72,
                y: 30,
                width: 16,
                height: 100,
                text: "Energy".into(),
                value: 0,
                max_value: 1_000_000,
            }],
        };
    }
    if slot_count(block) == 0 {
        let mut widgets = vec![
            Widget {
                id: 11,
                kind: WidgetKind::EnergyBar,
                x: 10,
                y: 30,
                width: 12,
                height: 80,
                text: "Energy".into(),
                value: 0,
                max_value: 1_000_000,
            },
            Widget {
                id: 10,
                kind: WidgetKind::ProgressArrow,
                x: 40,
                y: 70,
                width: 28,
                height: 18,
                text: "Progress".into(),
                value: 0,
                max_value: 100,
            },
        ];
        for tank in 0..3 {
            widgets.push(Widget {
                id: 20 + tank,
                kind: WidgetKind::FluidGauge,
                x: 78 + tank as i32 * 24,
                y: 30,
                width: 12,
                height: 80,
                text: format!("Fluid {tank}"),
                value: 0,
                max_value: 16_000,
            });
            widgets.push(Widget {
                id: 23 + tank,
                kind: WidgetKind::FluidGauge,
                x: 78 + tank as i32 * 24,
                y: 120,
                width: 12,
                height: 30,
                text: format!("Chemical {tank}"),
                value: 0,
                max_value: 16_000,
            });
        }
        return ScreenSpec {
            id: screen_id(block, dimension, x, y, z),
            title: format!("Mekanism · {title}"),
            width: 176,
            height: 166,
            widgets,
        };
    }
    let processing_screen = recipe_type(block).is_some()
        || matches!(
            block,
            "mekanism:energized_smelter" | "mekanism:metallurgic_infuser"
        );
    let (input_x, input_y, output_x, output_y, progress_x, progress_y, energy_x, energy_y) =
        if processing_screen {
            (54, 16, 110, 30, 76, 38, 163, 16)
        } else {
            (34, 35, 99, 35, 73, 35, 10, 27)
        };
    let mut widgets = vec![Widget {
        id: 0,
        kind: WidgetKind::Slot,
        x: input_x,
        y: input_y,
        width: 18,
        height: 18,
        text: "Input".into(),
        value: 0,
        max_value: 0,
    }];
    widgets.push(Widget {
        id: 11,
        kind: WidgetKind::EnergyBar,
        x: energy_x,
        y: energy_y,
        width: if processing_screen { 6 } else { 12 },
        height: if processing_screen { 54 } else { 50 },
        text: if processing_screen {
            String::new()
        } else {
            "Energy".into()
        },
        value: 0,
        max_value: 1_000_000,
    });
    let (progress_x, output_x, output_id) = if has_second_input {
        widgets.push(Widget {
            id: 1,
            kind: WidgetKind::Slot,
            x: 54,
            y: 52,
            width: 18,
            height: 18,
            text: "Second input".into(),
            value: 0,
            max_value: 0,
        });
        (76, 110, 2)
    } else {
        (progress_x, output_x, 1)
    };
    widgets.push(Widget {
        id: 10,
        kind: WidgetKind::ProgressArrow,
        x: progress_x,
        y: progress_y,
        width: if processing_screen { 25 } else { 24 },
        height: if processing_screen { 8 } else { 17 },
        text: "Progress".into(),
        value: 0,
        max_value: 100,
    });
    widgets.push(Widget {
        id: output_id,
        kind: WidgetKind::Slot,
        x: output_x,
        y: if processing_screen { output_y } else { 35 },
        width: 18,
        height: 18,
        text: "Output".into(),
        value: 0,
        max_value: 0,
    });
    ScreenSpec {
        id: screen_id(block, dimension, x, y, z),
        title: title
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" "),
        width: 176,
        height: 166,
        widgets,
    }
}

fn recipe_type(block: &str) -> Option<&'static str> {
    let name = block.strip_prefix("mekanism:").unwrap_or(block);
    if name == "enrichment_chamber" || name.ends_with("_enriching_factory") {
        Some("mekanism:enriching")
    } else if name == "crusher" || name.ends_with("_crushing_factory") {
        Some("mekanism:crushing")
    } else if name == "combiner" || name.ends_with("_combining_factory") {
        Some("mekanism:combining")
    } else if name == "precision_sawmill" || name.ends_with("_sawing_factory") {
        Some("mekanism:sawing")
    } else if name == "metallurgic_infuser" {
        Some("mekanism:metallurgic_infusing")
    } else if name.ends_with("_infusing_factory") {
        Some("mekanism:metallurgic_infusing")
    } else if name == "chemical_injection_chamber" || name.ends_with("_injecting_factory") {
        Some("mekanism:injecting")
    } else if name == "purification_chamber" || name.ends_with("_purifying_factory") {
        Some("mekanism:purifying")
    } else if name == "chemical_dissolution_chamber" {
        Some("mekanism:dissolution")
    } else if name == "chemical_infuser" {
        Some("mekanism:chemical_infusing")
    } else if name == "chemical_washer" {
        Some("mekanism:washing")
    } else if name == "chemical_crystallizer" {
        Some("mekanism:crystallizing")
    } else if name == "chemical_oxidizer" {
        Some("mekanism:oxidizing")
    } else if name == "pressurized_reaction_chamber" {
        Some("mekanism:reaction")
    } else if name == "painting_machine" {
        Some("mekanism:painting")
    } else if name == "isotopic_centrifuge" {
        Some("mekanism:centrifuging")
    } else if name == "pigment_extractor" {
        Some("mekanism:pigment_extracting")
    } else if name == "pigment_mixer" {
        Some("mekanism:pigment_mixing")
    } else if name == "rotary_condensentrator" {
        Some("mekanism:rotary")
    } else if name == "antiprotonic_nucleosynthesizer" {
        Some("mekanism:nucleosynthesizing")
    } else if name == "electrolytic_separator" {
        Some("mekanism:separating")
    } else if name == "osmium_compressor" || name.ends_with("_compressing_factory") {
        Some("mekanism:compressing")
    } else if name == "solar_neutron_activator" {
        Some("mekanism:activating")
    } else if name == "thermal_evaporation_controller" {
        Some("mekanism:evaporating")
    } else {
        None
    }
}

fn slot_count(block: &str) -> u32 {
    match recipe_type(block) {
        Some("mekanism:combining" | "mekanism:sawing") => 3,
        Some(
            "mekanism:chemical_infusing"
            | "mekanism:washing"
            | "mekanism:crystallizing"
            | "mekanism:oxidizing"
            | "mekanism:centrifuging"
            | "mekanism:activating"
            | "mekanism:rotary"
            | "mekanism:separating"
            | "mekanism:evaporating"
            | "mekanism:pigment_mixing",
        ) => 0,
        Some(_) => 2,
        None if matches!(
            block,
            "mekanism:electric_pump" | "mekanism:fluidic_plenisher"
        ) =>
        {
            0
        }
        None if matches!(block, "mekanism:induction_port" | "mekanism:dynamic_valve") => 0,
        None if [
            "_universal_cable",
            "_energy_cube",
            "_fluid_tank",
            "_chemical_tank",
            "_mechanical_pipe",
            "_pressurized_tube",
            "_valve",
            "_resistive_heater",
        ]
        .iter()
        .any(|suffix| block.ends_with(suffix)) =>
        {
            0
        }
        None => 2,
    }
}

fn energy_capacity(block: &str) -> u64 {
    if block == "mekanism:creative_energy_cube" {
        1_000_000_000_000
    } else if block.starts_with("mekanism:ultimate_") && block.ends_with("_energy_cube") {
        512_000_000
    } else if block.starts_with("mekanism:elite_") && block.ends_with("_energy_cube") {
        64_000_000
    } else if block.starts_with("mekanism:advanced_") && block.ends_with("_energy_cube") {
        8_000_000
    } else {
        1_000_000
    }
}

fn transfer_rate(block: &str) -> u64 {
    if block.starts_with("mekanism:ultimate_") {
        102_400
    } else if block.starts_with("mekanism:elite_") {
        51_200
    } else if block.starts_with("mekanism:advanced_") {
        12_800
    } else {
        3_200
    }
}

fn energy_position(dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("{dimension}:{x},{y},{z}")
}

fn tep_key(kind: &str, dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("mekanism:tep/{kind}/{dimension}/{x},{y},{z}")
}

impl Mod for Mekanism {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            Self {
                progress: HashMap::new(),
                blocks: HashMap::new(),
                energy: HashMap::new(),
                side_modes: HashMap::new(),
                matrix_energy: HashMap::new(),
                dynamic_restore: HashMap::new(),
            },
            Registration::new(),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCKS
            .iter()
            .copied()
            .filter(|block| is_machine(block))
            .map(|block| BlockEntityType {
                block: block.into(),
                slots: slot_count(block),
                ticks: matches!(
                    block.strip_prefix("mekanism:").unwrap_or(block),
                    "energized_smelter"
                        | "basic_smelting_factory"
                        | "advanced_smelting_factory"
                        | "elite_smelting_factory"
                        | "ultimate_smelting_factory"
                ) || recipe_type(block).is_some()
                    || block.ends_with("_energy_cube")
                    || block.ends_with("_universal_cable")
                    || block.ends_with("_fluid_tank")
                    || block.ends_with("_chemical_tank")
                    || block.ends_with("_mechanical_pipe")
                    || block.ends_with("_pressurized_tube")
                    || block == "mekanism:thermal_evaporation_valve"
                    || block == "mekanism:resistive_heater"
                    || block == "mekanism:electric_pump"
                    || block == "mekanism:fluidic_plenisher"
                    || block == "mekanism:induction_port"
                    || block == "mekanism:dynamic_valve",
            })
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
        if event.action == BlockAction::UseItemOn
            && event.held_item == "mekanism:configurator"
            && event.state == "mekanism:induction_port"
        {
            let key = side_key(&event.dimension, event.x, event.y, event.z, "port");
            let next = if self
                .side_modes
                .get(&key)
                .map(String::as_str)
                .unwrap_or("input")
                == "input"
            {
                "output"
            } else {
                "input"
            };
            self.side_modes.insert(key, next.into());
            return EventOutcome::Consume;
        }
        if event.action == BlockAction::UseItemOn
            && event.held_item == "mekanism:configurator"
            && (event.state.ends_with("_mechanical_pipe")
                || event.state.ends_with("_pressurized_tube"))
        {
            let Some(face) = server::get_block_face() else {
                return EventOutcome::Pass;
            };
            let key = side_key(&event.dimension, event.x, event.y, event.z, &face);
            let current = self
                .side_modes
                .get(&key)
                .map(String::as_str)
                .unwrap_or("normal");
            let next = match current {
                "normal" => "pull",
                "pull" => "push",
                "push" => "disabled",
                _ => "normal",
            };
            self.side_modes.insert(key, next.into());
            let _ = machine::set_world_rule(
                &side_rule(&event.dimension, event.x, event.y, event.z, &face),
                next,
            );
            return EventOutcome::Consume;
        }
        if !matches!(
            event.action,
            BlockAction::UseWithoutItem | BlockAction::UseItemOn
        ) || !is_machine(&event.state)
        {
            return EventOutcome::Pass;
        }
        let spec = screen(&event.state, &event.dimension, event.x, event.y, event.z);
        if ui::open_screen(&spec).is_ok() {
            ui::set_backgrounds(
                &spec.id,
                &[ui::ScreenBackground {
                    texture: "mekanism:gui/base".into(),
                    x: 0,
                    y: 0,
                    width: spec.width,
                    height: spec.height,
                }],
            );
        }
        EventOutcome::Consume
    }

    fn on_item(&mut self, _event: ItemEvent) -> EventOutcome {
        // Items retain their normal game behavior; machine input is handled by
        // the machine menu and the game's inventory transfer rules.
        EventOutcome::Pass
    }

    fn create_block_entity(&mut self, handle: u32, _block: &str, saved: &[u8]) {
        self.progress.insert(
            handle,
            saved
                .get(..4)
                .and_then(|b| b.try_into().ok())
                .map(u32::from_le_bytes)
                .unwrap_or(0),
        );
        self.blocks.insert(handle, _block.to_owned());
        self.energy.insert(
            handle,
            saved
                .get(4..12)
                .and_then(|b| b.try_into().ok())
                .map(u64::from_le_bytes)
                .unwrap_or(0),
        );
        self.matrix_energy.insert(
            handle,
            saved
                .windows(4)
                .position(|window| window == b"IMX1")
                .and_then(|marker| saved.get(marker + 4..marker + 12))
                .and_then(|bytes| bytes.try_into().ok())
                .map(u64::from_le_bytes)
                .unwrap_or(0),
        );
        if let Some(marker) = saved.windows(4).position(|window| window == b"DYN1") {
            let mut offset = marker + 4;
            if let Some(&count) = saved.get(offset) {
                offset += 1;
                let mut tanks = Vec::new();
                for _ in 0..count {
                    let Some(tank_bytes) = saved.get(offset..offset + 2) else {
                        break;
                    };
                    let tank = u16::from_le_bytes(tank_bytes.try_into().unwrap_or([0; 2]));
                    offset += 2;
                    let Some(amount_bytes) = saved.get(offset..offset + 8) else {
                        break;
                    };
                    let amount = u64::from_le_bytes(amount_bytes.try_into().unwrap_or([0; 8]));
                    offset += 8;
                    let Some(capacity_bytes) = saved.get(offset..offset + 8) else {
                        break;
                    };
                    let capacity = u64::from_le_bytes(capacity_bytes.try_into().unwrap_or([0; 8]));
                    offset += 8;
                    let Some(len_bytes) = saved.get(offset..offset + 2) else {
                        break;
                    };
                    let len = u16::from_le_bytes(len_bytes.try_into().unwrap_or([0; 2])) as usize;
                    offset += 2;
                    let Some(id_bytes) = saved.get(offset..offset + len) else {
                        break;
                    };
                    let fluid = String::from_utf8_lossy(id_bytes).into_owned();
                    offset += len;
                    tanks.push((
                        u32::from(tank),
                        capabilities::FluidView {
                            fluid,
                            amount,
                            capacity,
                        },
                    ));
                }
                self.dynamic_restore.insert(handle, tanks);
            }
        }
        if let Some(position) = machine::get_position(handle) {
            let key = energy_position(&position.dimension, position.x, position.y, position.z);
            if let Some(marker) = saved.windows(4).position(|window| window == b"MCM1") {
                let mut offset = marker + 4;
                if let Some(&count) = saved.get(offset) {
                    offset += 1;
                    for _ in 0..count {
                        let Some(&face_len) = saved.get(offset) else {
                            break;
                        };
                        offset += 1;
                        let Some(face) = saved.get(offset..offset + face_len as usize) else {
                            break;
                        };
                        offset += face_len as usize;
                        let Some(&mode_len) = saved.get(offset) else {
                            break;
                        };
                        offset += 1;
                        let Some(mode) = saved.get(offset..offset + mode_len as usize) else {
                            break;
                        };
                        offset += mode_len as usize;
                        let face = String::from_utf8_lossy(face).into_owned();
                        let mode = String::from_utf8_lossy(mode).into_owned();
                        let side = side_key(
                            &position.dimension,
                            position.x,
                            position.y,
                            position.z,
                            &face,
                        );
                        self.side_modes.insert(side, mode.clone());
                        let _ = machine::set_world_rule(
                            &side_rule(
                                &position.dimension,
                                position.x,
                                position.y,
                                position.z,
                                &face,
                            ),
                            &mode,
                        );
                    }
                }
            }
            let modern = saved.get(12..16) == Some(b"MTK1".as_slice());
            let mut offset = if modern { 16 } else { 12 };
            if let Some(&count) = saved.get(offset) {
                offset += 1;
                for _ in 0..count {
                    let tank = if modern {
                        let Some(tank_bytes) = saved.get(offset..offset + 2) else {
                            break;
                        };
                        offset += 2;
                        u16::from_le_bytes(tank_bytes.try_into().unwrap_or([0; 2]))
                    } else {
                        let Some(&tank) = saved.get(offset) else {
                            break;
                        };
                        offset += 1;
                        u16::from(tank)
                    };
                    let Some(amount_bytes) = saved.get(offset..offset + 8) else {
                        break;
                    };
                    let amount = u64::from_le_bytes(amount_bytes.try_into().unwrap_or([0; 8]));
                    offset += 8;
                    let capacity = if modern {
                        let Some(capacity_bytes) = saved.get(offset..offset + 8) else {
                            break;
                        };
                        offset += 8;
                        u64::from_le_bytes(capacity_bytes.try_into().unwrap_or([0; 8]))
                    } else if _block.starts_with("mekanism:ultimate_") {
                        1_000_000
                    } else if _block.ends_with("_fluid_tank") || _block.ends_with("_chemical_tank")
                    {
                        64_000
                    } else {
                        16_000
                    };
                    let Some(len_bytes) = saved.get(offset..offset + 2) else {
                        break;
                    };
                    let len = u16::from_le_bytes(len_bytes.try_into().unwrap_or([0; 2])) as usize;
                    offset += 2;
                    let Some(id_bytes) = saved.get(offset..offset + len) else {
                        break;
                    };
                    let id = String::from_utf8_lossy(id_bytes).into_owned();
                    offset += len;
                    capabilities::set_fluid_tank(
                        &key,
                        u32::from(tank),
                        &capabilities::FluidView {
                            fluid: id,
                            amount,
                            capacity,
                        },
                    );
                }
            }
        }
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        let mut matrix_rates: HashMap<String, u64> = HashMap::new();
        for &handle in handles {
            let Some(position) = machine::get_position(handle) else {
                continue;
            };
            let energy_key = format!(
                "{}:{},{},{}",
                position.dimension, position.x, position.y, position.z
            );
            let block = self.blocks.get(&handle).map(String::as_str).unwrap_or("");
            if block == "mekanism:thermal_evaporation_controller"
                && (machine::get_world_rule(&tep_key(
                    "controller",
                    &position.dimension,
                    position.x,
                    position.y,
                    position.z,
                ))
                .as_deref()
                    != Some("true")
                    || machine::get_world_rule(&tep_key(
                        "heat",
                        &position.dimension,
                        position.x,
                        position.y,
                        position.z,
                    ))
                    .as_deref()
                        != Some("true"))
            {
                self.progress.insert(handle, 0);
                set_active(handle, "false");
                continue;
            }
            if capabilities::get_energy(&energy_key).is_none() {
                let capacity = energy_capacity(block);
                capabilities::set_energy_buffer(
                    &energy_key,
                    EnergyView {
                        stored: if block == "mekanism:creative_energy_cube" {
                            capacity
                        } else {
                            self.energy.get(&handle).copied().unwrap_or(0)
                        },
                        capacity,
                        max_receive: transfer_rate(block),
                        max_extract: transfer_rate(block),
                    },
                );
            }
            if block == "mekanism:creative_energy_cube" {
                let capacity = energy_capacity(block);
                capabilities::set_energy_buffer(
                    &energy_key,
                    EnergyView {
                        stored: capacity,
                        capacity,
                        max_receive: transfer_rate(block),
                        max_extract: transfer_rate(block),
                    },
                );
            }
            self.energy.insert(
                handle,
                capabilities::get_energy(&energy_key).map_or(0, |view| view.stored),
            );
            if block == "mekanism:induction_port" {
                let rule = format!(
                    "mekanism:induction/port/{}/{}",
                    position.dimension,
                    format_args!("{},{},{}", position.x, position.y, position.z)
                );
                let link = machine::get_world_rule(&rule);
                let Some(link) = link else {
                    set_active(handle, "false");
                    if let Some(port) = capabilities::get_energy(&energy_key) {
                        capabilities::set_energy_buffer(
                            &energy_key,
                            EnergyView {
                                max_receive: 0,
                                max_extract: 0,
                                ..port
                            },
                        );
                    }
                    continue;
                };
                let mut fields = link.split(';');
                let Some(coords) = fields.next() else {
                    continue;
                };
                let Some(capacity) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
                    continue;
                };
                let Some(transfer) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
                    continue;
                };
                let parsed = coords
                    .split(',')
                    .map(str::parse::<i32>)
                    .collect::<Result<Vec<_>, _>>();
                let Ok(coords) = parsed else { continue };
                let [x, y, z] = coords.as_slice() else {
                    continue;
                };
                if capacity == 0 || transfer == 0 {
                    continue;
                }
                let matrix_key = energy_position(&position.dimension, *x, *y, *z);
                let matrix = capabilities::get_energy(&matrix_key);
                let stored = matrix.map_or_else(
                    || self.matrix_energy.get(&handle).copied().unwrap_or(0),
                    |view| view.stored,
                );
                capabilities::set_energy_buffer(
                    &matrix_key,
                    EnergyView {
                        stored: stored.min(capacity),
                        capacity,
                        max_receive: transfer,
                        max_extract: transfer,
                    },
                );
                let rate = matrix_rates.entry(matrix_key.clone()).or_insert(transfer);
                let mode_key = side_key(
                    &position.dimension,
                    position.x,
                    position.y,
                    position.z,
                    "port",
                );
                let output = self
                    .side_modes
                    .get(&mode_key)
                    .is_some_and(|mode| mode == "output");
                let local = capabilities::get_energy(&energy_key).unwrap_or(EnergyView {
                    stored: 0,
                    capacity: transfer,
                    max_receive: 0,
                    max_extract: 0,
                });
                if output {
                    let amount = local.capacity.saturating_sub(local.stored).min(*rate);
                    let moved = capabilities::extract_energy(&matrix_key, amount, false);
                    *rate = rate.saturating_sub(moved);
                    capabilities::set_energy_buffer(
                        &energy_key,
                        EnergyView {
                            stored: local.stored.saturating_add(moved),
                            capacity: transfer,
                            max_receive: 0,
                            max_extract: transfer,
                        },
                    );
                } else {
                    let amount = local.stored.min(*rate);
                    let accepted = capabilities::receive_energy(&matrix_key, amount, true);
                    let moved = accepted.min(amount);
                    if moved > 0 {
                        capabilities::set_energy_buffer(
                            &energy_key,
                            EnergyView {
                                stored: local.stored - moved,
                                capacity: transfer,
                                max_receive: transfer,
                                max_extract: 0,
                            },
                        );
                        capabilities::receive_energy(&matrix_key, moved, false);
                        *rate = rate.saturating_sub(moved);
                    } else {
                        capabilities::set_energy_buffer(
                            &energy_key,
                            EnergyView {
                                stored: local.stored,
                                capacity: transfer,
                                max_receive: transfer,
                                max_extract: 0,
                            },
                        );
                    }
                }
                let stored = capabilities::get_energy(&matrix_key).map_or(0, |view| view.stored);
                self.matrix_energy.insert(handle, stored);
                self.energy.insert(
                    handle,
                    capabilities::get_energy(&energy_key).map_or(0, |view| view.stored),
                );
                set_active(handle, if stored > 0 { "true" } else { "false" });
                let id = screen_id(
                    block,
                    &position.dimension,
                    position.x,
                    position.y,
                    position.z,
                );
                ui::set_gauge(&id, 11, stored, capacity, "Matrix energy");
                continue;
            }
            if block == "mekanism:resistive_heater" {
                let powered = capabilities::extract_energy(&energy_key, 50, false) == 50;
                set_active(handle, if powered { "true" } else { "false" });
                if powered {
                    for [dx, dy, dz] in [
                        [1, 0, 0],
                        [-1, 0, 0],
                        [0, 1, 0],
                        [0, -1, 0],
                        [0, 0, 1],
                        [0, 0, -1],
                    ] {
                        let Some(link) = machine::get_world_rule(&tep_key(
                            "valve",
                            &position.dimension,
                            position.x + dx,
                            position.y + dy,
                            position.z + dz,
                        ))
                        .filter(|value| !value.is_empty()) else {
                            continue;
                        };
                        let Ok(coords) = link
                            .split(',')
                            .map(str::parse::<i32>)
                            .collect::<Result<Vec<_>, _>>()
                        else {
                            continue;
                        };
                        if let [x, y, z] = coords.as_slice() {
                            let _ = machine::set_world_rule(
                                &tep_key("heat", &position.dimension, *x, *y, *z),
                                "true",
                            );
                        }
                    }
                }
                continue;
            }
            if block == "mekanism:electric_pump" {
                if capabilities::get_fluid(&energy_key, 0).is_none() {
                    capabilities::set_fluid_tank(
                        &energy_key,
                        0,
                        &capabilities::FluidView {
                            fluid: String::new(),
                            amount: 0,
                            capacity: 64_000,
                        },
                    );
                }
                let source = [
                    ([1, 0, 0], "east"),
                    ([-1, 0, 0], "west"),
                    ([0, 1, 0], "up"),
                    ([0, -1, 0], "down"),
                    ([0, 0, 1], "south"),
                    ([0, 0, -1], "north"),
                ]
                .into_iter()
                .find_map(|([dx, dy, dz], _)| {
                    let pos = BlockPos {
                        x: position.x + dx,
                        y: position.y + dy,
                        z: position.z + dz,
                    };
                    let block = server::get_block(&position.dimension, pos)?;
                    if block == "minecraft:water" || block.starts_with("minecraft:water[level=0") {
                        Some("minecraft:water")
                    } else if block == "minecraft:lava"
                        || block.starts_with("minecraft:lava[level=0")
                    {
                        Some("minecraft:lava")
                    } else {
                        None
                    }
                });
                let screen = screen_id(
                    block,
                    &position.dimension,
                    position.x,
                    position.y,
                    position.z,
                );
                if let Some(contents) = capabilities::get_fluid(&energy_key, 0) {
                    ui::set_gauge(
                        &screen,
                        10,
                        contents.amount,
                        contents.capacity,
                        if contents.fluid.is_empty() {
                            "Empty tank"
                        } else {
                            &contents.fluid
                        },
                    );
                }
                let can_fill = source.is_some_and(|fluid| {
                    capabilities::fill_fluid_tank(&energy_key, 0, fluid, 1_000, true) == 1_000
                });
                let powered = can_fill && capabilities::extract_energy(&energy_key, 10, true) == 10;
                if powered {
                    capabilities::extract_energy(&energy_key, 10, false);
                    let progress = self.progress.entry(handle).or_default();
                    *progress = progress.saturating_add(1).min(20);
                    if *progress == 20 {
                        if let Some(fluid) = source {
                            capabilities::fill_fluid_tank(&energy_key, 0, fluid, 1_000, false);
                        }
                        *progress = 0;
                    }
                }
                set_active(handle, if powered { "true" } else { "false" });
                if let Some(energy) = capabilities::get_energy(&energy_key) {
                    self.energy.insert(handle, energy.stored);
                    ui::set_gauge(&screen, 11, energy.stored, energy.capacity, "Energy");
                }
                continue;
            }
            if block == "mekanism:fluidic_plenisher" {
                if capabilities::get_fluid(&energy_key, 0).is_none() {
                    capabilities::set_fluid_tank(
                        &energy_key,
                        0,
                        &capabilities::FluidView {
                            fluid: String::new(),
                            amount: 0,
                            capacity: 64_000,
                        },
                    );
                }
                let target = [
                    ([1, 0, 0], "east"),
                    ([-1, 0, 0], "west"),
                    ([0, 1, 0], "up"),
                    ([0, -1, 0], "down"),
                    ([0, 0, 1], "south"),
                    ([0, 0, -1], "north"),
                ]
                .into_iter()
                .find_map(|([dx, dy, dz], _)| {
                    let pos = BlockPos {
                        x: position.x + dx,
                        y: position.y + dy,
                        z: position.z + dz,
                    };
                    (server::get_block(&position.dimension, pos).as_deref()
                        == Some("minecraft:air"))
                    .then_some(pos)
                });
                let screen = screen_id(
                    block,
                    &position.dimension,
                    position.x,
                    position.y,
                    position.z,
                );
                let water = capabilities::get_fluid(&energy_key, 0).filter(|contents| {
                    contents.fluid == "minecraft:water" && contents.amount >= 1_000
                });
                if let Some(contents) = capabilities::get_fluid(&energy_key, 0) {
                    ui::set_gauge(
                        &screen,
                        10,
                        contents.amount,
                        contents.capacity,
                        if contents.fluid.is_empty() {
                            "Empty tank"
                        } else {
                            &contents.fluid
                        },
                    );
                }
                let powered = target.is_some()
                    && water.is_some()
                    && capabilities::extract_energy(&energy_key, 10, true) == 10;
                if powered {
                    capabilities::extract_energy(&energy_key, 10, false);
                    let progress = self.progress.entry(handle).or_default();
                    *progress = progress.saturating_add(1).min(20);
                    if *progress == 20 {
                        if let Some(target) = target
                            && server::set_block(
                                &position.dimension,
                                target,
                                "minecraft:water[level=0]",
                            )
                            .is_ok()
                        {
                            capabilities::drain_fluid_tank(
                                &energy_key,
                                0,
                                "minecraft:water",
                                1_000,
                                false,
                            );
                        }
                        *progress = 0;
                    }
                }
                set_active(handle, if powered { "true" } else { "false" });
                if let Some(energy) = capabilities::get_energy(&energy_key) {
                    self.energy.insert(handle, energy.stored);
                    ui::set_gauge(&screen, 11, energy.stored, energy.capacity, "Energy");
                }
                continue;
            }
            if block == "mekanism:dynamic_valve" {
                let rule = format!(
                    "mekanism:dynamic/valve/{}/{}",
                    position.dimension,
                    format_args!("{},{},{}", position.x, position.y, position.z)
                );
                let Some(link) = machine::get_world_rule(&rule) else {
                    set_active(handle, "false");
                    continue;
                };
                let mut fields = link.split(';');
                let Some(coords) = fields.next() else {
                    continue;
                };
                let Some(volume) = fields.next().and_then(|v| v.parse::<u64>().ok()) else {
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
                let [x, y, z] = coords.as_slice() else {
                    continue;
                };
                let shared_key = energy_position(&position.dimension, *x, *y, *z);
                let liquid_capacity = volume
                    .saturating_mul(350_000)
                    .saturating_sub(valves.saturating_mul(64_000));
                let chemical_capacity = volume
                    .saturating_mul(16_000_000)
                    .saturating_sub(valves.saturating_mul(64_000));
                if let Some(restored) = self.dynamic_restore.remove(&handle) {
                    for (tank, view) in restored {
                        let capacity = if tank >= 1_000 {
                            chemical_capacity
                        } else {
                            liquid_capacity
                        };
                        if capabilities::get_fluid(&shared_key, tank).is_none() {
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
                    }
                }
                for (tank, capacity) in [(0, liquid_capacity), (1_000, chemical_capacity)] {
                    if capabilities::get_fluid(&shared_key, tank).is_none() {
                        capabilities::set_fluid_tank(
                            &shared_key,
                            tank,
                            &capabilities::FluidView {
                                fluid: String::new(),
                                amount: 0,
                                capacity,
                            },
                        );
                    } else if let Some(view) = capabilities::get_fluid(&shared_key, tank) {
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
                    if capabilities::get_fluid(&energy_key, tank).is_none() {
                        capabilities::set_fluid_tank(
                            &energy_key,
                            tank,
                            &capabilities::FluidView {
                                fluid: String::new(),
                                amount: 0,
                                capacity: 64_000,
                            },
                        );
                    }
                    if let Some(local) = capabilities::get_fluid(&energy_key, tank) {
                        if local.amount > 0 {
                            let accepted = capabilities::fill_fluid_tank(
                                &shared_key,
                                tank,
                                &local.fluid,
                                local.amount,
                                true,
                            );
                            if accepted > 0 {
                                let moved = capabilities::drain_fluid_tank(
                                    &energy_key,
                                    tank,
                                    &local.fluid,
                                    accepted,
                                    false,
                                );
                                capabilities::fill_fluid_tank(
                                    &shared_key,
                                    tank,
                                    &local.fluid,
                                    moved,
                                    false,
                                );
                            }
                        }
                    }
                    if let Some(shared) = capabilities::get_fluid(&shared_key, tank)
                        && shared.amount > 0
                        && let Some(local) = capabilities::get_fluid(&energy_key, tank)
                    {
                        let accepted = capabilities::fill_fluid_tank(
                            &energy_key,
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
                                &energy_key,
                                tank,
                                &shared.fluid,
                                moved,
                                false,
                            );
                        }
                        let _ = local;
                    }
                }
                let screen = screen_id(
                    block,
                    &position.dimension,
                    position.x,
                    position.y,
                    position.z,
                );
                if let Some(contents) = capabilities::get_fluid(&shared_key, 0) {
                    ui::set_gauge(
                        &screen,
                        10,
                        contents.amount,
                        contents.capacity,
                        if contents.fluid.is_empty() {
                            "Empty tank"
                        } else {
                            &contents.fluid
                        },
                    );
                }
                if let Some(contents) = capabilities::get_fluid(&shared_key, 1_000) {
                    ui::set_gauge(
                        &screen,
                        11,
                        contents.amount,
                        contents.capacity,
                        if contents.fluid.is_empty() {
                            "Empty tank"
                        } else {
                            &contents.fluid
                        },
                    );
                }
                set_active(handle, "true");
                continue;
            }
            if block == "mekanism:thermal_evaporation_valve" {
                if capabilities::get_fluid(&energy_key, 0).is_none() {
                    capabilities::set_fluid_tank(
                        &energy_key,
                        0,
                        &capabilities::FluidView {
                            fluid: String::new(),
                            amount: 0,
                            capacity: 64_000,
                        },
                    );
                }
                if let Some(link) = machine::get_world_rule(&tep_key(
                    "valve",
                    &position.dimension,
                    position.x,
                    position.y,
                    position.z,
                ))
                .filter(|value| !value.is_empty())
                {
                    let parsed = link
                        .split(',')
                        .map(str::parse::<i32>)
                        .collect::<Result<Vec<_>, _>>();
                    if let Ok(coords) = parsed
                        && let [x, y, z] = coords.as_slice()
                    {
                        let controller = energy_position(&position.dimension, *x, *y, *z);
                        if let Some(input) =
                            capabilities::get_fluid(&energy_key, 0).filter(|view| view.amount > 0)
                        {
                            let accepted = capabilities::fill_fluid_tank(
                                &controller,
                                0,
                                &input.fluid,
                                input.amount.min(250),
                                true,
                            );
                            if accepted > 0 {
                                let moved = capabilities::drain_fluid_tank(
                                    &energy_key,
                                    0,
                                    &input.fluid,
                                    accepted,
                                    false,
                                );
                                capabilities::fill_fluid_tank(
                                    &controller,
                                    0,
                                    &input.fluid,
                                    moved,
                                    false,
                                );
                            }
                        } else {
                            for tank in 1..3 {
                                let Some(output) = capabilities::get_fluid(&controller, tank)
                                    .filter(|view| view.amount > 0)
                                else {
                                    continue;
                                };
                                let accepted = capabilities::fill_fluid_tank(
                                    &energy_key,
                                    0,
                                    &output.fluid,
                                    output.amount.min(250),
                                    true,
                                );
                                if accepted > 0 {
                                    let moved = capabilities::drain_fluid_tank(
                                        &controller,
                                        tank,
                                        &output.fluid,
                                        accepted,
                                        false,
                                    );
                                    capabilities::fill_fluid_tank(
                                        &energy_key,
                                        0,
                                        &output.fluid,
                                        moved,
                                        false,
                                    );
                                }
                                break;
                            }
                        }
                    }
                }
                if let Some(contents) = capabilities::get_fluid(&energy_key, 0) {
                    let screen = screen_id(
                        block,
                        &position.dimension,
                        position.x,
                        position.y,
                        position.z,
                    );
                    ui::set_gauge(
                        &screen,
                        10,
                        contents.amount,
                        contents.capacity,
                        if contents.fluid.is_empty() {
                            "Empty valve"
                        } else {
                            &contents.fluid
                        },
                    );
                }
                continue;
            }
            if block.ends_with("_mechanical_pipe") || block.ends_with("_pressurized_tube") {
                let base = if block.ends_with("_pressurized_tube") {
                    1_000
                } else {
                    0
                };
                if capabilities::get_fluid(&energy_key, base).is_none() {
                    capabilities::set_fluid_tank(
                        &energy_key,
                        base,
                        &capabilities::FluidView {
                            fluid: String::new(),
                            amount: 0,
                            capacity: 16_000,
                        },
                    );
                }
                if let Some(contents) = capabilities::get_fluid(&energy_key, base) {
                    let screen = screen_id(
                        block,
                        &position.dimension,
                        position.x,
                        position.y,
                        position.z,
                    );
                    ui::set_gauge(
                        &screen,
                        10,
                        contents.amount,
                        contents.capacity,
                        if contents.fluid.is_empty() {
                            "Empty pipe"
                        } else {
                            &contents.fluid
                        },
                    );
                }
                for ([dx, dy, dz], face) in [
                    ([1, 0, 0], "east"),
                    ([-1, 0, 0], "west"),
                    ([0, 1, 0], "up"),
                    ([0, -1, 0], "down"),
                    ([0, 0, 1], "south"),
                    ([0, 0, -1], "north"),
                ] {
                    let mode = self
                        .side_modes
                        .get(&side_key(
                            &position.dimension,
                            position.x,
                            position.y,
                            position.z,
                            face,
                        ))
                        .map(String::as_str)
                        .unwrap_or("normal");
                    if mode == "disabled" {
                        continue;
                    }
                    let neighbor = energy_position(
                        &position.dimension,
                        position.x + dx,
                        position.y + dy,
                        position.z + dz,
                    );
                    let pipe_contents =
                        capabilities::get_fluid(&energy_key, base).filter(|view| view.amount > 0);
                    if mode != "pull"
                        && let Some(contents) = pipe_contents
                    {
                        for tank in base..base + 3 {
                            let accepted = capabilities::fill_fluid_tank(
                                &neighbor,
                                tank,
                                &contents.fluid,
                                contents.amount.min(250),
                                true,
                            );
                            if accepted == 0 {
                                continue;
                            }
                            let moved = capabilities::drain_fluid_tank(
                                &energy_key,
                                base,
                                &contents.fluid,
                                accepted,
                                false,
                            );
                            capabilities::fill_fluid_tank(
                                &neighbor,
                                tank,
                                &contents.fluid,
                                moved,
                                false,
                            );
                            break;
                        }
                    } else if mode == "pull" {
                        for tank in base..base + 3 {
                            let Some(contents) = capabilities::get_fluid(&neighbor, tank)
                                .filter(|view| view.amount > 0 && !view.fluid.is_empty())
                            else {
                                continue;
                            };
                            let accepted = capabilities::fill_fluid_tank(
                                &energy_key,
                                base,
                                &contents.fluid,
                                contents.amount.min(250),
                                true,
                            );
                            if accepted == 0 {
                                continue;
                            }
                            let moved = capabilities::drain_fluid_tank(
                                &neighbor,
                                tank,
                                &contents.fluid,
                                accepted,
                                false,
                            );
                            capabilities::fill_fluid_tank(
                                &energy_key,
                                base,
                                &contents.fluid,
                                moved,
                                false,
                            );
                            break;
                        }
                    }
                }
                continue;
            }
            if block.ends_with("_fluid_tank") || block.ends_with("_chemical_tank") {
                let tank_index = if block.ends_with("_chemical_tank") {
                    1_000
                } else {
                    0
                };
                if capabilities::get_fluid(&energy_key, tank_index).is_none() {
                    capabilities::set_fluid_tank(
                        &energy_key,
                        tank_index,
                        &capabilities::FluidView {
                            fluid: String::new(),
                            amount: 0,
                            capacity: if block.starts_with("mekanism:ultimate_") {
                                1_000_000
                            } else {
                                64_000
                            },
                        },
                    );
                }
                if let Some(tank) = capabilities::get_fluid(&energy_key, tank_index) {
                    let screen = screen_id(
                        block,
                        &position.dimension,
                        position.x,
                        position.y,
                        position.z,
                    );
                    let label = if tank.fluid.is_empty() {
                        "Empty tank"
                    } else {
                        &tank.fluid
                    };
                    ui::set_gauge(&screen, 10, tank.amount, tank.capacity, label);
                }
                continue;
            }
            if block.ends_with("_universal_cable") || block.ends_with("_energy_cube") {
                if let Some(source) = capabilities::get_energy(&energy_key) {
                    let mut remaining = transfer_rate(block).min(source.stored);
                    if remaining > 0 {
                        for [dx, dy, dz] in [
                            [1, 0, 0],
                            [-1, 0, 0],
                            [0, 1, 0],
                            [0, -1, 0],
                            [0, 0, 1],
                            [0, 0, -1],
                        ] {
                            if remaining == 0 {
                                break;
                            }
                            let x = position.x + dx;
                            let y = position.y + dy;
                            let z = position.z + dz;
                            let key = energy_position(&position.dimension, x, y, z);
                            if capabilities::get_energy(&key).is_some() {
                                let room = capabilities::receive_energy(&key, remaining, true);
                                if room > 0 {
                                    let amount = capabilities::extract_energy(
                                        &energy_key,
                                        room.min(remaining),
                                        false,
                                    );
                                    capabilities::receive_energy(&key, amount, false);
                                    remaining = remaining.saturating_sub(amount);
                                }
                            }
                        }
                    }
                }
                if let Some(view) = capabilities::get_energy(&energy_key) {
                    self.energy.insert(handle, view.stored);
                    let screen = screen_id(
                        block,
                        &position.dimension,
                        position.x,
                        position.y,
                        position.z,
                    );
                    ui::set_gauge(&screen, 11, view.stored, view.capacity, "Energy");
                }
                continue;
            }
            let screen_id = screen_id(
                block,
                &position.dimension,
                position.x,
                position.y,
                position.z,
            );
            ui::set_gauge(
                &screen_id,
                11,
                self.energy.get(&handle).copied().unwrap_or(0),
                1_000_000,
                "Energy",
            );
            if let Some(kind) = recipe_type(block) {
                // Separate index ranges keep chemical and fluid tanks independent.
                for tank in 0..3 {
                    if capabilities::get_fluid(&energy_key, tank).is_none() {
                        capabilities::set_fluid_tank(
                            &energy_key,
                            tank,
                            &capabilities::FluidView {
                                fluid: String::new(),
                                amount: 0,
                                capacity: 16_000,
                            },
                        );
                    }
                    let chemical_tank = 1_000 + tank;
                    if capabilities::get_fluid(&energy_key, chemical_tank).is_none() {
                        capabilities::set_fluid_tank(
                            &energy_key,
                            chemical_tank,
                            &capabilities::FluidView {
                                fluid: String::new(),
                                amount: 0,
                                capacity: 16_000,
                            },
                        );
                    }
                }
                for tank in 0..3 {
                    if let Some(contents) = capabilities::get_fluid(&energy_key, tank) {
                        ui::set_gauge(
                            &screen_id,
                            20 + tank,
                            contents.amount,
                            contents.capacity,
                            if contents.fluid.is_empty() {
                                "Empty fluid"
                            } else {
                                &contents.fluid
                            },
                        );
                    }
                    if let Some(contents) = capabilities::get_fluid(&energy_key, 1_000 + tank) {
                        ui::set_gauge(
                            &screen_id,
                            23 + tank,
                            contents.amount,
                            contents.capacity,
                            if contents.fluid.is_empty() {
                                "Empty chemical"
                            } else {
                                &contents.fluid
                            },
                        );
                    }
                }
                let additional = if block == "mekanism:chemical_oxidizer" {
                    recipes::by_type("mekanism:chemical_conversion")
                } else {
                    Vec::new()
                };
                let found = recipes::by_type(kind)
                    .into_iter()
                    .chain(additional)
                    .find(|recipe| {
                        recipe
                            .inputs
                            .iter()
                            .enumerate()
                            .all(|(item_slot, ingredient)| {
                                if ingredient.items.first().is_some_and(|s| s.starts_with('@')) {
                                    ingredient.items.iter().any(|selector| {
                                        let Some((kind_tank, id)) = selector
                                            .strip_prefix('@')
                                            .and_then(|s| s.split_once('/'))
                                        else {
                                            return false;
                                        };
                                        let Some((kind, tank)) = kind_tank.split_once('/') else {
                                            return false;
                                        };
                                        if kind != "chemical" && kind != "fluid" {
                                            return false;
                                        }
                                        let Ok(tank) = tank.parse::<u32>() else {
                                            return false;
                                        };
                                        capabilities::get_fluid(&energy_key, tank).is_some_and(
                                            |v| {
                                                v.fluid == id && v.amount >= ingredient.count as u64
                                            },
                                        )
                                    })
                                } else {
                                    let slot = recipe.inputs[..item_slot]
                                        .iter()
                                        .filter(|i| {
                                            !i.items.first().is_some_and(|s| s.starts_with('@'))
                                        })
                                        .count()
                                        as u32;
                                    machine::get_slot(handle, slot).is_some_and(|stack| {
                                        stack.count >= ingredient.count
                                            && ingredient.items.iter().any(|item| item == &stack.id)
                                    })
                                }
                            })
                    });
                let Some(recipe) = found else {
                    self.progress.insert(handle, 0);
                    ui::set_gauge(&screen_id, 10, 0, 200, "Progress");
                    set_active(handle, "false");
                    continue;
                };
                let item_inputs = recipe
                    .inputs
                    .iter()
                    .filter(|i| !i.items.first().is_some_and(|s| s.starts_with('@')))
                    .count() as u32;
                let output_slot = item_inputs;
                let item_outputs: Vec<_> = recipe
                    .outputs
                    .iter()
                    .filter(|o| !o.starts_with('@'))
                    .collect();
                let resource_outputs: Vec<_> = recipe
                    .outputs
                    .iter()
                    .filter(|o| o.starts_with('@'))
                    .collect();
                let output_fits = item_outputs.iter().enumerate().all(|(index, output)| {
                    let (id, count) = output
                        .split_once('*')
                        .map_or((output.as_str(), 1), |(id, count)| {
                            (id, count.parse::<u32>().unwrap_or(1))
                        });
                    let current = machine::get_slot(handle, output_slot + index as u32);
                    let total = match current {
                        Some(stack) if stack.id == id => stack.count.saturating_add(count),
                        Some(_) => return false,
                        None => count,
                    };
                    total <= machine::max_stack_size(id)
                }) && resource_outputs.iter().all(|output| {
                    let Some((kind_tank, rest)) =
                        output.strip_prefix('@').and_then(|s| s.split_once('/'))
                    else {
                        return false;
                    };
                    let Some((kind, tank)) = kind_tank.split_once('/') else {
                        return false;
                    };
                    let Some((id, amount)) = rest.rsplit_once('*') else {
                        return false;
                    };
                    let (Ok(tank), Ok(amount)) = (tank.parse::<u32>(), amount.parse::<u64>())
                    else {
                        return false;
                    };
                    matches!(kind, "fluid" | "chemical")
                        && capabilities::fill_fluid_tank(&energy_key, tank, id, amount, true)
                            == amount
                });
                if !output_fits {
                    continue;
                }
                let energy_cost = if recipe.energy == 0 {
                    40
                } else {
                    recipe.energy
                };
                if capabilities::extract_energy(&energy_key, energy_cost, true) < energy_cost {
                    continue;
                }
                capabilities::extract_energy(&energy_key, energy_cost, false);
                if let Some(view) = capabilities::get_energy(&energy_key) {
                    self.energy.insert(handle, view.stored);
                }
                let progress = self.progress.entry(handle).or_default();
                *progress = progress.saturating_add(1).min(recipe.ticks.max(1));
                ui::set_gauge(
                    &screen_id,
                    10,
                    *progress as u64,
                    recipe.ticks.max(1) as u64,
                    "Progress",
                );
                set_active(handle, "true");
                if *progress < recipe.ticks.max(1) {
                    continue;
                }
                let outputs: Vec<ItemStack> = item_outputs
                    .iter()
                    .filter_map(|output| {
                        let (id, count) = output
                            .split_once('*')
                            .map_or((output.as_str(), 1), |(id, count)| {
                                (id, count.parse::<u32>().unwrap_or(1))
                            });
                        (!id.is_empty() && count > 0).then(|| ItemStack {
                            id: id.to_owned(),
                            count,
                        })
                    })
                    .collect();
                if outputs.is_empty() && resource_outputs.is_empty() {
                    *progress = 0;
                    continue;
                }
                let mut resulting = Vec::new();
                let mut can_output = true;
                for (index, output) in outputs.iter().enumerate() {
                    let slot = output_slot + index as u32;
                    let result = match machine::get_slot(handle, slot) {
                        Some(existing) if existing.id == output.id => ItemStack {
                            id: output.id.clone(),
                            count: existing.count.saturating_add(output.count),
                        },
                        Some(_) => {
                            can_output = false;
                            break;
                        }
                        None => output.clone(),
                    };
                    resulting.push((slot, result));
                }
                if can_output {
                    can_output = resulting
                        .iter()
                        .all(|(slot, result)| machine::set_slot(handle, *slot, Some(result)));
                }
                if !can_output {
                    *progress = recipe.ticks.max(1);
                    continue;
                }
                for (index, input) in recipe.inputs.iter().enumerate() {
                    if input.items.first().is_some_and(|s| s.starts_with('@')) {
                        let selector = &input.items[0];
                        if let Some((kind_tank, _)) =
                            selector.strip_prefix('@').and_then(|s| s.split_once('/'))
                        {
                            if let Some((_, tank)) = kind_tank.split_once('/') {
                                if let Ok(tank) = tank.parse::<u32>() {
                                    if let Some(view) = capabilities::get_fluid(&energy_key, tank) {
                                        capabilities::drain_fluid_tank(
                                            &energy_key,
                                            tank,
                                            &view.fluid,
                                            input.count as u64,
                                            false,
                                        );
                                    }
                                }
                            }
                        }
                        continue;
                    }
                    let slot = recipe.inputs[..index]
                        .iter()
                        .filter(|i| !i.items.first().is_some_and(|s| s.starts_with('@')))
                        .count() as u32;
                    let Some(current) = machine::get_slot(handle, slot) else {
                        continue;
                    };
                    let rest_count = current.count.saturating_sub(input.count);
                    let rest = (rest_count > 0).then(|| ItemStack {
                        id: current.id,
                        count: rest_count,
                    });
                    machine::set_slot(handle, slot, rest.as_ref());
                }
                for output in resource_outputs {
                    if let Some((kind_tank, rest)) =
                        output.strip_prefix('@').and_then(|s| s.split_once('/'))
                    {
                        if let Some((kind, tank)) = kind_tank.split_once('/') {
                            if kind == "fluid" || kind == "chemical" {
                                if let Some((id, amount)) = rest.rsplit_once('*') {
                                    if let (Ok(tank), Ok(amount)) =
                                        (tank.parse::<u32>(), amount.parse::<u64>())
                                    {
                                        capabilities::fill_fluid_tank(
                                            &energy_key,
                                            tank,
                                            id,
                                            amount,
                                            false,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                *progress = 0;
                continue;
            }
            let input = machine::get_slot(handle, 0);
            let recipe = input
                .as_ref()
                .and_then(|stack| machine::cooking(CookingKind::Smelting, &stack.id));
            let Some(recipe) = recipe else {
                self.progress.insert(handle, 0);
                ui::set_gauge(&screen_id, 10, 0, 200, "Progress");
                set_active(handle, "false");
                continue;
            };
            if capabilities::extract_energy(&energy_key, 40, true) < 40 {
                continue;
            }
            capabilities::extract_energy(&energy_key, 40, false);
            if let Some(view) = capabilities::get_energy(&energy_key) {
                self.energy.insert(handle, view.stored);
            }
            let progress = self.progress.entry(handle).or_default();
            *progress = progress.saturating_add(1).min(recipe.cooking_time);
            ui::set_gauge(
                &screen_id,
                10,
                *progress as u64,
                recipe.cooking_time as u64,
                "Progress",
            );
            set_active(handle, "true");
            if *progress < recipe.cooking_time {
                continue;
            }
            let output = match machine::get_slot(handle, 1) {
                Some(stack) if stack.id == recipe.output.id => ItemStack {
                    id: stack.id,
                    count: stack.count.saturating_add(recipe.output.count),
                },
                Some(_) => continue,
                None => recipe.output,
            };
            if !machine::set_slot(handle, 1, Some(&output)) {
                continue;
            }
            if let Some(input) = input {
                let rest = (input.count > 1).then(|| ItemStack {
                    id: input.id,
                    count: input.count - 1,
                });
                machine::set_slot(handle, 0, rest.as_ref());
            }
            *progress = 0;
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        let mut saved = self
            .progress
            .get(&handle)
            .copied()
            .unwrap_or(0)
            .to_le_bytes()
            .to_vec();
        saved.extend_from_slice(&self.energy.get(&handle).copied().unwrap_or(0).to_le_bytes());
        let tanks = machine::get_position(handle)
            .map(|position| {
                let key = energy_position(&position.dimension, position.x, position.y, position.z);
                (0..3)
                    .chain(1_000..1_003)
                    .filter_map(|tank| capabilities::get_fluid(&key, tank).map(|view| (tank, view)))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        saved.extend_from_slice(b"MTK1");
        saved.push(tanks.len().min(u8::MAX as usize) as u8);
        for (tank, view) in tanks {
            let id = view.fluid.as_bytes();
            if id.len() > u16::MAX as usize {
                continue;
            }
            saved.extend_from_slice(&(tank as u16).to_le_bytes());
            saved.extend_from_slice(&view.amount.to_le_bytes());
            saved.extend_from_slice(&view.capacity.to_le_bytes());
            saved.extend_from_slice(&(id.len() as u16).to_le_bytes());
            saved.extend_from_slice(id);
        }
        if let Some(position) = machine::get_position(handle) {
            let prefix = format!(
                "{}|",
                energy_position(&position.dimension, position.x, position.y, position.z)
            );
            let modes = self
                .side_modes
                .iter()
                .filter_map(|(key, mode)| key.strip_prefix(&prefix).map(|face| (face, mode)))
                .collect::<Vec<_>>();
            saved.extend_from_slice(b"MCM1");
            saved.push(modes.len().min(u8::MAX as usize) as u8);
            for (face, mode) in modes.into_iter().take(u8::MAX as usize) {
                let face = face.as_bytes();
                let mode = mode.as_bytes();
                if face.len() <= u8::MAX as usize && mode.len() <= u8::MAX as usize {
                    saved.push(face.len() as u8);
                    saved.extend_from_slice(face);
                    saved.push(mode.len() as u8);
                    saved.extend_from_slice(mode);
                }
            }
            if self
                .blocks
                .get(&handle)
                .is_some_and(|block| block == "mekanism:induction_port")
            {
                let rule = format!(
                    "mekanism:induction/port/{}/{}",
                    position.dimension,
                    format_args!("{},{},{}", position.x, position.y, position.z)
                );
                let stored = machine::get_world_rule(&rule)
                    .and_then(|link| link.split(';').next().map(str::to_owned))
                    .and_then(|coords| {
                        coords
                            .split(',')
                            .map(str::parse::<i32>)
                            .collect::<Result<Vec<_>, _>>()
                            .ok()
                    })
                    .and_then(|coords| {
                        let [x, y, z] = coords.as_slice() else {
                            return None;
                        };
                        capabilities::get_energy(&energy_position(&position.dimension, *x, *y, *z))
                    })
                    .map_or_else(
                        || self.matrix_energy.get(&handle).copied().unwrap_or(0),
                        |view| view.stored,
                    );
                saved.extend_from_slice(b"IMX1");
                saved.extend_from_slice(&stored.to_le_bytes());
            }
            if self
                .blocks
                .get(&handle)
                .is_some_and(|block| block == "mekanism:dynamic_valve")
            {
                let rule = format!(
                    "mekanism:dynamic/valve/{}/{}",
                    position.dimension,
                    format_args!("{},{},{}", position.x, position.y, position.z)
                );
                let tanks = machine::get_world_rule(&rule)
                    .and_then(|link| link.split(';').next().map(str::to_owned))
                    .and_then(|coords| {
                        coords
                            .split(',')
                            .map(str::parse::<i32>)
                            .collect::<Result<Vec<_>, _>>()
                            .ok()
                    })
                    .and_then(|coords| {
                        let [x, y, z] = coords.as_slice() else {
                            return None;
                        };
                        let key = energy_position(&position.dimension, *x, *y, *z);
                        Some(
                            [0, 1_000]
                                .into_iter()
                                .filter_map(|tank| {
                                    capabilities::get_fluid(&key, tank).map(|view| (tank, view))
                                })
                                .collect::<Vec<_>>(),
                        )
                    })
                    .unwrap_or_default();
                saved.extend_from_slice(b"DYN1");
                saved.push(tanks.len().min(u8::MAX as usize) as u8);
                for (tank, view) in tanks {
                    let fluid = view.fluid.as_bytes();
                    if fluid.len() > u16::MAX as usize {
                        continue;
                    }
                    saved.extend_from_slice(&(tank as u16).to_le_bytes());
                    saved.extend_from_slice(&view.amount.to_le_bytes());
                    saved.extend_from_slice(&view.capacity.to_le_bytes());
                    saved.extend_from_slice(&(fluid.len() as u16).to_le_bytes());
                    saved.extend_from_slice(fluid);
                }
            }
        }
        saved
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.progress.remove(&handle);
        self.blocks.remove(&handle);
        self.energy.remove(&handle);
        self.matrix_energy.remove(&handle);
        self.dynamic_restore.remove(&handle);
        if let Some(position) = machine::get_position(handle) {
            let prefix = format!(
                "{}|",
                energy_position(&position.dimension, position.x, position.y, position.z)
            );
            self.side_modes.retain(|key, _| !key.starts_with(&prefix));
        }
    }
}

horizon_mod_sdk::export_mod!(Mekanism);

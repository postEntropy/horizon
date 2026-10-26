//! Mekanism: MoreMachine — the factory tier of Mekanism's advanced processors,
//! its large multiblock machines and the silver ore chain.
//!
//! The chemical reactions need Mekanism's own machine recipes, which the game
//! does not carry, so each factory keeps its four slots and shows its tier and
//! parallel operations.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

/// `(prefix, name, parallel operations)`; 0 means unlimited.
const TIERS: [(&str, &str, u64); 9] = [
    ("basic", "Basic", 3),
    ("advanced", "Advanced", 5),
    ("elite", "Elite", 7),
    ("ultimate", "Ultimate", 9),
    ("overclocked", "Overclocked", 11),
    ("dense", "Dense", 13),
    ("multiversal", "Multiversal", 15),
    ("quantum", "Quantum", 17),
    ("creative", "Creative", 0),
];

const MACHINES: [&str; 16] = [
    "centrifuging",
    "crystallizing",
    "dissolving",
    "lathing",
    "liquifying",
    "oxidizing",
    "painting",
    "pigment_extracting",
    "planting",
    "pressing",
    "pressurised_reacting",
    "recycling",
    "replicating",
    "rolling_mill",
    "stamping",
    "washing",
];

/// The singles that are not a tiered factory.
const OTHERS: [&str; 28] = [
    "ambient_gas_collector",
    "author_doll",
    "block_raw_silver",
    "block_silver",
    "chemical_replicator",
    "cnc_lathe",
    "cnc_rolling_mill",
    "cnc_stamper",
    "deepslate_silver_ore",
    "fluid_replicator",
    "large_antiprotonic_nucleosynthesizer",
    "large_chemical_infuser",
    "large_electrolytic_separator",
    "large_gas_burning_generator",
    "large_heat_generator",
    "large_pigment_mixer",
    "large_rotary_condensentrator",
    "large_solar_neutron_activator",
    "large_wind_generator",
    "modeler_doll",
    "planting_station",
    "presser",
    "recycler",
    "replicator",
    "silver_ore",
    "solar_heat_generator",
    "wireless_charging_station",
    "wireless_transmission_station",
];

const TANK_TIERS: [(&str, &str, u64); 4] = [
    ("basic", "Basic", 32_000),
    ("advanced", "Advanced", 64_000),
    ("elite", "Elite", 128_000),
    ("ultimate", "Ultimate", 256_000),
];

fn path_of(value: &str) -> &str {
    value.split([':', '[']).nth(1).unwrap_or(value)
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

fn label(id: u32, y: i32, text: String) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Label,
        x: 8,
        y,
        width: 160,
        height: 10,
        text,
        value: 0,
        max_value: 0,
    }
}

fn factory_screen(name: &str, tier: &str, parallel: u64) -> ScreenSpec {
    ScreenSpec {
        id: "screen:mekmm:factory".into(),
        title: format!("{tier} {name}"),
        width: 176,
        height: 166,
        widgets: vec![
            slot(0, 26, 17, "Input"),
            slot(1, 26, 35, "Input"),
            slot(2, 98, 17, "Output"),
            slot(3, 98, 35, "Output"),
            Widget {
                id: 4,
                kind: WidgetKind::EnergyBar,
                x: 8,
                y: 17,
                width: 12,
                height: 54,
                text: "FE".into(),
                value: 0,
                max_value: 1_000_000,
            },
            label(
                5,
                60,
                if parallel == 0 {
                    "Parallel operations: unlimited".to_string()
                } else {
                    format!("Parallel operations: {parallel}")
                },
            ),
            label(
                6,
                74,
                "Chemical processing runs on Mekanism recipes.".into(),
            ),
        ],
    }
}

fn tank_screen(tier: &str, size: &str, capacity: u64) -> ScreenSpec {
    ScreenSpec {
        id: "screen:mekmm:tank".into(),
        title: format!("{tier} {size} Chemical Tank"),
        width: 176,
        height: 120,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::FluidGauge,
                x: 8,
                y: 18,
                width: 18,
                height: 54,
                text: "mB".into(),
                value: 0,
                max_value: capacity,
            },
            label(1, 18, format!("Capacity: {capacity} mB")),
            label(
                2,
                32,
                "Holds one chemical; right-click with a gauge to read it.".into(),
            ),
        ],
    }
}

struct Mekmm;

impl Mod for Mekmm {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Mekmm, Registration::new().command("mekmm")))
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
        for (tier, _, _) in TIERS {
            for machine in MACHINES {
                hooks = hooks.block(&format!("mekmm:{tier}_{machine}_factory"));
            }
        }
        for (tier, _, _) in TANK_TIERS {
            hooks = hooks.block(&format!("mekmm:{tier}_mid_chemical_tank"));
            hooks = hooks.block(&format!("mekmm:{tier}_max_chemical_tank"));
        }
        for block in OTHERS {
            hooks = hooks.block(&format!("mekmm:{block}"));
            hooks = hooks.item(&format!("mekmm:{block}"));
        }
        for item in [
            "ingot_silver",
            "nugget_silver",
            "dust_silver",
            "dirty_dust_silver",
            "clump_silver",
            "shard_silver",
            "crystal_silver",
            "enriched_silver",
            "raw_silver",
            "silver_ore",
            "uu_matter",
            "scrap",
            "scrap_box",
            "empty_crystal",
            "reflector",
        ] {
            hooks = hooks.item(&format!("mekmm:{item}"));
        }
        for connector in [
            "connector",
            "connector_items",
            "connector_fluids",
            "connector_chemicals",
            "connector_energy",
            "connector_heat",
        ] {
            hooks = hooks.item(&format!("mekmm:{connector}"));
        }
        for (tier, _, _) in TIERS {
            for machine in MACHINES {
                hooks = hooks.item(&format!("mekmm:{tier}_{machine}_factory"));
            }
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);

        if let Some(rest) = path.strip_suffix("_factory") {
            for (tier, tier_name, parallel) in TIERS {
                if let Some(machine) = rest.strip_prefix(&format!("{tier}_")) {
                    let name = machine
                        .split('_')
                        .map(capitalize)
                        .collect::<Vec<_>>()
                        .join(" ");
                    let _ = ui::open_screen(&factory_screen(&name, tier_name, parallel));
                    return EventOutcome::Consume;
                }
            }
        }

        if let Some(rest) = path.strip_suffix("_chemical_tank") {
            for (tier, tier_name, capacity) in TANK_TIERS {
                if let Some(size) = rest.strip_prefix(&format!("{tier}_")) {
                    let capacity = if size == "max" {
                        capacity * 2
                    } else {
                        capacity
                    };
                    let _ = ui::open_screen(&tank_screen(tier_name, size, capacity));
                    return EventOutcome::Consume;
                }
            }
        }

        if !OTHERS.contains(&path) {
            return EventOutcome::Pass;
        }
        let message = match path {
            "silver_ore" | "deepslate_silver_ore" => {
                "⛏️ Silver ore: washes and enriches into silver dust."
            }
            "block_silver" | "block_raw_silver" => "🧱 A block of silver.",
            "chemical_replicator" | "fluid_replicator" | "replicator" => {
                "🧪 Replicator: turns UU matter into items and fluids."
            }
            "presser" | "cnc_lathe" | "cnc_rolling_mill" | "cnc_stamper" => {
                "🔧 CNC machine: shapes metals without a crafting grid."
            }
            "recycler" => "♻️ Recycler: turns junk into scrap for UU matter.",
            "planting_station" => "🌱 Planting station: plants seeds from the machine's buffer.",
            "solar_heat_generator" => "☀️ Solar heat generator: makes heat from sunlight.",
            "wireless_charging_station" => {
                "🔋 Wireless charging station: charges players' gear nearby."
            }
            "wireless_transmission_station" => {
                "📡 Wireless transmission station: moves energy over the air."
            }
            "ambient_gas_collector" => "🌫️ Ambient gas collector: gathers the gas around it.",
            "author_doll" | "modeler_doll" => "🪆 A doll of the mod's maker.",
            other if other.starts_with("large_") => "🏭 Large multiblock machine.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.item);
        if let Some(rest) = path.strip_suffix("_factory") {
            for (tier, tier_name, parallel) in TIERS {
                if rest.starts_with(&format!("{tier}_")) {
                    let msg = if parallel == 0 {
                        format!("🏭 {tier_name} factory: unlimited parallel operations.")
                    } else {
                        format!("🏭 {tier_name} factory: {parallel} parallel operations.")
                    };
                    chat::show(&msg);
                    return EventOutcome::Consume;
                }
            }
        }
        let message = match path {
            "ingot_silver" | "nugget_silver" | "dust_silver" | "dirty_dust_silver"
            | "clump_silver" | "shard_silver" | "crystal_silver" | "enriched_silver"
            | "raw_silver" => "⚪ Silver, in one of the steps of Mekanism's ore processing chain.",
            "uu_matter" => "🔮 UU matter: the feedstock of the replicators.",
            "scrap" | "scrap_box" => {
                "♻️ Scrap: compressed junk that the recycler makes UU matter from."
            }
            "empty_crystal" => "💠 Empty crystal: filled in a crystallizer.",
            "reflector" => "🪞 Reflector: focuses a laser.",
            connector if connector.starts_with("connector") => {
                "🔌 Connector: moves items, fluids, chemicals, energy or heat between machines."
            }
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "mekmm" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "MoreMachine: {} machines across {} tiers ({}); chemical tanks in {} tiers; silver ore chain.",
            MACHINES.len(),
            TIERS.len(),
            TIERS.map(|(_, name, _)| name).join(", "),
            TANK_TIERS.len()
        )))
    }
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

horizon_mod_sdk::export_mod!(Mekmm);

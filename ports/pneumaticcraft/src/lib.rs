//! PneumaticCraft: Repressurized — compressed air as the mod's power, and the
//! machines, drones and assembly lines that run on it.
//!
//! Compressors keep their pressure and the machines their slots; the assembly
//! line, the drones and the upgrades describe themselves.
mod items;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const MACHINE_SCREEN: &str = "screen:pneumaticcraft:machine";
const PRESSURE_SCREEN: &str = "screen:pneumaticcraft:pressure";
const TANK_SCREEN: &str = "screen:pneumaticcraft:tank";
const STORAGE_SCREEN: &str = "screen:pneumaticcraft:storage";

const MAX_PRESSURE: u64 = 20;

const COMPRESSORS: [(&str, &str); 10] = [
    ("manual_compressor", "hand-pumped"),
    ("air_compressor", "a furnace burning coal"),
    ("advanced_air_compressor", "a furnace burning coal, faster"),
    ("liquid_compressor", "a lava bucket"),
    ("advanced_liquid_compressor", "a lava bucket, faster"),
    ("electric_compressor", "Forge Energy"),
    ("thermal_compressor", "the heat of the world around it"),
    ("solar_compressor", "the sun"),
    ("flux_compressor", "Forge Energy, faster"),
    ("creative_compressor", "nothing at all"),
];

const UPGRADES: &[(&str, &str)] = &[
    ("speed", "makes the machine work faster"),
    ("volume", "adds air capacity to the machine"),
    ("security", "keeps the machine's inventory private"),
    ("dispenser", "lets a dispenser use the machine's air"),
    ("range", "extends the machine's reach"),
    ("magnet", "pulls the drops to the machine"),
    ("inventory", "adds an inventory to the machine"),
    ("charging", "charges the gear of the player who owns it"),
    (
        "armor",
        "upgrades a pneumatic helmet, chestplate, leggings or boots",
    ),
    ("air_conditioning", "cools the machine's surroundings"),
    ("muffler", "quietens the machine"),
    ("stomp", "lets the boots stomp the ground"),
    ("flippers", "lets the boots swim"),
    ("scuba", "lets the helmet breathe under water"),
    ("night_vision", "lets the helmet see in the dark"),
    ("jumping", "lets the leggings jump higher"),
    ("item_life", "keeps the machine's items from despawning"),
    ("standby", "turns the machine off when it is idle"),
    (
        "gilded",
        "gives the machine a golden tint, and a little magic",
    ),
    ("ender_visor", "lets the helmet look into the end"),
    ("block_tracker", "shows the tracked block"),
    ("entity_tracker", "shows the tracked entity"),
    ("chunkloader", "keeps the chunk loaded"),
    ("creative", "gives the machine unlimited air"),
    ("thaumcraft", "the old thaumic upgrade"),
    ("minigun", "turns the machine's air into a minigun"),
    ("elytra", "lets the chestplate fly like an elytra"),
    ("radiation_shielding", "shields the wearer from radiation"),
    ("search", "helps find the blocks you look for"),
    ("coordinate_tracker", "tracks a set of coordinates"),
    ("upgrade_matrix", "holds the machine's upgrades together"),
];

fn slot(id: u32, x: i32, y: i32) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Slot,
        x,
        y,
        width: 18,
        height: 18,
        text: String::new(),
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

fn title(value: &str) -> String {
    value
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn pressure_screen(name: &str, source: &str) -> ScreenSpec {
    ScreenSpec {
        id: PRESSURE_SCREEN.into(),
        title: title(name),
        width: 176,
        height: 116,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::EnergyBar,
                x: 8,
                y: 18,
                width: 16,
                height: 54,
                text: "bar".into(),
                value: 0,
                max_value: MAX_PRESSURE,
            },
            label(1, 18, format!("Pressure: up to {MAX_PRESSURE} bar")),
            label(2, 32, format!("Runs on {source}.")),
            label(3, 46, "Air flows through the pressure tubes.".into()),
        ],
    }
}

fn machine_screen(name: &str, slots: u32) -> ScreenSpec {
    let mut widgets: Vec<Widget> = (0..slots)
        .map(|index| slot(index, 26 + index as i32 * 22, 17))
        .collect();
    widgets.push(Widget {
        id: slots,
        kind: WidgetKind::EnergyBar,
        x: 8,
        y: 17,
        width: 12,
        height: 54,
        text: "bar".into(),
        value: 0,
        max_value: MAX_PRESSURE,
    });
    widgets.push(label(slots + 1, 42, title(name)));
    widgets.push(label(
        slots + 2,
        56,
        "It runs on compressed air from the tubes.".into(),
    ));
    ScreenSpec {
        id: MACHINE_SCREEN.into(),
        title: title(name),
        width: 176,
        height: 96,
        widgets,
    }
}

fn grid_screen(name: &str, slots: u32) -> ScreenSpec {
    let rows = slots.div_ceil(9);
    let widgets: Vec<Widget> = (0..slots)
        .map(|index| {
            slot(
                index,
                8 + (index % 9) as i32 * 18,
                18 + (index / 9) as i32 * 18,
            )
        })
        .collect();
    ScreenSpec {
        id: STORAGE_SCREEN.into(),
        title: title(name),
        width: 176,
        height: 18 * 2 + rows * 18,
        widgets,
    }
}

fn tank_screen(name: &str, capacity: u64) -> ScreenSpec {
    ScreenSpec {
        id: TANK_SCREEN.into(),
        title: title(name),
        width: 176,
        height: 110,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::FluidGauge,
                x: 8,
                y: 18,
                width: 16,
                height: 54,
                text: "mB".into(),
                value: 0,
                max_value: capacity,
            },
            label(1, 18, format!("Capacity: {capacity} mB")),
        ],
    }
}

fn path_of(value: &str) -> &str {
    value.split([':', '[']).nth(1).unwrap_or(value)
}

struct PneumaticCraft;

impl Mod for PneumaticCraft {
    fn init() -> Result<(Self, Registration), String> {
        Ok((PneumaticCraft, Registration::new().command("pneumatic")))
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
        for (block, _) in COMPRESSORS {
            hooks = hooks.block(&format!("pneumaticcraft:{block}"));
        }
        for block in [
            "thermopneumatic_processing_plant",
            "fluid_mixer",
            "assembly_controller",
            "assembly_drill",
            "assembly_laser",
            "assembly_platform",
            "assembly_io_unit_import",
            "assembly_io_unit_export",
            "tag_workbench",
            "aerial_interface",
            "drone_interface",
            "smart_chest",
            "reinforced_chest",
            "refinery",
            "refinery_output",
            "pressure_chamber_glass",
            "pressure_chamber_interface",
            "pressure_chamber_valve",
            "reinforced_pressure_chamber_valve",
            "charging_station",
            "programmable_controller",
            "security_station",
            "vacuum_trap",
            "vacuum_pump",
            "uv_light_box",
            "sentry_turret",
            "air_cannon",
            "heat_sink",
            "heat_pipe",
            "gas_lift",
            "elevator_base",
            "elevator_frame",
            "elevator_caller",
            "display_shelf",
            "display_table",
            "liquid_hopper",
            "omnidirectional_hopper",
            "pneumatic_dynamo",
            "pneumatic_generator",
            "spawner_extractor",
            "pressurized_spawner",
            "empty_spawner",
            "small_tank",
            "medium_tank",
            "large_tank",
            "huge_tank",
            "etching_tank",
            "kerosene_lamp",
        ] {
            hooks = hooks.block(&format!("pneumaticcraft:{block}"));
        }
        for item in items::ITEMS {
            hooks = hooks.item(&format!("pneumaticcraft:{item}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);

        if let Some((_, source)) = COMPRESSORS.iter().find(|(block, _)| *block == path) {
            let _ = ui::open_screen(&pressure_screen(path, source));
            return EventOutcome::Consume;
        }
        let slots = match path {
            "smart_chest" | "reinforced_chest" => Some(27),
            "assembly_controller" | "tag_workbench" | "aerial_interface" | "drone_interface" => {
                Some(9)
            }
            "assembly_io_unit_import" | "assembly_io_unit_export" => Some(1),
            "thermopneumatic_processing_plant" | "fluid_mixer" => Some(3),
            _ => None,
        };
        if let Some(slots) = slots {
            let screen = if slots > 4 {
                grid_screen(path, slots)
            } else {
                machine_screen(path, slots)
            };
            let _ = ui::open_screen(&screen);
            return EventOutcome::Consume;
        }
        if path.ends_with("_tank") {
            let capacity = if path.starts_with("huge") {
                512_000
            } else if path.starts_with("large") {
                128_000
            } else if path.starts_with("medium") {
                32_000
            } else {
                8_000
            };
            let _ = ui::open_screen(&tank_screen(path, capacity));
            return EventOutcome::Consume;
        }

        let message = match path {
            "refinery" => "🛢️ Refinery: cracks oil into diesel, kerosene, gasoline and LPG.",
            "refinery_output" => "🛢️ Refinery output: where the refinery's products come out.",
            "assembly_controller" => "🏭 Assembly controller: the brain of the assembly line.",
            "assembly_drill" => "🏭 Assembly drill: drills the assembly line's items.",
            "assembly_laser" => "🏭 Assembly laser: cuts the assembly line's items.",
            "assembly_platform" => "🏭 Assembly platform: holds the item being assembled.",
            "pressure_chamber_glass"
            | "pressure_chamber_valve"
            | "reinforced_pressure_chamber_valve" => {
                "🌡️ A pressure chamber part: the chamber is built from these."
            }
            "pressure_chamber_interface" => {
                "🌡️ Pressure chamber interface: the chamber's inventory."
            }
            "charging_station" => "🔌 Charging station: fills pneumatic gear and armor with air.",
            "programmable_controller" => {
                "🤖 Programmable controller: runs a program on the drones."
            }
            "security_station" => "🔒 Security station: keeps the area private.",
            "vacuum_trap" => "🕳️ Vacuum trap: sucks the mobs into it.",
            "vacuum_pump" => "💨 Vacuum pump: makes a vacuum out of air.",
            "uv_light_box" => "💡 UV light box: grows the mod's plants faster.",
            "sentry_turret" => "🔫 Sentry turret: shoots at what the security station forbids.",
            "air_cannon" => "💨 Air cannon: throws what it is given at what it is aimed at.",
            "heat_sink" => "🌡️ Heat sink: cools the machines beside it.",
            "heat_pipe" => "🌡️ Heat pipe: carries heat between machines.",
            "gas_lift" => "🛢️ Gas lift: pulls more oil out of a well.",
            "elevator_base" | "elevator_frame" | "elevator_caller" => {
                "🛗 An elevator part: the base calls the cabin to its floor."
            }
            "display_shelf" | "display_table" => "📦 A display shelf: shows an item.",
            "liquid_hopper" | "omnidirectional_hopper" => {
                "🪣 A hopper: moves items or fluids in the direction it points."
            }
            "pneumatic_dynamo" | "pneumatic_generator" => {
                "⚡ Turns compressed air back into Forge Energy."
            }
            "spawner_extractor" => "🕸️ Spawner extractor: pulls the essence out of a spawner.",
            "pressurized_spawner" | "empty_spawner" => "🕸️ A spawner that works on compressed air.",
            "kerosene_lamp" => "🏮 Kerosene lamp: a bright light that burns kerosene.",
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
        if let Some(upgrade) = path.strip_suffix("_upgrade")
            && let Some((_, effect)) = UPGRADES.iter().find(|(name, _)| *name == upgrade)
        {
            chat::show(&format!("⬆️ {} upgrade: {effect}.", title(upgrade)));
            return EventOutcome::Consume;
        }
        let message = match path {
            "air_canister" => "💨 Air canister: carries 3,000 mL of air.",
            "air_canister_array" => "💨 Air canister array: many canisters in one.",
            "amadron_tablet" => "📱 Amadron tablet: trades items and fluids with other players.",
            "gps_tool" => "📍 GPS tool: marks a block the drones can fly to.",
            "gps_area_tool" => "📍 GPS area tool: marks an area of blocks.",
            "jackhammer" => "⛏️ Jackhammer: mines with compressed air.",
            "manometer" => "🌡️ Manometer: reads the pressure of a tube.",
            "memory_stick" => "💾 Memory stick: copies a machine's settings.",
            "drone" => "🛸 Drone: flies a program, carries items and places blocks.",
            "logistics_drone" => "🛸 Logistics drone: keeps the logistics frames stocked.",
            "collector_drone" => "🛸 Collector drone: picks items up.",
            "guard_drone" => "🛸 Guard drone: patrols and shoots.",
            "harvesting_drone" => "🛸 Harvesting drone: farms for you.",
            "assembly_program_drill"
            | "assembly_program_laser"
            | "assembly_program_drill_laser" => {
                "🏭 An assembly program: tells the assembly line what to do."
            }
            "programmer" => "🖥️ Programmer: writes the drone programs.",
            "programming_puzzle" => "🧩 Programming puzzle: a piece of a drone program.",
            "bandage" => "🩹 Bandage: stops the bleeding.",
            "plastic" => "⚙️ Plastic: made from oil, used everywhere.",
            _ if path.ends_with("_bucket") => "🪣 A bucket of a PneumaticCraft fluid.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "pneumatic" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "PneumaticCraft: {} compressors up to {MAX_PRESSURE} bar; {} upgrades; drones, the assembly line and the refinery.",
            COMPRESSORS.len(),
            UPGRADES.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(PneumaticCraft);

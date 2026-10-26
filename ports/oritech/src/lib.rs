//! Oritech: the machines, generators, reactors and accelerators of a tech mod
//! built around addons.
//!
//! Machines keep their slots and their energy buffer; the storages and tanks
//! report what they hold. The processing itself runs on Oritech's recipes.
mod items;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const MACHINE_SCREEN: &str = "screen:oritech:machine";
const STORAGE_SCREEN: &str = "screen:oritech:storage";
const TANK_SCREEN: &str = "screen:oritech:tank";
const ENERGY_SCREEN: &str = "screen:oritech:energy";

const MACHINES: &[(&str, u32)] = &[
    ("assembler_block", 3),
    ("atomic_forge_block", 3),
    ("centrifuge_block", 3),
    ("deep_drill_block", 3),
    ("enchanter_block", 3),
    ("foundry_block", 3),
    ("laser_arm_block", 3),
    ("machine_frame_block", 3),
    ("powered_furnace_block", 3),
    ("pulverizer_block", 3),
    ("refinery_block", 3),
    ("spawner_controller_block", 3),
    ("charger_block", 1),
    ("cooler_block", 1),
    ("destroyer_block", 1),
    ("placer_block", 1),
    ("pump_block", 1),
    ("treefeller_block", 1),
    ("unstable_container", 9),
];

const STORAGES: &[(&str, u32)] = &[
    ("small_storage_block", 27),
    ("large_storage_block", 54),
    ("creative_storage_block", 27),
];

const GENERATORS: [&str; 6] = [
    "basic_generator_block",
    "big_solar_panel_block",
    "bio_generator_block",
    "fuel_generator_block",
    "lava_generator_block",
    "steam_engine_block",
];

const ADDONS: &[(&str, &str)] = &[
    ("acceptor", "lets the machine take items from any side"),
    ("burst", "adds a short burst of speed, at a cost in energy"),
    ("capacitor", "adds an energy buffer to the machine"),
    ("combi", "combines several addons in one"),
    ("efficiency", "spends less energy per operation"),
    ("fluid", "lets the machine work with fluids"),
    ("hunter", "makes the machine hunt the mobs around it"),
    (
        "inventory_proxy",
        "lets the machine reach a distant inventory",
    ),
    ("processing", "adds another output to the operation"),
    ("redstone", "pauses the machine on a redstone signal"),
    ("silk_touch", "gives the blocks it breaks silk touch"),
    ("speed", "makes the machine faster"),
    ("ultimate", "all the bonuses at once"),
    ("yield", "adds drops to every operation"),
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
        text: "RF".into(),
        value: 0,
        max_value: 1_000_000,
    });
    widgets.push(label(slots + 1, 42, title(name)));
    widgets.push(label(
        slots + 2,
        56,
        "Addons plugged into the frame change what it does.".into(),
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

fn gauge_screen(screen: &str, name: &str, unit: &str, capacity: u64) -> ScreenSpec {
    ScreenSpec {
        id: screen.into(),
        title: title(name),
        width: 176,
        height: 110,
        widgets: vec![
            Widget {
                id: 0,
                kind: if unit == "RF" {
                    WidgetKind::EnergyBar
                } else {
                    WidgetKind::FluidGauge
                },
                x: 8,
                y: 18,
                width: 16,
                height: 54,
                text: unit.into(),
                value: 0,
                max_value: capacity,
            },
            label(1, 18, format!("Capacity: {capacity} {unit}")),
        ],
    }
}

fn path_of(value: &str) -> &str {
    value.split([':', '[']).nth(1).unwrap_or(value)
}

struct Oritech;

impl Mod for Oritech {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Oritech, Registration::new().command("oritech")))
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
        for (block, _) in MACHINES.iter().chain(STORAGES) {
            hooks = hooks.block(&format!("oritech:{block}"));
        }
        for block in GENERATORS {
            hooks = hooks.block(&format!("oritech:{block}"));
        }
        for block in [
            "small_tank_block",
            "creative_tank_block",
            "reactor_controller",
            "reactor_condenser",
            "reactor_heat_pipe",
            "reactor_reflector",
            "reactor_rod",
            "reactor_vent",
            "reactor_absorber_port",
            "reactor_energy_port",
            "reactor_fuel_port",
            "reactor_redstone_port",
            "item_filter_block",
            "pipe_booster_block",
            "particle_collector_block",
            "spawner_cage_block",
            "accelerator_controller",
            "accelerator_motor",
            "accelerator_ring",
            "accelerator_sensor",
            "resource_node_coal",
            "resource_node_copper",
            "resource_node_diamond",
            "resource_node_emerald",
            "resource_node_gold",
            "resource_node_iron",
            "resource_node_nickel",
            "resource_node_platinum",
            "resource_node_redstone",
            "resource_node_uranium",
        ] {
            hooks = hooks.block(&format!("oritech:{block}"));
        }
        for (addon, _) in ADDONS {
            hooks = hooks.item(&format!("oritech:machine_{addon}_addon"));
        }
        for item in items::ITEMS {
            hooks = hooks.item(&format!("oritech:{item}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);

        if let Some((_, slots)) = STORAGES.iter().find(|(block, _)| *block == path) {
            let _ = ui::open_screen(&grid_screen(path, *slots));
            return EventOutcome::Consume;
        }
        if let Some((_, slots)) = MACHINES.iter().find(|(block, _)| *block == path) {
            let _ = ui::open_screen(&machine_screen(path, *slots));
            return EventOutcome::Consume;
        }
        if GENERATORS.contains(&path) {
            let _ = ui::open_screen(&gauge_screen(ENERGY_SCREEN, path, "RF", 1_000_000));
            return EventOutcome::Consume;
        }
        if path.ends_with("_tank_block") {
            let _ = ui::open_screen(&gauge_screen(TANK_SCREEN, path, "mB", 64_000));
            return EventOutcome::Consume;
        }
        if path.starts_with("reactor_") {
            chat::show("☢️ A reactor part: build the reactor out of these and feed it fuel.");
            return EventOutcome::Consume;
        }
        if path.starts_with("accelerator_") {
            chat::show("🌀 An accelerator part: the ring throws particles for the science.");
            return EventOutcome::Consume;
        }
        if path.starts_with("resource_node_") {
            chat::show("⛏️ A resource node: the deep drill mines it dry.");
            return EventOutcome::Consume;
        }
        let message = match path {
            "item_filter_block" => "🔎 Item filter: decides what a pipe lets through.",
            "pipe_booster_block" => "🚀 Pipe booster: speeds up the pipes around it.",
            "particle_collector_block" => {
                "✨ Particle collector: gathers what the accelerator throws."
            }
            "spawner_cage_block" => {
                "🕸️ Spawner cage: holds a mob's soul for the spawner controller."
            }
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
        if let Some(addon) = path
            .strip_prefix("machine_")
            .and_then(|rest| rest.strip_suffix("_addon"))
            && let Some((_, effect)) = ADDONS.iter().find(|(name, _)| *name == addon)
        {
            chat::show(&format!("🔧 Machine addon: {effect}."));
            return EventOutcome::Consume;
        }
        let message = match path {
            "advanced_augment_station" | "arcane_augment_station" | "simple_augment_station" => {
                "🧪 Augment station: fits augments into your gear."
            }
            "augment_application_block" => "🧪 Augment application block: applies the augments.",
            "drone" => "🛸 Drone: flies to a port and carries what it is told to.",
            "banana" => "🍌 A banana. It does nothing, and that is the point.",
            _ if path.ends_with("_ingot")
                || path.ends_with("_dust")
                || path.ends_with("_nugget") =>
            {
                "⚙️ A metal of Oritech, ready for its machines."
            }
            _ if path.ends_with("_gem") || path.ends_with("_crystal") => {
                "💠 A crystal the accelerator and the forge work with."
            }
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "oritech" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Oritech: {} machines, {} storages, {} generators and {} addons.",
            MACHINES.len(),
            STORAGES.len(),
            GENERATORS.len(),
            ADDONS.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(Oritech);

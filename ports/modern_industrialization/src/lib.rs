//! Modern Industrialization: the machine tiers, multiblocks, hatches and
//! hundreds of materials and fluids of a full industrial chain.
//!
//! Machines keep their item slots and their energy; hatches, tanks, barrels and
//! storage units report what they hold. The recipes themselves need MI's own
//! machine data, which the game does not carry.
mod items;

mod blocks;

use blocks::{ENERGY_BLOCKS, FLUID_BLOCKS, ITEM_BLOCKS, TEXT_BLOCKS};
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const MACHINE_SCREEN: &str = "screen:modern_industrialization:machine";
const TANK_SCREEN: &str = "screen:modern_industrialization:tank";
const ENERGY_SCREEN: &str = "screen:modern_industrialization:energy";
const STORAGE_SCREEN: &str = "screen:modern_industrialization:storage";

fn path_of(value: &str) -> &str {
    value.split([':', '[']).nth(1).unwrap_or(value)
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

fn describe(path: &str) -> &'static str {
    if path.ends_with("_ore") {
        "⛏️ An ore: mine it, then macerate it into dust."
    } else if path.contains("_machine_casing") || path.ends_with("_casing") {
        "🧱 A machine casing: the frame of a tier's machines."
    } else if path.ends_with("_hull") {
        "🏗️ A machine hull: the base every machine is built on."
    } else if path.ends_with("_bricks") {
        "🧱 Fireproof bricks for the multiblocks."
    } else if path.ends_with("_pipe") || path.ends_with("_cable") {
        "🔗 A pipe or cable: moves items, fluids or power between machines."
    } else if path.ends_with("_block") || path.ends_with("_tank") || path.ends_with("_barrel") {
        "🧱 A storage block of a material."
    } else {
        "🏭 A machine or a fluid of Modern Industrialization."
    }
}

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

fn grid_screen(screen: &str, name: &str, slots: u32) -> ScreenSpec {
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
        id: screen.into(),
        title: title(name),
        width: 176,
        height: 18 * 2 + rows * 18,
        widgets,
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
        text: "EU".into(),
        value: 0,
        max_value: 1_000_000,
    });
    widgets.push(label(slots + 1, 42, title(name)));
    widgets.push(label(
        slots + 2,
        56,
        "Recipes run on Modern Industrialization's machine data.".into(),
    ));
    ScreenSpec {
        id: MACHINE_SCREEN.into(),
        title: title(name),
        width: 176,
        height: 96,
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
                kind: if unit == "EU" {
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

struct ModernIndustrialization;

impl Mod for ModernIndustrialization {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ModernIndustrialization, Registration::new().command("mi")))
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
        for (block, _) in ITEM_BLOCKS {
            hooks = hooks.block(&format!("modern_industrialization:{block}"));
        }
        for (block, _) in FLUID_BLOCKS {
            hooks = hooks.block(&format!("modern_industrialization:{block}"));
        }
        for (block, _) in ENERGY_BLOCKS {
            hooks = hooks.block(&format!("modern_industrialization:{block}"));
        }
        for block in TEXT_BLOCKS {
            hooks = hooks.block(&format!("modern_industrialization:{block}"));
        }
        for item in items::ITEMS {
            hooks = hooks.item(&format!("modern_industrialization:{item}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);

        if let Some((_, slots)) = ITEM_BLOCKS.iter().find(|(block, _)| *block == path) {
            if *slots > 4 {
                let _ = ui::open_screen(&grid_screen(STORAGE_SCREEN, path, *slots));
            } else {
                let _ = ui::open_screen(&machine_screen(path, *slots));
            }
            return EventOutcome::Consume;
        }
        if let Some((_, capacity)) = FLUID_BLOCKS.iter().find(|(block, _)| *block == path) {
            let _ = ui::open_screen(&gauge_screen(TANK_SCREEN, path, "mB", *capacity));
            return EventOutcome::Consume;
        }
        if let Some((_, capacity)) = ENERGY_BLOCKS.iter().find(|(block, _)| *block == path) {
            let _ = ui::open_screen(&gauge_screen(ENERGY_SCREEN, path, "EU", *capacity));
            return EventOutcome::Consume;
        }
        if TEXT_BLOCKS.contains(&path) {
            chat::show(describe(path));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.item);
        let message = match path {
            "wrench" => "🔧 Wrench: rotates machines and picks them up unchanged.",
            "hammer" => "🔨 Hammer: shapes plates and curved plates by hand.",
            "forge_hammer" => "🔨 Forge hammer: the first machine of the bronze tier.",
            "guidebook" => "📖 Guidebook: everything Modern Industrialization makes.",
            "uu_matter" => "🔮 UU matter: the replicator's feedstock.",
            _ if path.ends_with("_bucket") => "🪣 A bucket of a Modern Industrialization fluid.",
            _ if path.ends_with("_ingot")
                || path.ends_with("_dust")
                || path.ends_with("_plate")
                || path.ends_with("_rod")
                || path.ends_with("_bolt")
                || path.ends_with("_ring")
                || path.ends_with("_gear")
                || path.ends_with("_wire")
                || path.ends_with("_cable")
                || path.ends_with("_double_ingot")
                || path.ends_with("_curved_plate")
                || path.ends_with("_drill_head")
                || path.ends_with("_blade") =>
            {
                "⚙️ A material component: plates, rods, gears and wires feed the machines."
            }
            _ if path.ends_with("_hull") => "🏗️ A machine hull: the base of every machine.",
            _ if path.ends_with("_casing") => "🧱 A machine casing.",
            _ if path.ends_with("_hatch") => {
                "🔌 A hatch: feeds items, fluids or energy into a multiblock."
            }
            _ if path.ends_with("_tank") || path.ends_with("_barrel") => "🛢️ Fluid storage.",
            _ if path.ends_with("_upgrade") => "⬆️ An overclock upgrade for a machine.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "mi" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Modern Industrialization: {} item machines and hatches, {} fluid and {} energy blocks, all in four tiers.",
            ITEM_BLOCKS.len(),
            FLUID_BLOCKS.len(),
            ENERGY_BLOCKS.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(ModernIndustrialization);

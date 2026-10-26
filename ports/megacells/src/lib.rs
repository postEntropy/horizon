//! MEGA Cells: AE2 storage cells and crafting CPUs in sizes past the vanilla
//! ones — 1M to 256M bytes — plus the bulk cell and the sky stone blocks.
//!
//! The cells keep their data in AE2's own cell format, so the port reports
//! each cell's capacity and kind, and the multiblock parts keep their slots.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

/// `(size, bytes)`; 1k = 1,024 bytes in AE2's units.
const SIZES: [(&str, u64); 5] = [
    ("1m", 1_048_576),
    ("4m", 4_194_304),
    ("16m", 16_777_216),
    ("64m", 67_108_864),
    ("256m", 268_435_456),
];

const KINDS: [(&str, &str); 7] = [
    ("item", "Item"),
    ("fluid", "Fluid"),
    ("chemical", "Chemical"),
    ("experience", "Experience"),
    ("mana", "Mana"),
    ("soul", "Soul"),
    ("source", "Source"),
];

const ENERGY_CAPACITY: u64 = 67_108_864;

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

fn pattern_screen(title: &str) -> ScreenSpec {
    let mut widgets: Vec<Widget> = (0..9)
        .map(|index| slot(index, 8 + index as i32 * 18, 18, ""))
        .collect();
    widgets.push(label(9, 42, "Patterns and their inputs.".into()));
    ScreenSpec {
        id: "screen:megacells:patterns".into(),
        title: title.into(),
        width: 176,
        height: 100,
        widgets,
    }
}

fn cpu_screen(title: &str, storage: u64) -> ScreenSpec {
    ScreenSpec {
        id: "screen:megacells:cpu".into(),
        title: title.into(),
        width: 176,
        height: 130,
        widgets: vec![
            label(0, 18, format!("Crafting storage: {storage} bytes")),
            label(1, 32, "4 co-processing threads per accelerator.".into()),
            Widget {
                id: 2,
                kind: WidgetKind::ProgressArrow,
                x: 8,
                y: 52,
                width: 24,
                height: 17,
                text: "CPU".into(),
                value: 0,
                max_value: 100,
            },
            label(3, 78, "The CPU runs the patterns its storage holds.".into()),
        ],
    }
}

fn energy_screen() -> ScreenSpec {
    ScreenSpec {
        id: "screen:megacells:energy".into(),
        title: "MEGA Energy Cell".into(),
        width: 176,
        height: 120,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::EnergyBar,
                x: 8,
                y: 18,
                width: 16,
                height: 54,
                text: "AE".into(),
                value: 0,
                max_value: ENERGY_CAPACITY,
            },
            label(1, 18, format!("Capacity: {ENERGY_CAPACITY} AE")),
            label(2, 32, "Passive drain: 0 AE/t while idle.".into()),
        ],
    }
}

struct MegaCells;

fn cell_of(path: &str) -> Option<(String, u64)> {
    let (_, size) = SIZES
        .iter()
        .find(|(size, _)| path.ends_with(&format!("_{size}")))?;
    let portable = path.starts_with("portable_");
    let kind = KINDS
        .iter()
        .find(|(kind, _)| path.contains(&format!("{kind}_")) || path.contains(&format!("_{kind}_")))
        .map(|(_, name)| *name)
        .unwrap_or("Item");
    let name = if portable { "Portable" } else { "Storage" };
    Some((format!("{kind} {name}"), *size))
}

impl Mod for MegaCells {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MegaCells, Registration::new().command("megacells")))
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
        for block in [
            "mega_crafting_unit",
            "mega_crafting_accelerator",
            "mega_crafting_monitor",
            "mega_energy_cell",
            "mega_interface",
            "mega_emc_interface",
            "mega_pattern_provider",
            "sky_bronze_block",
            "sky_osmium_block",
            "sky_steel_block",
        ] {
            hooks = hooks.block(&format!("megacells:{block}"));
        }
        for (size, _) in SIZES {
            hooks = hooks.block(&format!("megacells:{size}_crafting_storage"));
        }
        for (kind, _) in KINDS {
            for (size, _) in SIZES {
                hooks = hooks.item(&format!("megacells:{kind}_storage_cell_{size}"));
                hooks = hooks.item(&format!("megacells:portable_{kind}_cell_{size}"));
            }
        }
        hooks
            .item("megacells:bulk_item_cell")
            .item("megacells:compression_card")
            .item("megacells:decompression_module")
            .item("megacells:greater_energy_card")
            .item("megacells:cell_dock")
            .item("megacells:accumulation_processor")
            .item("megacells:portable_cell_workbench")
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);

        if let Some((size, bytes)) = SIZES
            .iter()
            .find(|(size, _)| path.ends_with(&format!("{size}_crafting_storage")))
        {
            let _ = ui::open_screen(&cpu_screen("Crafting Storage", *bytes));
            chat::show(&format!("🧮 Crafting storage {size}: {bytes} bytes."));
            return EventOutcome::Consume;
        }

        match path {
            "mega_crafting_unit" | "mega_crafting_accelerator" | "mega_crafting_monitor" => {
                let _ = ui::open_screen(&cpu_screen("MEGA Crafting CPU", 0));
            }
            "mega_interface" | "mega_emc_interface" => {
                let _ = ui::open_screen(&pattern_screen("MEGA Interface"));
            }
            "mega_pattern_provider" => {
                let _ = ui::open_screen(&pattern_screen("MEGA Pattern Provider"));
            }
            "mega_energy_cell" => {
                let _ = ui::open_screen(&energy_screen());
                chat::show("🔋 MEGA energy cell: 64 MAE of buffer for the network.");
            }
            "sky_bronze_block" | "sky_osmium_block" | "sky_steel_block" => {
                chat::show("🪨 Sky stone block: decoration and crafting material for MEGA parts.");
            }
            _ => return EventOutcome::Pass,
        }
        EventOutcome::Consume
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.item);
        if let Some((name, bytes)) = cell_of(path) {
            chat::show(&format!("💾 {name} cell: {bytes} bytes."));
            return EventOutcome::Consume;
        }
        let message = match path {
            "bulk_item_cell" => "💾 Bulk item cell: stores one item in enormous quantity.",
            "compression_card" => {
                "🃏 Compression card: auto-compresses ores and ingots into blocks."
            }
            "decompression_module" => "🃏 Decompression module: splits blocks back into items.",
            "greater_energy_card" => {
                "🃏 Greater energy card: feeds the network's energy into cells."
            }
            "cell_dock" => "🔌 Cell dock: keeps up to five storage cells on the network.",
            "accumulation_processor" => "🔧 Accumulation processor: MEGA's advanced circuit.",
            "portable_cell_workbench" => "🧰 Portable cell workbench: partitions a cell on the go.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "megacells" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "MEGA Cells: sizes {}; kinds {}; energy cell {ENERGY_CAPACITY} AE.",
            SIZES
                .iter()
                .map(|(size, bytes)| format!("{size} ({bytes} B)"))
                .collect::<Vec<_>>()
                .join(", "),
            KINDS.map(|(_, name)| name).join(", ")
        )))
    }
}

horizon_mod_sdk::export_mod!(MegaCells);

//! Cooking for Blockheads: the kitchen multiblock's fridge, oven, counter and
//! cabinet store items, and the oven smelts from its own slots.
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    machine::{self, CookingKind, ItemStack},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const COLORS: [&str; 16] = [
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

const FRIDGE_SLOTS: u32 = 54;
const CABINET_SLOTS: u32 = 27;

#[derive(Clone, Copy, Default)]
struct Oven {
    progress: u32,
    burn: u32,
    burn_max: u32,
}

impl Oven {
    fn encode(self) -> Vec<u8> {
        let mut out = Vec::with_capacity(12);
        out.extend_from_slice(&self.progress.to_le_bytes());
        out.extend_from_slice(&self.burn.to_le_bytes());
        out.extend_from_slice(&self.burn_max.to_le_bytes());
        out
    }

    fn decode(bytes: &[u8]) -> Self {
        let mut oven = Oven::default();
        if bytes.len() >= 12 {
            oven.progress = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
            oven.burn = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
            oven.burn_max = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        }
        oven
    }
}

fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

/// The block without its colour prefix, so every colour of a family shares a
/// behaviour.
fn family(path: &str) -> &str {
    COLORS
        .iter()
        .find_map(|color| {
            path.strip_prefix(color)
                .and_then(|rest| rest.strip_prefix('_'))
        })
        .unwrap_or(path)
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

/// A grid of slots, nine to a row.
fn grid(count: u32, top: i32) -> Vec<Widget> {
    (0..count)
        .map(|index| {
            let column = index % 9;
            let row = index / 9;
            slot(index, 8 + column as i32 * 18, top + row as i32 * 18, "")
        })
        .collect()
}

fn storage_screen(title: &str, slots: u32) -> ScreenSpec {
    let rows = slots.div_ceil(9);
    ScreenSpec {
        id: "screen:cookingforblockheads:storage".into(),
        title: title.into(),
        width: (8 * 2 + 9 * 18) as u32,
        height: 18 * 2 + rows * 18,
        widgets: grid(slots, 18),
    }
}

fn oven_screen() -> ScreenSpec {
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
                max_value: 200,
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
                max_value: 200,
            },
        ],
    }
}

fn cooking_table_screen() -> ScreenSpec {
    let mut widgets = grid(9, 17);
    widgets.push(slot(9, 124, 35, "Result"));
    widgets.push(label(10, 76, "Cooking Table"));
    ScreenSpec {
        id: "screen:cookingforblockheads:cooking_table".into(),
        title: "Cooking Table".into(),
        width: 176,
        height: 166,
        widgets,
    }
}

struct CookingForBlockheads {
    ovens: HashMap<u32, Oven>,
}

impl Mod for CookingForBlockheads {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            CookingForBlockheads {
                ovens: HashMap::new(),
            },
            Registration::new().command("kitchen"),
        ))
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
        for color in COLORS {
            for family in [
                "fridge",
                "oven",
                "counter",
                "cabinet",
                "sink",
                "cooking_table",
                "connector",
                "kitchen_floor",
            ] {
                hooks = hooks.block(&format!("cookingforblockheads:{color}_{family}"));
            }
        }
        for block in [
            "cabinet",
            "connector",
            "cooking_table",
            "counter",
            "cow_jar",
            "cutting_board",
            "fruit_basket",
            "milk_jar",
            "sink",
            "spice_rack",
            "toaster",
            "tool_rack",
        ] {
            hooks = hooks.block(&format!("cookingforblockheads:{block}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let family = family(path_of(&event.state));
        let screen = match family {
            "fridge" => Some(storage_screen("Fridge", FRIDGE_SLOTS)),
            "cabinet" => Some(storage_screen("Cabinet", CABINET_SLOTS)),
            "counter" => Some(storage_screen("Counter", 1)),
            "oven" => Some(oven_screen()),
            "cooking_table" => Some(cooking_table_screen()),
            _ => None,
        };
        if let Some(screen) = screen {
            let _ = ui::open_screen(&screen);
            return EventOutcome::Consume;
        }

        let message = match family {
            "sink" => "🚰 The sink gives infinite water while the kitchen is connected.",
            "connector" => "🔗 Connector: joins counters, fridges and ovens into one kitchen.",
            "kitchen_floor" => {
                "🧱 Kitchen floor: walkable tiles that count as part of the kitchen."
            }
            "cow_jar" | "milk_jar" => "🥛 Milk jar: the kitchen counts it as a milk source.",
            "toaster" => "🍞 Toaster: toasts bread from the kitchen's stock.",
            "cutting_board" => "🔪 Cutting board: prepares ingredients for the kitchen.",
            "spice_rack" => "🧂 Spice rack: supplies spices to every recipe.",
            "tool_rack" => "🍴 Tool rack: keeps the kitchen's utensils handy.",
            "fruit_basket" => "🧺 Fruit basket: holds the kitchen's fruit.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "kitchen" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(
            "Cooking for Blockheads: counters connect the kitchen; fridges, ovens, cabinets and the cooking table keep their own slots and the oven smelts from them."
                .into(),
        ))
    }
}

horizon_mod_sdk::export_mod!(CookingForBlockheads);

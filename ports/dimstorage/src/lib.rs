//! DimStorage: the Dimensional Chest and Dimensional Tank, each block sharing
//! its contents with every block on the same frequency.
//!
//! The chest's slots live in its block entity; the frequency is kept per
//! position so the buttons on its screen can change it.
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const CHEST_SLOTS: u32 = 27;
const MAX_FREQUENCY: u32 = 4095;
const SCREEN: &str = "screen:dimstorage:chest";

struct DimStorage {
    /// `dimension:x,y,z` -> frequency.
    frequencies: HashMap<String, u32>,
    /// The block whose screen is open, for the buttons.
    open: Option<String>,
}

fn position(dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("{dimension}:{x},{y},{z}")
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

fn chest_screen(frequency: u32, locked: bool) -> ScreenSpec {
    let mut widgets: Vec<Widget> = (0..CHEST_SLOTS)
        .map(|index| {
            let column = index % 9;
            let row = index / 9;
            slot(index, 8 + column as i32 * 18, 18 + row as i32 * 18)
        })
        .collect();
    widgets.push(Widget {
        id: 100,
        kind: WidgetKind::Button,
        x: 8,
        y: 78,
        width: 18,
        height: 14,
        text: "-".into(),
        value: frequency as u64,
        max_value: MAX_FREQUENCY as u64,
    });
    widgets.push(Widget {
        id: 101,
        kind: WidgetKind::Button,
        x: 30,
        y: 78,
        width: 18,
        height: 14,
        text: "+".into(),
        value: frequency as u64,
        max_value: MAX_FREQUENCY as u64,
    });
    widgets.push(Widget {
        id: 102,
        kind: WidgetKind::Button,
        x: 52,
        y: 78,
        width: 18,
        height: 14,
        text: "Lock".into(),
        value: u64::from(locked),
        max_value: 1,
    });
    widgets.push(Widget {
        id: 103,
        kind: WidgetKind::Label,
        x: 76,
        y: 81,
        width: 92,
        height: 10,
        text: format!(
            "Frequency: {frequency}{}",
            if locked { " (locked)" } else { "" }
        ),
        value: 0,
        max_value: 0,
    });
    ScreenSpec {
        id: SCREEN.into(),
        title: format!("Dimensional Chest · {frequency}"),
        width: 176,
        height: 100,
        widgets,
    }
}

fn tank_screen(frequency: u32) -> ScreenSpec {
    ScreenSpec {
        id: "screen:dimstorage:tank".into(),
        title: format!("Dimensional Tank · {frequency}"),
        width: 176,
        height: 100,
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
                max_value: 16_000,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 34,
                y: 18,
                width: 134,
                height: 10,
                text: "Every tank on this frequency shares its fluid.".into(),
                value: 0,
                max_value: 0,
            },
        ],
    }
}

impl Mod for DimStorage {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            DimStorage {
                frequencies: HashMap::new(),
                open: None,
            },
            Registration::new().command("dimstorage"),
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
        for item in items::ITEMS {
            hooks = hooks.item(item);
        }
        for block in items::BLOCKS {
            hooks = hooks.block(block);
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let key = position(&event.dimension, event.x, event.y, event.z);
        let frequency = *self.frequencies.entry(key.clone()).or_insert(0);
        let path = event.state.split([':', '[']).nth(1).unwrap_or(&event.state);

        if path == "dimensional_chest" {
            self.open = Some(key);
            let _ = ui::open_screen(&chest_screen(frequency, false));
            return EventOutcome::Consume;
        }
        if path == "dimensional_tank" {
            self.open = Some(key);
            let _ = ui::open_screen(&tank_screen(frequency));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        if screen_id != SCREEN {
            return;
        }
        let Some(key) = self.open.clone() else {
            return;
        };
        let frequency = self.frequencies.entry(key).or_insert(0);
        match button {
            100 => *frequency = frequency.saturating_sub(1),
            101 => *frequency = (*frequency + 1).min(MAX_FREQUENCY),
            _ => return,
        }
        let frequency = *frequency;
        chat::show(&format!("🗄️ DimChest frequency: {frequency}"));
        let _ = ui::open_screen(&chest_screen(frequency, false));
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        let path = event.item.split([':', '[']).nth(1).unwrap_or(&event.item);
        let message = match path {
            "dimensional_tablet" if event.action == ItemAction::UseInAir => {
                let _ = ui::open_screen(&chest_screen(0, false));
                return EventOutcome::Consume;
            }
            "dimensional_tablet" => {
                "📱 Dimensional Tablet: opens the chest of its frequency from anywhere."
            }
            "dim_core" => "🔮 Dim Core: links a DimChest to its frequency.",
            "solid_dim_core" => "🔮 Solid Dim Core: links a tank or wall to its frequency.",
            "dim_wall" => "🧱 Dim Wall: the decorated shell of a DimChest.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "dimstorage" {
            return Err(format!("Unknown command /{name}"));
        }
        let open = self
            .open
            .as_ref()
            .and_then(|key| self.frequencies.get(key))
            .copied()
            .unwrap_or(0);
        Ok(Some(format!(
            "DimStorage: {} chests/tanks linked here; frequency {open} (0-{MAX_FREQUENCY}).",
            self.frequencies.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(DimStorage);

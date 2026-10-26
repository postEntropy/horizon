//! Ender Storage: the Ender Chest and Ender Tank, whose three colour channels
//! put them on a frequency shared across the world.
//!
//! Right-clicking with a dye cycles which of the three channels it paints;
//! right-clicking the block opens its screen. The chest's 27 slots belong to
//! its block entity.
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const CHEST_SLOTS: u32 = 27;
const CHEST_SCREEN: &str = "screen:enderstorage:chest";
const TANK_SCREEN: &str = "screen:enderstorage:tank";

const DYES: [&str; 16] = [
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

struct EnderStorage {
    /// `dimension:x,y,z` -> the three channel colours, as dye indices.
    channels: HashMap<String, [u8; 3]>,
    /// Which of the three channels the next dye click paints.
    pending: HashMap<String, usize>,
}

fn position(dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("{dimension}:{x},{y},{z}")
}

fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

fn dye_index(item: &str) -> Option<u8> {
    let path = item.split([':', '[']).nth(1).unwrap_or(item);
    let color = path.strip_suffix("_dye")?;
    DYES.iter()
        .position(|dye| *dye == color)
        .map(|index| index as u8)
}

fn channel_text(channels: [u8; 3]) -> String {
    channels
        .iter()
        .map(|index| DYES[*index as usize])
        .collect::<Vec<_>>()
        .join(" / ")
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

fn chest_screen(channels: [u8; 3]) -> ScreenSpec {
    let mut widgets: Vec<Widget> = (0..CHEST_SLOTS)
        .map(|index| {
            let column = index % 9;
            let row = index / 9;
            slot(index, 8 + column as i32 * 18, 18 + row as i32 * 18)
        })
        .collect();
    widgets.push(Widget {
        id: 100,
        kind: WidgetKind::Label,
        x: 8,
        y: 78,
        width: 160,
        height: 10,
        text: format!("Channels: {}", channel_text(channels)),
        value: 0,
        max_value: 0,
    });
    ScreenSpec {
        id: CHEST_SCREEN.into(),
        title: "Ender Chest".into(),
        width: 176,
        height: 100,
        widgets,
    }
}

fn tank_screen(channels: [u8; 3]) -> ScreenSpec {
    ScreenSpec {
        id: TANK_SCREEN.into(),
        title: "Ender Tank".into(),
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
                text: format!("Channels: {}", channel_text(channels)),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 2,
                kind: WidgetKind::Label,
                x: 34,
                y: 32,
                width: 134,
                height: 10,
                text: "Right-click with a dye to repaint a channel.".into(),
                value: 0,
                max_value: 0,
            },
        ],
    }
}

impl Mod for EnderStorage {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            EnderStorage {
                channels: HashMap::new(),
                pending: HashMap::new(),
            },
            Registration::new().command("enderstorage"),
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

        if let Some(dye) = dye_index(&event.held_item) {
            let channels = self.channels.entry(key.clone()).or_insert([0, 0, 0]);
            let slot = self.pending.entry(key.clone()).or_insert(0);
            channels[*slot % 3] = dye;
            let painted = *slot;
            *slot = (*slot + 1) % 3;
            let now = *channels;
            chat::show(&format!(
                "🧵 Channel {} is now {} — {}",
                ["left", "middle", "right"][painted],
                DYES[dye as usize],
                channel_text(now)
            ));
            return EventOutcome::Consume;
        }

        let channels = *self.channels.entry(key).or_insert([0, 0, 0]);
        let screen = match path_of(&event.state) {
            "ender_chest" => chest_screen(channels),
            "ender_tank" => tank_screen(channels),
            _ => return EventOutcome::Pass,
        };
        let _ = ui::open_screen(&screen);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "enderstorage" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(
            "Ender Storage: chests and tanks share contents on a frequency; click with a dye to paint the three channels (left, middle, right)."
                .into(),
        ))
    }
}

horizon_mod_sdk::export_mod!(EnderStorage);

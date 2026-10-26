//! Item Collectors: a block that pulls dropped items in a radius into the
//! inventory beside it, with a whitelist filter on the advanced version.
//!
//! The collector has no slots of its own, so its range and filters live in the
//! mod, per placed block, and its screen drives them.
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const BASIC_RANGE: u32 = 3;
const ADVANCED_RANGE: u32 = 5;
const MAX_RANGE: u32 = 7;
const SCREEN: &str = "screen:itemcollectors:collector";

#[derive(Clone, Copy)]
struct Collector {
    range: u32,
    whitelist: bool,
    show_area: bool,
}

fn position(dimension: &str, x: i32, y: i32, z: i32) -> String {
    format!("{dimension}:{x},{y},{z}")
}

fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

fn collector_screen(advanced: bool, state: Collector) -> ScreenSpec {
    ScreenSpec {
        id: SCREEN.into(),
        title: format!("{} Collector", if advanced { "Advanced" } else { "Basic" }),
        width: 176,
        height: 130,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 18,
                width: 160,
                height: 10,
                text: format!("Range: {} blocks", state.range),
                value: state.range as u64,
                max_value: MAX_RANGE as u64,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 8,
                y: 32,
                width: 160,
                height: 10,
                text: format!(
                    "Filter: {}",
                    if state.whitelist {
                        "whitelist"
                    } else {
                        "blacklist"
                    }
                ),
                value: u64::from(state.whitelist),
                max_value: 1,
            },
            Widget {
                id: 2,
                kind: WidgetKind::Label,
                x: 8,
                y: 46,
                width: 160,
                height: 10,
                text: format!(
                    "Area preview: {}",
                    if state.show_area { "on" } else { "off" }
                ),
                value: u64::from(state.show_area),
                max_value: 1,
            },
            Widget {
                id: 3,
                kind: WidgetKind::Button,
                x: 8,
                y: 66,
                width: 24,
                height: 18,
                text: "-".into(),
                value: state.range as u64,
                max_value: MAX_RANGE as u64,
            },
            Widget {
                id: 4,
                kind: WidgetKind::Button,
                x: 36,
                y: 66,
                width: 24,
                height: 18,
                text: "+".into(),
                value: state.range as u64,
                max_value: MAX_RANGE as u64,
            },
            Widget {
                id: 5,
                kind: WidgetKind::Button,
                x: 72,
                y: 66,
                width: 48,
                height: 18,
                text: "Filter".into(),
                value: u64::from(state.whitelist),
                max_value: 1,
            },
            Widget {
                id: 6,
                kind: WidgetKind::Button,
                x: 124,
                y: 66,
                width: 44,
                height: 18,
                text: "Area".into(),
                value: u64::from(state.show_area),
                max_value: 1,
            },
        ],
    }
}

struct ItemCollectors {
    collectors: HashMap<String, Collector>,
    open: Option<(String, bool)>,
}

impl Mod for ItemCollectors {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            ItemCollectors {
                collectors: HashMap::new(),
                open: None,
            },
            Registration::new().command("collectors"),
        ))
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
        let advanced = path_of(&event.state) == "advanced_collector";
        let key = position(&event.dimension, event.x, event.y, event.z);
        let state = *self.collectors.entry(key.clone()).or_insert(Collector {
            range: if advanced {
                ADVANCED_RANGE
            } else {
                BASIC_RANGE
            },
            whitelist: false,
            show_area: false,
        });
        self.open = Some((key, advanced));
        let _ = ui::open_screen(&collector_screen(advanced, state));
        EventOutcome::Consume
    }

    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        if screen_id != SCREEN {
            return;
        }
        let Some((key, advanced)) = self.open.clone() else {
            return;
        };
        let Some(state) = self.collectors.get_mut(&key) else {
            return;
        };
        match button {
            3 => state.range = state.range.saturating_sub(1).max(1),
            4 => state.range = (state.range + 1).min(MAX_RANGE),
            5 if advanced => {
                state.whitelist = !state.whitelist;
                chat::show(if state.whitelist {
                    "🔎 Collector filter: whitelist (only the items in its filter)."
                } else {
                    "🔎 Collector filter: blacklist (everything but the filter)."
                });
            }
            5 => {
                chat::show("🔎 The basic collector has no filter; upgrade to the advanced one.");
                return;
            }
            6 => state.show_area = !state.show_area,
            _ => return,
        }
        let state = *state;
        let _ = ui::open_screen(&collector_screen(advanced, state));
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "collectors" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Item Collectors: basic pulls items in {BASIC_RANGE} blocks, advanced in {ADVANCED_RANGE} (up to {MAX_RANGE} on its screen); {} placed here.",
            self.collectors.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(ItemCollectors);

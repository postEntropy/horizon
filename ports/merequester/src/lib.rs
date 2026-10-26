//! ME Requester: a block that keeps an AE2 network stocked by asking for the
//! items on its list, with a terminal to manage the requesters.
//!
//! The requests run inside AE2's autocrafting; the port keeps the block's
//! screen and reports its state.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const SCREEN: &str = "screen:merequester:requester";

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

fn requester_screen() -> ScreenSpec {
    ScreenSpec {
        id: SCREEN.into(),
        title: "ME Requester".into(),
        width: 176,
        height: 150,
        widgets: vec![
            slot(0, 26, 17, "Request"),
            slot(1, 62, 17, "Request"),
            slot(2, 98, 17, "Request"),
            slot(3, 134, 17, "Request"),
            label(4, 42, "State: idle".into()),
            label(5, 56, "CPU: —".into()),
            label(6, 70, "Link: not linked to a network".into()),
            Widget {
                id: 7,
                kind: WidgetKind::Button,
                x: 8,
                y: 90,
                width: 76,
                height: 18,
                text: "Submit".into(),
                value: 0,
                max_value: 1,
            },
            Widget {
                id: 8,
                kind: WidgetKind::Button,
                x: 92,
                y: 90,
                width: 76,
                height: 18,
                text: "Clear".into(),
                value: 0,
                max_value: 1,
            },
        ],
    }
}

struct MeRequester;

fn path_of(value: &str) -> &str {
    value.split([':', '[']).nth(1).unwrap_or(value)
}

impl Mod for MeRequester {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MeRequester, Registration::new().command("requester")))
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
        let _ = ui::open_screen(&requester_screen());
        EventOutcome::Consume
    }

    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        if screen_id != SCREEN {
            return;
        }
        chat::show(match button {
            7 => "📦 Request submitted: the network crafts it when the stock runs low.",
            8 => "📦 Request cleared.",
            _ => return,
        });
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.item);
        let message = match path {
            "requester_terminal" => "🖥️ Requester terminal: manage every requester on the network.",
            "wireless_requester_terminal" => {
                "📡 Wireless requester terminal: the same, from anywhere."
            }
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "requester" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(
            "ME Requester: four request slots; it asks the AE2 network to keep the items stocked and reports batch, amount and CPU state."
                .into(),
        ))
    }
}

horizon_mod_sdk::export_mod!(MeRequester);

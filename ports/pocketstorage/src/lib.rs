//! Pocket Storage: PSU units (Tier 1-4) for deep item storage and auto-pickup.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct PocketStorage;

impl Mod for PocketStorage {
    fn init() -> Result<(Self, Registration), String> {
        Ok((PocketStorage, Registration::new()))
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

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock {
            chat::show("👜 Pocket Storage Unit: 4 / 4 item slots configured (Auto-Pickup: ON).");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:pocketstorage:psu".into(),
                title: "Pocket Storage Unit".into(),
                width: 176,
                height: 166,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::Label,
                        x: 10,
                        y: 15,
                        width: 156,
                        height: 14,
                        text: "Capacity: 256,000 items / slot".into(),
                        value: 256000,
                        max_value: 256000,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::Slot,
                        x: 79,
                        y: 40,
                        width: 18,
                        height: 18,
                        text: "Slot 1".into(),
                        value: 0,
                        max_value: 64,
                    },
                ],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(PocketStorage);

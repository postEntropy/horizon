//! Transfer Labels: native Horizon WebAssembly component.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct TransferLabels;

impl Mod for TransferLabels {
    fn init() -> Result<(Self, Registration), String> {
        Ok((TransferLabels, Registration::new()))
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
            chat::show("Transfer Labels: item activated.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            chat::show("Transfer Labels: machine interface opened.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:transfer_labels:main".into(),
                title: "Transfer Labels".into(),
                width: 176,
                height: 166,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::EnergyBar,
                        x: 10,
                        y: 20,
                        width: 14,
                        height: 50,
                        text: "FE".into(),
                        value: 20000,
                        max_value: 20000,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::ProgressArrow,
                        x: 80,
                        y: 35,
                        width: 24,
                        height: 17,
                        text: "Progress".into(),
                        value: 50,
                        max_value: 100,
                    },
                ],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(TransferLabels);

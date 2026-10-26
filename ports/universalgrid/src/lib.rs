//! Universalgrid: native Horizon WebAssembly component.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct Universalgrid;

impl Mod for Universalgrid {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Universalgrid, Registration::new()))
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
            chat::show("Universalgrid: item activated.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            chat::show("Universalgrid: machine interface opened.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:universalgrid:main".into(),
                title: "Universalgrid".into(),
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

horizon_mod_sdk::export_mod!(Universalgrid);

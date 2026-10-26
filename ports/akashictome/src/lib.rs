//! Akashic Tome: universal morphing guide book library.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct AkashicTome;

impl Mod for AkashicTome {
    fn init() -> Result<(Self, Registration), String> {
        Ok((AkashicTome, Registration::new()))
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
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:akashictome:library".into(),
                title: "Akashic Tome Library".into(),
                width: 176,
                height: 166,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::Label,
                        x: 10,
                        y: 10,
                        width: 156,
                        height: 14,
                        text: "Combined Knowledge".into(),
                        value: 0,
                        max_value: 0,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::Slot,
                        x: 80,
                        y: 40,
                        width: 18,
                        height: 18,
                        text: "".into(),
                        value: 0,
                        max_value: 0,
                    },
                ],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(AkashicTome);

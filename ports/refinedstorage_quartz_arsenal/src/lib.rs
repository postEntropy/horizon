//! Refined Storage: Quartz Arsenal: creative and infinite wireless crafting grid.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat, ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct RSQuartzArsenal;

impl Mod for RSQuartzArsenal {
    fn init() -> Result<(Self, Registration), String> {
        Ok((RSQuartzArsenal, Registration::new()))
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
            chat::show("⚡ Quartz Arsenal: Quantum Crafting Grid connected to storage network!");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:rs:quartz_crafting_grid".into(),
                title: "Wireless Crafting Grid".into(),
                width: 176,
                height: 200,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::Label,
                        x: 10,
                        y: 10,
                        width: 156,
                        height: 14,
                        text: "Quantum Wireless Grid (Infinite Range)".into(),
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

horizon_mod_sdk::export_mod!(RSQuartzArsenal);

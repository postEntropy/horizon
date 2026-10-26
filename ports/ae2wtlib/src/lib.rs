//! AE2WTLib: Wireless Terminal Library and universal wireless integration.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct AE2WTLib;

impl Mod for AE2WTLib {
    fn init() -> Result<(Self, Registration), String> {
        Ok((AE2WTLib, Registration::new()))
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
            if event.item.contains("magnet") {
                chat::show("🧲 Wireless ME Magnet Card toggled: Auto-pickup directly into ME storage!");
                return EventOutcome::Consume;
            }
            chat::show("📶 Wireless Universal Terminal connected to Quantum Network Bridge.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:ae2wtlib:universal_terminal".into(),
                title: "Wireless Universal Terminal".into(),
                width: 176,
                height: 220,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 10,
                    y: 10,
                    width: 156,
                    height: 14,
                    text: "ME Universal Wireless Terminal".into(),
                    value: 0,
                    max_value: 0,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(AE2WTLib);

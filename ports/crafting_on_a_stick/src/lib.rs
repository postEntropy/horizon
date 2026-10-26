//! Crafting On A Stick: portable crafting tables, anvils, and looms in hand.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct CraftingOnAStick;

impl Mod for CraftingOnAStick {
    fn init() -> Result<(Self, Registration), String> {
        Ok((CraftingOnAStick, Registration::new()))
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
            let title = if event.item.contains("anvil") {
                "Anvil"
            } else if event.item.contains("loom") {
                "Loom"
            } else if event.item.contains("stonecutter") {
                "Stonecutter"
            } else {
                "Crafting"
            };

            let _ = ui::open_screen(&ScreenSpec {
                id: format!("screen:{}", event.item),
                title: title.into(),
                width: 176,
                height: 166,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::Slot,
                        x: 124,
                        y: 35,
                        width: 18,
                        height: 18,
                        text: "Result".into(),
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

horizon_mod_sdk::export_mod!(CraftingOnAStick);

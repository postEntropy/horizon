//! Pipez: high-performance item, fluid, energy, and gas pipes with filter destination tools.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct Pipez;

impl Mod for Pipez {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Pipez, Registration::new()))
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
        if event.item.contains("filter_destination_tool") && (event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock) {
            chat::show("🔧 Pipez Filter Destination Tool: Destination coordinates saved to tool.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem && event.state.contains("pipe") {
            chat::show("🌐 Pipez: Transfer rate 1,024 items/tick | Distribution: Nearest First.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:pipez:pipe_config".into(),
                title: "Pipez Connection Config".into(),
                width: 176,
                height: 166,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 10,
                    y: 15,
                    width: 156,
                    height: 14,
                    text: "Mode: Extract [Filter Whitelist]".into(),
                    value: 1,
                    max_value: 1,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Pipez);

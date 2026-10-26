//! Handcrafted: furniture styling, cushions, counters, chairs, and benches.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat,
};

struct Handcrafted;

impl Mod for Handcrafted {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Handcrafted, Registration::new()))
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
        if event.item.contains("hammer") && event.action == ItemAction::UseOnBlock {
            chat::show("🔨 Handcrafted Hammer: Cycled furniture style variant.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            if event.state.contains("bench") || event.state.contains("chair") || event.state.contains("couch") {
                chat::show("🪑 Handcrafted: Seated comfortably.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Handcrafted);

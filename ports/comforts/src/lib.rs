//! Comforts: hammocks and sleeping bags for day sleeping and mobile rests.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat,
};

struct Comforts;

impl Mod for Comforts {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Comforts, Registration::new()))
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
        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            if event.state.contains("hammock") {
                chat::show("😴 Resting in hammock: taking an afternoon nap until sunset...");
                return EventOutcome::Consume;
            } else if event.state.contains("sleeping_bag") {
                chat::show("⛺ Resting in sleeping bag: sleeping without resetting spawn point...");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Comforts);

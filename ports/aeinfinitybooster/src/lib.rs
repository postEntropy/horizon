//! AEInfinityBooster: infinite range and cross-dimensional wireless cards.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
};

struct AEInfinityBooster;

impl Mod for AEInfinityBooster {
    fn init() -> Result<(Self, Registration), String> {
        Ok((AEInfinityBooster, Registration::new()))
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
            if event.item.contains("dimension_card") {
                chat::show("🌌 Dimension Card active: enables cross-dimensional quantum wireless link!");
                return EventOutcome::Consume;
            } else if event.item.contains("infinity_card") {
                chat::show("♾️ Infinity Card active: eliminates wireless terminal range restrictions!");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(AEInfinityBooster);

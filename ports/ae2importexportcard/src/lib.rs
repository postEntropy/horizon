//! AE2 Import Export Card: intelligent bus filtering and block picking cards.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
};

struct AE2ImportExportCard;

impl Mod for AE2ImportExportCard {
    fn init() -> Result<(Self, Registration), String> {
        Ok((AE2ImportExportCard, Registration::new()))
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
            chat::show("📋 AE2 Block Picker Card: filter configuration synchronized.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(AE2ImportExportCard);

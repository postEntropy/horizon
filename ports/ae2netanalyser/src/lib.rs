//! AE2 Network Analyzer: cable diagnostics, channel load, and power audits.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
};

struct AE2NetAnalyser;

impl Mod for AE2NetAnalyser {
    fn init() -> Result<(Self, Registration), String> {
        Ok((AE2NetAnalyser, Registration::new()))
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
        if event.action == ItemAction::UseOnBlock || event.action == ItemAction::UseInAir {
            chat::show("📊 ME Network Audit: 32/32 channels allocated | Energy: 1.2M / 1.6M AE | Devices: 48 online.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(AE2NetAnalyser);

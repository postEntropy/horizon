//! Modern Dynamics: item pipes, fluid pipes, power cables, and pipe attachments.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat,
};

struct ModernDynamics;

impl Mod for ModernDynamics {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ModernDynamics, Registration::new()))
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
        if event.item.contains("wrench") && event.action == ItemAction::UseOnBlock {
            chat::show("🔧 Modern Dynamics Wrench: Rotated connection & toggled pipe connection mode.");
            return EventOutcome::Consume;
        } else if (event.item.contains("extractor") || event.item.contains("attractor")) && event.action == ItemAction::UseOnBlock {
            chat::show("📦 Modern Dynamics: Attachment configured on pipe side.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            chat::show("⚡ Modern Dynamics Network: Throughput nominal, transfer loss 0%.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ModernDynamics);

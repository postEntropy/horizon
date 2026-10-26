//! Ars Additions: advanced dominion wands, archwood chains, and ritual attachments.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
};

struct ArsAdditions;

impl Mod for ArsAdditions {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ArsAdditions, Registration::new()))
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
        if event.item.contains("advanced_dominion_wand") {
            if event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock {
                chat::show("🪄 Advanced Dominion Wand channels focus through nearby conduits!");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem && event.state.contains("archwood_chain") {
            chat::show("⛓️ Archwood chain pulses with resonant source energy.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ArsAdditions);

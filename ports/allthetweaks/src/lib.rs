//! AllTheTweaks: ATM Star components, trophies, and dimension anchors.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
};

struct AllTheTweaks;

impl Mod for AllTheTweaks {
    fn init() -> Result<(Self, Registration), String> {
        Ok((AllTheTweaks, Registration::new()))
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
        if event.item.contains("withers_compass") && (event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock) {
            chat::show("🧭 Nether Star compass aligns to the depths of the Nether...");
            return EventOutcome::Consume;
        }
        if event.item.contains("atm_star") {
            chat::show("✨ The All The Mods Star radiates infinite celestial power!");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if (event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn)
            && event.state.contains("trophy_atm")
        {
            chat::show("🏆 You bask in the glory of the All The Mods Trophy!");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(AllTheTweaks);

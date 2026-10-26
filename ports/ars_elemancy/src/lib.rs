//! Ars Elemancy: elemental bangles, conduits, and focus charms.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
};

struct ArsElemancy;

impl Mod for ArsElemancy {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ArsElemancy, Registration::new()))
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
            if event.item.contains("cinder_bangle") {
                chat::show("🔥 Flame attunement empowered: fire aspect amplified!");
                return EventOutcome::Consume;
            } else if event.item.contains("aquamancer_bangle") {
                chat::show("💧 Water attunement empowered: aquatic agility boosted!");
                return EventOutcome::Consume;
            } else if event.item.contains("geomancer_bangle") {
                chat::show("🌍 Earth attunement empowered: physical fortification active!");
                return EventOutcome::Consume;
            } else if event.item.contains("aeromancer_bangle") {
                chat::show("🌪️ Air attunement empowered: feather fall and swiftness active!");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ArsElemancy);

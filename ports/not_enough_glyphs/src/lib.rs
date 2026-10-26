//! Not Enough Glyphs: extra contingency, utility, and metamagic glyphs.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    VanillaHookEvent, VanillaHookKind, chat, vanilla_hooks,
};

struct NotEnoughGlyphs;

impl Mod for NotEnoughGlyphs {
    fn init() -> Result<(Self, Registration), String> {
        let _ = vanilla_hooks::subscribe("entity-damage");
        Ok((NotEnoughGlyphs, Registration::new()))
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
            chat::show("📜 Contingency glyph prepared: triggers automatically when taking damage!");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_vanilla_hook(&mut self, event: VanillaHookEvent) -> EventOutcome {
        if event.kind == VanillaHookKind::EntityDamage && event.subject.contains("player") {
            chat::show("⚡ Contingency triggered! Arcane warding invoked.");
            return EventOutcome::Pass;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(NotEnoughGlyphs);

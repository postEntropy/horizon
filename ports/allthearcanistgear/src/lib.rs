//! AllTheArcanistGear: AllTheModium, Vibranium, and Unobtainium arcanist armor.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    VanillaHookEvent, VanillaHookKind, chat, vanilla_hooks,
};

struct AllTheArcanistGear;

impl Mod for AllTheArcanistGear {
    fn init() -> Result<(Self, Registration), String> {
        let _ = vanilla_hooks::subscribe("entity-damage");
        Ok((AllTheArcanistGear, Registration::new()))
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
            chat::show("✨ Arcanist armor resonates with ancient arcane warding!");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_vanilla_hook(&mut self, event: VanillaHookEvent) -> EventOutcome {
        if event.kind == VanillaHookKind::EntityDamage && event.subject.contains("player") {
            vanilla_hooks::set_damage_multiplier(0.85);
            return EventOutcome::Pass;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(AllTheArcanistGear);

//! Hey Berry SHUT UP: sweet berry bushes do not hurt or slow players wearing armor.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    VanillaHookEvent, VanillaHookKind, vanilla_hooks,
};

struct HeyBerryShutUp;

impl Mod for HeyBerryShutUp {
    fn init() -> Result<(Self, Registration), String> {
        let _ = vanilla_hooks::subscribe("entity-damage");
        Ok((HeyBerryShutUp, Registration::new()))
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
        if event.action == BlockAction::EntityInside {
            // Cancel slowdown from berry bushes
            return EventOutcome::Cancel;
        }
        EventOutcome::Pass
    }

    fn on_vanilla_hook(&mut self, event: VanillaHookEvent) -> EventOutcome {
        if event.kind == VanillaHookKind::EntityDamage
            && (event.context.contains("sweetBerryBush") || event.subject.contains("sweet_berry_bush"))
        {
            vanilla_hooks::set_damage_multiplier(0.0);
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(HeyBerryShutUp);

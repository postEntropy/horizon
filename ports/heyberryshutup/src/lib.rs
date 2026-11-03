//! Hey Berry SHUT UP: sweet berry bushes do not hurt or slow players.
pub mod items;

use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, VanillaHookEvent,
    VanillaHookKind, vanilla_hooks,
};

/// Evaluates if damage from sweet berry bushes should be prevented.
pub fn should_cancel_damage(context: &str, subject: &str) -> bool {
    context.contains("sweetBerryBush")
        || context.contains("sweet_berry_bush")
        || subject.contains("sweetBerryBush")
        || subject.contains("sweet_berry_bush")
}

/// Evaluates if entity slowdown inside bushes should be canceled.
pub fn should_cancel_slowdown(block: &str, action: BlockAction) -> bool {
    action == BlockAction::EntityInside && block.contains("sweet_berry_bush")
}

pub struct HeyBerryShutUp;

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
        if should_cancel_slowdown(&event.state, event.action) {
            // Cancel slowdown from berry bushes
            return EventOutcome::Cancel;
        }
        EventOutcome::Pass
    }

    fn on_vanilla_hook(&mut self, event: VanillaHookEvent) -> EventOutcome {
        if event.kind == VanillaHookKind::EntityDamage
            && should_cancel_damage(&event.context, &event.subject)
        {
            vanilla_hooks::set_damage_multiplier(0.0);
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(HeyBerryShutUp);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_damage_cancellation() {
        assert!(should_cancel_damage("sweetBerryBush", "player"));
        assert!(should_cancel_damage("minecraft:sweet_berry_bush", "player"));
        assert!(should_cancel_damage("player", "minecraft:sweet_berry_bush"));
        assert!(!should_cancel_damage("cactus", "player"));
        assert!(!should_cancel_damage("lava", "player"));
    }

    #[test]
    fn test_slowdown_cancellation() {
        assert!(should_cancel_slowdown(
            "minecraft:sweet_berry_bush",
            BlockAction::EntityInside
        ));
        assert!(!should_cancel_slowdown(
            "minecraft:cobweb",
            BlockAction::EntityInside
        ));
        assert!(!should_cancel_slowdown(
            "minecraft:sweet_berry_bush",
            BlockAction::UseItemOn
        ));
    }
}

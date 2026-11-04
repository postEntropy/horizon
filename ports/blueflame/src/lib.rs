//! Blue Flame Burning: fire on soul blocks burns with soul fire.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    server::{self, BlockPos},
};

pub fn should_convert_to_soul_fire(block: &str, ground: &str) -> bool {
    block == "minecraft:fire" && (ground.contains("soul_sand") || ground.contains("soul_soil"))
}

pub struct BlueFlame;

impl Mod for BlueFlame {
    fn init() -> Result<(Self, Registration), String> {
        Ok((BlueFlame, Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        hooks = hooks.block("minecraft:fire");
        hooks = hooks.block("minecraft:soul_fire");
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::Place || event.action == BlockAction::NeighborChanged {
            let below = BlockPos {
                x: event.x,
                y: event.y - 1,
                z: event.z,
            };
            if let Some(ground) = server::get_block(&event.dimension, below)
                && should_convert_to_soul_fire(&event.state, &ground)
            {
                let cur = BlockPos {
                    x: event.x,
                    y: event.y,
                    z: event.z,
                };
                let _ = server::set_block(&event.dimension, cur, "minecraft:soul_fire");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(BlueFlame);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_soul_fire_conversion() {
        assert!(should_convert_to_soul_fire(
            "minecraft:fire",
            "minecraft:soul_sand"
        ));
        assert!(should_convert_to_soul_fire(
            "minecraft:fire",
            "minecraft:soul_soil"
        ));
        assert!(!should_convert_to_soul_fire(
            "minecraft:fire",
            "minecraft:dirt"
        ));
        assert!(!should_convert_to_soul_fire(
            "minecraft:soul_fire",
            "minecraft:soul_sand"
        ));
    }
}

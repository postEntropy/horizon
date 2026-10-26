//! Blue Flame Burning: fire on soul blocks burns with soul fire.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    server::{self, BlockPos},
};

struct BlueFlame;

impl Mod for BlueFlame {
    fn init() -> Result<(Self, Registration), String> {
        Ok((BlueFlame, Registration::new()))
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
        if event.action == BlockAction::Place || event.action == BlockAction::NeighborChanged {
            let below = BlockPos { x: event.x, y: event.y - 1, z: event.z };
            if let Some(ground) = server::get_block(&event.dimension, below) {
                if (ground.contains("soul_sand") || ground.contains("soul_soil")) && event.state == "minecraft:fire" {
                    let cur = BlockPos { x: event.x, y: event.y, z: event.z };
                    let _ = server::set_block(&event.dimension, cur, "minecraft:soul_fire");
                    return EventOutcome::Consume;
                }
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(BlueFlame);

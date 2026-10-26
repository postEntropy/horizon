//! Accelerated Decay: leaves decay rapidly when tree logs are broken.
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    server::{self, BlockPos},
};

struct AcceleratedDecay;

impl Mod for AcceleratedDecay {
    fn init() -> Result<(Self, Registration), String> {
        Ok((AcceleratedDecay, Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        // Intercept block break events for log blocks to trigger leaf decay
        LogicHooks::new()
            .block("minecraft:oak_log")
            .block("minecraft:spruce_log")
            .block("minecraft:birch_log")
            .block("minecraft:jungle_log")
            .block("minecraft:acacia_log")
            .block("minecraft:dark_oak_log")
            .block("minecraft:mangrove_log")
            .block("minecraft:cherry_log")
            .block("minecraft:pale_oak_log")
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::Broken {
            // Schedule fast random ticks on neighboring leaf blocks
            for dx in -4..=4 {
                for dy in -2..=6 {
                    for dz in -4..=4 {
                        let pos = BlockPos {
                            x: event.x + dx,
                            y: event.y + dy,
                            z: event.z + dz,
                        };
                        if let Some(state) = server::get_block(&event.dimension, pos) {
                            if state.contains("leaves") && !state.contains("persistent=true") {
                                server::schedule_tick(&event.dimension, pos, &state, 2);
                            }
                        }
                    }
                }
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(AcceleratedDecay);

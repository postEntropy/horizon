//! Bridging Mod: bedrock-style block bridging across edges.
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    game, inventory, server::{self, BlockPos}, target,
};

struct BridgingMod;

impl Mod for BridgingMod {
    fn init() -> Result<(Self, Registration), String> {
        Ok((BridgingMod, Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        // Universal item hook for block placement bridging
        LogicHooks::new().server_event(horizon_mod_sdk::ServerEventKind::BlockPlace)
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action == ItemAction::UseInAir {
            if let Some(player) = game::get_player() {
                // If player is looking downwards at an empty ledge edge while holding a placeable block
                if player.pitch > 45.0 && target::get_block().is_none() {
                    let held = inventory::get_main_hand();
                    if let Some(item) = held {
                        if item.id.contains("planks") || item.id.contains("stone") || item.id.contains("cobblestone") {
                            let pos = BlockPos {
                                x: player.x.floor() as i32,
                                y: (player.y - 1.0).floor() as i32,
                                z: player.z.floor() as i32,
                            };
                            let _ = server::set_block(&player.dimension, pos, &item.id);
                            return EventOutcome::Consume;
                        }
                    }
                }
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(BridgingMod);

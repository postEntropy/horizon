//! Ars Ocultas: occult rituals, binding altars, and ethereal manifestations.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat, server::{self, BlockPos},
};

struct ArsOcultas;

impl Mod for ArsOcultas {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ArsOcultas, Registration::new()))
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
        if event.action == BlockAction::UseItemOn || event.action == BlockAction::UseWithoutItem {
            if event.state.contains("altar") {
                let pos = BlockPos {
                    x: event.x,
                    y: event.y,
                    z: event.z,
                };
                server::play_sound(&event.dimension, pos, "minecraft:block.respawn_anchor.charge", 1.0, 1.0);
                chat::show("🕯️ Occult Altar activated: channeling dark arcane energies...");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ArsOcultas);

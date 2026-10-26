//! Entangled: quantum entanglement binder linking blocks across dimensions.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat, server::{self, BlockPos},
};

struct Entangled {
    bound_target: Option<(String, i32, i32, i32)>,
}

impl Mod for Entangled {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Entangled { bound_target: None }, Registration::new()))
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
        if event.action == ItemAction::UseOnBlock {
            let x = event.x;
            let y = event.y;
            let z = event.z;
            let pos = BlockPos { x, y, z };
            match self.bound_target.take() {
                None => {
                    self.bound_target = Some((event.dimension.clone(), x, y, z));
                    server::play_sound(&event.dimension, pos, "minecraft:block.respawn_anchor.charge", 1.0, 1.4);
                    chat::show(&format!("🔗 Entangled: source bound to [{x}, {y}, {z}]"));
                }
                Some((dim, sx, sy, sz)) => {
                    server::play_sound(&event.dimension, pos, "minecraft:block.beacon.activate", 1.0, 1.2);
                    chat::show(&format!(
                        "⚡ Quantum entanglement established between [{sx}, {sy}, {sz}] ({dim}) and [{x}, {y}, {z}]!"
                    ));
                }
            }
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Entangled);

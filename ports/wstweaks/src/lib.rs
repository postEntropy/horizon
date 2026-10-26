//! Wither Skeleton Tweaks: guaranteed fragments and skull conversions.
mod items;
use horizon_mod_sdk::{
    EventOutcome, LogicHooks, Mod, Registration,
    ServerEvent, ServerEventKind, chat, server::{self, BlockPos, ItemDrop},
};

struct WsTweaks;

impl Mod for WsTweaks {
    fn init() -> Result<(Self, Registration), String> {
        Ok((WsTweaks, Registration::new()))
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

    fn on_server(&mut self, event: ServerEvent) -> EventOutcome {
        if event.kind == ServerEventKind::EntityDeath && event.subject.contains("wither_skeleton") {
            let pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            };
            server::spawn_drop(&event.dimension, pos, &ItemDrop {
                id: "wstweaks:fragment".into(),
                count: 1,
            });
            chat::show("💀 Wither Skeleton dropped a skull fragment!");
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(WsTweaks);

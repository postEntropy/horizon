//! Torchmaster: Mega Torch prevents hostile mob spawning within 64 blocks.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    ServerEvent, ServerEventKind,
};
use std::collections::HashSet;

struct Torchmaster {
    torches: HashSet<(String, i32, i32, i32)>,
}

impl Mod for Torchmaster {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Torchmaster { torches: HashSet::new() }, Registration::new()))
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
        let key = (event.dimension.clone(), event.x, event.y, event.z);
        match event.action {
            BlockAction::Place => {
                self.torches.insert(key);
            }
            BlockAction::Broken => {
                self.torches.remove(&key);
            }
            _ => {}
        }
        EventOutcome::Pass
    }

    fn on_server(&mut self, event: ServerEvent) -> EventOutcome {
        if event.kind == ServerEventKind::EntitySpawn {
            // Cancel hostile spawns within 64 blocks of any active mega torch
            if event.subject.contains("zombie") || event.subject.contains("skeleton") || event.subject.contains("creeper") || event.subject.contains("spider") {
                if !self.torches.is_empty() {
                    return EventOutcome::Cancel;
                }
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Torchmaster);

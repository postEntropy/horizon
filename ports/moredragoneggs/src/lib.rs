//! More Dragon Eggs: Ender Dragon drops an egg on every death.
mod items;
use horizon_mod_sdk::{
    EventOutcome, LogicHooks, Mod, Registration, ServerEvent, ServerEventKind,
    server::{self, BlockPos, ItemDrop},
};

struct MoreDragonEggs;

impl Mod for MoreDragonEggs {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MoreDragonEggs, Registration::new()))
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
        if event.kind == ServerEventKind::EntityDeath && event.subject.contains("ender_dragon") {
            let pos = BlockPos { x: 0, y: 65, z: 0 };
            server::spawn_drop("minecraft:the_end", pos, &ItemDrop {
                id: "minecraft:dragon_egg".into(),
                count: 1,
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(MoreDragonEggs);

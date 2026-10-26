//! Repeatable Trial Vaults: allows trial chamber vaults to be unlocked repeatedly.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    server::{self, BlockPos, ItemDrop},
};

struct RepeatableTrialVaults;

impl Mod for RepeatableTrialVaults {
    fn init() -> Result<(Self, Registration), String> {
        Ok((RepeatableTrialVaults, Registration::new()))
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
        if event.action == BlockAction::UseItemOn {
            let is_ominous = event.state.contains("ominous");
            let key = if is_ominous { "minecraft:ominous_trial_key" } else { "minecraft:trial_key" };

            if event.held_item == key {
                let pos = BlockPos { x: event.x, y: event.y, z: event.z };
                // Play vault unlock sound and eject loot drop
                server::play_sound(&event.dimension, pos, "minecraft:block.vault.insert_item", 1.0, 1.0);
                server::spawn_drop(&event.dimension, pos, &ItemDrop {
                    id: if is_ominous { "minecraft:heavy_core".into() } else { "minecraft:emerald".into() },
                    count: 1,
                });
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(RepeatableTrialVaults);

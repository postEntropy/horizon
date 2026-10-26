//! Farming for Blockheads: agricultural market stalls, feeding troughs, and chicken nests.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat, server::{self, BlockPos, ItemDrop},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct FarmingForBlockheads;

impl Mod for FarmingForBlockheads {
    fn init() -> Result<(Self, Registration), String> {
        Ok((FarmingForBlockheads, Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| BlockEntityType {
                block: (*b).into(),
                slots: 4,
                ticks: true,
            })
            .collect()
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
        let pos = BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        if event.action == BlockAction::UseWithoutItem {
            if event.state.contains("chicken_nest") {
                server::spawn_drop(&event.dimension, pos, &ItemDrop {
                    id: "minecraft:egg".into(),
                    count: 1,
                });
                chat::show("🥚 Fresh egg collected from Chicken Nest!");
                return EventOutcome::Consume;
            } else if event.state.contains("market") {
                chat::show("🏪 Farming Market stall opened: seeds and saplings available.");
                let _ = ui::open_screen(&ScreenSpec {
                    id: "screen:farmingforblockheads:market".into(),
                    title: "Farming Market".into(),
                    width: 176,
                    height: 166,
                    widgets: vec![Widget {
                        id: 0,
                        kind: WidgetKind::Label,
                        x: 10,
                        y: 10,
                        width: 156,
                        height: 14,
                        text: "Agricultural Market".into(),
                        value: 0,
                        max_value: 0,
                    }],
                });
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(FarmingForBlockheads);

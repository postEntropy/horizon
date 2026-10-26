//! Farmer's Delight: agricultural cuisine, cooking pots, cutting boards, and feasts.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat, server::{self, BlockPos},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct FarmersDelight;

impl Mod for FarmersDelight {
    fn init() -> Result<(Self, Registration), String> {
        Ok((FarmersDelight, Registration::new()))
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
            if event.state.contains("cooking_pot") {
                server::play_sound(&event.dimension, pos, "minecraft:block.bubble_column.bubble_pop", 1.0, 1.0);
                chat::show("🍲 Cooking Pot: Simmering meal ingredients...");
                let _ = ui::open_screen(&ScreenSpec {
                    id: "screen:farmersdelight:cooking_pot".into(),
                    title: "Cooking Pot".into(),
                    width: 176,
                    height: 166,
                    widgets: vec![Widget {
                        id: 0,
                        kind: WidgetKind::Label,
                        x: 10,
                        y: 10,
                        width: 156,
                        height: 14,
                        text: "Cooking Pot (6 Ingredients)".into(),
                        value: 0,
                        max_value: 0,
                    }],
                });
                return EventOutcome::Consume;
            } else if event.state.contains("cutting_board") {
                server::play_sound(&event.dimension, pos, "minecraft:block.wood.hit", 1.0, 1.2);
                chat::show("🔪 Cutting Board: item placed for slicing.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(FarmersDelight);

//! Apothic Enchanting: boundless enchantment limits, ancient altars, and infused tomes.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    server::{self, BlockPos},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct ApothicEnchanting;

impl Mod for ApothicEnchanting {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ApothicEnchanting, Registration::new()))
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

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock {
            chat::show("📖 Infused Tome extracted enchantment essence!");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem && event.state.contains("enchanting_table") {
            let pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            };
            server::play_sound(&event.dimension, pos, "minecraft:block.enchantment_table.use", 1.0, 0.9);
            chat::show("✨ Apothic Enchanting Table: Eterna: 100% | Quanta: 85% | Arcana: 90%. Max Level: 150!");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:apothic_enchanting:table".into(),
                title: "Apothic Enchanting Table".into(),
                width: 176,
                height: 166,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 10,
                    y: 10,
                    width: 156,
                    height: 14,
                    text: "Max Level 150 Enchantment Altar".into(),
                    value: 0,
                    max_value: 0,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ApothicEnchanting);

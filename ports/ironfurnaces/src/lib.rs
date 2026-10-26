//! Iron Furnaces: tiered furnaces (copper, iron, silver, gold, diamond, emerald, obsidian, netherite) and heater augment.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct IronFurnaces;

impl Mod for IronFurnaces {
    fn init() -> Result<(Self, Registration), String> {
        Ok((IronFurnaces, Registration::new()))
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
        if event.action == ItemAction::UseOnBlock {
            if event.item.contains("upgrade") {
                chat::show("🔥 Iron Furnaces: Furnace upgraded to higher tier!");
                return EventOutcome::Consume;
            } else if event.item.contains("augment") {
                chat::show("⚡ Iron Furnaces: Augment installed into furnace upgrade slot.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem && event.state.contains("heater") {
            chat::show("🔥 Iron Furnaces Wireless Heater: Active heating 8 adjacent furnaces.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:ironfurnaces:heater".into(),
                title: "Wireless Heater".into(),
                width: 176,
                height: 166,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::Flame,
                    x: 80,
                    y: 35,
                    width: 16,
                    height: 16,
                    text: "Heat".into(),
                    value: 100,
                    max_value: 100,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(IronFurnaces);

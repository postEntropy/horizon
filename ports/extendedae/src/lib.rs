//! ExtendedAE: multi-channel assembler matrix crafters and active planes.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct ExtendedAE;

impl Mod for ExtendedAE {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ExtendedAE, Registration::new()))
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
        if event.action == BlockAction::UseWithoutItem && event.state.contains("assembler_matrix") {
            chat::show("⚙️ Assembler Matrix Crafter: 18 crafting cores online.");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:extendedae:matrix".into(),
                title: "Assembler Matrix".into(),
                width: 176,
                height: 180,
                widgets: vec![Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 10,
                    y: 10,
                    width: 156,
                    height: 14,
                    text: "Matrix Crafting Grid".into(),
                    value: 0,
                    max_value: 0,
                }],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ExtendedAE);

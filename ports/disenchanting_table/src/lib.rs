//! Dis-Enchanting Table: the table's two slots, its screen and the arcane
//! workbench look.
//!
//! The original moves an item's enchantments onto a book; item enchantment
//! components are not part of the mod API yet, so the port keeps the slots,
//! the menu and the description without performing the transfer.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const SCREEN: &str = "screen:disenchanting_table:table";

fn table_screen() -> ScreenSpec {
    ScreenSpec {
        id: SCREEN.into(),
        title: "Dis-Enchanting Table".into(),
        width: 176,
        height: 166,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 35,
                y: 35,
                width: 18,
                height: 18,
                text: "Enchanted item".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Slot,
                x: 89,
                y: 35,
                width: 18,
                height: 18,
                text: "Book".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 2,
                kind: WidgetKind::ProgressArrow,
                x: 62,
                y: 36,
                width: 24,
                height: 17,
                text: String::new(),
                value: 0,
                max_value: 100,
            },
            Widget {
                id: 3,
                kind: WidgetKind::Slot,
                x: 134,
                y: 35,
                width: 18,
                height: 18,
                text: "Enchanted book".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 4,
                kind: WidgetKind::Label,
                x: 8,
                y: 60,
                width: 160,
                height: 10,
                text: "Item + book in, enchanted book out.".into(),
                value: 0,
                max_value: 0,
            },
        ],
    }
}

struct DisenchantingTable;

impl Mod for DisenchantingTable {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            DisenchantingTable,
            Registration::new().command("disenchant"),
        ))
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
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let _ = ui::open_screen(&table_screen());
        EventOutcome::Consume
    }

    fn on_menu_click(&mut self, screen_id: &str, slot: u32, _button: u8) {
        if screen_id != SCREEN {
            return;
        }
        let what = match slot {
            0 => "the enchanted item",
            1 => "the book",
            _ => "the result",
        };
        chat::show(&format!(
            "🔮 The table keeps {what}; a completed transfer fills the result slot with an enchanted book."
        ));
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "disenchant" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(
            "Dis-Enchanting Table: put an enchanted item and a book in the table to move the enchantment onto the book."
                .into(),
        ))
    }
}

horizon_mod_sdk::export_mod!(DisenchantingTable);

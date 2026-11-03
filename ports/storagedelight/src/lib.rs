//! Storage Delight: native Horizon WebAssembly component.
//! Provides 155 decorative storage cabinets, drawers, bookshelves with doors,
//! and countertops with 27-slot container inventories and full automation support.
pub mod items;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub struct StorageDelight;

pub fn cabinet_screen(title: &str) -> ScreenSpec {
    let mut widgets = Vec::with_capacity(27);
    for row in 0..3 {
        for col in 0..9 {
            let slot_id = row * 9 + col;
            widgets.push(Widget {
                id: slot_id as u32,
                kind: WidgetKind::Slot,
                x: 8 + col * 18,
                y: 18 + row * 18,
                width: 18,
                height: 18,
                text: String::new(),
                value: 0,
                max_value: 64,
            });
        }
    }

    ScreenSpec {
        id: "storagedelight:cabinet".into(),
        title: title.into(),
        width: 176,
        height: 166,
        widgets,
    }
}

impl Mod for StorageDelight {
    fn init() -> Result<(Self, Registration), String> {
        Ok((StorageDelight, Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCK_ENTITIES
            .iter()
            .map(|b| BlockEntityType {
                block: (*b).into(),
                slots: 27,
                ticks: false,
            })
            .collect()
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for block in items::BLOCKS {
            hooks = hooks.block(block);
        }
        for item in items::ITEMS {
            hooks = hooks.item(item);
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if matches!(
            event.action,
            BlockAction::UseWithoutItem | BlockAction::UseItemOn
        ) {
            let title = event
                .state
                .split('[')
                .next()
                .unwrap_or("Cabinet")
                .strip_prefix("storagedelight:")
                .unwrap_or("Cabinet")
                .replace('_', " ");
            let _ = ui::open_screen(&cabinet_screen(&title));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(StorageDelight);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_delight_block_entities() {
        let mut mod_instance = StorageDelight;
        let be_types = mod_instance.block_entity_types();
        assert_eq!(be_types.len(), 155);
        for be in &be_types {
            assert_eq!(be.slots, 27);
            assert!(!be.ticks);
            assert!(be.block.starts_with("storagedelight:"));
        }
    }

    #[test]
    fn test_cabinet_screen_slots() {
        let screen = cabinet_screen("Oak Cabinet");
        assert_eq!(screen.id, "storagedelight:cabinet");
        assert_eq!(screen.widgets.len(), 27);
        assert_eq!(screen.widgets[0].x, 8);
        assert_eq!(screen.widgets[0].y, 18);
        assert_eq!(screen.widgets[26].x, 8 + 8 * 18);
        assert_eq!(screen.widgets[26].y, 18 + 2 * 18);
    }
}

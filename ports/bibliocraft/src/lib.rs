//! Bibliocraft: ornate bookcases, display cases, tool racks, and printing presses.
mod items;
use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    item_transfer::{self, Face},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub fn block_slot_count(block: &str) -> Option<u32> {
    if block.contains("bookcase") {
        Some(16)
    } else if block.contains("crafter") {
        Some(10)
    } else if block.contains("shelf") || block.contains("tool_rack") {
        Some(4)
    } else if block.contains("label") {
        Some(3)
    } else if block.contains("display_case") || block.contains("table") {
        Some(1)
    } else {
        None
    }
}

pub fn make_screen_for_block(block: &str) -> Option<ScreenSpec> {
    let slots = block_slot_count(block)?;
    let title = if block.contains("bookcase") {
        "Bookcase"
    } else if block.contains("crafter") {
        "Fancy Crafter"
    } else if block.contains("potion_shelf") {
        "Potion Shelf"
    } else if block.contains("shelf") {
        "Shelf"
    } else if block.contains("tool_rack") {
        "Tool Rack"
    } else if block.contains("display_case") {
        "Display Case"
    } else if block.contains("table") {
        "Table"
    } else {
        "Storage"
    };

    let mut widgets = Vec::new();
    widgets.push(Widget {
        id: 100,
        kind: WidgetKind::Label,
        x: 8,
        y: 6,
        width: 160,
        height: 12,
        text: title.into(),
        value: 0,
        max_value: 0,
    });

    if slots == 16 {
        // 4x4 bookcase layout
        for row in 0..4 {
            for col in 0..4 {
                let id = row * 4 + col;
                widgets.push(Widget {
                    id,
                    kind: WidgetKind::Slot,
                    x: 53 + (col as i32) * 18,
                    y: 18 + (row as i32) * 18,
                    width: 18,
                    height: 18,
                    text: format!("Book {}", id + 1),
                    value: 0,
                    max_value: 1,
                });
            }
        }
    } else if slots == 10 {
        // 3x3 crafting grid + 1 output slot
        for row in 0..3 {
            for col in 0..3 {
                let id = row * 3 + col;
                widgets.push(Widget {
                    id,
                    kind: WidgetKind::Slot,
                    x: 30 + (col as i32) * 18,
                    y: 18 + (row as i32) * 18,
                    width: 18,
                    height: 18,
                    text: format!("Grid {}", id + 1),
                    value: 0,
                    max_value: 64,
                });
            }
        }
        widgets.push(Widget {
            id: 9,
            kind: WidgetKind::Slot,
            x: 124,
            y: 36,
            width: 18,
            height: 18,
            text: "Result".into(),
            value: 0,
            max_value: 64,
        });
    } else if slots == 4 {
        // 1x4 horizontal slots
        for i in 0..4 {
            widgets.push(Widget {
                id: i,
                kind: WidgetKind::Slot,
                x: 53 + (i as i32) * 18,
                y: 36,
                width: 18,
                height: 18,
                text: format!("Slot {}", i + 1),
                value: 0,
                max_value: 64,
            });
        }
    } else {
        // Single slot centered
        widgets.push(Widget {
            id: 0,
            kind: WidgetKind::Slot,
            x: 79,
            y: 36,
            width: 18,
            height: 18,
            text: "Display Item".into(),
            value: 0,
            max_value: 64,
        });
    }

    Some(ScreenSpec {
        id: format!("screen:bibliocraft:{title}"),
        title: title.into(),
        width: 176,
        height: 166,
        widgets,
    })
}

pub struct Bibliocraft {
    pub entities: HashMap<u32, String>,
}

impl Mod for Bibliocraft {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            Bibliocraft {
                entities: HashMap::new(),
            },
            Registration::new(),
        ))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        items::BLOCKS
            .iter()
            .filter_map(|&b| {
                block_slot_count(b).map(|slots| BlockEntityType {
                    block: b.into(),
                    slots,
                    ticks: false,
                })
            })
            .collect()
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, _saved: &[u8]) {
        self.entities.insert(handle, block.to_string());
        if let Some(slots) = block_slot_count(block) {
            for slot in 0..slots {
                let _ = item_transfer::set_slot_access(handle, slot, Face::Up, true, true);
                let _ = item_transfer::set_slot_access(handle, slot, Face::Down, false, true);
                let _ = item_transfer::set_slot_access(handle, slot, Face::North, true, true);
                let _ = item_transfer::set_slot_access(handle, slot, Face::South, true, true);
                let _ = item_transfer::set_slot_access(handle, slot, Face::West, true, true);
                let _ = item_transfer::set_slot_access(handle, slot, Face::East, true, true);
            }
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        self.entities
            .get(&handle)
            .map(|name| name.as_bytes().to_vec())
            .unwrap_or_default()
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.entities.remove(&handle);
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
        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            if let Some(spec) = make_screen_for_block(&event.state) {
                if ui::open_screen(&spec).is_ok() {
                    return EventOutcome::Consume;
                }
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Bibliocraft);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_counts() {
        assert_eq!(block_slot_count("bibliocraft:oak_bookcase"), Some(16));
        assert_eq!(block_slot_count("bibliocraft:birch_shelf"), Some(4));
        assert_eq!(block_slot_count("bibliocraft:spruce_tool_rack"), Some(4));
        assert_eq!(block_slot_count("bibliocraft:acacia_display_case"), Some(1));
        assert_eq!(block_slot_count("bibliocraft:oak_table"), Some(1));
        assert_eq!(block_slot_count("bibliocraft:oak_fancy_crafter"), Some(10));
        assert_eq!(block_slot_count("bibliocraft:oak_fancy_sign"), None);
    }

    #[test]
    fn test_screen_specs() {
        let bookcase_screen = make_screen_for_block("bibliocraft:oak_bookcase").unwrap();
        assert_eq!(bookcase_screen.title, "Bookcase");
        assert_eq!(bookcase_screen.widgets.len(), 17); // 1 title label + 16 slots

        let shelf_screen = make_screen_for_block("bibliocraft:oak_shelf").unwrap();
        assert_eq!(shelf_screen.title, "Shelf");
        assert_eq!(shelf_screen.widgets.len(), 5); // 1 title label + 4 slots
    }
}

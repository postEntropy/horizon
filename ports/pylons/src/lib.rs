//! Pylons: automated chunk pylons (harvester, expulsion, infusion, interdiction, protection).
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    item_transfer::{self, Face},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub fn harvester_screen() -> ScreenSpec {
    let mut widgets = vec![
        Widget {
            id: 0,
            kind: WidgetKind::Label,
            x: 8,
            y: 8,
            width: 160,
            height: 12,
            text: "Harvester Pylon (9x9 Area)".into(),
            value: 0,
            max_value: 0,
        },
        // Tool slot (id 1)
        Widget {
            id: 1,
            kind: WidgetKind::Slot,
            x: 26,
            y: 35,
            width: 18,
            height: 18,
            text: "Tool (Hoe/Axe)".into(),
            value: 0,
            max_value: 1,
        },
        // Progress arrow
        Widget {
            id: 2,
            kind: WidgetKind::ProgressArrow,
            x: 52,
            y: 35,
            width: 24,
            height: 17,
            text: "Harvest".into(),
            value: 80,
            max_value: 100,
        },
    ];

    // 9 harvest inventory slots (3x3 grid)
    for row in 0..3 {
        for col in 0..3 {
            let slot_idx = 3 + row * 3 + col;
            widgets.push(Widget {
                id: slot_idx as u32,
                kind: WidgetKind::Slot,
                x: 88 + col as i32 * 18,
                y: 18 + row as i32 * 18,
                width: 18,
                height: 18,
                text: format!("Drop {}", row * 3 + col + 1),
                value: 0,
                max_value: 64,
            });
        }
    }

    ScreenSpec {
        id: "screen:pylons:harvester".into(),
        title: "Harvester Pylon".into(),
        width: 176,
        height: 166,
        widgets,
    }
}

pub fn infusion_screen() -> ScreenSpec {
    let mut widgets = vec![Widget {
        id: 0,
        kind: WidgetKind::Label,
        x: 8,
        y: 8,
        width: 160,
        height: 12,
        text: "Infusion Pylon (Active Effects)".into(),
        value: 0,
        max_value: 0,
    }];

    for i in 0..4 {
        widgets.push(Widget {
            id: (i + 1) as u32,
            kind: WidgetKind::Slot,
            x: 52 + (i % 2) * 36,
            y: 28 + (i / 2) * 36,
            width: 18,
            height: 18,
            text: format!("Potion Filter {}", i + 1),
            value: 0,
            max_value: 1,
        });
    }

    ScreenSpec {
        id: "screen:pylons:infusion".into(),
        title: "Infusion Pylon".into(),
        width: 176,
        height: 166,
        widgets,
    }
}

pub fn filter_pylon_screen(id: &str, title: &str, filter_name: &str) -> ScreenSpec {
    ScreenSpec {
        id: id.into(),
        title: title.into(),
        width: 176,
        height: 140,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 8,
                width: 160,
                height: 12,
                text: title.into(),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Slot,
                x: 79,
                y: 35,
                width: 18,
                height: 18,
                text: filter_name.into(),
                value: 0,
                max_value: 1,
            },
        ],
    }
}

pub struct Pylons;

impl Mod for Pylons {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Pylons, Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        vec![
            BlockEntityType {
                block: "pylons:harvester_pylon".into(),
                slots: 10,
                ticks: true,
            },
            BlockEntityType {
                block: "pylons:infusion_pylon".into(),
                slots: 4,
                ticks: true,
            },
            BlockEntityType {
                block: "pylons:expulsion_pylon".into(),
                slots: 1,
                ticks: true,
            },
            BlockEntityType {
                block: "pylons:interdiction_pylon".into(),
                slots: 1,
                ticks: true,
            },
            BlockEntityType {
                block: "pylons:protection_pylon".into(),
                slots: 1,
                ticks: true,
            },
        ]
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, _saved: &[u8]) {
        if block.contains("harvester_pylon") {
            // Slot 0 (tool): insert from top or sides
            let _ = item_transfer::set_slot_access(handle, 0, Face::Up, true, false);
            let _ = item_transfer::set_slot_access(handle, 0, Face::North, true, false);
            // Slots 1..=9 (harvested crops): extract from bottom
            for slot in 1..10 {
                let _ = item_transfer::set_slot_access(handle, slot, Face::Down, false, true);
            }
        } else if block.contains("infusion_pylon") {
            for slot in 0..4 {
                let _ = item_transfer::set_slot_access(handle, slot, Face::Up, true, false);
            }
        } else {
            let _ = item_transfer::set_slot_access(handle, 0, Face::Up, true, false);
        }
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
            if event.item.contains("clear_") {
                chat::show("⚡ Pylons: Filter cleared and reset.");
                return EventOutcome::Consume;
            } else if event.item.contains("filter") {
                chat::show("⚡ Pylons: Filter configured for target entity / potion.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            if event.state.contains("harvester_pylon") {
                let _ = ui::open_screen(&harvester_screen());
                return EventOutcome::Consume;
            } else if event.state.contains("infusion_pylon") {
                let _ = ui::open_screen(&infusion_screen());
                return EventOutcome::Consume;
            } else if event.state.contains("expulsion_pylon") {
                let _ = ui::open_screen(&filter_pylon_screen(
                    "screen:pylons:expulsion",
                    "Expulsion Pylon",
                    "Player Filter",
                ));
                return EventOutcome::Consume;
            } else if event.state.contains("interdiction_pylon") {
                let _ = ui::open_screen(&filter_pylon_screen(
                    "screen:pylons:interdiction",
                    "Interdiction Pylon",
                    "Mob Filter",
                ));
                return EventOutcome::Consume;
            } else if event.state.contains("protection_pylon") {
                let _ = ui::open_screen(&filter_pylon_screen(
                    "screen:pylons:protection",
                    "Protection Pylon",
                    "Territory Filter",
                ));
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Pylons);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_harvester_screen() {
        let screen = harvester_screen();
        assert_eq!(screen.id, "screen:pylons:harvester");
        // Title label + Tool slot + Progress arrow + 9 drop slots = 12 widgets
        assert_eq!(screen.widgets.len(), 12);
    }

    #[test]
    fn test_infusion_screen() {
        let screen = infusion_screen();
        assert_eq!(screen.id, "screen:pylons:infusion");
        // Title + 4 filter slots = 5 widgets
        assert_eq!(screen.widgets.len(), 5);
    }

    #[test]
    fn test_filter_screen() {
        let screen = filter_pylon_screen(
            "screen:pylons:expulsion",
            "Expulsion Pylon",
            "Player Filter",
        );
        assert_eq!(screen.id, "screen:pylons:expulsion");
        assert_eq!(screen.widgets.len(), 2);
    }
}

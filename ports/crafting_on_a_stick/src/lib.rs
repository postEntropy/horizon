//! Crafting On A Stick: portable crafting tables, anvils, smithing tables, and looms in hand.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StickStation {
    CraftingTable,
    Anvil,
    Grindstone,
    SmithingTable,
    Stonecutter,
    Loom,
    CartographyTable,
}

impl StickStation {
    pub fn from_item(item: &str) -> Option<Self> {
        if item.contains("crafting_table") {
            Some(StickStation::CraftingTable)
        } else if item.contains("anvil") {
            Some(StickStation::Anvil)
        } else if item.contains("grindstone") {
            Some(StickStation::Grindstone)
        } else if item.contains("smithing_table") {
            Some(StickStation::SmithingTable)
        } else if item.contains("stonecutter") {
            Some(StickStation::Stonecutter)
        } else if item.contains("loom") {
            Some(StickStation::Loom)
        } else if item.contains("cartography_table") {
            Some(StickStation::CartographyTable)
        } else {
            None
        }
    }
}

pub fn station_screen(station: StickStation) -> ScreenSpec {
    match station {
        StickStation::CraftingTable => {
            let mut widgets = vec![Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 6,
                width: 160,
                height: 12,
                text: "Crafting on a Stick".into(),
                value: 0,
                max_value: 0,
            }];
            // 3x3 crafting grid slots (id 1..=9)
            for row in 0..3 {
                for col in 0..3 {
                    widgets.push(Widget {
                        id: (1 + row * 3 + col) as u32,
                        kind: WidgetKind::Slot,
                        x: 30 + col * 18,
                        y: 17 + row * 18,
                        width: 18,
                        height: 18,
                        text: format!("Grid {}", row * 3 + col + 1),
                        value: 0,
                        max_value: 64,
                    });
                }
            }
            // Arrow indicator
            widgets.push(Widget {
                id: 10,
                kind: WidgetKind::ProgressArrow,
                x: 90,
                y: 35,
                width: 22,
                height: 15,
                text: "Craft".into(),
                value: 100,
                max_value: 100,
            });
            // Result slot (id 11)
            widgets.push(Widget {
                id: 11,
                kind: WidgetKind::Slot,
                x: 124,
                y: 35,
                width: 18,
                height: 18,
                text: "Result".into(),
                value: 0,
                max_value: 64,
            });

            ScreenSpec {
                id: "screen:crafting_on_a_stick:crafting_table".into(),
                title: "Crafting Table".into(),
                width: 176,
                height: 166,
                widgets,
            }
        }
        StickStation::Anvil => ScreenSpec {
            id: "screen:crafting_on_a_stick:anvil".into(),
            title: "Anvil on a Stick".into(),
            width: 176,
            height: 166,
            widgets: vec![
                Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 8,
                    y: 6,
                    width: 160,
                    height: 12,
                    text: "Repair & Name".into(),
                    value: 0,
                    max_value: 0,
                },
                Widget {
                    id: 1,
                    kind: WidgetKind::Slot,
                    x: 27,
                    y: 47,
                    width: 18,
                    height: 18,
                    text: "Item".into(),
                    value: 0,
                    max_value: 1,
                },
                Widget {
                    id: 2,
                    kind: WidgetKind::Slot,
                    x: 76,
                    y: 47,
                    width: 18,
                    height: 18,
                    text: "Material".into(),
                    value: 0,
                    max_value: 64,
                },
                Widget {
                    id: 3,
                    kind: WidgetKind::ProgressArrow,
                    x: 102,
                    y: 47,
                    width: 22,
                    height: 15,
                    text: "Hammer".into(),
                    value: 100,
                    max_value: 100,
                },
                Widget {
                    id: 4,
                    kind: WidgetKind::Slot,
                    x: 134,
                    y: 47,
                    width: 18,
                    height: 18,
                    text: "Result".into(),
                    value: 0,
                    max_value: 1,
                },
            ],
        },
        StickStation::Grindstone => ScreenSpec {
            id: "screen:crafting_on_a_stick:grindstone".into(),
            title: "Grindstone on a Stick".into(),
            width: 176,
            height: 166,
            widgets: vec![
                Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 8,
                    y: 6,
                    width: 160,
                    height: 12,
                    text: "Disenchant & Repair".into(),
                    value: 0,
                    max_value: 0,
                },
                Widget {
                    id: 1,
                    kind: WidgetKind::Slot,
                    x: 49,
                    y: 19,
                    width: 18,
                    height: 18,
                    text: "Top".into(),
                    value: 0,
                    max_value: 1,
                },
                Widget {
                    id: 2,
                    kind: WidgetKind::Slot,
                    x: 49,
                    y: 40,
                    width: 18,
                    height: 18,
                    text: "Bottom".into(),
                    value: 0,
                    max_value: 1,
                },
                Widget {
                    id: 3,
                    kind: WidgetKind::Slot,
                    x: 129,
                    y: 34,
                    width: 18,
                    height: 18,
                    text: "Result".into(),
                    value: 0,
                    max_value: 1,
                },
            ],
        },
        StickStation::SmithingTable => ScreenSpec {
            id: "screen:crafting_on_a_stick:smithing_table".into(),
            title: "Upgrade Gear".into(),
            width: 176,
            height: 166,
            widgets: vec![
                Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 8,
                    y: 6,
                    width: 160,
                    height: 12,
                    text: "Smithing on a Stick".into(),
                    value: 0,
                    max_value: 0,
                },
                Widget {
                    id: 1,
                    kind: WidgetKind::Slot,
                    x: 19,
                    y: 45,
                    width: 18,
                    height: 18,
                    text: "Template".into(),
                    value: 0,
                    max_value: 1,
                },
                Widget {
                    id: 2,
                    kind: WidgetKind::Slot,
                    x: 39,
                    y: 45,
                    width: 18,
                    height: 18,
                    text: "Base".into(),
                    value: 0,
                    max_value: 1,
                },
                Widget {
                    id: 3,
                    kind: WidgetKind::Slot,
                    x: 59,
                    y: 45,
                    width: 18,
                    height: 18,
                    text: "Addition".into(),
                    value: 0,
                    max_value: 1,
                },
                Widget {
                    id: 4,
                    kind: WidgetKind::ProgressArrow,
                    x: 88,
                    y: 46,
                    width: 22,
                    height: 15,
                    text: "Upgrade".into(),
                    value: 100,
                    max_value: 100,
                },
                Widget {
                    id: 5,
                    kind: WidgetKind::Slot,
                    x: 124,
                    y: 45,
                    width: 18,
                    height: 18,
                    text: "Result".into(),
                    value: 0,
                    max_value: 1,
                },
            ],
        },
        StickStation::Stonecutter => ScreenSpec {
            id: "screen:crafting_on_a_stick:stonecutter".into(),
            title: "Stonecutter on a Stick".into(),
            width: 176,
            height: 166,
            widgets: vec![
                Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 8,
                    y: 6,
                    width: 160,
                    height: 12,
                    text: "Stonecutter".into(),
                    value: 0,
                    max_value: 0,
                },
                Widget {
                    id: 1,
                    kind: WidgetKind::Slot,
                    x: 20,
                    y: 33,
                    width: 18,
                    height: 18,
                    text: "Input".into(),
                    value: 0,
                    max_value: 64,
                },
                Widget {
                    id: 2,
                    kind: WidgetKind::Slot,
                    x: 143,
                    y: 33,
                    width: 18,
                    height: 18,
                    text: "Result".into(),
                    value: 0,
                    max_value: 64,
                },
            ],
        },
        StickStation::Loom => ScreenSpec {
            id: "screen:crafting_on_a_stick:loom".into(),
            title: "Loom on a Stick".into(),
            width: 176,
            height: 166,
            widgets: vec![
                Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 8,
                    y: 6,
                    width: 160,
                    height: 12,
                    text: "Loom".into(),
                    value: 0,
                    max_value: 0,
                },
                Widget {
                    id: 1,
                    kind: WidgetKind::Slot,
                    x: 13,
                    y: 26,
                    width: 18,
                    height: 18,
                    text: "Banner".into(),
                    value: 0,
                    max_value: 1,
                },
                Widget {
                    id: 2,
                    kind: WidgetKind::Slot,
                    x: 33,
                    y: 26,
                    width: 18,
                    height: 18,
                    text: "Dye".into(),
                    value: 0,
                    max_value: 64,
                },
                Widget {
                    id: 3,
                    kind: WidgetKind::Slot,
                    x: 23,
                    y: 45,
                    width: 18,
                    height: 18,
                    text: "Pattern".into(),
                    value: 0,
                    max_value: 1,
                },
                Widget {
                    id: 4,
                    kind: WidgetKind::Slot,
                    x: 134,
                    y: 45,
                    width: 18,
                    height: 18,
                    text: "Result".into(),
                    value: 0,
                    max_value: 1,
                },
            ],
        },
        StickStation::CartographyTable => ScreenSpec {
            id: "screen:crafting_on_a_stick:cartography_table".into(),
            title: "Cartography on a Stick".into(),
            width: 176,
            height: 166,
            widgets: vec![
                Widget {
                    id: 0,
                    kind: WidgetKind::Label,
                    x: 8,
                    y: 6,
                    width: 160,
                    height: 12,
                    text: "Cartography Table".into(),
                    value: 0,
                    max_value: 0,
                },
                Widget {
                    id: 1,
                    kind: WidgetKind::Slot,
                    x: 15,
                    y: 15,
                    width: 18,
                    height: 18,
                    text: "Map".into(),
                    value: 0,
                    max_value: 1,
                },
                Widget {
                    id: 2,
                    kind: WidgetKind::Slot,
                    x: 15,
                    y: 52,
                    width: 18,
                    height: 18,
                    text: "Paper".into(),
                    value: 0,
                    max_value: 64,
                },
                Widget {
                    id: 3,
                    kind: WidgetKind::Slot,
                    x: 145,
                    y: 39,
                    width: 18,
                    height: 18,
                    text: "Result".into(),
                    value: 0,
                    max_value: 1,
                },
            ],
        },
    }
}

pub struct CraftingOnAStick;

impl Mod for CraftingOnAStick {
    fn init() -> Result<(Self, Registration), String> {
        Ok((CraftingOnAStick, Registration::new()))
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
            if event.item.contains("uncraft_") {
                let station_name = event
                    .item
                    .trim_start_matches("crafting_on_a_stick:uncraft_");
                chat::show(&format!("🔨 Disassembled into stick + {station_name}."));
                return EventOutcome::Consume;
            }

            if let Some(station) = StickStation::from_item(&event.item) {
                let screen = station_screen(station);
                let _ = ui::open_screen(&screen);
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(CraftingOnAStick);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_station_identification() {
        assert_eq!(
            StickStation::from_item("crafting_on_a_stick:crafting_table"),
            Some(StickStation::CraftingTable)
        );
        assert_eq!(
            StickStation::from_item("crafting_on_a_stick:chipped_anvil"),
            Some(StickStation::Anvil)
        );
        assert_eq!(
            StickStation::from_item("crafting_on_a_stick:grindstone"),
            Some(StickStation::Grindstone)
        );
        assert_eq!(
            StickStation::from_item("crafting_on_a_stick:smithing_table"),
            Some(StickStation::SmithingTable)
        );
        assert_eq!(
            StickStation::from_item("crafting_on_a_stick:stonecutter"),
            Some(StickStation::Stonecutter)
        );
        assert_eq!(
            StickStation::from_item("crafting_on_a_stick:loom"),
            Some(StickStation::Loom)
        );
        assert_eq!(
            StickStation::from_item("crafting_on_a_stick:cartography_table"),
            Some(StickStation::CartographyTable)
        );
    }

    #[test]
    fn test_crafting_table_screen_spec() {
        let screen = station_screen(StickStation::CraftingTable);
        assert_eq!(screen.id, "screen:crafting_on_a_stick:crafting_table");
        // Label + 9 grid slots + arrow + result slot = 12 widgets
        assert_eq!(screen.widgets.len(), 12);
    }

    #[test]
    fn test_smithing_table_screen_spec() {
        let screen = station_screen(StickStation::SmithingTable);
        assert_eq!(screen.id, "screen:crafting_on_a_stick:smithing_table");
        // Label + 3 input slots + arrow + result slot = 6 widgets
        assert_eq!(screen.widgets.len(), 6);
    }
}

//! Productive Metalworks: multiblock foundry controller, metal casting drains, and capacitors.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    item_transfer::{self, Face},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub fn casting_screen(title: &str) -> ScreenSpec {
    ScreenSpec {
        id: "screen:productivemetalworks:casting".into(),
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
            // Slot 0: Cast/Mold
            Widget {
                id: 1,
                kind: WidgetKind::Slot,
                x: 50,
                y: 35,
                width: 18,
                height: 18,
                text: "Cast".into(),
                value: 0,
                max_value: 1,
            },
            // Flame/Cooling progress
            Widget {
                id: 2,
                kind: WidgetKind::Flame,
                x: 80,
                y: 36,
                width: 14,
                height: 14,
                text: "Cooling".into(),
                value: 0,
                max_value: 100,
            },
            // Slot 1: Output
            Widget {
                id: 3,
                kind: WidgetKind::Slot,
                x: 108,
                y: 35,
                width: 18,
                height: 18,
                text: "Result".into(),
                value: 0,
                max_value: 64,
            },
        ],
    }
}

pub fn foundry_screen() -> ScreenSpec {
    ScreenSpec {
        id: "screen:productivemetalworks:foundry".into(),
        title: "Foundry Controller".into(),
        width: 176,
        height: 166,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 8,
                width: 160,
                height: 12,
                text: "Foundry Controller (1450°C)".into(),
                value: 1450,
                max_value: 2000,
            },
            Widget {
                id: 1,
                kind: WidgetKind::FluidGauge,
                x: 140,
                y: 20,
                width: 16,
                height: 60,
                text: "Molten Metal".into(),
                value: 8000,
                max_value: 16000,
            },
            Widget {
                id: 2,
                kind: WidgetKind::Flame,
                x: 80,
                y: 35,
                width: 16,
                height: 16,
                text: "Heat".into(),
                value: 95,
                max_value: 100,
            },
            Widget {
                id: 3,
                kind: WidgetKind::Slot,
                x: 40,
                y: 35,
                width: 18,
                height: 18,
                text: "Raw Metal".into(),
                value: 0,
                max_value: 64,
            },
        ],
    }
}

pub struct ProductiveMetalworks;

impl Mod for ProductiveMetalworks {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ProductiveMetalworks, Registration::new()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        let mut types = vec![
            BlockEntityType {
                block: "productivemetalworks:casting_table".into(),
                slots: 2,
                ticks: true,
            },
            BlockEntityType {
                block: "productivemetalworks:casting_basin".into(),
                slots: 1,
                ticks: true,
            },
        ];

        for &block in items::BLOCKS {
            if block.contains("foundry_controller") {
                types.push(BlockEntityType {
                    block: block.into(),
                    slots: 4,
                    ticks: true,
                });
            }
        }

        types
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, _saved: &[u8]) {
        if block.contains("casting_table") {
            // Slot 0 (cast): top/sides insert
            let _ = item_transfer::set_slot_access(handle, 0, Face::Up, true, false);
            // Slot 1 (output): bottom extract
            let _ = item_transfer::set_slot_access(handle, 1, Face::Down, false, true);
        } else if block.contains("casting_basin") {
            // Slot 0 (output block): bottom extract
            let _ = item_transfer::set_slot_access(handle, 0, Face::Down, false, true);
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

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            if event.state.contains("foundry_controller") {
                let _ = ui::open_screen(&foundry_screen());
                chat::show(
                    "🔥 Productive Metalworks: Foundry Multiblock operational. Temperature: 1,450°C.",
                );
                return EventOutcome::Consume;
            } else if event.state.contains("casting_table") {
                let _ = ui::open_screen(&casting_screen("Casting Table"));
                return EventOutcome::Consume;
            } else if event.state.contains("casting_basin") {
                let _ = ui::open_screen(&casting_screen("Casting Basin"));
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(ProductiveMetalworks);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screens_generation() {
        let casting = casting_screen("Casting Table");
        assert_eq!(casting.id, "screen:productivemetalworks:casting");
        assert_eq!(casting.widgets.len(), 4);

        let foundry = foundry_screen();
        assert_eq!(foundry.id, "screen:productivemetalworks:foundry");
        assert_eq!(foundry.widgets.len(), 4);
    }
}

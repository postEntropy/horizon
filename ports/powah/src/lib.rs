//! Powah: Furnator, Magmator, Thermo Generator, Solar Panels, Energy Cells, and Energizing Orbs.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct Powah;

impl Mod for Powah {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Powah, Registration::new()))
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
        if event.item.contains("wrench") && event.action == ItemAction::UseOnBlock {
            chat::show("⚡ Powah Wrench: Configured generator I/O side mode.");
            return EventOutcome::Consume;
        } else if event.item.contains("player_transmitter") && (event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock) {
            chat::show("🔋 Powah Player Transmitter: Linked to wireless power network.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            chat::show("⚡ Powah: Generator active (Generating 1,200 FE/t | Buffer: 1,000,000 FE).");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:powah:generator".into(),
                title: "Powah Generator".into(),
                width: 176,
                height: 166,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::EnergyBar,
                        x: 140,
                        y: 18,
                        width: 14,
                        height: 52,
                        text: "FE".into(),
                        value: 1000000,
                        max_value: 1000000,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::Flame,
                        x: 80,
                        y: 35,
                        width: 16,
                        height: 16,
                        text: "Heat".into(),
                        value: 80,
                        max_value: 100,
                    },
                ],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Powah);

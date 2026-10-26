//! Hostile Neural Networks: Deep Learner, Data Models, Simulation Chamber, and Loot Fabricator.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct HostileNetworks;

impl Mod for HostileNetworks {
    fn init() -> Result<(Self, Registration), String> {
        Ok((HostileNetworks, Registration::new()))
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
        if event.item.contains("deep_learner") && (event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock) {
            chat::show("🧠 Deep Learner: 4 / 4 Active Data Models [Wither: Tier 4 (Self-Aware)].");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:hostilenetworks:deep_learner".into(),
                title: "Deep Learner HUD".into(),
                width: 200,
                height: 160,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::Label,
                        x: 10,
                        y: 15,
                        width: 180,
                        height: 14,
                        text: "Model 1: Wither Skeleton [Tier: Self-Aware]".into(),
                        value: 100,
                        max_value: 100,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::Label,
                        x: 10,
                        y: 35,
                        width: 180,
                        height: 14,
                        text: "Model 2: Ender Dragon [Tier: Flawed]".into(),
                        value: 40,
                        max_value: 100,
                    },
                ],
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            if event.state.contains("sim_chamber") {
                chat::show("⚡ Simulation Chamber: Running simulations | Output: 100% success rate.");
                let _ = ui::open_screen(&ScreenSpec {
                    id: "screen:hostilenetworks:sim_chamber".into(),
                    title: "Simulation Chamber".into(),
                    width: 176,
                    height: 166,
                    widgets: vec![
                        Widget {
                            id: 0,
                            kind: WidgetKind::Slot,
                            x: 40,
                            y: 30,
                            width: 18,
                            height: 18,
                            text: "Model".into(),
                            value: 0,
                            max_value: 1,
                        },
                        Widget {
                            id: 1,
                            kind: WidgetKind::Slot,
                            x: 80,
                            y: 30,
                            width: 18,
                            height: 18,
                            text: "Matrix".into(),
                            value: 0,
                            max_value: 64,
                        },
                        Widget {
                            id: 2,
                            kind: WidgetKind::ProgressArrow,
                            x: 60,
                            y: 55,
                            width: 56,
                            height: 12,
                            text: "Simulating".into(),
                            value: 80,
                            max_value: 100,
                        },
                    ],
                });
                return EventOutcome::Consume;
            } else if event.state.contains("loot_fabricator") {
                chat::show("🏭 Loot Fabricator: Synthesizing drops from predictions.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(HostileNetworks);

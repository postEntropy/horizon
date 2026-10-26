//! FTB Quests: quest book UI, task completion, loot crate rewards, and quest screens.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent,
    LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

struct FtbQuests;

impl Mod for FtbQuests {
    fn init() -> Result<(Self, Registration), String> {
        Ok((FtbQuests, Registration::new()))
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
        if event.item.contains("book") {
            chat::show("📖 FTB Quests: Opening Quest Book...");
            let _ = ui::open_screen(&ScreenSpec {
                id: "screen:ftbquests:quest_book".into(),
                title: "FTB Quests - Chapter Progression".into(),
                width: 256,
                height: 180,
                widgets: vec![
                    Widget {
                        id: 0,
                        kind: WidgetKind::Label,
                        x: 10,
                        y: 10,
                        width: 236,
                        height: 14,
                        text: "All The Mods 10 Quests: Chapter 1 - Getting Started".into(),
                        value: 1,
                        max_value: 10,
                    },
                    Widget {
                        id: 1,
                        kind: WidgetKind::ProgressArrow,
                        x: 10,
                        y: 35,
                        width: 236,
                        height: 16,
                        text: "Progression: 45% Complete".into(),
                        value: 45,
                        max_value: 100,
                    },
                ],
            });
            return EventOutcome::Consume;
        }

        if event.item.contains("lootcrate") && (event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock) {
            chat::show("🎁 FTB Quests: Opened Loot Crate! Reward claimed.");
            return EventOutcome::Consume;
        }

        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::UseWithoutItem {
            if event.state.contains("screen") {
                chat::show("🖥️ FTB Quests: Quest Task Screen opened.");
                let _ = ui::open_screen(&ScreenSpec {
                    id: "screen:ftbquests:task_screen".into(),
                    title: "FTB Quest Task Screen".into(),
                    width: 176,
                    height: 166,
                    widgets: vec![
                        Widget {
                            id: 0,
                            kind: WidgetKind::Label,
                            x: 10,
                            y: 15,
                            width: 156,
                            height: 14,
                            text: "Active Task: Submit Items".into(),
                            value: 1,
                            max_value: 1,
                        },
                        Widget {
                            id: 1,
                            kind: WidgetKind::Slot,
                            x: 79,
                            y: 40,
                            width: 18,
                            height: 18,
                            text: "Input".into(),
                            value: 0,
                            max_value: 64,
                        },
                    ],
                });
                return EventOutcome::Consume;
            } else if event.state.contains("loot_crate_opener") {
                chat::show("📦 FTB Quests: Loot Crate Opener active.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(FtbQuests);

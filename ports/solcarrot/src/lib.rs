//! Spice of Life: Carrot Edition: grants extra hearts for eating diverse foods.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};
use std::collections::HashSet;

struct SolCarrot {
    eaten_foods: HashSet<String>,
}

impl Mod for SolCarrot {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            SolCarrot {
                eaten_foods: HashSet::new(),
            },
            Registration::new(),
        ))
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
        if event.item == "solcarrot:food_book" {
            if event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock {
                let count = self.eaten_foods.len();
                let milestone = ((count / 5) + 1) * 5;
                let needed = milestone.saturating_sub(count);
                let _ = ui::open_screen(&ScreenSpec {
                    id: "screen:solcarrot:food_book".into(),
                    title: "Food Book".into(),
                    width: 176,
                    height: 166,
                    widgets: vec![
                        Widget {
                            id: 0,
                            kind: WidgetKind::Label,
                            x: 10,
                            y: 10,
                            width: 156,
                            height: 14,
                            text: format!("Unique foods eaten: {count}"),
                            value: 0,
                            max_value: 0,
                        },
                        Widget {
                            id: 1,
                            kind: WidgetKind::Label,
                            x: 10,
                            y: 30,
                            width: 156,
                            height: 14,
                            text: format!("Eat {needed} more for +1 Heart!"),
                            value: 0,
                            max_value: 0,
                        },
                    ],
                });
                return EventOutcome::Consume;
            }
        }

        if event.action == ItemAction::FinishUsing {
            let newly_discovered = self.eaten_foods.insert(event.item.clone());
            if newly_discovered {
                let total = self.eaten_foods.len();
                if total % 5 == 0 {
                    chat::show(&format!(
                        "❤️ Milestone reached! ({total} foods eaten) +1 Max Heart!"
                    ));
                } else {
                    let next = ((total / 5) + 1) * 5;
                    chat::show(&format!(
                        "🥕 New food tasted! ({total} foods eaten, {rem} to milestone)",
                        rem = next - total
                    ));
                }
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(SolCarrot);

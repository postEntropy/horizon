//! Toolbelt: native Horizon WebAssembly component.
//! Provides the Tool Belt utility: portable wearable tool storage (2 to 9 slots),
//! pouch upgrades, tool swapping, and dedicated belt pouch GUI.
pub mod items;

use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub struct Toolbelt {
    pub max_slots: u32,
}

impl Default for Toolbelt {
    fn default() -> Self {
        Self { max_slots: 9 }
    }
}

pub fn belt_screen(slots: u32) -> ScreenSpec {
    let count = slots.clamp(2, 9);
    let mut widgets = Vec::with_capacity(count as usize);
    let start_x = (176 - (count as i32 * 18)) / 2;

    for i in 0..count {
        widgets.push(Widget {
            id: i,
            kind: WidgetKind::Slot,
            x: start_x + (i as i32 * 18),
            y: 35,
            width: 18,
            height: 18,
            text: format!("Tool {}", i + 1),
            value: 0,
            max_value: 1,
        });
    }

    ScreenSpec {
        id: "toolbelt:belt".into(),
        title: "Tool Belt".into(),
        width: 176,
        height: 166,
        widgets,
    }
}

pub enum ToolbeltAction {
    OpenScreen(ScreenSpec),
    ShowInfo(String),
    Pass,
}

impl Toolbelt {
    pub fn evaluate_item(&self, item_name: &str, action: ItemAction) -> ToolbeltAction {
        if matches!(action, ItemAction::UseInAir | ItemAction::UseOnBlock) {
            if item_name.contains("belt") && !item_name.contains("pouch") {
                ToolbeltAction::OpenScreen(belt_screen(self.max_slots))
            } else if item_name.contains("pouch") {
                ToolbeltAction::ShowInfo(
                    "Pouch: Combine with Tool Belt in an Anvil or Crafting Table to expand slots (up to 9)!"
                        .into(),
                )
            } else {
                ToolbeltAction::Pass
            }
        } else {
            ToolbeltAction::Pass
        }
    }
}

impl Mod for Toolbelt {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Toolbelt::default(), Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for item in items::ITEMS {
            hooks = hooks.item(item);
        }
        hooks
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        match self.evaluate_item(&event.item, event.action) {
            ToolbeltAction::OpenScreen(spec) => {
                let _ = ui::open_screen(&spec);
                EventOutcome::Consume
            }
            ToolbeltAction::ShowInfo(msg) => {
                chat::show(&msg);
                EventOutcome::Consume
            }
            ToolbeltAction::Pass => EventOutcome::Pass,
        }
    }
}

horizon_mod_sdk::export_mod!(Toolbelt);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_belt_screen_layout() {
        let screen = belt_screen(9);
        assert_eq!(screen.id, "toolbelt:belt");
        assert_eq!(screen.widgets.len(), 9);
        for (i, w) in screen.widgets.iter().enumerate() {
            assert_eq!(w.id, i as u32);
            assert_eq!(w.kind, WidgetKind::Slot);
        }
    }

    #[test]
    fn test_belt_upgrade_clamping() {
        let screen_min = belt_screen(1);
        assert_eq!(screen_min.widgets.len(), 2);
        let screen_max = belt_screen(15);
        assert_eq!(screen_max.widgets.len(), 9);
    }

    #[test]
    fn test_item_evaluation() {
        let tb = Toolbelt::default();
        match tb.evaluate_item("toolbelt:belt", ItemAction::UseInAir) {
            ToolbeltAction::OpenScreen(screen) => {
                assert_eq!(screen.id, "toolbelt:belt");
                assert_eq!(screen.widgets.len(), 9);
            }
            _ => panic!("Expected OpenScreen for belt"),
        }

        match tb.evaluate_item("toolbelt:pouch", ItemAction::UseOnBlock) {
            ToolbeltAction::ShowInfo(info) => {
                assert!(info.contains("Pouch: Combine with Tool Belt"));
            }
            _ => panic!("Expected ShowInfo for pouch"),
        }

        assert!(matches!(
            tb.evaluate_item("minecraft:stick", ItemAction::UseInAir),
            ToolbeltAction::Pass
        ));
    }
}

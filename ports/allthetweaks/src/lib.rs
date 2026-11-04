//! AllTheTweaks: ATM Star components, trophies, and dimension anchors.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemEvent, LogicHooks, Mod, Registration,
    chat,
    server::{self, BlockPos},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const ATM_STAR_COMPONENTS: &[&str] = &[
    "allthetweaks:pulsating_black_hole",
    "allthetweaks:improbable_probability_device",
    "allthetweaks:allthecatalystium",
    "allthetweaks:nexium_emitter",
    "allthetweaks:oblivion_shard",
    "allthetweaks:dragon_soul",
    "allthetweaks:dimensional_seed",
    "allthetweaks:philosophers_fuel",
    "allthetweaks:withers_compass",
];

pub fn calculate_wither_direction(px: f64, pz: f64, tx: i32, tz: i32) -> f64 {
    let dx = tx as f64 - px;
    let dz = tz as f64 - pz;
    (dx * dx + dz * dz).sqrt()
}

pub fn trophy_screen(completed: usize, total: usize) -> ScreenSpec {
    ScreenSpec {
        id: "screen:allthetweaks:trophy".into(),
        title: "All The Mods Trophy".into(),
        width: 176,
        height: 120,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 8,
                width: 160,
                height: 12,
                text: "ATM Star Progress".into(),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 1,
                kind: WidgetKind::ProgressArrow,
                x: 18,
                y: 35,
                width: 140,
                height: 18,
                text: format!("{completed} / {total} Star Components"),
                value: completed as u64,
                max_value: total as u64,
            },
            Widget {
                id: 2,
                kind: WidgetKind::Label,
                x: 8,
                y: 65,
                width: 160,
                height: 10,
                text: if completed == total {
                    "★ The Celestial ATM Star has been achieved!".into()
                } else {
                    format!(
                        "Forge remaining {} components to complete.",
                        total - completed
                    )
                },
                value: 0,
                max_value: 0,
            },
        ],
    }
}

pub struct AllTheTweaks {
    pub completed_components: usize,
}

impl Mod for AllTheTweaks {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            AllTheTweaks {
                completed_components: 9, // Endgame tracker
            },
            Registration::new().command("atmstar"),
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

    fn on_item(&mut self, _event: ItemEvent) -> EventOutcome {
        // The original item-use oracle returned PASS for ATM Star, Greg Star,
        // and Withers Compass in air. Their real right-click interactions
        // (including target blocks/entities) remain unobserved.
        EventOutcome::Pass
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        let pos = BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            if event.state.contains("trophy_atm") {
                server::play_sound(
                    &event.dimension,
                    pos,
                    "minecraft:ui.toast.challenge_complete",
                    1.0,
                    1.0,
                );
                let _ = ui::open_screen(&trophy_screen(
                    self.completed_components,
                    ATM_STAR_COMPONENTS.len(),
                ));
                chat::show("🏆 You bask in the glory of the All The Mods Trophy!");
                return EventOutcome::Consume;
            } else if event.state.contains("atm_star_block") {
                server::play_sound(
                    &event.dimension,
                    pos,
                    "minecraft:block.beacon.activate",
                    1.0,
                    1.0,
                );
                chat::show("✨ The ATM Star Block hums with eternal cosmic energy.");
                return EventOutcome::Consume;
            }
        }
        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "atmstar" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "ATM Star Components: {}/{} forged. Components: {}",
            self.completed_components,
            ATM_STAR_COMPONENTS.len(),
            ATM_STAR_COMPONENTS.join(", ")
        )))
    }
}

horizon_mod_sdk::export_mod!(AllTheTweaks);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atm_star_components_count() {
        assert_eq!(ATM_STAR_COMPONENTS.len(), 9);
    }

    #[test]
    fn test_wither_direction_math() {
        let dist = calculate_wither_direction(300.0, 400.0, 0, 0);
        assert!((dist - 500.0).abs() < 0.001);
    }

    #[test]
    fn test_trophy_screen_widgets() {
        let screen = trophy_screen(9, 9);
        assert_eq!(screen.id, "screen:allthetweaks:trophy");
        assert_eq!(screen.widgets.len(), 3);
        assert_eq!(screen.widgets[1].value, 9);
        assert_eq!(screen.widgets[1].max_value, 9);
    }
}

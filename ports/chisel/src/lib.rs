//! Chisel Reborn: the chisel re-shapes a block into the next variant of its
//! own group, or into the panel of the same material.
//!
//! The block table in `variants.rs` was generated from the mod's carried
//! blockstates, so a right-click cycles the same order the original chisel
//! shows in its menu.
mod items;
mod variants;

use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
    server::{self, BlockPos},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const MODES: [&str; 2] = ["Variants", "Panels"];
const MODE_SCREEN: &str = "screen:chisel:mode";

fn block_id(state: &str) -> &str {
    state.split('[').next().unwrap_or(state)
}

/// `chisel:<group>/<variant>` split into its two parts.
fn split_id(id: &str) -> Option<(&str, &str)> {
    id.strip_prefix("chisel:")?.split_once('/')
}

fn variants_of(name: &str) -> Option<&'static [&'static str]> {
    variants::GROUPS
        .iter()
        .find(|(group, _)| *group == name)
        .map(|(_, variants)| *variants)
}

struct Chisel {
    mode: usize,
}

impl Chisel {
    fn mode_screen(&self) -> ScreenSpec {
        ScreenSpec {
            id: MODE_SCREEN.into(),
            title: format!("Chisel · {}", MODES[self.mode]),
            width: 176,
            height: 110,
            widgets: vec![
                Widget {
                    id: 0,
                    kind: WidgetKind::Button,
                    x: 8,
                    y: 24,
                    width: 76,
                    height: 20,
                    text: "Variants".into(),
                    value: (self.mode == 0) as u64,
                    max_value: 1,
                },
                Widget {
                    id: 1,
                    kind: WidgetKind::Button,
                    x: 92,
                    y: 24,
                    width: 76,
                    height: 20,
                    text: "Panels".into(),
                    value: (self.mode == 1) as u64,
                    max_value: 1,
                },
                Widget {
                    id: 2,
                    kind: WidgetKind::Label,
                    x: 8,
                    y: 58,
                    width: 160,
                    height: 10,
                    text: "Right-click a Chisel block to re-shape it.".into(),
                    value: 0,
                    max_value: 0,
                },
                Widget {
                    id: 3,
                    kind: WidgetKind::Label,
                    x: 8,
                    y: 72,
                    width: 160,
                    height: 10,
                    text: format!(
                        "{} groups, {} variants carried.",
                        variants::GROUPS.len(),
                        variants::GROUPS.iter().map(|(_, v)| v.len()).sum::<usize>()
                    ),
                    value: 0,
                    max_value: 0,
                },
            ],
        }
    }

    /// The next block the chisel turns `id` into.
    fn next_block(&self, family: &str, variant: &str) -> Option<String> {
        if self.mode == 1 {
            let panels = variants_of("panel")?;
            return panels
                .contains(&variant)
                .then(|| format!("chisel:panel/{variant}"));
        }
        let list = variants_of(family)?;
        let index = list.iter().position(|entry| *entry == variant)?;
        let next = list[(index + 1) % list.len()];
        Some(format!("chisel:{family}/{next}"))
    }
}

impl Mod for Chisel {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Chisel { mode: 0 }, Registration::new().command("chisel")))
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
        if event.action == ItemAction::UseInAir {
            let _ = ui::open_screen(&self.mode_screen());
            return EventOutcome::Consume;
        }
        if event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }

        let pos = BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        let target = if event.target.is_empty() {
            match server::get_block(&event.dimension, pos) {
                Some(state) => state,
                None => return EventOutcome::Pass,
            }
        } else {
            event.target.clone()
        };

        let Some((group_name, variant)) = split_id(block_id(&target)) else {
            chat::show("🔨 The chisel only re-shapes Chisel blocks.");
            return EventOutcome::Consume;
        };
        let Some(next) = self.next_block(group_name, variant) else {
            chat::show(&format!(
                "🔨 {} has no panel variant; switch the chisel to Variants.",
                variant.replace('_', " ")
            ));
            return EventOutcome::Consume;
        };

        if server::set_block(&event.dimension, pos, &next).is_ok() {
            server::play_sound(&event.dimension, pos, "chisel:chisel_sound", 0.8, 1.0);
            chat::show(&format!("🔨 {}", next.trim_start_matches("chisel:")));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        if screen_id != MODE_SCREEN {
            return;
        }
        self.mode = (button as usize).min(MODES.len() - 1);
        chat::show(&format!("🔨 Chisel mode: {}", MODES[self.mode]));
        let _ = ui::open_screen(&self.mode_screen());
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "chisel" {
            return Err(format!("Unknown command /{name}"));
        }
        match args.trim() {
            "" => Ok(Some(format!(
                "Chisel: mode {} · {} groups · right-click a Chisel block to cycle it, right-click air to switch mode.",
                MODES[self.mode],
                variants::GROUPS.len()
            ))),
            "variants" => Ok(Some(format!(
                "Groups: {}",
                variants::GROUPS
                    .iter()
                    .map(|(group, _)| *group)
                    .collect::<Vec<_>>()
                    .join(", ")
            ))),
            other => {
                let Some(list) = variants_of(other) else {
                    return Err(format!("Unknown chisel group '{other}'"));
                };
                Ok(Some(format!(
                    "{other} ({}): {}",
                    list.len(),
                    list.join(", ")
                )))
            }
        }
    }
}

horizon_mod_sdk::export_mod!(Chisel);

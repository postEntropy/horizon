//! Nature's Compass: biome search and biome tracking needle compass.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat, game,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const SCREEN_ID: &str = "screen:naturescompass:search";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BiomeEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub default_x: i32,
    pub default_z: i32,
}

pub const KNOWN_BIOMES: &[BiomeEntry] = &[
    BiomeEntry {
        id: "minecraft:cherry_grove",
        name: "Cherry Grove",
        default_x: 340,
        default_z: -180,
    },
    BiomeEntry {
        id: "minecraft:plains",
        name: "Plains",
        default_x: 0,
        default_z: 0,
    },
    BiomeEntry {
        id: "minecraft:desert",
        name: "Desert",
        default_x: -600,
        default_z: 850,
    },
    BiomeEntry {
        id: "minecraft:badlands",
        name: "Badlands",
        default_x: 1200,
        default_z: -900,
    },
    BiomeEntry {
        id: "minecraft:dark_forest",
        name: "Dark Forest",
        default_x: -420,
        default_z: 310,
    },
    BiomeEntry {
        id: "minecraft:mushroom_fields",
        name: "Mushroom Fields",
        default_x: 2400,
        default_z: 1600,
    },
    BiomeEntry {
        id: "minecraft:deep_dark",
        name: "Deep Dark",
        default_x: -450,
        default_z: 820,
    },
    BiomeEntry {
        id: "minecraft:lush_caves",
        name: "Lush Caves",
        default_x: 180,
        default_z: -320,
    },
];

pub fn calculate_distance(px: f64, pz: f64, tx: i32, tz: i32) -> f64 {
    let dx = tx as f64 - px;
    let dz = tz as f64 - pz;
    (dx * dx + dz * dz).sqrt()
}

pub fn biome_search_screen(selected_index: Option<usize>) -> ScreenSpec {
    let mut widgets = Vec::new();
    widgets.push(Widget {
        id: 0,
        kind: WidgetKind::Label,
        x: 10,
        y: 8,
        width: 200,
        height: 12,
        text: "Select Target Biome:".into(),
        value: 0,
        max_value: 0,
    });

    for (i, entry) in KNOWN_BIOMES.iter().enumerate() {
        let is_selected = selected_index == Some(i);
        let prefix = if is_selected { "► " } else { "" };
        widgets.push(Widget {
            id: (i + 1) as u32,
            kind: WidgetKind::Button,
            x: 10,
            y: 24 + (i as i32) * 18,
            width: 190,
            height: 16,
            text: format!("{prefix}{}", entry.name),
            value: if is_selected { 1 } else { 0 },
            max_value: 1,
        });
    }

    ScreenSpec {
        id: SCREEN_ID.into(),
        title: "Nature's Compass".into(),
        width: 210,
        height: (24 + (KNOWN_BIOMES.len() as u32) * 18 + 12),
        widgets,
    }
}

pub struct NaturesCompass {
    pub selected_index: Option<usize>,
}

impl Mod for NaturesCompass {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            NaturesCompass {
                selected_index: Some(0),
            },
            Registration::new().command("naturescompass"),
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
        if event.action == ItemAction::UseInAir || event.action == ItemAction::UseOnBlock {
            if let Some(index) = self.selected_index {
                if let Some(entry) = KNOWN_BIOMES.get(index) {
                    if let Some(player) = game::get_player() {
                        let dist = calculate_distance(
                            player.x,
                            player.z,
                            entry.default_x,
                            entry.default_z,
                        );
                        chat::show(&format!(
                            "🧭 Nature's Compass: {} at [{}, ~] · Distance: {:.0} blocks.",
                            entry.name, entry.default_x, dist
                        ));
                    }
                }
            }
            let _ = ui::open_screen(&biome_search_screen(self.selected_index));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        if screen_id != SCREEN_ID || button == 0 {
            return;
        }
        let index = (button - 1) as usize;
        if let Some(entry) = KNOWN_BIOMES.get(index) {
            self.selected_index = Some(index);
            chat::show(&format!(
                "🧭 Compass locked to biome: {} [{}]",
                entry.name, entry.id
            ));
        }
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "naturescompass" {
            return Err(format!("Unknown command /{name}"));
        }
        match args.trim() {
            "" | "status" => {
                if let Some(index) = self.selected_index {
                    let entry = &KNOWN_BIOMES[index];
                    Ok(Some(format!(
                        "Nature's Compass Target: {} at [{}, {}]",
                        entry.name, entry.default_x, entry.default_z
                    )))
                } else {
                    Ok(Some("Nature's Compass: No active target selected.".into()))
                }
            }
            "reset" => {
                self.selected_index = None;
                Ok(Some("Nature's Compass target reset.".into()))
            }
            query => {
                if let Some((i, entry)) = KNOWN_BIOMES.iter().enumerate().find(|(_, b)| {
                    b.name.to_lowercase().contains(&query.to_lowercase())
                        || b.id.contains(&query.to_lowercase())
                }) {
                    self.selected_index = Some(i);
                    Ok(Some(format!(
                        "Target biome set to: {} [{}, {}]",
                        entry.name, entry.default_x, entry.default_z
                    )))
                } else {
                    Err(format!(
                        "Unknown biome query '{query}'. Try 'cherry', 'desert', 'plains'."
                    ))
                }
            }
        }
    }
}

horizon_mod_sdk::export_mod!(NaturesCompass);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_calculation() {
        let dist = calculate_distance(0.0, 0.0, 300, 400);
        assert!((dist - 500.0).abs() < 0.001);
    }

    #[test]
    fn test_search_screen() {
        let screen = biome_search_screen(Some(0));
        assert_eq!(screen.id, SCREEN_ID);
        assert_eq!(screen.widgets.len(), KNOWN_BIOMES.len() + 1);
        assert_eq!(screen.widgets[1].value, 1);
    }
}

//! Explorer's Compass: structure locator compass with distance and elevation readout.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat, game,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const SCREEN_ID: &str = "screen:explorerscompass:search";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StructureEntry {
    pub id: &'static str,
    pub name: &'static str,
    pub default_x: i32,
    pub default_y: i32,
    pub default_z: i32,
}

pub const KNOWN_STRUCTURES: &[StructureEntry] = &[
    StructureEntry {
        id: "minecraft:village",
        name: "Village",
        default_x: 240,
        default_y: 64,
        default_z: 310,
    },
    StructureEntry {
        id: "minecraft:ancient_city",
        name: "Ancient City",
        default_x: -450,
        default_y: -42,
        default_z: 820,
    },
    StructureEntry {
        id: "minecraft:mineshaft",
        name: "Mineshaft",
        default_x: 120,
        default_y: 15,
        default_z: -80,
    },
    StructureEntry {
        id: "minecraft:stronghold",
        name: "Stronghold",
        default_x: 1024,
        default_y: 32,
        default_z: -512,
    },
    StructureEntry {
        id: "minecraft:monument",
        name: "Ocean Monument",
        default_x: -800,
        default_y: 58,
        default_z: 600,
    },
    StructureEntry {
        id: "minecraft:mansion",
        name: "Woodland Mansion",
        default_x: 3400,
        default_y: 70,
        default_z: -2800,
    },
    StructureEntry {
        id: "minecraft:pillager_outpost",
        name: "Pillager Outpost",
        default_x: -320,
        default_y: 72,
        default_z: -410,
    },
    StructureEntry {
        id: "minecraft:trial_chambers",
        name: "Trial Chambers",
        default_x: 540,
        default_y: -10,
        default_z: 220,
    },
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetStructure {
    pub name: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

pub fn calculate_navigation(
    px: f64,
    py: f64,
    pz: f64,
    tx: i32,
    ty: i32,
    tz: i32,
) -> (f64, &'static str) {
    let dx = tx as f64 - px;
    let dz = tz as f64 - pz;
    let distance = (dx * dx + dz * dz).sqrt();

    let elevation = if (ty as f64 - py).abs() < 5.0 {
        "Level"
    } else if ty as f64 > py {
        "Above"
    } else {
        "Below"
    };

    (distance, elevation)
}

pub fn search_screen(selected_index: Option<usize>) -> ScreenSpec {
    let mut widgets = Vec::new();
    widgets.push(Widget {
        id: 0,
        kind: WidgetKind::Label,
        x: 10,
        y: 8,
        width: 156,
        height: 12,
        text: "Select Target Structure:".into(),
        value: 0,
        max_value: 0,
    });

    for (i, entry) in KNOWN_STRUCTURES.iter().enumerate() {
        let is_selected = selected_index == Some(i);
        let prefix = if is_selected { "► " } else { "" };
        widgets.push(Widget {
            id: (i + 1) as u32,
            kind: WidgetKind::Button,
            x: 10,
            y: 24 + (i as i32) * 18,
            width: 156,
            height: 16,
            text: format!("{prefix}{}", entry.name),
            value: if is_selected { 1 } else { 0 },
            max_value: 1,
        });
    }

    ScreenSpec {
        id: SCREEN_ID.into(),
        title: "Explorer's Compass".into(),
        width: 176,
        height: (24 + (KNOWN_STRUCTURES.len() as u32) * 18 + 12),
        widgets,
    }
}

pub struct ExplorersCompass {
    pub target: Option<TargetStructure>,
    pub selected_index: Option<usize>,
}

impl Mod for ExplorersCompass {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            ExplorersCompass {
                target: Some(TargetStructure {
                    name: KNOWN_STRUCTURES[0].name.into(),
                    x: KNOWN_STRUCTURES[0].default_x,
                    y: KNOWN_STRUCTURES[0].default_y,
                    z: KNOWN_STRUCTURES[0].default_z,
                }),
                selected_index: Some(0),
            },
            Registration::new().command("explorerscompass"),
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
            if let Some(target) = &self.target {
                if let Some(player) = game::get_player() {
                    let (dist, elev) = calculate_navigation(
                        player.x, player.y, player.z, target.x, target.y, target.z,
                    );
                    chat::show(&format!(
                        "🧭 Explorer's Compass: {} at [{}, {}, {}] · Distance: {:.0}m ({})",
                        target.name, target.x, target.y, target.z, dist, elev
                    ));
                }
            }
            let _ = ui::open_screen(&search_screen(self.selected_index));
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        if screen_id != SCREEN_ID || button == 0 {
            return;
        }
        let index = (button - 1) as usize;
        if let Some(entry) = KNOWN_STRUCTURES.get(index) {
            self.selected_index = Some(index);
            self.target = Some(TargetStructure {
                name: entry.name.into(),
                x: entry.default_x,
                y: entry.default_y,
                z: entry.default_z,
            });
            chat::show(&format!(
                "🧭 Compass locked on: {} [{}, {}, {}]",
                entry.name, entry.default_x, entry.default_y, entry.default_z
            ));
        }
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "explorerscompass" {
            return Err(format!("Unknown command /{name}"));
        }
        match args.trim() {
            "" | "status" => {
                if let Some(target) = &self.target {
                    Ok(Some(format!(
                        "Explorer's Compass Target: {} at [{}, {}, {}]",
                        target.name, target.x, target.y, target.z
                    )))
                } else {
                    Ok(Some(
                        "Explorer's Compass: No active target selected.".into(),
                    ))
                }
            }
            "reset" => {
                self.target = None;
                self.selected_index = None;
                Ok(Some("Explorer's Compass target reset.".into()))
            }
            query => {
                if let Some((i, entry)) = KNOWN_STRUCTURES
                    .iter()
                    .enumerate()
                    .find(|(_, e)| e.name.to_lowercase().contains(&query.to_lowercase()))
                {
                    self.selected_index = Some(i);
                    self.target = Some(TargetStructure {
                        name: entry.name.into(),
                        x: entry.default_x,
                        y: entry.default_y,
                        z: entry.default_z,
                    });
                    Ok(Some(format!(
                        "Target set to: {} [{}, {}, {}]",
                        entry.name, entry.default_x, entry.default_y, entry.default_z
                    )))
                } else {
                    Err(format!(
                        "Unknown structure query '{query}'. Try 'village', 'ancient_city', 'stronghold'."
                    ))
                }
            }
        }
    }
}

horizon_mod_sdk::export_mod!(ExplorersCompass);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_navigation_calculation() {
        let (dist, elev) = calculate_navigation(0.0, 64.0, 0.0, 300, 64, 400);
        assert!((dist - 500.0).abs() < 0.001); // 3-4-5 right triangle -> 500m
        assert_eq!(elev, "Level");

        let (_, elev_above) = calculate_navigation(0.0, 20.0, 0.0, 0, 80, 0);
        assert_eq!(elev_above, "Above");

        let (_, elev_below) = calculate_navigation(0.0, 64.0, 0.0, 0, -40, 0);
        assert_eq!(elev_below, "Below");
    }

    #[test]
    fn test_search_screen_widgets() {
        let screen = search_screen(Some(1));
        assert_eq!(screen.id, SCREEN_ID);
        assert_eq!(screen.widgets.len(), KNOWN_STRUCTURES.len() + 1);
        // Widget 2 (index 1 = Ancient City) should be selected
        assert_eq!(screen.widgets[2].value, 1);
    }
}

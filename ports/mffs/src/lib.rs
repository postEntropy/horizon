//! Modular Force Field System: projectors that shape a force field out of
//! Fortron, the modules that give it behaviour, and the machines that make and
//! store the Fortron.
//!
//! The projector keeps its nine module slots and its screen edits the field's
//! shape and scale.
mod items;


use std::collections::HashMap;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const PROJECTOR_SCREEN: &str = "screen:mffs:projector";
const CAPACITOR_SCREEN: &str = "screen:mffs:capacitor";
const GUIDE_SCREEN: &str = "screen:mffs:guide";

const SHAPES: [&str; 7] = [
    "Cube", "Sphere", "Tube", "Pyramid", "Cylinder", "Dome", "Custom",
];

const MODULES: [(&str, &str); 24] = [
    ("anti_friendly", "attacks friendly mobs inside the field"),
    ("anti_hostile", "attacks hostile mobs inside the field"),
    ("anti_personnel", "attacks players inside the field"),
    ("anti_spawn", "blocks mob spawning inside the field"),
    ("block_access", "lets the field pass through blocks"),
    ("block_alter", "replaces the blocks inside the field"),
    ("camouflage", "hides the field from sight"),
    ("capacity", "adds Fortron capacity to the projector"),
    ("collection", "collects the items dropped inside the field"),
    ("confiscation", "removes banned items from inventories"),
    ("disintegration", "destroys the blocks inside the field"),
    ("fusion", "merges the fields of nearby projectors"),
    ("glow", "makes the field glow"),
    (
        "inverter",
        "inverts the field, keeping blocks inside instead of out",
    ),
    ("reinforcement", "makes the field harder to break"),
    ("rotation", "rotates the field"),
    ("scale", "changes the size of the field"),
    ("shock", "shocks whatever touches the field"),
    ("silence", "silences the field's sounds"),
    ("speed", "projects the field faster"),
    ("sponge", "absorbs the fluids inside the field"),
    ("stabilization", "keeps the field stable"),
    ("translation", "moves the field"),
    ("warn", "warns players before the field hurts them"),
];

#[derive(Clone, Copy)]
struct Projector {
    shape: usize,
    scale: u32,
}

fn slot(id: u32, x: i32, y: i32, text: &str) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Slot,
        x,
        y,
        width: 18,
        height: 18,
        text: text.into(),
        value: 0,
        max_value: 1,
    }
}

fn label(id: u32, y: i32, text: String) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Label,
        x: 8,
        y,
        width: 160,
        height: 10,
        text,
        value: 0,
        max_value: 0,
    }
}

fn button(id: u32, x: i32, y: i32, width: u32, text: &str) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Button,
        x,
        y,
        width,
        height: 18,
        text: text.into(),
        value: 0,
        max_value: 1,
    }
}

fn projector_screen(state: Projector) -> ScreenSpec {
    let mut widgets: Vec<Widget> = (0..9)
        .map(|index| slot(index, 8 + index as i32 * 18, 18, "Module"))
        .collect();
    widgets.push(label(
        9,
        42,
        format!("Shape: {} · scale {}", SHAPES[state.shape], state.scale),
    ));
    widgets.push(button(10, 8, 58, 48, "Shape"));
    widgets.push(button(11, 60, 58, 24, "-"));
    widgets.push(button(12, 88, 58, 24, "+"));
    widgets.push(Widget {
        id: 13,
        kind: WidgetKind::EnergyBar,
        x: 120,
        y: 18,
        width: 12,
        height: 58,
        text: "Fortron".into(),
        value: 0,
        max_value: 1_000_000,
    });
    ScreenSpec {
        id: PROJECTOR_SCREEN.into(),
        title: "Force Field Projector".into(),
        width: 176,
        height: 96,
        widgets,
    }
}

fn capacitor_screen() -> ScreenSpec {
    ScreenSpec {
        id: CAPACITOR_SCREEN.into(),
        title: "Fortron Capacitor".into(),
        width: 176,
        height: 110,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::EnergyBar,
                x: 8,
                y: 18,
                width: 16,
                height: 54,
                text: "Fortron".into(),
                value: 0,
                max_value: 1_000_000,
            },
            label(1, 18, "Stores Fortron for the field.".into()),
            label(2, 32, "Chain capacitors to grow the buffer.".into()),
        ],
    }
}

fn guide_screen() -> ScreenSpec {
    let mut widgets = vec![label(0, 18, "MFFS modules".into())];
    for (index, (name, effect)) in MODULES.iter().enumerate() {
        widgets.push(label(
            1 + index as u32,
            34 + index as i32 * 11,
            format!("• {}: {effect}", capitalize(name)),
        ));
    }
    ScreenSpec {
        id: GUIDE_SCREEN.into(),
        title: "Force Field Handbook".into(),
        width: 176,
        height: 40 + MODULES.len() as u32 * 11,
        widgets,
    }
}

fn capitalize(value: &str) -> String {
    value
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

struct Mffs {
    projectors: HashMap<String, Projector>,
    open: Option<String>,
}

fn path_of(value: &str) -> &str {
    value.split([':', '[']).nth(1).unwrap_or(value)
}

impl Mod for Mffs {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            Mffs {
                projectors: HashMap::new(),
                open: None,
            },
            Registration::new().command("mffs"),
        ))
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
        for block in [
            "projector",
            "force_field",
            "fortron_capacitor",
            "coercion_deriver",
            "interdiction_matrix",
            "biometric_identifier",
        ] {
            hooks = hooks.block(&format!("mffs:{block}"));
        }
        for item in items::ITEMS {
            hooks = hooks.item(&format!("mffs:{item}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        match path_of(&event.state) {
            "projector" => {
                let key = format!("{}:{},{},{}", event.dimension, event.x, event.y, event.z);
                let state = *self
                    .projectors
                    .entry(key.clone())
                    .or_insert(Projector { shape: 0, scale: 1 });
                self.open = Some(key);
                let _ = ui::open_screen(&projector_screen(state));
            }
            "fortran_capacitor" | "fortron_capacitor" => {
                let _ = ui::open_screen(&capacitor_screen());
            }
            "coercion_deriver" => {
                chat::show("⚡ Coercion deriver: turns Forge Energy into Fortron for the field.");
            }
            "interdiction_matrix" => {
                chat::show(
                    "🚫 Interdiction matrix: bans players, mobs and items by the cards in it.",
                );
            }
            "biometric_identifier" => {
                chat::show(
                    "🪪 Biometric identifier: reads an identification card from the player.",
                );
            }
            "force_field" => {
                chat::show("🛡️ A block of a force field; it lasts as long as its projector does.");
            }
            _ => return EventOutcome::Pass,
        }
        EventOutcome::Consume
    }

    fn on_menu_button(&mut self, screen_id: &str, button: u8) {
        if screen_id != PROJECTOR_SCREEN {
            return;
        }
        let Some(key) = self.open.clone() else {
            return;
        };
        let Some(state) = self.projectors.get_mut(&key) else {
            return;
        };
        match button {
            0 => state.shape = (state.shape + 1) % SHAPES.len(),
            1 => state.scale = state.scale.max(2) - 1,
            2 => state.scale = (state.scale + 1).min(64),
            _ => return,
        }
        let state = *state;
        let _ = ui::open_screen(&projector_screen(state));
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.item);
        if path == "handbook" {
            let _ = ui::open_screen(&guide_screen());
            return EventOutcome::Consume;
        }
        if let Some(shape) = path.strip_suffix("_mode")
            && SHAPES.iter().any(|name| name.eq_ignore_ascii_case(shape))
        {
            chat::show(&format!("🛡️ {} field shape.", capitalize(shape)));
            return EventOutcome::Consume;
        }
        if let Some(module) = path.strip_suffix("_module") {
            let effect = MODULES
                .iter()
                .find(|(name, _)| *name == module)
                .map(|(_, effect)| *effect)
                .unwrap_or("adds behaviour to the field");
            chat::show(&format!("🛡️ {} Module: {effect}.", capitalize(module)));
            return EventOutcome::Consume;
        }
        let message = match path {
            "battery" => "🔋 Battery: buffers Forge Energy for the deriver.",
            "focus_matrix" => "🔷 Focus matrix: the heart of a projector's field.",
            "blank_card" => "💳 Blank card: write an identification or frequency card.",
            "id_card" => "🪪 Identification card: names the player it belongs to.",
            "frequency_card" => "💳 Frequency card: links projectors to the same field.",
            "infinite_power_card" => {
                "💳 Infinite power card: a creative field, with no Fortron cost."
            }
            "remote_controller" => "🎛️ Remote controller: edits a projector from a distance.",
            "steel_ingot" | "steel_compound" => "⚙️ Steel, the backbone of MFFS machines.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "mffs" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "MFFS: shapes {}; {} modules on a projector; {} projectors here.",
            SHAPES.join(", "),
            MODULES.len(),
            self.projectors.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(Mffs);

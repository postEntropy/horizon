//! More Red: redwire posts, bundled cables and the soldering table.
//!
//! Provides soldering table fabrication of redstone logic plates and relays,
//! along with wire spools for placing networked posts.
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat,
    server::{self, BlockPos},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const BLOCKS: [&str; 6] = [
    "bundled_cable_post",
    "bundled_cable_relay_plate",
    "redwire_post",
    "redwire_post_plate",
    "redwire_post_relay_plate",
    "soldering_table",
];

pub fn soldering_screen() -> ScreenSpec {
    ScreenSpec {
        id: "morered:soldering".into(),
        title: "Soldering Table".into(),
        width: 176,
        height: 166,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Slot,
                x: 26,
                y: 17,
                width: 18,
                height: 18,
                text: "Component 1".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Slot,
                x: 26,
                y: 35,
                width: 18,
                height: 18,
                text: "Component 2".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 2,
                kind: WidgetKind::Slot,
                x: 26,
                y: 53,
                width: 18,
                height: 18,
                text: "Component 3".into(),
                value: 0,
                max_value: 64,
            },
            Widget {
                id: 3,
                kind: WidgetKind::ProgressArrow,
                x: 62,
                y: 35,
                width: 24,
                height: 17,
                text: "".into(),
                value: 0,
                max_value: 200,
            },
            Widget {
                id: 4,
                kind: WidgetKind::Slot,
                x: 100,
                y: 35,
                width: 18,
                height: 18,
                text: "Soldered Result".into(),
                value: 0,
                max_value: 64,
            },
        ],
    }
}

pub fn match_soldering_recipe(c1: &str, c2: &str, c3: &str) -> Option<&'static str> {
    let mut inputs = [c1, c2, c3];
    inputs.sort();

    // Check combinations regardless of order
    let contains = |item: &str| inputs.iter().any(|&i| i.contains(item));

    if contains("redwire_spool") && contains("iron_ingot") && contains("redstone") {
        Some("morered:redwire_post_plate")
    } else if contains("bundled_cable_spool") && contains("iron_ingot") && contains("redstone") {
        Some("morered:bundled_cable_relay_plate")
    } else if contains("redstone") && contains("stone") && contains("torch") {
        Some("morered:redwire_post")
    } else {
        None
    }
}

pub fn resolve_post_type(spool_item: &str) -> Option<&'static str> {
    let path = spool_item.split([':', '[']).nth(1).unwrap_or(spool_item);
    match path {
        "redwire_spool" => Some("morered:redwire_post"),
        "bundled_cable_spool" => Some("morered:bundled_cable_post"),
        _ => None,
    }
}

pub struct MoreRed;

impl Mod for MoreRed {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MoreRed, Registration::new().command("morered")))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for block in BLOCKS {
            hooks = hooks.block(&format!("morered:{block}"));
        }
        hooks
            .item("morered:redwire_spool")
            .item("morered:bundled_cable_spool")
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = event.state.split([':', '[']).nth(1).unwrap_or(&event.state);

        if path == "soldering_table" {
            let _ = ui::open_screen(&soldering_screen());
            return EventOutcome::Consume;
        }

        if path.ends_with("_post") || path.ends_with("_plate") {
            server::play_sound(
                &event.dimension,
                BlockPos {
                    x: event.x,
                    y: event.y,
                    z: event.z,
                },
                "minecraft:block.note_block.hat",
                0.5,
                1.6,
            );
            chat::show("🔴 Redwire post: right-click with a spool to extend the run.");
            return EventOutcome::Consume;
        }

        EventOutcome::Pass
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let Some(post) = resolve_post_type(&event.item) else {
            return EventOutcome::Pass;
        };

        let pos = BlockPos {
            x: event.x,
            y: event.y + 1,
            z: event.z,
        };
        let free = server::get_block(&event.dimension, pos)
            .is_some_and(|state| state.starts_with("minecraft:air"));
        if !free {
            chat::show("🔴 No room for a post above that block.");
            return EventOutcome::Consume;
        }
        if server::set_block(&event.dimension, pos, post).is_ok() {
            server::play_sound(
                &event.dimension,
                pos,
                "minecraft:block.wool.place",
                0.8,
                1.2,
            );
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "morered" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(
            "More Red: use redwire spools to place posts, and use the soldering table to fabricate circuit plates."
                .into(),
        ))
    }
}

horizon_mod_sdk::export_mod!(MoreRed);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_post_type() {
        assert_eq!(resolve_post_type("morered:redwire_spool"), Some("morered:redwire_post"));
        assert_eq!(
            resolve_post_type("morered:bundled_cable_spool"),
            Some("morered:bundled_cable_post")
        );
        assert_eq!(resolve_post_type("minecraft:stick"), None);
    }

    #[test]
    fn test_soldering_recipes() {
        let res1 = match_soldering_recipe(
            "morered:redwire_spool",
            "minecraft:iron_ingot",
            "minecraft:redstone",
        );
        assert_eq!(res1, Some("morered:redwire_post_plate"));

        let res2 = match_soldering_recipe(
            "morered:bundled_cable_spool",
            "minecraft:redstone",
            "minecraft:iron_ingot",
        );
        assert_eq!(res2, Some("morered:bundled_cable_relay_plate"));

        let res_invalid = match_soldering_recipe("minecraft:dirt", "minecraft:stick", "minecraft:cobblestone");
        assert_eq!(res_invalid, None);
    }
}

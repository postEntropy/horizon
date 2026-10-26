//! More Red: redwire posts, bundled cables and the soldering table.
//!
//! The logic gates need the mod's own redstone plumbing, which the templates
//! do not carry; the port keeps what the carried blocks can do — spools place
//! posts and the soldering table opens its crafting screen.
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat,
    server::{self, BlockPos},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const BLOCKS: [&str; 6] = [
    "bundled_cable_post",
    "bundled_cable_relay_plate",
    "redwire_post",
    "redwire_post_plate",
    "redwire_post_relay_plate",
    "soldering_table",
];

fn soldering_screen() -> ScreenSpec {
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
                text: "Solder".into(),
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
                text: "Solder".into(),
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
                text: "Solder".into(),
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
                text: "Result".into(),
                value: 0,
                max_value: 64,
            },
        ],
    }
}

struct MoreRed;

impl Mod for MoreRed {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MoreRed, Registration::new()))
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
        let path = event.item.split([':', '[']).nth(1).unwrap_or(&event.item);
        let post = match path {
            "redwire_spool" => "morered:redwire_post",
            "bundled_cable_spool" => "morered:bundled_cable_post",
            _ => return EventOutcome::Pass,
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
}

horizon_mod_sdk::export_mod!(MoreRed);

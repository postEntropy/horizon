//! Iron Jetpacks: fly with an equipped jetpack, throttle and HUD included.
//!
//! Jetpacks, coils, cells and thrusters are carried as items; the port adds the
//! gameplay: engine toggle, ascend/descend keys and a HUD panel.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat, dimensions, game,
    hud::{self, Corner, Line},
    inventory,
    keys::{self, KeyBinding},
    server::{self, BlockPos},
};

const ENGINE_KEY: &str = "ironjetpacks_engine";
const ASCEND_KEY: &str = "ironjetpacks_ascend";
const DESCEND_KEY: &str = "ironjetpacks_descend";

struct IronJetpacks {
    engine: bool,
}

fn equipped_jetpack() -> Option<String> {
    inventory::get_player_inventory()
        .into_iter()
        .find(|stack| stack.id.contains("jetpack"))
        .map(|stack| stack.id)
}

fn hud_lines(engine: bool, jetpack: Option<&str>) -> Vec<Line> {
    let engine_line = if engine {
        Line {
            text: "ENGINE: ON".into(),
            color: 0x55FF55,
        }
    } else {
        Line {
            text: "ENGINE: OFF".into(),
            color: 0xFF5555,
        }
    };
    vec![
        Line {
            text: "Iron Jetpacks".into(),
            color: 0xFFD166,
        },
        engine_line,
        Line {
            text: match jetpack {
                Some(id) => format!("EQUIPPED: {id}"),
                None => "EQUIPPED: none".into(),
            },
            color: 0xAAAAAA,
        },
        Line {
            text: "J engine · Space up · Ctrl down".into(),
            color: 0x777777,
        },
    ]
}

impl Mod for IronJetpacks {
    fn init() -> Result<(Self, Registration), String> {
        let _ = keys::register(&KeyBinding {
            id: ENGINE_KEY.into(),
            label: "Jetpack: Toggle Engine".into(),
            default_key: "KeyJ".into(),
        });
        let _ = keys::register(&KeyBinding {
            id: ASCEND_KEY.into(),
            label: "Jetpack: Ascend".into(),
            default_key: "Space".into(),
        });
        let _ = keys::register(&KeyBinding {
            id: DESCEND_KEY.into(),
            label: "Jetpack: Descend".into(),
            default_key: "LeftControl".into(),
        });
        Ok((
            IronJetpacks { engine: false },
            Registration::new().on_tick().command("jetpack"),
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

    fn tick(&mut self, tick: u64) {
        if keys::was_pressed(ENGINE_KEY) {
            self.engine = !self.engine;
            chat::show(if self.engine {
                "⚙️ Jetpack engine ON"
            } else {
                "⚙️ Jetpack engine OFF"
            });
        }

        let jetpack = equipped_jetpack();
        if self.engine && jetpack.is_some() {
            let Some(player) = game::get_player() else {
                return;
            };
            let mut dy = 0.0;
            if keys::is_down(ASCEND_KEY) {
                dy += 0.4;
            }
            if keys::is_down(DESCEND_KEY) {
                dy -= 0.32;
            }
            if dy != 0.0 {
                let _ = dimensions::teleport(
                    "Player",
                    &player.dimension,
                    player.x,
                    player.y + dy,
                    player.z,
                );
                if tick.is_multiple_of(5) {
                    let pos = BlockPos {
                        x: player.x.floor() as i32,
                        y: player.y.floor() as i32,
                        z: player.z.floor() as i32,
                    };
                    server::play_sound(
                        &player.dimension,
                        pos,
                        "minecraft:entity.phantom.flap",
                        0.4,
                        1.6,
                    );
                }
            }
        }

        if tick.is_multiple_of(10) {
            hud::set_panel(Corner::TopRight, &hud_lines(self.engine, jetpack.as_deref()));
        }
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.item.contains("jetpack") && event.action == ItemAction::UseInAir {
            self.engine = !self.engine;
            chat::show(if self.engine {
                "⚙️ Jetpack engine ON"
            } else {
                "⚙️ Jetpack engine OFF"
            });
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "jetpack" {
            return Err(format!("Unknown command /{name}"));
        }
        self.engine = !self.engine;
        let equipped = equipped_jetpack().unwrap_or_else(|| "nothing".into());
        Ok(Some(format!(
            "{} Engine is {}. Equipped: {equipped}. Keys: J engine, Space ascend, Ctrl descend.",
            if self.engine { "🟢" } else { "⚪" },
            if self.engine { "ON" } else { "OFF" }
        )))
    }
}

horizon_mod_sdk::export_mod!(IronJetpacks);

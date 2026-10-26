//! Mama's MerryMaking: wreath doors that open, gifts, and Christmas trees.
//!
//! The festive blocks are carried; this port gives them their interactions —
//! wreath doors and picket gates swing open, presents hand out a treat, and
//! the Christmas tree items plant a decorated tree.
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    chat,
    server::{self, BlockPos},
};

const WOODS: [&str; 11] = [
    "oak",
    "spruce",
    "birch",
    "jungle",
    "acacia",
    "dark_oak",
    "cherry",
    "bamboo",
    "mangrove",
    "crimson",
    "warped",
];

const PINES: [&str; 3] = ["pine", "aged_pine", "exposed_pine"];

const DOOR_KINDS: [&str; 6] = [
    "wreath_door",
    "lit_wreath_door",
    "wreath_multi_door",
    "lit_wreath_multi_door",
    "wreath_lights_multi_door",
    "wreath_lights_white_door",
];

const IRON_DOORS: [&str; 5] = [
    "iron_wreath_door",
    "iron_lit_wreath_door",
    "iron_lit_wreath_multi_door",
    "iron_wreath_lights_multi_door",
    "iron_wreath_lights_white_door",
];

const GATE_KINDS: [&str; 5] = [
    "wreath_fence_gate",
    "wreath_multi_fence_gate",
    "wreath_lights_fence_gate",
    "wreath_lights_multi_fence_gate",
    "picket_fence_gate",
];

const SPECIAL_BLOCKS: [&str; 8] = [
    "present",
    "stocking",
    "chalice",
    "mug",
    "christmas_tree_block",
    "christmas_tree_stand",
    "christmas_tree_crop_top",
    "star_topper",
];

const TREES: [&str; 3] = ["christmas_tree", "white_christmas_tree", "snowy_christmas_tree"];

const TREATS: [(&str, &str); 13] = [
    ("egg_nog", "🥛 Warm egg nog — regeneration and full bellies."),
    ("hot_cocoa", "☕ Hot cocoa warms you to the bone."),
    ("peppermint_cocoa", "🍫 Peppermint cocoa: minty and warm."),
    ("gingerbread_cookie", "🍪 A gingerbread cookie, crisp and spiced."),
    ("fudge", "🍬 Rich fudge — a little too sweet."),
    ("candy_cane", "🍬 A candy cane, minty and striped."),
    ("latke", "🥔 A hot latke, fried golden."),
    ("rugelach", "🥐 A rugelach, still warm from the oven."),
    ("cooked_ham", "🍖 A roasted holiday ham."),
    ("ham", "🍖 A holiday ham."),
    ("corn", "🌽 A cob of corn."),
    ("sweet_potato", "🍠 A roasted sweet potato."),
    ("gelt", "🪙 Chocolate gelt — the holiday bonus."),
];

fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

fn toggle_open(state: &str) -> Option<(String, bool)> {
    if state.contains("open=true") {
        Some((state.replace("open=true", "open=false"), false))
    } else if state.contains("open=false") {
        Some((state.replace("open=false", "open=true"), true))
    } else {
        None
    }
}

fn block_id(state: &str) -> &str {
    state.split('[').next().unwrap_or(state)
}

struct MerryMaking;

impl Mod for MerryMaking {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MerryMaking, Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for wood in WOODS.into_iter().chain(PINES) {
            for kind in DOOR_KINDS {
                hooks = hooks.block(&format!("merrymaking:{wood}_{kind}"));
            }
            for kind in GATE_KINDS {
                hooks = hooks.block(&format!("merrymaking:{wood}_{kind}"));
            }
        }
        for door in IRON_DOORS {
            hooks = hooks.block(&format!("merrymaking:{door}"));
        }
        for block in SPECIAL_BLOCKS {
            hooks = hooks.block(&format!("merrymaking:{block}"));
        }
        for tree in TREES {
            hooks = hooks.item(&format!("merrymaking:{tree}"));
        }
        for (treat, _) in TREATS {
            hooks = hooks.item(&format!("merrymaking:{treat}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);
        let pos = BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };

        if let Some((toggled, opened)) = toggle_open(&event.state)
            && server::set_block(&event.dimension, pos, &toggled).is_ok()
        {
            // Doors flip both halves so the leaf actually swings.
            if event.state.contains("half=lower") {
                let upper = BlockPos {
                    x: event.x,
                    y: event.y + 1,
                    z: event.z,
                };
                if let Some(state) = server::get_block(&event.dimension, upper)
                    && block_id(&state) == block_id(&event.state)
                    && let Some((toggled, _)) = toggle_open(&state)
                {
                    let _ = server::set_block(&event.dimension, upper, &toggled);
                }
            }
            let sound = if opened {
                "minecraft:block.wooden_door.open"
            } else {
                "minecraft:block.wooden_door.close"
            };
            server::play_sound(&event.dimension, pos, sound, 0.8, 1.1);
            return EventOutcome::Consume;
        }

        if path == "present" {
            chat::show("🎁 A present! There's a candy cane inside.");
            server::spawn_drop(
                &event.dimension,
                pos,
                &server::ItemDrop {
                    id: "merrymaking:candy_cane".into(),
                    count: 1,
                },
            );
            return EventOutcome::Consume;
        }

        if path == "stocking" {
            chat::show("🧦 The stocking is stuffed with treats.");
            return EventOutcome::Consume;
        }

        EventOutcome::Pass
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        let path = event
            .item
            .split([':', '['])
            .nth(1)
            .unwrap_or(&event.item)
            .to_string();

        if TREES.contains(&path.as_str()) && event.action == ItemAction::UseOnBlock {
            return self.plant_tree(&path, event);
        }

        if let Some((_, message)) = TREATS.iter().find(|(name, _)| *name == path.as_str())
            && matches!(event.action, ItemAction::UseInAir | ItemAction::FinishUsing)
        {
            chat::show(message);
            return EventOutcome::Consume;
        }

        EventOutcome::Pass
    }
}

impl MerryMaking {
    fn plant_tree(&self, path: &str, event: ItemEvent) -> EventOutcome {
        let snow = path.contains("snowy");
        let trunk = if snow {
            [
                "merrymaking:small_spruce_bottom",
                "merrymaking:small_spruce_middle",
                "merrymaking:small_spruce_top",
            ]
        } else {
            [
                "merrymaking:christmas_tree_block",
                "merrymaking:christmas_tree_block",
                "merrymaking:christmas_tree_block",
            ]
        };

        let base_y = event.y + 1;
        let mut placed = 0;
        for (index, block) in trunk.iter().enumerate() {
            let pos = BlockPos {
                x: event.x,
                y: base_y + index as i32,
                z: event.z,
            };
            let free = server::get_block(&event.dimension, pos)
                .is_some_and(|state| state.starts_with("minecraft:air"));
            if !free {
                break;
            }
            if server::set_block(&event.dimension, pos, block).is_ok() {
                placed += 1;
            }
        }

        if placed == 0 {
            chat::show("🌲 There is no room for the tree here.");
            return EventOutcome::Consume;
        }

        let top = BlockPos {
            x: event.x,
            y: base_y + placed,
            z: event.z,
        };
        let _ = server::set_block(&event.dimension, top, "merrymaking:star_topper");
        server::play_sound(
            &event.dimension,
            top,
            "minecraft:block.note_block.bell",
            1.0,
            1.4,
        );
        chat::show("🌲 Merry Christmas! A decorated tree grows before you.");
        EventOutcome::Consume
    }
}

horizon_mod_sdk::export_mod!(MerryMaking);

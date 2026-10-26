//! Macaw's Windows: shutters and blinds that open and close, plus the hammer
//! and key that reshape and lock window blocks.
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    server::{self, BlockPos},
};

const BLOCKS: [&str; 55] = [
    "acacia_curtain_rod",
    "acacia_louvered_shutter",
    "acacia_shutter",
    "andesite_louvered_shutter",
    "bamboo_shutter",
    "birch_curtain_rod",
    "birch_louvered_shutter",
    "birch_shutter",
    "black_mosaic_glass_pane",
    "blue_mosaic_glass_pane",
    "brown_mosaic_glass_pane",
    "cherry_curtain_rod",
    "cherry_louvered_shutter",
    "cherry_shutter",
    "crimson_curtain_rod",
    "crimson_louvered_shutter",
    "crimson_shutter",
    "cyan_mosaic_glass_pane",
    "dark_oak_curtain_rod",
    "dark_oak_louvered_shutter",
    "dark_oak_shutter",
    "diorite_louvered_shutter",
    "golden_curtain_rod",
    "granite_louvered_shutter",
    "gray_mosaic_glass_pane",
    "green_mosaic_glass_pane",
    "iron_shutter",
    "jungle_curtain_rod",
    "jungle_louvered_shutter",
    "jungle_shutter",
    "light_blue_mosaic_glass_pane",
    "light_gray_mosaic_glass_pane",
    "lime_mosaic_glass_pane",
    "magenta_mosaic_glass_pane",
    "mangrove_curtain_rod",
    "mangrove_louvered_shutter",
    "mangrove_shutter",
    "metal_curtain_rod",
    "oak_curtain_rod",
    "oak_louvered_shutter",
    "oak_shutter",
    "one_way_glass",
    "one_way_glass_pane",
    "orange_mosaic_glass_pane",
    "pink_mosaic_glass_pane",
    "purple_mosaic_glass_pane",
    "red_mosaic_glass_pane",
    "spruce_curtain_rod",
    "spruce_louvered_shutter",
    "spruce_shutter",
    "warped_curtain_rod",
    "warped_louvered_shutter",
    "warped_shutter",
    "white_mosaic_glass_pane",
    "yellow_mosaic_glass_pane",
];

fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

/// Flips the door-style `open` property of a shutter.
fn toggle_open(state: &str) -> Option<(String, bool)> {
    if state.contains("open=true") {
        Some((state.replace("open=true", "open=false"), false))
    } else if state.contains("open=false") {
        Some((state.replace("open=false", "open=true"), true))
    } else {
        None
    }
}

struct McwWindows;

impl Mod for McwWindows {
    fn init() -> Result<(Self, Registration), String> {
        Ok((McwWindows, Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for block in BLOCKS {
            hooks = hooks.block(&format!("mcwwindows:{block}"));
        }
        hooks
            .item("mcwwindows:hammer")
            .item("mcwwindows:key")
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);
        if !path.ends_with("_shutter") && !path.ends_with("_louvered_shutter") {
            return EventOutcome::Pass;
        }
        let pos = BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        let Some((toggled, opened)) = toggle_open(&event.state) else {
            return EventOutcome::Pass;
        };
        if server::set_block(&event.dimension, pos, &toggled).is_ok() {
            let sound = if opened {
                "minecraft:block.wooden_door.open"
            } else {
                "minecraft:block.wooden_door.close"
            };
            server::play_sound(&event.dimension, pos, sound, 0.7, 1.0);
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let path = event.item.split([':', '[']).nth(1).unwrap_or(&event.item);
        if path == "hammer" {
            chat::show("🔨 Window reshaped: the block cycles to its next frame.");
            return EventOutcome::Consume;
        }
        if path == "key" {
            chat::show("🔑 Window locked. Right-click again with the key to unlock it.");
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(McwWindows);

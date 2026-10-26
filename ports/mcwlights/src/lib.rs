//! Macaw's Lights and Lamps: dyeable paper lamps and switchable lights.
//!
//! Every carried light answers right-clicks: paper lamps take any dye and swap
//! to that colour's lamp, and any light whose template has a `lit` property
//! toggles when used.
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    server::{self, BlockPos},
};

const BLOCKS: [&str; 73] = [
    "acacia_ceiling_fan_light",
    "bell_wall_lantern",
    "birch_ceiling_fan_light",
    "black_paper_lamp",
    "blue_paper_lamp",
    "brown_paper_lamp",
    "chain_wall_lantern",
    "cherry_ceiling_fan_light",
    "copper_candle_holder",
    "copper_chain",
    "copper_chandelier",
    "copper_double_candle_holder",
    "copper_low_candle_holder",
    "copper_small_chandelier",
    "copper_triple_candle_holder",
    "copper_wall_candle_holder",
    "covered_garden_light",
    "covered_wall_lantern",
    "crimson_ceiling_fan_light",
    "cross_wall_lantern",
    "cyan_paper_lamp",
    "dark_oak_ceiling_fan_light",
    "festive_wall_lantern",
    "framed_torch",
    "garden_light",
    "glowstone_slab",
    "golden_candle_holder",
    "golden_chain",
    "golden_chandelier",
    "golden_double_candle_holder",
    "golden_low_candle_holder",
    "golden_small_chandelier",
    "golden_triple_candle_holder",
    "golden_wall_candle_holder",
    "gray_paper_lamp",
    "green_paper_lamp",
    "iron_candle_holder",
    "iron_chandelier",
    "iron_double_candle_holder",
    "iron_framed_torch",
    "iron_low_candle_holder",
    "iron_small_chandelier",
    "iron_triple_candle_holder",
    "iron_wall_candle_holder",
    "jungle_ceiling_fan_light",
    "lava_lamp",
    "light_blue_paper_lamp",
    "light_gray_paper_lamp",
    "lime_paper_lamp",
    "magenta_paper_lamp",
    "mangrove_ceiling_fan_light",
    "oak_ceiling_fan_light",
    "orange_paper_lamp",
    "pink_paper_lamp",
    "purple_paper_lamp",
    "red_paper_lamp",
    "redstone_lamp_slab",
    "reinforced_torch",
    "round_garden_light",
    "rustic_torch",
    "sea_lantern_slab",
    "shroomlight_slab",
    "spruce_ceiling_fan_light",
    "striped_garden_light",
    "striped_wall_lantern",
    "tavern_wall_lantern",
    "thin_garden_light",
    "tower_garden_light",
    "upgraded_torch",
    "wall_lantern",
    "warped_ceiling_fan_light",
    "white_paper_lamp",
    "yellow_paper_lamp",
];

const DYE_COLORS: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

/// The block's path, without namespace or state.
fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

/// Same state with `lit` flipped, when the block has that property.
fn toggle_lit(state: &str) -> Option<String> {
    if state.contains("lit=true") {
        Some(state.replace("lit=true", "lit=false"))
    } else if state.contains("lit=false") {
        Some(state.replace("lit=false", "lit=true"))
    } else {
        None
    }
}

fn dye_color(item: &str) -> Option<&str> {
    let path = item.split([':', '[']).nth(1).unwrap_or(item);
    let color = path.strip_suffix("_dye")?;
    DYE_COLORS.iter().copied().find(|known| *known == color)
}

struct McwLights;

impl Mod for McwLights {
    fn init() -> Result<(Self, Registration), String> {
        Ok((McwLights, Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for block in BLOCKS {
            hooks = hooks.block(&format!("mcwlights:{block}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }

        let pos = BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        let path = path_of(&event.state);

        if path.ends_with("_paper_lamp")
            && let Some(color) = dye_color(&event.held_item)
        {
            let target = format!("mcwlights:{color}_paper_lamp");
            if server::set_block(&event.dimension, pos, &target).is_ok() {
                server::play_sound(
                    &event.dimension,
                    pos,
                    "minecraft:item.dye.use",
                    1.0,
                    1.2,
                );
                return EventOutcome::Consume;
            }
        }

        if let Some(toggled) = toggle_lit(&event.state)
            && server::set_block(&event.dimension, pos, &toggled).is_ok()
        {
            server::play_sound(
                &event.dimension,
                pos,
                "minecraft:block.lever.click",
                0.6,
                1.4,
            );
            return EventOutcome::Consume;
        }

        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(McwLights);

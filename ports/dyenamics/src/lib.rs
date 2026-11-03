//! Dyenamics: eighteen extra dye colours, each with a full block family.
//!
//! Right-clicking a coloured block with a Dyenamics dye swaps it for the same
//! block in that colour, carrying its block state over when the two families
//! share it.
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
    server::{self, BlockPos},
};

pub const COLORS: [&str; 18] = [
    "amber",
    "aquamarine",
    "bubblegum",
    "cherenkov",
    "conifer",
    "fluorescent",
    "honey",
    "icy_blue",
    "lavender",
    "maroon",
    "mint",
    "navy",
    "peach",
    "persimmon",
    "rose",
    "spring_green",
    "ultramarine",
    "wine",
];

pub const VANILLA_COLORS: [&str; 16] = [
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

/// Longest suffix first, so `concrete_powder` wins over `concrete`.
pub const FAMILIES: [&str; 12] = [
    "stained_glass_pane",
    "concrete_powder",
    "glazed_terracotta",
    "stained_glass",
    "shulker_box",
    "wall_banner",
    "terracotta",
    "concrete",
    "banner",
    "carpet",
    "wool",
    "bed",
];

pub fn is_color(name: &str) -> bool {
    VANILLA_COLORS.contains(&name) || COLORS.contains(&name)
}

/// The block family of a coloured block state (`minecraft:white_wool[..]`).
pub fn family_of(state: &str) -> Option<&'static str> {
    let path = state.split([':', '[']).nth(1).unwrap_or(state);
    FAMILIES.iter().copied().find(|family| {
        path.strip_suffix(*family)
            .and_then(|head| head.strip_suffix('_'))
            .is_some_and(is_color)
    })
}

/// The `[properties]` suffix of a block state, empty when it has none.
pub fn properties(state: &str) -> &str {
    match state.find('[') {
        Some(i) => &state[i..],
        None => "",
    }
}

pub struct Dyenamics;

impl Mod for Dyenamics {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Dyenamics, Registration::new().command("dyenamics")))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for color in COLORS {
            hooks = hooks.item(&format!("dyenamics:{color}_dye"));
        }
        hooks
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        let path = event.item.split([':', '[']).nth(1).unwrap_or(&event.item);
        let Some(color) = path.strip_suffix("_dye") else {
            return EventOutcome::Pass;
        };
        if !COLORS.contains(&color) {
            return EventOutcome::Pass;
        }

        match event.action {
            ItemAction::UseInAir => {
                chat::show(&format!(
                    "🎨 {color} dye: right-click a coloured block to recolour its whole family."
                ));
                EventOutcome::Consume
            }
            ItemAction::UseOnBlock => {
                let pos = BlockPos {
                    x: event.x,
                    y: event.y,
                    z: event.z,
                };
                let Some(state) = server::get_block(&event.dimension, pos) else {
                    return EventOutcome::Pass;
                };
                let Some(family) = family_of(&state) else {
                    return EventOutcome::Pass;
                };

                let target = format!("dyenamics:{color}_{family}");
                let with_properties = format!("{target}{}", properties(&state));
                if server::set_block(&event.dimension, pos, &with_properties).is_ok()
                    || server::set_block(&event.dimension, pos, &target).is_ok()
                {
                    server::play_sound(&event.dimension, pos, "minecraft:item.dye.use", 1.0, 1.0);
                    return EventOutcome::Consume;
                }
                EventOutcome::Pass
            }
            _ => EventOutcome::Pass,
        }
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "dyenamics" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Dyenamics palette ({} dyes): {} — families: {}",
            COLORS.len(),
            COLORS.join(", "),
            FAMILIES.join(", ")
        )))
    }
}

horizon_mod_sdk::export_mod!(Dyenamics);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_family_recognition() {
        assert_eq!(family_of("minecraft:white_wool"), Some("wool"));
        assert_eq!(family_of("minecraft:orange_concrete"), Some("concrete"));
        assert_eq!(
            family_of("minecraft:blue_concrete_powder"),
            Some("concrete_powder")
        );
        assert_eq!(
            family_of("dyenamics:aquamarine_stained_glass[waterlogged=false]"),
            Some("stained_glass")
        );
        assert_eq!(
            family_of("dyenamics:rose_bed[facing=north,part=foot]"),
            Some("bed")
        );
        assert_eq!(family_of("minecraft:stone"), None);
    }

    #[test]
    fn test_properties_extraction() {
        assert_eq!(
            properties("minecraft:red_bed[facing=east,part=head]"),
            "[facing=east,part=head]"
        );
        assert_eq!(properties("minecraft:white_wool"), "");
    }

    #[test]
    fn test_colors_count() {
        assert_eq!(COLORS.len(), 18);
        assert_eq!(VANILLA_COLORS.len(), 16);
        assert_eq!(FAMILIES.len(), 12);
    }
}

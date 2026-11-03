//! Macaw's Furniture: chairs, tables, desks, drawers, cabinets, wardrobes and
//! bookshelves in every wood type, plus the coloured couches and chaises.
//!
//! The storage pieces keep their slots; the rest describe themselves.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    item_transfer::{self, Face},
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

pub const WOODS: [&str; 20] = [
    "acacia",
    "birch",
    "cherry",
    "crimson",
    "dark_oak",
    "jungle",
    "mangrove",
    "oak",
    "spruce",
    "warped",
    "stripped_acacia",
    "stripped_birch",
    "stripped_cherry",
    "stripped_crimson",
    "stripped_dark_oak",
    "stripped_jungle",
    "stripped_mangrove",
    "stripped_oak",
    "stripped_spruce",
    "stripped_warped",
];

/// `(family, slots)`: 0 for a piece that only describes itself.
pub const FAMILIES: [(&str, u32); 31] = [
    ("bookshelf", 27),
    ("bookshelf_cupboard", 27),
    ("bookshelf_drawer", 1),
    ("chair", 0),
    ("coffee_table", 0),
    ("counter", 1),
    ("covered_desk", 0),
    ("cupboard_counter", 27),
    ("desk", 0),
    ("double_drawer", 2),
    ("double_drawer_counter", 2),
    ("double_kitchen_cabinet", 54),
    ("double_wardrobe", 54),
    ("drawer", 1),
    ("drawer_counter", 2),
    ("end_table", 0),
    ("glass_kitchen_cabinet", 27),
    ("glass_table", 0),
    ("kitchen_cabinet", 27),
    ("kitchen_sink", 0),
    ("large_drawer", 2),
    ("lower_bookshelf_drawer", 1),
    ("lower_triple_drawer", 3),
    ("modern_chair", 0),
    ("modern_desk", 0),
    ("modern_wardrobe", 27),
    ("stool_chair", 0),
    ("striped_chair", 0),
    ("table", 0),
    ("triple_drawer", 3),
    ("wardrobe", 27),
];

pub const COLORS: [&str; 16] = [
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

pub fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

pub fn family_of(path: &str) -> Option<(&'static str, u32)> {
    FAMILIES
        .iter()
        .copied()
        .filter(|(family, _)| path.ends_with(&format!("_{family}")))
        .max_by_key(|(family, _)| family.len())
}

pub fn title(family: &str) -> String {
    family
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

pub fn storage_screen(family: &str, slots: u32) -> ScreenSpec {
    let rows = slots.div_ceil(9);
    let widgets = (0..slots)
        .map(|index| Widget {
            id: index,
            kind: WidgetKind::Slot,
            x: 8 + (index % 9) as i32 * 18,
            y: 18 + (index / 9) as i32 * 18,
            width: 18,
            height: 18,
            text: String::new(),
            value: 0,
            max_value: 64,
        })
        .collect();
    ScreenSpec {
        id: "screen:mcwfurnitures:storage".into(),
        title: title(family),
        width: 176,
        height: 18 * 2 + rows * 18,
        widgets,
    }
}

pub struct McwFurnitures;

impl Mod for McwFurnitures {
    fn init() -> Result<(Self, Registration), String> {
        Ok((McwFurnitures, Registration::new().command("furniture")))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        let mut types = Vec::new();
        for wood in WOODS {
            for (family, slots) in FAMILIES {
                if slots > 0 {
                    types.push(BlockEntityType {
                        block: format!("mcwfurnitures:{wood}_{family}"),
                        slots,
                        ticks: false,
                    });
                }
            }
        }
        types
    }

    fn create_block_entity(&mut self, handle: u32, block: &str, _saved: &[u8]) {
        let path = path_of(block);
        if let Some((_, slots)) = family_of(path) {
            for slot in 0..slots {
                for face in [
                    Face::Up,
                    Face::Down,
                    Face::North,
                    Face::South,
                    Face::East,
                    Face::West,
                ] {
                    let _ = item_transfer::set_slot_access(handle, slot, face, true, true);
                }
            }
        }
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for wood in WOODS {
            for (family, _) in FAMILIES {
                hooks = hooks.block(&format!("mcwfurnitures:{wood}_{family}"));
            }
        }
        for color in COLORS {
            hooks = hooks.block(&format!("mcwfurnitures:{color}_couch"));
            hooks = hooks.block(&format!("mcwfurnitures:{color}_chaise"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);

        let Some((family, slots)) = family_of(path) else {
            if path.ends_with("_couch") || path.ends_with("_chaise") {
                chat::show(&format!(
                    "🛋️ {}: right-click to sit down.",
                    title(path.strip_suffix("_couch").unwrap_or(path))
                ));
                return EventOutcome::Consume;
            }
            return EventOutcome::Pass;
        };

        if slots > 0 {
            let _ = ui::open_screen(&storage_screen(family, slots));
            return EventOutcome::Consume;
        }

        let message = match family {
            "chair" | "modern_chair" | "stool_chair" | "striped_chair" => {
                format!("🪑 {}: right-click to sit; sneak to get up.", title(family))
            }
            "coffee_table" | "end_table" | "table" | "glass_table" => {
                format!("🪵 {}: a decorative surface.", title(family))
            }
            "desk" | "covered_desk" | "modern_desk" => {
                format!("🗄️ {}: a desk with a writing surface.", title(family))
            }
            "kitchen_sink" => "🚰 Kitchen sink: runs water for the kitchen.".into(),
            other => format!("🪑 {}.", title(other)),
        };
        chat::show(&message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "furniture" {
            return Err(format!("Unknown command /{name}"));
        }
        let storage: Vec<&str> = FAMILIES
            .iter()
            .filter(|(_, slots)| *slots > 0)
            .map(|(family, _)| *family)
            .collect();
        Ok(Some(format!(
            "Macaw's Furniture: {} pieces ({} woods, {} storage shapes, {} colours).",
            WOODS.len() * FAMILIES.len() + COLORS.len() * 2,
            WOODS.len(),
            storage.len(),
            COLORS.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(McwFurnitures);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_family_and_slots() {
        assert_eq!(family_of("oak_wardrobe"), Some(("wardrobe", 27)));
        assert_eq!(family_of("birch_bookshelf"), Some(("bookshelf", 27)));
        assert_eq!(family_of("spruce_chair"), Some(("chair", 0)));
        assert_eq!(
            family_of("cherry_double_wardrobe"),
            Some(("double_wardrobe", 54))
        );
        assert_eq!(family_of("dark_oak_drawer"), Some(("drawer", 1)));
    }

    #[test]
    fn test_storage_screen_widgets() {
        let screen = storage_screen("wardrobe", 27);
        assert_eq!(screen.id, "screen:mcwfurnitures:storage");
        assert_eq!(screen.title, "Wardrobe");
        assert_eq!(screen.widgets.len(), 27);
    }
}

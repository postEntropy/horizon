//! Productive Bees: advanced beehives with frames, the centrifuge that turns
//! combs into resources, and the jars, incubators and breeding chambers around
//! them.
mod items;

mod hives;

use hives::ADVANCED_HIVES;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const HIVE_SCREEN: &str = "screen:productivebees:hive";
const CENTRIFUGE_SCREEN: &str = "screen:productivebees:centrifuge";
const STORAGE_SCREEN: &str = "screen:productivebees:storage";

const NESTS: [&str; 26] = [
    "acacia_wood_nest",
    "bamboo_hive",
    "birch_wood_nest",
    "bumble_bee_nest",
    "cherry_wood_nest",
    "coarse_dirt_nest",
    "crimson_bee_nest",
    "dark_oak_wood_nest",
    "end_stone_nest",
    "glowstone_nest",
    "gravel_nest",
    "jungle_wood_nest",
    "mangrove_wood_nest",
    "nether_brick_nest",
    "nether_gold_nest",
    "nether_quartz_nest",
    "oak_wood_nest",
    "obsidian_nest",
    "sand_nest",
    "snow_nest",
    "soul_sand_nest",
    "spruce_wood_nest",
    "stone_nest",
    "sugar_cane_nest",
    "sugarbag_nest",
    "warped_bee_nest",
];

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
        max_value: 64,
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

fn title(value: &str) -> String {
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

fn hive_screen() -> ScreenSpec {
    ScreenSpec {
        id: HIVE_SCREEN.into(),
        title: "Advanced Beehive".into(),
        width: 176,
        height: 116,
        widgets: vec![
            slot(0, 26, 17, "Frame"),
            slot(1, 48, 17, "Frame"),
            slot(2, 70, 17, "Frame"),
            slot(3, 98, 17, "Honey"),
            label(4, 42, "Frames speed the bees up.".into()),
            label(5, 58, "The honey slot holds the harvest.".into()),
        ],
    }
}

fn centrifuge_screen() -> ScreenSpec {
    ScreenSpec {
        id: CENTRIFUGE_SCREEN.into(),
        title: "Centrifuge".into(),
        width: 176,
        height: 110,
        widgets: vec![
            slot(0, 62, 17, "Comb"),
            slot(1, 98, 17, "Comb"),
            slot(2, 134, 17, "Comb"),
            Widget {
                id: 3,
                kind: WidgetKind::ProgressArrow,
                x: 8,
                y: 21,
                width: 24,
                height: 17,
                text: "Spin".into(),
                value: 0,
                max_value: 100,
            },
            label(
                4,
                48,
                "Combs become honey, wax and the bee's resource.".into(),
            ),
        ],
    }
}

fn grid_screen(name: &str, slots: u32) -> ScreenSpec {
    let rows = slots.div_ceil(9);
    let widgets: Vec<Widget> = (0..slots)
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
        id: STORAGE_SCREEN.into(),
        title: name.into(),
        width: 176,
        height: 18 * 2 + rows * 18,
        widgets,
    }
}

fn path_of(value: &str) -> &str {
    value.split([':', '[']).nth(1).unwrap_or(value)
}

struct ProductiveBees;

impl Mod for ProductiveBees {
    fn init() -> Result<(Self, Registration), String> {
        Ok((ProductiveBees, Registration::new().command("bees")))
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
        for hive in ADVANCED_HIVES {
            hooks = hooks.block(&format!("productivebees:{hive}"));
        }
        for nest in NESTS {
            hooks = hooks.block(&format!("productivebees:{nest}"));
        }
        for block in [
            "centrifuge",
            "powered_centrifuge",
            "heated_centrifuge",
            "bottler",
            "breeding_chamber",
            "incubator",
            "catcher",
            "gene_indexer",
            "honey_generator",
            "jar_oak",
            "cryo_stasis",
            "configurable_comb",
            "feeder",
            "amber",
            "wax_block",
        ] {
            hooks = hooks.block(&format!("productivebees:{block}"));
        }
        for item in items::ITEMS {
            hooks = hooks.item(&format!("productivebees:{item}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);

        if ADVANCED_HIVES.contains(&path) {
            let _ = ui::open_screen(&hive_screen());
            return EventOutcome::Consume;
        }
        match path {
            "centrifuge" | "powered_centrifuge" | "heated_centrifuge" => {
                let _ = ui::open_screen(&centrifuge_screen());
                return EventOutcome::Consume;
            }
            "bottler" => {
                let _ = ui::open_screen(&grid_screen("Bottler", 9));
                return EventOutcome::Consume;
            }
            "breeding_chamber" => {
                let _ = ui::open_screen(&grid_screen("Breeding Chamber", 2));
                return EventOutcome::Consume;
            }
            _ => {}
        }
        if NESTS.contains(&path) {
            chat::show(&format!(
                "🐝 {}: a wild nest; break it with silk touch to take the bees with it.",
                title(path)
            ));
            return EventOutcome::Consume;
        }
        let message = match path {
            "incubator" => "🥚 Incubator: hatches a bee egg into a larva.",
            "catcher" => "🪤 Catcher: catches the bees that fly into it.",
            "gene_indexer" => "🧬 Gene indexer: stores the genes you sample from the bees.",
            "honey_generator" => "🍯 Honey generator: burns honey for Forge Energy.",
            "jar_oak" => "🫙 A bee jar: keeps a single bee inside.",
            "cryo_stasis" => "❄️ Cryo stasis: holds a bee without it ageing.",
            "configurable_comb" => "🍯 A comb you can configure into another bee's comb.",
            "feeder" => "🍽️ Feeder: keeps the hives stocked with what the bees eat.",
            "amber" => "🟡 Amber: a bee caught in resin.",
            "wax_block" => "🟨 A block of wax.",
            _ if path.starts_with("expansion_box") => {
                "📦 Expansion box: adds room to the advanced beehive beside it."
            }
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.item);
        let message = match path {
            "honey_treat" => "🍯 Honey treat: breeds two bees.",
            "honey_treat_genetic" => "🍯 Genetic honey treat: passes the parents' genes on.",
            "treat_on_a_stick" => "🍯 A honey treat on a stick: leads the bees around.",
            "honey_bucket" => "🍯 A bucket of honey.",
            "wax" => "🟨 Wax: combs melt into it, and it seals the hives.",
            "gene" => "🧬 A gene, sampled from a bee.",
            "gene_bottle" => "🧬 A bottle of genes.",
            "guide_book" => "📖 A guide to the bees and their breeding.",
            "bee_cage" => "🐝 A bee cage: carries one bee.",
            "sturdy_bee_cage" => "🐝 A sturdy bee cage: carries a bee without losing it.",
            "stone_chip" | "wood_chip" | "wither_skull_chip" | "obsidian_shard" => {
                "🪨 A chip the centrifuge makes from a comb."
            }
            "draconic_chunk" | "draconic_dust" => "🐉 Draconic remains, from a dragon bee.",
            _ if path.starts_with("comb_") || path.starts_with("honeycomb_") => {
                "🍯 A comb: the centrifuge turns it into the bee's resource."
            }
            _ if path.starts_with("spawn_egg_") => "🐝 A bee spawn egg.",
            _ if path.ends_with("_petrified_honey") => "🍯 Petrified honey: a hive's old store.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "bees" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Productive Bees: {} advanced beehives, {} wild nests, centrifuges, jars and the breeding chamber.",
            ADVANCED_HIVES.len(),
            NESTS.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(ProductiveBees);

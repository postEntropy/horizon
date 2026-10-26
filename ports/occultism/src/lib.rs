//! Occultism: summon spirits into bowls, bind them into books and let them work
//! the mod's machines, all inside the pentacles drawn with chalk.
//!
//! The sacrificial bowls keep the offering slot and the storage controller
//! keeps the network's items.
mod items;

use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const BOWL_SCREEN: &str = "screen:occultism:bowl";
const STORAGE_SCREEN: &str = "screen:occultism:storage";
const GUIDE_SCREEN: &str = "screen:occultism:guide";

const BOWLS: [&str; 10] = [
    "sacrificial_bowl",
    "copper_sacrificial_bowl",
    "silver_sacrificial_bowl",
    "golden_sacrificial_bowl",
    "iesnium_sacrificial_bowl",
    "dark_sacrificial_bowl",
    "dark_copper_sacrificial_bowl",
    "dark_golden_sacrificial_bowl",
    "dark_silver_sacrificial_bowl",
    "dark_iesnium_sacrificial_bowl",
];

const STORAGE: [&str; 6] = [
    "storage_controller",
    "storage_controller_dark",
    "storage_controller_base",
    "storage_controller_base_dark",
    "storage_controller_stabilized",
    "storage_controller_stabilized_dark",
];

const WORMHOLES: [&str; 4] = [
    "entity_wormhole",
    "entity_wormhole_dark",
    "stable_wormhole",
    "stable_wormhole_dark",
];

const SPIRITS: [&str; 4] = ["foliot", "djinni", "afrit", "marid"];

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

fn bowl_screen() -> ScreenSpec {
    ScreenSpec {
        id: BOWL_SCREEN.into(),
        title: "Sacrificial Bowl".into(),
        width: 176,
        height: 100,
        widgets: vec![
            slot(0, 80, 17, "Offering"),
            label(
                1,
                46,
                "Put the offering in; the pentacle does the rest.".into(),
            ),
        ],
    }
}

fn storage_screen() -> ScreenSpec {
    let widgets: Vec<Widget> = (0..27)
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
        title: "Storage Controller".into(),
        width: 176,
        height: 96,
        widgets,
    }
}

fn guide_screen() -> ScreenSpec {
    let mut widgets = vec![label(0, 18, "The spirits and what they do".into())];
    let lines = [
        "Foliot: the simple worker — farms, cuts wood, moves items.",
        "Djinni: the skilled worker — manages machines and weather.",
        "Afrit: the strong worker — smelts, crushes, crystallizes.",
        "Marid: the master worker — the fastest of the four.",
        "Bind one with a book of binding inside its pentacle.",
    ];
    for (index, line) in lines.iter().enumerate() {
        widgets.push(label(
            1 + index as u32,
            34 + index as i32 * 11,
            (*line).into(),
        ));
    }
    ScreenSpec {
        id: GUIDE_SCREEN.into(),
        title: "Dictionary of Spirits".into(),
        width: 176,
        height: 110,
        widgets,
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

fn path_of(value: &str) -> &str {
    value.split([':', '[']).nth(1).unwrap_or(value)
}

struct Occultism;

impl Mod for Occultism {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Occultism, Registration::new().command("occultism")))
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
        for bowl in BOWLS {
            hooks = hooks.block(&format!("occultism:{bowl}"));
        }
        for controller in STORAGE {
            hooks = hooks.block(&format!("occultism:{controller}"));
        }
        for block in [
            "spirit_fire",
            "spirit_campfire",
            "spirit_grindstone",
            "spirit_lantern",
            "iesnium_anvil",
            "celestial_chalice",
            "eldritch_chalice",
            "dimensional_battlefield",
            "dimensional_extractor",
            "dimensional_matrix",
            "dimensional_mineshaft",
            "otherstone_pedestal",
            "otherstone_tablet",
        ] {
            hooks = hooks.block(&format!("occultism:{block}"));
        }
        for tier in 0..=5 {
            for suffix in ["", "_dark"] {
                hooks = hooks.block(&format!("occultism:storage_stabilizer_tier{tier}{suffix}"));
            }
        }
        for wormhole in WORMHOLES {
            hooks = hooks.block(&format!("occultism:{wormhole}"));
        }
        for spirit in SPIRITS {
            hooks = hooks.item(&format!("occultism:{spirit}"));
            hooks = hooks.item(&format!("occultism:{spirit}_essence"));
            hooks = hooks.item(&format!("occultism:book_of_binding_{spirit}"));
            hooks = hooks.item(&format!("occultism:book_of_binding_bound_{spirit}"));
        }
        for item in items::ITEMS {
            hooks = hooks.item(&format!("occultism:{item}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);
        if BOWLS.contains(&path) {
            let _ = ui::open_screen(&bowl_screen());
            return EventOutcome::Consume;
        }
        if STORAGE.contains(&path) {
            let _ = ui::open_screen(&storage_screen());
            return EventOutcome::Consume;
        }
        if path.starts_with("storage_stabilizer_tier") {
            let tier = path
                .trim_start_matches("storage_stabilizer_tier")
                .trim_end_matches("_dark");
            chat::show(&format!(
                "📦 Storage stabilizer tier {tier}: {}.",
                if path.ends_with("_dark") {
                    "a dark stabilizer, for the dark storage network"
                } else {
                    "widens the storage network by one block"
                }
            ));
            return EventOutcome::Consume;
        }
        if WORMHOLES.contains(&path) {
            chat::show(if path.starts_with("stable") {
                "🌀 Stable wormhole: a permanent link any entity can step through."
            } else {
                "🌀 Wormhole: a link for entities, open only while the ritual holds."
            });
            return EventOutcome::Consume;
        }
        let message = match path {
            "spirit_fire" => "🔥 Spirit fire: the flame that turns offerings into spirits.",
            "spirit_campfire" => {
                "🔥 Spirit campfire: spirit fire that cooks and keeps the pentacle lit."
            }
            "spirit_grindstone" => "⚙️ Spirit grindstone: grinds items with a spirit's help.",
            "spirit_lantern" => "🏮 Spirit lantern: light made from a spirit.",
            "iesnium_anvil" => "⚒️ Iesnium anvil: an anvil that never wears out.",
            "celestial_chalice" => "🏆 Celestial chalice: an offering to the spirits above.",
            "eldritch_chalice" => "🏆 Eldritch chalice: an offering to the things below.",
            "dimensional_battlefield" => "🌀 Dimensional battlefield: summons the spirits' arena.",
            "dimensional_extractor" => {
                "🌀 Dimensional extractor: pulls resources from other dimensions."
            }
            "dimensional_matrix" => "🌀 Dimensional matrix: the heart of the storage network.",
            "dimensional_mineshaft" => {
                "🌀 Dimensional mineshaft: spirits mine the dimensions for you."
            }
            "otherstone_pedestal" => "🪨 Otherstone pedestal: holds an item for a ritual.",
            "otherstone_tablet" => "🪨 Otherstone tablet: keeps a ritual's record.",
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
        if path == "dictionary_of_spirits" {
            let _ = ui::open_screen(&guide_screen());
            return EventOutcome::Consume;
        }
        if let Some(spirit) = SPIRITS.iter().find(|spirit| path == **spirit) {
            chat::show(&format!(
                "👁️ {}: an unbound spirit, waiting for its pentacle.",
                title(spirit)
            ));
            return EventOutcome::Consume;
        }
        if let Some(spirit) = path.strip_prefix("book_of_binding_") {
            let spirit = spirit.strip_prefix("bound_").unwrap_or(spirit);
            chat::show(&format!(
                "📕 Book of binding ({}): use it in the {} pentacle to bind the spirit.",
                title(spirit),
                title(spirit)
            ));
            return EventOutcome::Consume;
        }
        if let Some(rest) = path.strip_prefix("book_of_calling_") {
            chat::show(&format!(
                "📗 Book of calling ({}): calls a bound {} back to a job.",
                title(rest),
                title(rest.split('_').next().unwrap_or(rest))
            ));
            return EventOutcome::Consume;
        }
        if path.starts_with("summon_") || path.starts_with("possess_") {
            chat::show(&format!(
                "📜 Ritual: {} — a ritual dummy that describes the circle it needs.",
                title(path)
            ));
            return EventOutcome::Consume;
        }
        if path.starts_with("pentacle_") {
            chat::show(&format!(
                "⭐ {}: the chalk pattern a ritual is drawn in.",
                title(path)
            ));
            return EventOutcome::Consume;
        }
        if path.starts_with("familiar_") {
            chat::show(&format!("🐾 {}: a familiar the ring keeps.", title(path)));
            return EventOutcome::Consume;
        }
        if let Some(color) = path.strip_prefix("chalk_") {
            chat::show(&format!(
                "🖍️ {} chalk: draws the pentacles on the ground.",
                title(color.trim_end_matches("_impure"))
            ));
            return EventOutcome::Consume;
        }
        let message = match path {
            "divination_rod" => {
                "🪄 Divination rod: points at the ores and other things you ask for."
            }
            "true_sight_staff" => "🪄 True sight staff: shows the otherworld around you.",
            "ritual_satchel_t1" | "ritual_satchel_t2" => {
                "🎒 Ritual satchel: holds a whole pentacle's worth of chalk."
            }
            "satchel" | "ender_satchel" => "🎒 Satchel: a bigger inventory, on your back.",
            "soul_gem" => "💎 Soul gem: holds the soul of a mob.",
            "fragile_soul_gem" => "💎 Fragile soul gem: it shatters when its mob dies.",
            "iesnium_ingot" | "iesnium_nugget" | "raw_iesnium" => "⚪ Iesnium, the spirits' metal.",
            "iesnium_pickaxe" => "⛏️ Iesnium pickaxe: mines what ordinary picks cannot.",
            "butcher_knife" => "🔪 Butcher knife: harvests more from the animals you kill.",
            "brush" => "🖌️ Brush: cleans the otherworld's ruins.",
            "storage_remote" => "📡 Storage remote: reaches the storage network from anywhere.",
            "storage_remote_inert" => "📡 An inert storage remote: bind it to a network first.",
            "knowledge_tablet" => "📱 Knowledge tablet: the occultist's notebook.",
            _ if path.ends_with("_dust") => {
                "✨ Spirit dust, ground from the otherworld's crystals."
            }
            _ if path.ends_with("_essence") => "✨ Essence, dropped by a spirit.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "occultism" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Occultism: {} spirits ({}); sacrificial bowls, pentacles, the storage network and the dimensional mineshaft.",
            SPIRITS.len(),
            SPIRITS.map(title).join(", ")
        )))
    }
}

horizon_mod_sdk::export_mod!(Occultism);

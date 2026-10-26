//! Neo Vitae: the Blood Altar, its runes and orbs, the rituals and the
//! spiritus network that a blood mage builds.
//!
//! The Blood Altar keeps its four slots and its Life Essence; the tanks, the ARA
//! Vitae and the spirit caches show what they hold.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const ALTAR_SCREEN: &str = "screen:neovitae:altar";
const FORGE_SCREEN: &str = "screen:neovitae:forge";
const BLOOD_SCREEN: &str = "screen:neovitae:blood";

const BLOOD_CAPACITY: u64 = 1_000_000;

const ORBS: [(&str, &str, u64); 6] = [
    ("weak", "Weak", 5_000),
    ("apprentice", "Apprentice", 25_000),
    ("magician", "Magician", 150_000),
    ("master", "Master", 1_000_000),
    ("archmage", "Archmage", 10_000_000),
    ("transcendent", "Transcendent", 100_000_000),
];

const SIGILS: [(&str, &str); 17] = [
    ("air", "lets you breathe under water"),
    ("blood_light", "places a light where you look"),
    (
        "bound_treasures",
        "keeps your treasures bound to you when you die",
    ),
    ("damned", "sends what you hit to the void"),
    ("divination", "finds the ores around you"),
    ("fast_miner", "gives you haste while you mine"),
    ("frost", "freezes the water you walk on"),
    ("green_grove", "grows the plants around you"),
    ("holding", "adds an extra inventory"),
    ("lava", "keeps the lava from burning you"),
    ("magnetism", "pulls the drops to you"),
    ("necromancy", "raises the dead you leave behind"),
    ("phantom_bridge", "builds a bridge of light under your feet"),
    ("seer", "gives you night vision"),
    ("suppression", "suppresses the mobs around you"),
    ("teleposition", "teleports you to the point you marked"),
    ("void", "voids the items you throw away"),
];

const RITUAL_STONES: [&str; 9] = [
    "air_ritual_stone",
    "earth_ritual_stone",
    "fire_ritual_stone",
    "water_ritual_stone",
    "imperfect_ritual_stone",
    "demon_ritual_stone",
    "master_ritual_stone",
    "deus_ritual_stone",
    "inverted_master_ritual_stone",
];

const ROUTING: [&str; 5] = [
    "input_routing_node",
    "output_routing_node",
    "master_routing_node",
    "omni_routing_node",
    "routing_conduit",
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

fn altar_screen() -> ScreenSpec {
    let mut widgets: Vec<Widget> = (0..4)
        .map(|index| slot(index, 26 + index as i32 * 22, 17, "Slot"))
        .collect();
    widgets.push(Widget {
        id: 4,
        kind: WidgetKind::FluidGauge,
        x: 8,
        y: 17,
        width: 12,
        height: 54,
        text: "LP".into(),
        value: 0,
        max_value: BLOOD_CAPACITY,
    });
    widgets.push(label(5, 42, "Blood Altar".into()));
    widgets.push(label(
        6,
        56,
        "Life Essence comes from the sacrificial knife.".into(),
    ));
    ScreenSpec {
        id: ALTAR_SCREEN.into(),
        title: "Blood Altar".into(),
        width: 176,
        height: 96,
        widgets,
    }
}

fn forge_screen() -> ScreenSpec {
    ScreenSpec {
        id: FORGE_SCREEN.into(),
        title: "Hellfire Forge".into(),
        width: 176,
        height: 96,
        widgets: vec![
            slot(0, 62, 17, "Item"),
            slot(1, 80, 17, "Spiritus"),
            slot(2, 98, 17, "Spiritus"),
            label(3, 46, "Spiritus makes the demonic items.".into()),
        ],
    }
}

fn gauge_screen(name: &str) -> ScreenSpec {
    ScreenSpec {
        id: BLOOD_SCREEN.into(),
        title: name.into(),
        width: 176,
        height: 110,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::FluidGauge,
                x: 8,
                y: 18,
                width: 16,
                height: 54,
                text: "LP".into(),
                value: 0,
                max_value: BLOOD_CAPACITY,
            },
            label(1, 18, format!("Capacity: {BLOOD_CAPACITY} LP")),
        ],
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

struct NeoVitae;

impl Mod for NeoVitae {
    fn init() -> Result<(Self, Registration), String> {
        Ok((NeoVitae, Registration::new().command("vitae")))
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
        for block in [
            "blood_altar",
            "hellfire_forge",
            "ara_vitae",
            "blood_tank",
            "blood_battery",
            "spirit_cache",
            "spirit_accumulator",
            "incense_altar",
            "athanor",
            "tabula_vitae",
            "alchemy_array",
            "crystal_cluster",
            "crystallarium_maleficum",
            "vas_maleficum",
            "spira_infernalis",
            "teleposer",
            "mimic",
            "ethereal_mimic",
            "phantom_bridge",
            "spatial_rift",
            "vitae_link",
            "orb_filling_link",
            "spike_trap",
            "spikes",
            "blood_lantern",
            "blood_light",
        ] {
            hooks = hooks.block(&format!("neovitae:{block}"));
        }
        for stone in RITUAL_STONES {
            hooks = hooks.block(&format!("neovitae:{stone}"));
        }
        for node in ROUTING {
            hooks = hooks.block(&format!("neovitae:{node}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);
        match path {
            "blood_altar" => {
                let _ = ui::open_screen(&altar_screen());
                return EventOutcome::Consume;
            }
            "hellfire_forge" => {
                let _ = ui::open_screen(&forge_screen());
                return EventOutcome::Consume;
            }
            "ara_vitae" | "blood_tank" | "blood_battery" | "spirit_cache"
            | "spirit_accumulator" => {
                let _ = ui::open_screen(&gauge_screen(&title(path)));
                return EventOutcome::Consume;
            }
            _ => {}
        }

        if RITUAL_STONES.contains(&path) {
            chat::show(&format!(
                "🩸 {}: a ritual stone; the altar's runes are built on these.",
                title(path)
            ));
            return EventOutcome::Consume;
        }
        if ROUTING.contains(&path) {
            chat::show(&format!(
                "🩸 {}: moves items and spiritus through the altar's network.",
                title(path)
            ));
            return EventOutcome::Consume;
        }

        let message = match path {
            "incense_altar" => "🕯️ Incense altar: burns incense to keep a ritual going.",
            "athanor" => "⚗️ Athanor: makes the alchemy arrays.",
            "tabula_vitae" => "📜 Tabula Vitae: the blood mage's notebook.",
            "alchemy_array" => "🩸 Alchemy array: a chalk circle that does one thing when powered.",
            "crystal_cluster" => "💠 Crystal cluster: grows spiritus crystals.",
            "crystallarium_maleficum" => "💠 Crystallarium: grows a spiritus crystal from a seed.",
            "vas_maleficum" => "🏺 Vas Maleficum: a vessel for the spirits of the dead.",
            "spira_infernalis" => "🌀 Spira Infernalis: a spiral that pulls spiritus in.",
            "teleposer" => "🌀 Teleposer: moves blocks and entities between teleposers.",
            "mimic" => "🎭 Mimic: copies the block it was made from.",
            "ethereal_mimic" => "🎭 Ethereal mimic: a mimic that can be passed through.",
            "phantom_bridge" => "🌉 Phantom bridge: a bridge of light you can walk on.",
            "spatial_rift" => "🌀 Spatial rift: tears a hole between two points.",
            "vitae_link" => "🩸 Vitae link: moves Life Essence between altars.",
            "orb_filling_link" => "🩸 Orb filling link: fills the orbs of the altar next to it.",
            "spike_trap" | "spikes" => {
                "🩸 Spikes: hurt whatever walks over them, and feed the altar."
            }
            "blood_lantern" | "blood_light" => "🩸 A light lit by Life Essence.",
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

        if let Some(rest) = path.strip_prefix("blood_orb_") {
            let tier = rest.split('_').next().unwrap_or(rest);
            if let Some((_, name, capacity)) = ORBS.iter().find(|(orb, _, _)| *orb == tier) {
                chat::show(&format!("🩸 {name} Blood Orb: holds up to {capacity} LP."));
                return EventOutcome::Consume;
            }
        }
        if let Some(sigil) = path.strip_prefix("sigil_") {
            let effect = SIGILS
                .iter()
                .find(|(name, _)| *name == sigil)
                .map(|(_, effect)| *effect)
                .unwrap_or("carries a blood mage's spell");
            chat::show(&format!("🩸 Sigil of {}: {effect}.", title(sigil)));
            return EventOutcome::Consume;
        }
        if let Some(rune) = path.strip_prefix("rune_") {
            chat::show(&format!(
                "🩸 Rune of {}: a rune placed on the altar to change how it works.",
                title(rune.trim_start_matches("2_"))
            ));
            return EventOutcome::Consume;
        }
        if path.ends_with("_charge") || path.contains("_charge_") {
            chat::show("💥 A shaped charge: digs a pattern out of the world.");
            return EventOutcome::Consume;
        }
        let message = match path {
            "ritual_stone" => "🩸 Ritual stone: the master block of a ritual circle.",
            "ritual_diviner" => "📜 Ritual diviner: tells you which stones a ritual needs.",
            "ritual_ledger" => "📜 Ritual ledger: keeps the rituals you have learned.",
            "ritual_reader" => "📜 Ritual reader: copies a ritual into the ledger.",
            "ritual_designer" => "📜 Ritual designer: writes a new ritual.",
            "blood_orb_weak_empty" => "🩸 An empty blood orb.",
            _ if path.ends_with("_dust") => "🩸 A dust used in the altar's recipes.",
            _ if path.ends_with("_crystal") => "💠 A spiritus crystal.",
            _ if path.ends_with("_scribe_tool") => "🖊️ A scribe tool for the alchemy arrays.",
            _ if path.ends_with("_dagger") => "🗡️ A throwing dagger, tipped with blood.",
            _ if path.contains("alchemy_flask") => {
                "🍶 An alchemy flask: holds a potion made at the altar."
            }
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "vitae" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Neo Vitae: {} blood orbs ({} LP at the top); {} sigils; {} ritual stones; the altar holds {BLOOD_CAPACITY} LP.",
            ORBS.len(),
            ORBS[ORBS.len() - 1].2,
            SIGILS.len(),
            RITUAL_STONES.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(NeoVitae);

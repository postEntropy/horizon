//! MineColonies: colonies of worker huts, each with a job and a storage of its
//! own, plus the town hall that ties them together.
//!
//! The huts open their management screen and report the job they hold; the
//! colony simulation itself (citizens, pathfinding, requests) is the mod's own
//! server code, which a block hook cannot run.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const HUT_SCREEN: &str = "screen:minecolonies:hut";
const STORAGE_SCREEN: &str = "screen:minecolonies:storage";

const HUTS: &[&str] = &[
    "alchemist",
    "archery",
    "baker",
    "barracks",
    "barrackstower",
    "beekeeper",
    "blacksmith",
    "builder",
    "chickenherder",
    "citizen",
    "combatacademy",
    "composter",
    "concretemixer",
    "cook",
    "cowboy",
    "crusher",
    "deliveryman",
    "dyer",
    "enchanter",
    "farmer",
    "field",
    "fisherman",
    "fletcher",
    "florist",
    "gatehouse",
    "glassblower",
    "graveyard",
    "guardtower",
    "hospital",
    "kitchen",
    "library",
    "lumberjack",
    "mechanic",
    "miner",
    "mysticalsite",
    "netherworker",
    "plantation",
    "plantationfield",
    "rabbithutch",
    "sawmill",
    "school",
    "shepherd",
    "sifter",
    "smeltery",
    "stable",
    "stonemason",
    "stonesmeltery",
    "swineherder",
    "tavern",
    "townhall",
    "university",
    "warehouse",
];

const STORAGES: &[(&str, u32)] = &[
    ("barrel_block", 27),
    ("postbox", 27),
    ("stash", 27),
    ("supplycamp", 27),
    ("supplychest", 27),
];

fn path_of(value: &str) -> &str {
    value.split([':', '[']).nth(1).unwrap_or(value)
}

fn title(hut: &str) -> String {
    hut.split('_')
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

fn describe(hut: &str) -> &'static str {
    match hut {
        "townhall" => {
            "the heart of the colony: it lists the citizens, the buildings and the colony's claims"
        }
        "builder" => "a builder takes the build requests and raises the colony's buildings",
        "farmer" => "a farmer plants and harvests the colony's fields",
        "lumberjack" => "a lumberjack fells trees for the colony's wood",
        "miner" => "a miner digs the colony's mineshaft and brings back the ores",
        "guardtower" | "barracks" | "barrackstower" | "combatacademy" | "archery" => {
            "the colony's guards train and defend here"
        }
        "cook" | "kitchen" | "baker" | "tavern" => "the colony's food is made and served here",
        "warehouse" | "deliveryman" => {
            "the deliverymen keep the colony's requests stocked from here"
        }
        "university" | "school" | "library" => "the colonists study here to raise their skills",
        "hospital" => "the colony's doctor treats the wounded citizens here",
        "enchanter" => "the colony's enchanter works the enchanting table here",
        "graveyard" => "the colony's dead are buried here",
        "netherworker" => "a netherworker mines the nether through a portal",
        "rabbithutch" | "chickenherder" | "cowboy" | "shepherd" | "swineherder" => {
            "the colony's animals are kept here"
        }
        "composter" => "a composter turns the colony's leftovers into compost",
        "crusher" => "a crusher breaks the cobblestone the colony digs up",
        "sifter" => "a sifter sifts gravel and sand for the colony",
        "smeltery" | "stonesmeltery" => "the colony's ores and stone are smelted here",
        "sawmill" => "the colony's logs are sawn into planks here",
        "stonemason" => "the colony's stone is cut into shape here",
        "glassblower" => "the colony's glass is blown here",
        "dyer" => "the colony's wool is dyed here",
        "florist" => "the colony's flowers are grown here",
        "beekeeper" => "the colony's bees are kept here",
        "alchemist" => "the colony's potions are brewed here",
        "mechanic" => "the colony's tools are repaired here",
        "fletcher" => "the colony's bows and arrows are made here",
        "concretemixer" => "the colony's concrete is mixed here",
        "mysticalsite" => "a mystical site the colony protects",
        "plantation" | "plantationfield" | "field" => "a field the colony's farmers work",
        "gatehouse" => "the colony's gate is held here",
        "citizen" => "the citizens' home",
        _ => "a colony building",
    }
}

fn slot_grid(slots: u32, screen: &str, title: &str) -> ScreenSpec {
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
        id: screen.into(),
        title: title.into(),
        width: 176,
        height: 18 * 2 + rows * 18,
        widgets,
    }
}

fn hut_screen(hut: &str) -> ScreenSpec {
    let name = title(hut);
    ScreenSpec {
        id: HUT_SCREEN.into(),
        title: format!("{name} Hut"),
        width: 176,
        height: 120,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::Label,
                x: 8,
                y: 18,
                width: 160,
                height: 10,
                text: format!("{name}: {}", describe(hut)),
                value: 0,
                max_value: 0,
            },
            Widget {
                id: 1,
                kind: WidgetKind::Label,
                x: 8,
                y: 32,
                width: 160,
                height: 10,
                text: "Level 1 · assign a worker from the town hall.".into(),
                value: 1,
                max_value: 5,
            },
            Widget {
                id: 2,
                kind: WidgetKind::ProgressArrow,
                x: 8,
                y: 50,
                width: 24,
                height: 17,
                text: "Work".into(),
                value: 0,
                max_value: 100,
            },
            Widget {
                id: 3,
                kind: WidgetKind::Label,
                x: 8,
                y: 76,
                width: 160,
                height: 10,
                text: "Build it with the build tool and supply it from the warehouse.".into(),
                value: 0,
                max_value: 0,
            },
        ],
    }
}

struct MineColonies {
    /// Buildings placed here, for `/colony`.
    huts: Vec<String>,
}

impl Mod for MineColonies {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            MineColonies { huts: Vec::new() },
            Registration::new().command("colony"),
        ))
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
        for hut in HUTS {
            hooks = hooks.block(&format!("minecolonies:blockhut{hut}"));
        }
        for block in [
            "barrel_block",
            "postbox",
            "stash",
            "supplycamp",
            "supplychest",
            "blockconstructiontape",
            "simplequarry",
            "mediumquarry",
            "largequarry",
            "blockminecoloniesgrave",
            "blockminecoloniesnamedgrave",
            "blockminecoloniesrack",
            "blockwaypoint",
            "decorationcontroller",
            "colonysign",
            "colony_banner",
            "colony_wall_banner",
        ] {
            hooks = hooks.block(&format!("minecolonies:{block}"));
        }
        for item in [
            "supplycamp",
            "supplychest",
            "sceptergold",
            "sceptersteel",
            "scepterpermission",
            "caliper",
            "ancienttome",
            "adventure_token",
        ] {
            hooks = hooks.item(&format!("minecolonies:{item}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);

        if let Some(hut) = path.strip_prefix("blockhut") {
            let _ = ui::open_screen(&hut_screen(hut));
            let key = format!("{}:{},{},{}", event.dimension, event.x, event.y, event.z);
            if !self.huts.contains(&key) {
                self.huts.push(key);
            }
            return EventOutcome::Consume;
        }

        if let Some((storage, slots)) = STORAGES.iter().find(|(name, _)| *name == path) {
            let title = if *storage == "supplycamp" {
                "Supply Camp"
            } else if *storage == "supplychest" {
                "Supply Chest"
            } else {
                "Colony Storage"
            };
            let _ = ui::open_screen(&slot_grid(*slots, STORAGE_SCREEN, title));
            return EventOutcome::Consume;
        }

        let message = match path {
            "blockconstructiontape" => {
                "🚧 Construction tape: the builder's outline for a new building."
            }
            "simplequarry" | "mediumquarry" | "largequarry" => {
                "⛏️ Quarry: the colony miners dig it out level by level."
            }
            "blockminecoloniesgrave" | "blockminecoloniesnamedgrave" => "🪦 A colonist's grave.",
            "blockminecoloniesrack" => "🗄️ A rack for the colony's tools.",
            "blockwaypoint" => "📍 Waypoint: a point the citizens path to.",
            "decorationcontroller" => "🎨 Decoration controller: places the colony's decorations.",
            "colonysign" => "🪧 Colony sign: shows the colony's name and owner.",
            "colony_banner" | "colony_wall_banner" => "🚩 The colony's banner.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let message = match path_of(&event.item) {
            "supplycamp" => "🏕️ Supply camp: place it to found a colony with a starter camp.",
            "supplychest" => "📦 Supply chest: place it to found a colony with a starter chest.",
            "sceptergold" => {
                "🪄 Gold scepter: the colony leader's tool, for claiming and managing."
            }
            "sceptersteel" => "🪄 Steel scepter: a colony manager's tool.",
            "scepterpermission" => "🪄 Permission scepter: hands out a citizen's permissions.",
            "caliper" => "📏 Caliper: measures the space a building needs.",
            "ancienttome" => "📖 Ancient tome: raises a colonist's skill.",
            "adventure_token" => "🎟️ Adventure token: buys colony goods from the tavern.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "colony" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "MineColonies: {} hut kinds, {} here; storage in barrels, postboxes, stashes and supply chests.",
            HUTS.len(),
            self.huts.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(MineColonies);

//! The NeoForge oracle's facts (`tools/neoforge_oracle.py export`, in
//! `<work>/oracle/facts.json`): every block, item, recipe and tag the pack's
//! mods register, read from the running game, and the vanilla blocks beside
//! them. Carry-over uses them in place of guesses: a block's template is the
//! vanilla block of the same class and states whose shapes and sounds match
//! best, and the declared properties are the mod's own numbers.
use serde::Deserialize;
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};

#[derive(Debug, Deserialize)]
pub struct Facts {
    pub blocks: BTreeMap<String, Block>,
    pub items: BTreeMap<String, Item>,
    #[serde(default)]
    pub tags: Tags,
}

#[derive(Debug, Default, Deserialize)]
pub struct Tags {
    #[serde(default)]
    pub block: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub item: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct Block {
    /// The class and its superclasses, most derived first.
    pub class: Vec<String>,
    pub properties: BTreeMap<String, Vec<String>>,
    pub default: String,
    pub item: String,
    pub explosion_resistance: f32,
    pub friction: f32,
    pub speed_factor: f32,
    pub jump_factor: f32,
    #[serde(default)]
    pub block_set_type: Option<SetType>,
    #[serde(default)]
    pub wood_type: Option<String>,
    pub states: BTreeMap<String, State>,
}

#[derive(Debug, Deserialize)]
pub struct SetType {
    pub name: String,
    pub can_open_by_hand: bool,
}

#[derive(Debug, Deserialize)]
pub struct State {
    pub hardness: f32,
    pub emission: u8,
    pub requires_tool: bool,
    pub map_color_id: u8,
    pub sound: Value,
    pub outline: Value,
    pub collision: Value,
    /// What breaking the state drops, per tool (`hand`, `diamond_pickaxe`,
    /// `shears`, `silk_touch_pickaxe`...): item id → count.
    #[serde(default)]
    pub drops: BTreeMap<String, BTreeMap<String, u32>>,
}

#[derive(Debug, Deserialize)]
pub struct Item {
    pub class: String,
    pub max_stack_size: u8,
    pub rarity: String,
    #[serde(default)]
    pub block: Option<String>,
    #[serde(default)]
    pub food: Option<Value>,
    #[serde(default)]
    pub crafting_remainder: Option<String>,
}

const VANILLA_PACKAGE: &str = "net.minecraft.";
struct ObservedFdFood {
    id: &'static str,
    effect: &'static str,
    duration: i64,
    eat_seconds: f64,
    can_always_eat: bool,
    probability: f64,
    visible: bool,
    show_icon: bool,
    use_remainder: Option<&'static str>,
}

struct ObservedFdSpecialFood {
    nutrition: u64,
    saturation: f64,
    use_remainder: Option<&'static str>,
}

/// These subclasses alter the vanilla food facts (or add a right-click item
/// interaction). Keep support limited to the individually consumed IDs whose
/// full public result was captured in the L4 black-box trace.
fn observed_fd_special_food(id: &str, item: &Item, food: &Value) -> Option<ObservedFdSpecialFood> {
    let (
        class,
        facts_nutrition,
        facts_saturation,
        always,
        eat_seconds,
        nutrition,
        saturation,
        remainder,
        effects,
    ) = match id {
        "farmersdelight:apple_cider" => (
            "vectorwing.farmersdelight.common.item.DrinkableItem",
            0,
            0.0,
            true,
            1.6,
            4,
            0.0,
            Some("minecraft:glass_bottle"),
            json!([{"id":"minecraft:absorption","duration":1200,"amplifier":0,"probability":1.0,"ambient":false,"visible":true,"show_icon":true}]),
        ),
        "farmersdelight:bone_broth" => (
            "vectorwing.farmersdelight.common.item.DrinkableItem",
            8,
            11.2,
            false,
            1.6,
            12,
            11.2,
            Some("minecraft:bowl"),
            json!([{"id":"farmersdelight:nourishment","duration":1200,"amplifier":0,"probability":1.0,"ambient":false,"visible":false,"show_icon":false}]),
        ),
        "farmersdelight:dog_food" => (
            "vectorwing.farmersdelight.common.item.DogFoodItem",
            4,
            1.6,
            false,
            1.6,
            4,
            1.6,
            Some("minecraft:bowl"),
            json!([]),
        ),
        "farmersdelight:melon_popsicle" => (
            "vectorwing.farmersdelight.common.item.PopsicleItem",
            3,
            1.2,
            true,
            0.8,
            7,
            1.2,
            None,
            json!([]),
        ),
        "farmersdelight:tomato_sauce" => (
            "vectorwing.farmersdelight.common.item.ConsumableItem",
            4,
            3.2,
            false,
            1.6,
            8,
            3.2,
            Some("minecraft:bowl"),
            json!([]),
        ),
        _ => return None,
    };
    (item.class == class
        && food["nutrition"].as_u64() == Some(facts_nutrition)
        && food["saturation"].as_f64() == Some(facts_saturation)
        && food["can_always_eat"].as_bool() == Some(always)
        && food["eat_seconds"].as_f64() == Some(eat_seconds)
        && food["effects"] == effects)
        .then_some(ObservedFdSpecialFood {
            nutrition,
            saturation,
            use_remainder: remainder,
        })
}

const OBSERVED_FD_FOODS: &[ObservedFdFood] = &[
    ObservedFdFood {
        id: "farmersdelight:bacon_and_eggs",
        effect: "farmersdelight:nourishment",
        duration: 1200,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:baked_cod_stew",
        effect: "farmersdelight:nourishment",
        duration: 6000,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:beef_stew",
        effect: "farmersdelight:nourishment",
        duration: 3600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:chicken_soup",
        effect: "farmersdelight:nourishment",
        duration: 3600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:cooked_rice",
        effect: "farmersdelight:nourishment",
        duration: 600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:fish_stew",
        effect: "farmersdelight:nourishment",
        duration: 3600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:fried_rice",
        effect: "farmersdelight:nourishment",
        duration: 3600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:gleaming_salad",
        effect: "farmersdelight:nourishment",
        duration: 6000,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:grilled_salmon",
        effect: "farmersdelight:nourishment",
        duration: 3600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:honey_glazed_ham",
        effect: "farmersdelight:nourishment",
        duration: 6000,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:mushroom_rice",
        effect: "farmersdelight:nourishment",
        duration: 3600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:noodle_soup",
        effect: "farmersdelight:nourishment",
        duration: 6000,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:onion_soup",
        effect: "farmersdelight:nourishment",
        duration: 3600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:pasta_with_meatballs",
        effect: "farmersdelight:nourishment",
        duration: 3600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:pasta_with_mutton_chop",
        effect: "farmersdelight:nourishment",
        duration: 3600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:pumpkin_soup",
        effect: "farmersdelight:nourishment",
        duration: 6000,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:ratatouille",
        effect: "farmersdelight:nourishment",
        duration: 1200,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:roast_chicken",
        effect: "farmersdelight:nourishment",
        duration: 6000,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:roasted_mutton_chops",
        effect: "farmersdelight:nourishment",
        duration: 6000,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:shepherds_pie",
        effect: "farmersdelight:nourishment",
        duration: 6000,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:squid_ink_pasta",
        effect: "farmersdelight:nourishment",
        duration: 6000,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:steak_and_potatoes",
        effect: "farmersdelight:nourishment",
        duration: 3600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:stuffed_pumpkin",
        effect: "farmersdelight:nourishment",
        duration: 6000,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:vegetable_noodles",
        effect: "farmersdelight:nourishment",
        duration: 6000,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:vegetable_soup",
        effect: "farmersdelight:nourishment",
        duration: 3600,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:apple_pie_slice",
        effect: "minecraft:speed",
        duration: 600,
        eat_seconds: 0.8,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: None,
    },
    ObservedFdFood {
        id: "farmersdelight:cake_slice",
        effect: "minecraft:speed",
        duration: 400,
        eat_seconds: 0.8,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: None,
    },
    ObservedFdFood {
        id: "farmersdelight:chocolate_pie_slice",
        effect: "minecraft:speed",
        duration: 600,
        eat_seconds: 0.8,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: None,
    },
    ObservedFdFood {
        id: "farmersdelight:fruit_salad",
        effect: "minecraft:regeneration",
        duration: 100,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: true,
        show_icon: true,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:glow_berry_custard",
        effect: "minecraft:glowing",
        duration: 100,
        eat_seconds: 1.6,
        can_always_eat: true,
        probability: 1.0,
        visible: true,
        show_icon: true,
        use_remainder: Some("minecraft:glass_bottle"),
    },
    ObservedFdFood {
        id: "farmersdelight:mixed_salad",
        effect: "minecraft:regeneration",
        duration: 100,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 1.0,
        visible: true,
        show_icon: true,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:nether_salad",
        effect: "minecraft:nausea",
        duration: 240,
        eat_seconds: 1.6,
        can_always_eat: false,
        probability: 0.3,
        visible: true,
        show_icon: true,
        use_remainder: Some("minecraft:bowl"),
    },
    ObservedFdFood {
        id: "farmersdelight:pumpkin_pie_slice",
        effect: "minecraft:speed",
        duration: 600,
        eat_seconds: 0.8,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: None,
    },
    ObservedFdFood {
        id: "farmersdelight:sweet_berry_cheesecake_slice",
        effect: "minecraft:speed",
        duration: 600,
        eat_seconds: 0.8,
        can_always_eat: false,
        probability: 1.0,
        visible: false,
        show_icon: false,
        use_remainder: None,
    },
];

fn observed_fd_food(id: &str, item: &Item, food: &Value) -> Option<&'static ObservedFdFood> {
    if item.class != "vectorwing.farmersdelight.common.item.ConsumableItem" {
        return None;
    }
    let observed = OBSERVED_FD_FOODS
        .iter()
        .find(|observed| observed.id == id)?;
    let effects = food["effects"].as_array()?;
    let effect = effects.first()?;
    (effects.len() == 1
        && food["eat_seconds"].as_f64() == Some(observed.eat_seconds)
        && food["can_always_eat"].as_bool() == Some(observed.can_always_eat)
        && effect["id"].as_str() == Some(observed.effect)
        && effect["duration"].as_i64() == Some(observed.duration)
        && effect["amplifier"].as_i64() == Some(0)
        && effect["probability"].as_f64() == Some(observed.probability)
        && effect["ambient"].as_bool() == Some(false)
        && effect["visible"].as_bool() == Some(observed.visible)
        && effect["show_icon"].as_bool() == Some(observed.show_icon))
    .then_some(observed)
}

/// A state key with its properties sorted, so keys compare across blocks.
/// Which states of a block drop the block itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelfDrop {
    /// Every state: `dropSelf`.
    Always,
    /// Only the states where this property has this value.
    When(String, String),
}

/// A state key's `property=value` pairs.
fn pairs(key: &str) -> BTreeSet<(String, String)> {
    key.split(',')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
}

pub fn state_key(key: &str) -> String {
    let mut parts: Vec<&str> = key.split(',').filter(|p| !p.is_empty()).collect();
    parts.sort_unstable();
    parts.join(",")
}

impl Block {
    /// The first vanilla class in the chain.
    pub fn vanilla_class(&self) -> &str {
        self.class
            .iter()
            .find(|c| c.starts_with(VANILLA_PACKAGE))
            .map_or("", String::as_str)
    }
    /// Its own class is a mod class: behaviour code the template may not have.
    pub fn custom_class(&self) -> bool {
        self.class
            .first()
            .is_some_and(|c| !c.starts_with(VANILLA_PACKAGE))
    }
    /// Every state drops one `item` (the block's own) with every tool, except
    /// where the state needs a tool and gets none: vanilla's `dropSelf`. Or
    /// only the states where one property has one value do, and the others
    /// drop nothing with any tool: a door's lower half (`createDoorTable`).
    /// Mods often register these tables in code, so their jar has no file.
    pub fn drops_itself(&self, item: &str) -> Option<SelfDrop> {
        let one = BTreeMap::from([(item.to_owned(), 1)]);
        let (mut dropping, mut nothing) = (Vec::new(), Vec::new());
        for (key, state) in &self.states {
            if state.drops.is_empty() {
                continue;
            }
            let mut drops = false;
            let mut short = false;
            for got in state.drops.values() {
                if *got == one {
                    drops = true;
                } else if got.is_empty() {
                    short = true;
                } else {
                    return None;
                }
            }
            match (drops, short) {
                (true, true) if !state.requires_tool => return None,
                (true, _) => dropping.push(pairs(key)),
                (false, _) => nothing.push(pairs(key)),
            }
        }
        let (first, rest) = dropping.split_first()?;
        if nothing.is_empty() {
            return Some(SelfDrop::Always);
        }
        first
            .iter()
            .filter(|pair| rest.iter().all(|s| s.contains(*pair)))
            .find(|pair| nothing.iter().all(|s| !s.contains(*pair)))
            .map(|(p, v)| SelfDrop::When(p.clone(), v.clone()))
    }
    fn default_state(&self) -> Option<&State> {
        self.states
            .get(&self.default)
            .or_else(|| self.states.values().next())
    }
    fn same_states(&self, other: &Block) -> bool {
        self.properties.len() == other.properties.len()
            && self.properties.iter().all(|(k, v)| {
                other.properties.get(k).is_some_and(|w| {
                    v.iter().collect::<BTreeSet<_>>() == w.iter().collect::<BTreeSet<_>>()
                })
            })
    }
    fn by_key(&self) -> BTreeMap<String, &State> {
        self.states.iter().map(|(k, s)| (state_key(k), s)).collect()
    }
}

/// The template carry-over gives a mod block.
#[derive(Debug, Clone, PartialEq)]
pub struct Choice {
    pub template: String,
    /// `data/<mod>/horizon/block/<name>.json`'s `properties`.
    pub properties: Map<String, Value>,
    /// What the template does not reproduce, for the report.
    pub differences: Vec<String>,
}

impl Facts {
    pub fn read(path: &Path) -> Result<Facts, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The vanilla block that best stands in for `id`: the same vanilla
    /// class and states, then the most states with the same outline and
    /// collision, then the same sounds, set type, wood type and hardness.
    /// `None` when no vanilla block has its class and states.
    pub fn template(&self, id: &str, allowed: &dyn Fn(&str) -> bool) -> Option<Choice> {
        let block = self.blocks.get(id)?;
        let class = block.vanilla_class();
        let own = block.by_key();
        let ours = block.default_state()?;
        let mut best: Option<(i64, &str, &Block)> = None;
        for (name, v) in &self.blocks {
            if !name.starts_with("minecraft:")
                || v.vanilla_class() != class
                || v.custom_class()
                || !block.same_states(v)
                || !allowed(name)
            {
                continue;
            }
            let theirs = v.by_key();
            let shapes = own
                .iter()
                .filter(|(k, s)| {
                    theirs
                        .get(*k)
                        .is_some_and(|t| t.outline == s.outline && t.collision == s.collision)
                })
                .count() as i64;
            let Some(td) = v.default_state() else {
                continue;
            };
            let same_set = match (&block.block_set_type, &v.block_set_type) {
                (Some(a), Some(b)) => {
                    (a.name == b.name) as i64 * 2
                        + (a.can_open_by_hand == b.can_open_by_hand) as i64 * 8
                }
                _ => 0,
            };
            let score = shapes * 1000
                + (td.sound == ours.sound) as i64 * 100
                + same_set * 10
                + (block.wood_type.is_some() && block.wood_type == v.wood_type) as i64 * 20
                + (td.hardness == ours.hardness) as i64 * 5
                + (td.map_color_id == ours.map_color_id) as i64;
            if best.is_none_or(|(s, n, _)| score > s || (score == s && name.as_str() < n)) {
                best = Some((score, name, v));
            }
        }
        let (_, template, t) = best?;
        Some(self.choice(block, template, t, allowed))
    }

    fn choice(
        &self,
        block: &Block,
        template: &str,
        t: &Block,
        allowed: &dyn Fn(&str) -> bool,
    ) -> Choice {
        let (ours, theirs) = (block.default_state(), t.default_state());
        let mut properties = Map::new();
        let mut differences = Vec::new();
        if let Some(s) = ours {
            properties.insert("hardness".into(), json!(s.hardness));
            properties.insert(
                "explosion_resistance".into(),
                json!(block.explosion_resistance),
            );
            properties.insert("light".into(), json!(s.emission));
            properties.insert("requires_tool".into(), json!(s.requires_tool));
            properties.insert("friction".into(), json!(block.friction));
            properties.insert("map_color".into(), json!(s.map_color_id));
            if theirs.is_none_or(|t| t.sound != s.sound) {
                match self.sound_block(&s.sound, allowed) {
                    Some(b) => {
                        properties.insert("sounds".into(), json!(b));
                    }
                    None => differences.push("sounds: no vanilla block has them".into()),
                }
            }
        }
        if (block.speed_factor, block.jump_factor) != (t.speed_factor, t.jump_factor) {
            differences.push(format!(
                "speed and jump factors {} {} (template {} {})",
                block.speed_factor, block.jump_factor, t.speed_factor, t.jump_factor
            ));
        }
        let theirs = t.by_key();
        let mut shapes = 0;
        let mut varying = BTreeSet::new();
        let first = ours.map(|s| (s.hardness, s.emission, s.requires_tool, s.map_color_id));
        for (k, s) in block.by_key() {
            let Some(v) = theirs.get(&k) else {
                continue;
            };
            if v.outline != s.outline || v.collision != s.collision {
                shapes += 1;
            }
            if Some((s.hardness, s.emission, s.requires_tool, s.map_color_id)) != first {
                varying.insert("per-state hardness, light or map colour");
            }
        }
        if shapes > 0 {
            differences.push(format!(
                "shapes differ from {template} in {shapes} of {} states",
                block.states.len()
            ));
        }
        differences.extend(varying.into_iter().map(str::to_owned));
        if block.custom_class() {
            differences.push(format!(
                "class {} (behaviour beyond {})",
                block.class[0],
                block.vanilla_class()
            ));
        }
        Choice {
            template: template.to_owned(),
            properties,
            differences,
        }
    }

    /// A vanilla block whose default state has these sounds.
    fn sound_block(&self, sound: &Value, allowed: &dyn Fn(&str) -> bool) -> Option<&str> {
        self.blocks
            .iter()
            .filter(|(n, _)| n.starts_with("minecraft:") && allowed(n))
            .find(|(_, b)| b.default_state().is_some_and(|s| &s.sound == sound))
            .map(|(n, _)| n.as_str())
    }

    /// An item declaration for a mod item that is not a declared block's:
    /// a vanilla template item with its stack size and rarity, and what it
    /// does not reproduce.
    pub fn item(&self, id: &str) -> Result<(Value, Vec<String>), String> {
        let item = self.items.get(id).ok_or("not in the oracle")?;
        if item.block.is_some() {
            return Err("a block item without a block".into());
        }
        let mut differences = Vec::new();
        let observed_fd_consumption = item
            .food
            .as_ref()
            .and_then(|food| observed_fd_food(id, item, food));
        let observed_fd_special = item
            .food
            .as_ref()
            .and_then(|food| observed_fd_special_food(id, item, food));
        let observed_fd_supported =
            observed_fd_consumption.is_some() || observed_fd_special.is_some();
        if !item.class.starts_with(VANILLA_PACKAGE) && !observed_fd_supported {
            differences.push(format!("class {} (behaviour beyond Item)", item.class));
        }
        let mut decl = json!({"template": "minecraft:stick"});
        if let Some(food) = &item.food {
            // A subclass may change consumption beyond FOOD; do not guess it.
            if item.class != "net.minecraft.world.item.Item" && !observed_fd_supported {
                return Err("food with custom item behavior".into());
            }
            let effects = food["effects"]
                .as_array()
                .ok_or("food effects were not exported")?;
            let mut supported_effects = Vec::with_capacity(effects.len());
            if effects.len() > 64 {
                return Err("too many food effects".into());
            }
            for effect in effects {
                let effect_id = effect["id"].as_str().ok_or("missing food effect id")?;
                let vanilla_name = effect_id.strip_prefix("minecraft:");
                let item_namespace = id.split_once(':').map(|(namespace, _)| namespace);
                let farmer_nourishment = effect_id == "farmersdelight:nourishment"
                    && item_namespace == Some("farmersdelight")
                    && observed_fd_supported;
                if !farmer_nourishment && vanilla_name.is_none() {
                    return Err("custom food effect is not supported".into());
                }
                // Matches the effect ids available to the game's brewing::effect lookup.
                if let Some(name) = vanilla_name
                    && !matches!(
                        name,
                        "speed"
                            | "slowness"
                            | "haste"
                            | "mining_fatigue"
                            | "strength"
                            | "instant_health"
                            | "instant_damage"
                            | "jump_boost"
                            | "nausea"
                            | "regeneration"
                            | "resistance"
                            | "fire_resistance"
                            | "water_breathing"
                            | "invisibility"
                            | "blindness"
                            | "night_vision"
                            | "hunger"
                            | "weakness"
                            | "poison"
                            | "wither"
                            | "health_boost"
                            | "absorption"
                            | "saturation"
                            | "glowing"
                            | "levitation"
                            | "luck"
                            | "unluck"
                            | "slow_falling"
                            | "conduit_power"
                            | "dolphins_grace"
                            | "bad_omen"
                            | "hero_of_the_village"
                            | "darkness"
                            | "trial_omen"
                            | "raid_omen"
                            | "wind_charged"
                            | "weaving"
                            | "oozing"
                            | "infested"
                            | "breath_of_the_nautilus"
                    )
                {
                    return Err("unknown vanilla food effect".into());
                }
                let show_icon = effect["show_icon"]
                    .as_bool()
                    .ok_or("missing food effect icon flag")?;
                let duration = effect["duration"]
                    .as_i64()
                    .filter(|&n| (-1..=i32::MAX as i64).contains(&n))
                    .ok_or("invalid food effect duration")?;
                let amplifier = effect["amplifier"]
                    .as_i64()
                    .filter(|&n| (0..=255).contains(&n))
                    .ok_or("invalid food effect amplifier")?;
                let probability = effect["probability"]
                    .as_f64()
                    .filter(|&n| n.is_finite() && (0.0..=1.0).contains(&n))
                    .ok_or("invalid food effect probability")?;
                let ambient = effect["ambient"]
                    .as_bool()
                    .ok_or("missing food effect ambient flag")?;
                let visible = effect["visible"]
                    .as_bool()
                    .ok_or("missing food effect visibility flag")?;
                supported_effects.push(json!({"id":effect_id,"duration":duration,"amplifier":amplifier,"probability":probability,"ambient":ambient,"visible":visible,"show_icon":show_icon}));
            }
            let nutrition = food["nutrition"]
                .as_u64()
                .filter(|&v| v <= i32::MAX as u64)
                .ok_or("invalid food nutrition")?;
            let saturation = food["saturation"]
                .as_f64()
                .filter(|v| v.is_finite() && *v >= 0.0 && *v <= f32::MAX as f64)
                .ok_or("invalid food saturation")?;
            let duration = food["eat_seconds"]
                .as_f64()
                .filter(|v| v.is_finite() && *v > 0.0 && *v <= f32::MAX as f64)
                .ok_or("invalid food consumption duration")?;
            let always = food["can_always_eat"]
                .as_bool()
                .ok_or("missing food can_always_eat")?;
            decl["template"] = json!("minecraft:apple");
            let nutrition = observed_fd_special
                .as_ref()
                .map_or(nutrition, |food| food.nutrition);
            let saturation = observed_fd_special
                .as_ref()
                .map_or(saturation, |food| food.saturation);
            decl["food"] = json!({"nutrition":nutrition, "saturation":saturation,
                "can_always_eat":always, "consume_seconds":duration, "effects":supported_effects});
            if let Some(remainder) = food.get("use_remainder") {
                let remainder = remainder.as_str().ok_or("invalid food use remainder")?;
                if food["use_remainder_count"].as_u64() != Some(1) {
                    return Err("food use remainder count is missing or unsupported".into());
                }
                decl["use_remainder"] = json!(remainder);
            } else if let Some(remainder) =
                observed_fd_consumption.and_then(|food| food.use_remainder)
            {
                // The food component omits the result for these subclasses;
                // record the observed one-item finish result.
                decl["use_remainder"] = json!(remainder);
            } else if let Some(remainder) = observed_fd_special
                .as_ref()
                .and_then(|food| food.use_remainder)
            {
                decl["use_remainder"] = json!(remainder);
            }
        }
        if let Some(remainder) = &item.crafting_remainder {
            decl["crafting_remainder"] = json!(remainder);
        }
        if item.max_stack_size != 64 {
            decl["max_stack_size"] = json!(item.max_stack_size);
        }
        if item.rarity != "common" {
            decl["rarity"] = json!(item.rarity);
        }
        Ok((decl, differences))
    }

    /// A tag's members the content mod may name: vanilla's and its own.
    pub fn tag_members(&self, kind: &str, tag: &str, own: &str) -> Option<Vec<String>> {
        let map = if kind == "block" {
            &self.tags.block
        } else {
            &self.tags.item
        };
        let members = map.get(tag)?;
        Some(
            members
                .iter()
                .filter(|m| {
                    m.split_once(':')
                        .is_some_and(|(ns, _)| ns == "minecraft" || ns == own)
                })
                .cloned()
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(hardness: f32, sound: &str, outline: Value) -> Value {
        json!({"hardness": hardness, "emission": 0, "requires_tool": false, "map_color_id": 13,
            "sound": {"break": sound}, "outline": outline, "collision": outline})
    }
    fn block(class: &[&str], props: Value, states: Value) -> Value {
        let default = states
            .as_object()
            .unwrap()
            .keys()
            .next()
            .unwrap()
            .to_owned();
        json!({"class": class, "properties": props, "default": default, "item": "x",
            "explosion_resistance": 3.0, "friction": 0.6, "speed_factor": 1.0,
            "jump_factor": 1.0, "states": states})
    }

    #[test]
    fn ordinary_food_uses_observed_components_and_refuses_incomplete_data() {
        let mut facts: Facts = serde_json::from_value(json!({"blocks":{}, "items":{"test:food":{
            "class":"net.minecraft.world.item.Item", "max_stack_size":16, "rarity":"common",
            "crafting_remainder":"minecraft:bowl", "food":{"nutrition":7,"saturation":3.5,
            "can_always_eat":true,"eat_seconds":0.8,"effects":[],"use_remainder":"minecraft:bowl","use_remainder_count":1}
        }}}))
        .unwrap();
        let (item, _) = facts.item("test:food").unwrap();
        assert_eq!(item["template"], "minecraft:apple");
        assert_eq!(
            item["food"],
            json!({"nutrition":7,"saturation":3.5,"can_always_eat":true,"consume_seconds":0.8,"effects":[]})
        );
        assert_eq!(item["use_remainder"], "minecraft:bowl");
        assert_eq!(item["crafting_remainder"], "minecraft:bowl");
        let food = facts
            .items
            .get_mut("test:food")
            .unwrap()
            .food
            .as_mut()
            .unwrap();
        food["effects"] = json!([{ "id":"minecraft:hunger", "duration":600, "amplifier":0,
            "probability":0.3, "ambient":false, "visible":true, "show_icon":true }]);
        let (food_item, _) = facts.item("test:food").unwrap();
        assert_eq!(food_item["food"]["effects"][0]["id"], "minecraft:hunger");
        let food = facts
            .items
            .get_mut("test:food")
            .unwrap()
            .food
            .as_mut()
            .unwrap();
        food["effects"][0]["show_icon"] = json!(false);
        let (food_item, _) = facts.item("test:food").unwrap();
        assert_eq!(food_item["food"]["effects"][0]["show_icon"], false);
        let food = facts
            .items
            .get_mut("test:food")
            .unwrap()
            .food
            .as_mut()
            .unwrap();
        food["effects"] = json!([{ "id":"minecraft:speed" }]);
        assert!(facts.item("test:food").is_err());
        facts
            .items
            .get_mut("test:food")
            .unwrap()
            .food
            .as_mut()
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove("effects");
        assert!(facts.item("test:food").is_err());
    }

    #[test]
    fn farmersdelight_nourishment_is_preserved_as_a_namespaced_effect() {
        let mut facts: Facts = serde_json::from_value(json!({"blocks":{}, "items":{"farmersdelight:cooked_rice":{
            "class":"vectorwing.farmersdelight.common.item.ConsumableItem", "max_stack_size":16, "rarity":"common",
            "crafting_remainder":"minecraft:bowl",
            "food":{"nutrition":6,"saturation":4.8,"can_always_eat":false,"eat_seconds":1.6,
            "effects":[{"id":"farmersdelight:nourishment","duration":600,"amplifier":0,
            "probability":1.0,"ambient":false,"visible":false,"show_icon":false}]}
        }}})).unwrap();
        let (food, differences) = facts.item("farmersdelight:cooked_rice").unwrap();
        assert!(differences.is_empty());
        assert_eq!(food["use_remainder"], "minecraft:bowl");
        assert_eq!(food["crafting_remainder"], "minecraft:bowl");
        assert_eq!(
            food["food"]["effects"],
            json!([{"id":"farmersdelight:nourishment","duration":600,"amplifier":0,
                "probability":1.0,"ambient":false,"visible":false,"show_icon":false}])
        );
        facts
            .items
            .get_mut("farmersdelight:cooked_rice")
            .unwrap()
            .food
            .as_mut()
            .unwrap()["effects"][0]["id"] = json!("othermod:nourishment");
        facts
            .items
            .get_mut("farmersdelight:cooked_rice")
            .unwrap()
            .class = "net.minecraft.world.item.Item".into();
        assert_eq!(
            facts.item("farmersdelight:cooked_rice").unwrap_err(),
            "custom food effect is not supported"
        );
    }

    #[test]
    fn nourishment_import_is_limited_to_black_box_consumable_group() {
        let facts: Facts = serde_json::from_value(json!({"blocks":{}, "items":{
            "farmersdelight:bone_broth": {
                "class":"vectorwing.farmersdelight.common.item.DrinkableItem", "max_stack_size":16, "rarity":"common",
                "food":{"nutrition":6,"saturation":3.6,"can_always_eat":false,"eat_seconds":1.6,
                "effects":[{"id":"farmersdelight:nourishment","duration":1200,"amplifier":0,
                "probability":1.0,"ambient":false,"visible":false,"show_icon":false}]}
            },
            "farmersdelight:unknown_food": {
                "class":"vectorwing.farmersdelight.common.item.ConsumableItem", "max_stack_size":16, "rarity":"common",
                "food":{"nutrition":6,"saturation":3.6,"can_always_eat":false,"eat_seconds":1.6,
                "effects":[{"id":"farmersdelight:nourishment","duration":1200,"amplifier":0,
                "probability":1.0,"ambient":false,"visible":false,"show_icon":false}]}
            }
        }})).unwrap();
        assert_eq!(
            facts.item("farmersdelight:bone_broth").unwrap_err(),
            "food with custom item behavior"
        );
        assert_eq!(
            facts.item("farmersdelight:unknown_food").unwrap_err(),
            "food with custom item behavior"
        );
    }

    #[test]
    fn vanilla_effect_import_keeps_observed_flags_and_use_remainders() {
        let facts: Facts = serde_json::from_value(json!({"blocks":{}, "items":{
            "farmersdelight:apple_pie_slice": {
                "class":"vectorwing.farmersdelight.common.item.ConsumableItem", "max_stack_size":16, "rarity":"common",
                "food":{"nutrition":3,"saturation":1.8,"can_always_eat":false,"eat_seconds":0.8,
                "effects":[{"id":"minecraft:speed","duration":600,"amplifier":0,
                "probability":1.0,"ambient":false,"visible":false,"show_icon":false}]}
            },
            "farmersdelight:glow_berry_custard": {
                "class":"vectorwing.farmersdelight.common.item.ConsumableItem", "max_stack_size":16, "rarity":"common",
                "food":{"nutrition":6,"saturation":3.6,"can_always_eat":true,"eat_seconds":1.6,
                "effects":[{"id":"minecraft:glowing","duration":100,"amplifier":0,
                "probability":1.0,"ambient":false,"visible":true,"show_icon":true}]}
            },
            "farmersdelight:nether_salad": {
                "class":"vectorwing.farmersdelight.common.item.ConsumableItem", "max_stack_size":16, "rarity":"common",
                "food":{"nutrition":6,"saturation":3.6,"can_always_eat":false,"eat_seconds":1.6,
                "effects":[{"id":"minecraft:nausea","duration":240,"amplifier":0,
                "probability":0.3,"ambient":false,"visible":true,"show_icon":true}]}
            }
        }})).unwrap();
        let (slice, differences) = facts.item("farmersdelight:apple_pie_slice").unwrap();
        assert!(differences.is_empty());
        assert!(slice.get("use_remainder").is_none());
        assert_eq!(slice["food"]["effects"][0]["show_icon"], false);
        let (custard, _) = facts.item("farmersdelight:glow_berry_custard").unwrap();
        assert_eq!(custard["use_remainder"], "minecraft:glass_bottle");
        assert_eq!(custard["food"]["effects"][0]["visible"], true);
        let (salad, _) = facts.item("farmersdelight:nether_salad").unwrap();
        assert_eq!(salad["use_remainder"], "minecraft:bowl");
        assert_eq!(salad["food"]["effects"][0]["probability"], 0.3);
    }

    #[test]
    fn remaining_farmer_food_import_uses_consumed_values_and_exact_ids() {
        let facts: Facts = serde_json::from_value(json!({"blocks":{}, "items":{
            "farmersdelight:apple_cider": {
                "class":"vectorwing.farmersdelight.common.item.DrinkableItem", "max_stack_size":16, "rarity":"common",
                "crafting_remainder":"minecraft:glass_bottle",
                "food":{"nutrition":0,"saturation":0.0,"can_always_eat":true,"eat_seconds":1.6,
                "effects":[{"id":"minecraft:absorption","duration":1200,"amplifier":0,"probability":1.0,"ambient":false,"visible":true,"show_icon":true}]}
            },
            "farmersdelight:bone_broth": {
                "class":"vectorwing.farmersdelight.common.item.DrinkableItem", "max_stack_size":16, "rarity":"common",
                "crafting_remainder":"minecraft:bowl",
                "food":{"nutrition":8,"saturation":11.2,"can_always_eat":false,"eat_seconds":1.6,
                "effects":[{"id":"farmersdelight:nourishment","duration":1200,"amplifier":0,"probability":1.0,"ambient":false,"visible":false,"show_icon":false}]}
            },
            "farmersdelight:dog_food": {
                "class":"vectorwing.farmersdelight.common.item.DogFoodItem", "max_stack_size":16, "rarity":"common",
                "crafting_remainder":"minecraft:bowl",
                "food":{"nutrition":4,"saturation":1.6,"can_always_eat":false,"eat_seconds":1.6,"effects":[]}
            },
            "farmersdelight:melon_popsicle": {
                "class":"vectorwing.farmersdelight.common.item.PopsicleItem", "max_stack_size":64, "rarity":"common",
                "food":{"nutrition":3,"saturation":1.2,"can_always_eat":true,"eat_seconds":0.8,"effects":[]}
            },
            "farmersdelight:tomato_sauce": {
                "class":"vectorwing.farmersdelight.common.item.ConsumableItem", "max_stack_size":64, "rarity":"common",
                "crafting_remainder":"minecraft:bowl",
                "food":{"nutrition":4,"saturation":3.2,"can_always_eat":false,"eat_seconds":1.6,"effects":[]}
            }
        }})).unwrap();
        for (id, nutrition, saturation, remainder) in [
            (
                "farmersdelight:apple_cider",
                4,
                0.0,
                Some("minecraft:glass_bottle"),
            ),
            (
                "farmersdelight:bone_broth",
                12,
                11.2,
                Some("minecraft:bowl"),
            ),
            ("farmersdelight:dog_food", 4, 1.6, Some("minecraft:bowl")),
            ("farmersdelight:melon_popsicle", 7, 1.2, None),
            (
                "farmersdelight:tomato_sauce",
                8,
                3.2,
                Some("minecraft:bowl"),
            ),
        ] {
            let (decl, differences) = facts.item(id).unwrap();
            assert!(differences.is_empty(), "{id}: {differences:?}");
            assert_eq!(decl["food"]["nutrition"], nutrition, "{id}");
            assert_eq!(decl["food"]["saturation"], saturation, "{id}");
            assert_eq!(
                decl.get("use_remainder").and_then(Value::as_str),
                remainder,
                "{id}"
            );
        }
        assert_eq!(
            facts.item("farmersdelight:unknown_food").unwrap_err(),
            "not in the oracle"
        );
    }

    #[test]
    fn templates_follow_class_shapes_and_sounds() {
        let cube = json!([[0, 0, 0, 1, 1, 1]]);
        let low = json!([[0, 0, 0, 1, 0.9375, 1]]);
        let block_class = ["net.minecraft.world.level.block.Block"];
        let path = ["net.minecraft.world.level.block.DirtPathBlock"];
        let facts: Facts = serde_json::from_value(json!({
            "blocks": {
                "minecraft:stone": block(&block_class, json!({}), json!({"": state(1.5, "stone", cube.clone())})),
                "minecraft:oak_planks": block(&block_class, json!({}), json!({"": state(2.0, "wood", cube.clone())})),
                "minecraft:dirt_path": block(&path, json!({}), json!({"": state(0.65, "grass", low.clone())})),
                "m:wood_tile": block(&["m.Tile", block_class[0]], json!({}), json!({"": state(2.5, "wood", cube.clone())})),
                "m:path": block(&["m.Path", block_class[0]], json!({}), json!({"": state(1.0, "stone", low.clone())})),
                "m:gadget": block(&["m.Gadget", block_class[0]], json!({"mode": ["a", "b"]}),
                    json!({"mode=a": state(1.0, "stone", cube.clone()), "mode=b": state(1.0, "stone", cube)})),
            },
            "items": {}
        }))
        .unwrap();
        let any = |_: &str| true;
        let tile = facts.template("m:wood_tile", &any).unwrap();
        assert_eq!(tile.template, "minecraft:oak_planks");
        assert_eq!(tile.properties["hardness"], json!(2.5));
        assert!(!tile.properties.contains_key("sounds"));
        assert_eq!(
            tile.differences,
            ["class m.Tile (behaviour beyond net.minecraft.world.level.block.Block)"]
        );
        // A plain Block is never templated on a DirtPathBlock: shapes differ, and say so.
        let path = facts.template("m:path", &any).unwrap();
        assert_eq!(path.template, "minecraft:stone");
        assert!(
            path.differences[0].starts_with("shapes differ"),
            "{:?}",
            path.differences
        );
        assert_eq!(facts.template("m:gadget", &any), None);
        let tile = facts
            .template("m:wood_tile", &|n: &str| n != "minecraft:oak_planks")
            .unwrap();
        assert_eq!(tile.template, "minecraft:stone");
        assert_eq!(tile.differences[0], "sounds: no vanilla block has them");
    }

    #[test]
    fn drops_itself_is_vanillas_drop_self() {
        let cube = json!([[0, 0, 0, 1, 1, 1]]);
        let with = |requires_tool: bool, drops: Value| {
            let mut s = state(1.5, "stone", cube.clone());
            s["requires_tool"] = json!(requires_tool);
            s["drops"] = drops;
            let b: Block = serde_json::from_value(block(&[], json!({}), json!({"": s}))).unwrap();
            b.drops_itself("m:tile")
        };
        let me = json!({"m:tile": 1});
        let always = Some(SelfDrop::Always);
        assert_eq!(with(false, json!({"hand": me, "shears": me})), always);
        // A state that needs a tool drops nothing by hand, as dropSelf does.
        assert_eq!(
            with(true, json!({"hand": {}, "diamond_pickaxe": me})),
            always
        );
        assert_eq!(
            with(false, json!({"hand": {}, "diamond_pickaxe": me})),
            None
        );
        // Two of itself (a double slab), or something else, is not dropSelf.
        assert_eq!(with(false, json!({"hand": {"m:tile": 2}})), None);
        assert_eq!(with(false, json!({"hand": {"m:other": 1}})), None);
        assert_eq!(with(false, json!({})), None);
    }

    #[test]
    fn a_door_drops_from_its_lower_half() {
        let cube = json!([[0, 0, 0, 1, 1, 1]]);
        let me = json!({"hand": {"m:door": 1}, "shears": {"m:door": 1}});
        let none = json!({"hand": {}, "shears": {}});
        let door = |upper: &Value| {
            let mut states = serde_json::Map::new();
            for (key, drops) in [
                ("half=lower,open=false", &me),
                ("half=lower,open=true", &me),
                ("half=upper,open=false", upper),
                ("half=upper,open=true", upper),
            ] {
                let mut s = state(3.0, "wood", cube.clone());
                s["drops"] = drops.clone();
                states.insert(key.to_owned(), s);
            }
            let props = json!({"half": ["upper", "lower"], "open": ["true", "false"]});
            let b: Block =
                serde_json::from_value(block(&[], props, Value::Object(states))).unwrap();
            b.drops_itself("m:door")
        };
        assert_eq!(
            door(&none),
            Some(SelfDrop::When("half".to_owned(), "lower".to_owned()))
        );
        assert_eq!(door(&me), Some(SelfDrop::Always));
        // Something else from the upper half: not a door table.
        assert_eq!(door(&json!({"hand": {"m:other": 1}})), None);
    }

    #[test]
    fn state_keys_sort() {
        assert_eq!(
            state_key("half=lower,facing=north"),
            "facing=north,half=lower"
        );
        assert_eq!(state_key(""), "");
    }
}

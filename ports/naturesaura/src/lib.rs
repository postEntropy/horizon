//! Nature's Aura: aura made by the world around you — trees, flowers, animals,
//! fireworks — stored in caches and spent by the mod's devices.
//!
//! The Natural Altar and the Offering Table keep their slots; the caches and
//! troves show how much aura they hold.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEntityType, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod,
    Registration, chat,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

const ALTAR_SCREEN: &str = "screen:naturesaura:altar";
const TABLE_SCREEN: &str = "screen:naturesaura:table";
const AURA_SCREEN: &str = "screen:naturesaura:aura";
const CRATE_SCREEN: &str = "screen:naturesaura:crate";
const GUIDE_SCREEN: &str = "screen:naturesaura:guide";

const AURA_CAPACITY: u64 = 1_000_000;

const GENERATORS: [(&str, &str); 9] = [
    ("animal_generator", "animals breathing nearby"),
    ("chorus_generator", "chorus flowers growing"),
    ("firework_generator", "fireworks set off"),
    ("flower_generator", "flowers blooming around it"),
    ("moss_generator", "moss spreading over stone"),
    ("oak_generator", "an oak tree growing beside it"),
    ("potion_generator", "potions being drunk"),
    ("projectile_generator", "arrows and projectiles flying"),
    ("slime_split_generator", "slimes splitting"),
];

const TOKENS: [(&str, &str); 8] = [
    ("anger", "Anger"),
    ("euphoria", "Euphoria"),
    ("fear", "Fear"),
    ("grief", "Grief"),
    ("joy", "Joy"),
    ("rage", "Rage"),
    ("sorrow", "Sorrow"),
    ("terror", "Terror"),
];

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

fn grid_screen(screen: &str, title: &str, slots: u32) -> ScreenSpec {
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
        id: screen.into(),
        title: title.into(),
        width: 176,
        height: 18 * 2 + rows * 18,
        widgets,
    }
}

fn gauge_screen(title: &str) -> ScreenSpec {
    ScreenSpec {
        id: AURA_SCREEN.into(),
        title: title.into(),
        width: 176,
        height: 110,
        widgets: vec![
            Widget {
                id: 0,
                kind: WidgetKind::EnergyBar,
                x: 8,
                y: 18,
                width: 16,
                height: 54,
                text: "Aura".into(),
                value: 0,
                max_value: AURA_CAPACITY,
            },
            label(1, 18, format!("Capacity: {AURA_CAPACITY} aura")),
            label(2, 32, "Aura flows in from the world around it.".into()),
        ],
    }
}

fn guide_screen() -> ScreenSpec {
    let mut widgets = vec![label(0, 18, "Aura comes from the world".into())];
    for (index, (_, source)) in GENERATORS.iter().enumerate() {
        widgets.push(label(
            1 + index as u32,
            36 + index as i32 * 11,
            format!("• {source}"),
        ));
    }
    widgets.push(label(
        1 + GENERATORS.len() as u32,
        36 + GENERATORS.len() as i32 * 11 + 6,
        "Bottle it, or store it in an aura cache or trove.".into(),
    ));
    ScreenSpec {
        id: GUIDE_SCREEN.into(),
        title: "Aura and You".into(),
        width: 176,
        height: 60 + GENERATORS.len() as u32 * 11,
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

struct NaturesAura;

impl Mod for NaturesAura {
    fn init() -> Result<(Self, Registration), String> {
        Ok((NaturesAura, Registration::new().command("aura")))
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
            "nature_altar",
            "offering_table",
            "auto_crafter",
            "ender_crate",
            "aura_cache",
            "aura_trove",
            "aura_detector",
            "aura_timer",
            "furnace_heater",
            "rf_converter",
            "chunk_loader",
            "item_distributor",
            "light",
            "time_changer",
            "weather_changer",
            "placer",
            "powder_placer",
            "field_creator",
            "snow_creator",
            "spring",
            "spawn_lamp",
            "pickup_stopper",
            "hopper_upgrade",
            "grated_chute",
            "lower_limiter",
            "generator_limit_remover",
            "conversion_catalyst",
            "crushing_catalyst",
            "blast_furnace_booster",
            "animal_container",
            "animal_spawner",
            "wood_stand",
            "dimension_rail_overworld",
            "dimension_rail_nether",
            "dimension_rail_end",
            "aura_bloom",
            "aura_cactus",
            "aura_mushroom",
            "crimson_aura_mushroom",
            "warped_aura_mushroom",
            "ancient_log",
            "ancient_leaves",
            "decayed_leaves",
            "golden_leaves",
            "infused_stone",
            "infused_iron_block",
            "depth_ingot_block",
            "sky_ingot_block",
            "tainted_gold_block",
            "end_flower",
        ] {
            hooks = hooks.block(&format!("naturesaura:{block}"));
        }
        for (generator, _) in GENERATORS {
            hooks = hooks.block(&format!("naturesaura:{generator}"));
        }
        for (token, _) in TOKENS {
            hooks = hooks.item(&format!("naturesaura:token_{token}"));
        }
        hooks
            .item("naturesaura:aura_bottle")
            .item("naturesaura:vacuum_bottle")
            .item("naturesaura:book")
            .item("naturesaura:aura_cache")
            .item("naturesaura:aura_trove")
            .item("naturesaura:bottle_two_the_rebottling")
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);
        match path {
            "nature_altar" => {
                let _ = ui::open_screen(&grid_screen(ALTAR_SCREEN, "Natural Altar", 9));
                return EventOutcome::Consume;
            }
            "offering_table" => {
                let _ = ui::open_screen(&grid_screen(TABLE_SCREEN, "Offering Table", 3));
                return EventOutcome::Consume;
            }
            "auto_crafter" => {
                let _ = ui::open_screen(&grid_screen(ALTAR_SCREEN, "Auto Crafter", 9));
                return EventOutcome::Consume;
            }
            "ender_crate" => {
                let _ = ui::open_screen(&grid_screen(CRATE_SCREEN, "Ender Crate", 27));
                return EventOutcome::Consume;
            }
            "aura_cache" | "aura_trove" => {
                let _ = ui::open_screen(&gauge_screen(&title(path)));
                return EventOutcome::Consume;
            }
            _ => {}
        }

        if let Some((_, source)) = GENERATORS.iter().find(|(generator, _)| *generator == path) {
            chat::show(&format!("✨ Aura generator: makes aura from {source}."));
            return EventOutcome::Consume;
        }
        {
            let text = match path {
                "aura_detector" => "🔍 Aura detector: reads the aura of the chunk it stands in.",
                "aura_timer" => "⏱️ Aura timer: pulses a redstone signal on an aura budget.",
                "furnace_heater" => "🔥 Furnace heater: heats a furnace without fuel, using aura.",
                "rf_converter" => "⚡ RF converter: turns aura into Forge Energy.",
                "chunk_loader" => "📦 Chunk loader: keeps a chunk loaded, paid in aura.",
                "item_distributor" => {
                    "📤 Item distributor: spreads items over the inventories around it."
                }
                "light" => "💡 Aura light: a lamp that runs on aura.",
                "time_changer" => "🌗 Time changer: skips to day or night, for a lot of aura.",
                "weather_changer" => {
                    "🌧️ Weather changer: clears or brings rain, for a lot of aura."
                }
                "placer" => "🌱 Placer: plants what the world grows.",
                "powder_placer" => "🌱 Powder placer: plants aura flowers on a timer.",
                "field_creator" => "🌾 Field creator: tills the ground around it.",
                "snow_creator" => "❄️ Snow creator: lays snow, using aura.",
                "spring" => "💧 Spring: sources water where it stands.",
                "spawn_lamp" => "🔆 Spawn lamp: lights an area so mobs cannot spawn.",
                "pickup_stopper" => "✋ Pickup stopper: keeps items from being picked up.",
                "hopper_upgrade" => "⬆️ Hopper upgrade: makes the hopper below move items faster.",
                "grated_chute" => "📦 Grated chute: moves items down through a block.",
                "lower_limiter" => "🔒 Lower limiter: caps the aura of the area.",
                "generator_limit_remover" => {
                    "🔓 Generator limit remover: lets generators stack aura further."
                }
                "conversion_catalyst" => {
                    "🔄 Conversion catalyst: turns one aura flower into another."
                }
                "crushing_catalyst" => "💥 Crushing catalyst: crushes blocks into their drops.",
                "blast_furnace_booster" => {
                    "🔥 Blast furnace booster: speeds a blast furnace up with aura."
                }
                "animal_container" => "🐄 Animal container: picks up and carries an animal.",
                "animal_spawner" => "🐄 Animal spawner: brings the carried animal back.",
                "wood_stand" => "🪵 Wood stand: holds a log for the ritual around it.",
                "dimension_rail_overworld" | "dimension_rail_nether" | "dimension_rail_end" => {
                    "🚃 Dimension rail: carries a minecart between dimensions."
                }
                "aura_bloom"
                | "aura_cactus"
                | "aura_mushroom"
                | "crimson_aura_mushroom"
                | "warped_aura_mushroom" => {
                    "🌸 An aura flower: it makes aura while it grows, and it can be crushed for its powder."
                }
                "ancient_log" | "ancient_leaves" => {
                    "🌳 Ancient wood: dark wood that grows on aura."
                }
                "decayed_leaves" => "🍂 Decayed leaves: what is left when the aura runs out.",
                "golden_leaves" => "🍂 Golden leaves: a tree touched by aura.",
                "infused_stone" | "infused_iron_block" => {
                    "🧱 Infused stone and iron, made at the altar."
                }
                "depth_ingot_block" | "sky_ingot_block" | "tainted_gold_block" => {
                    "🧱 A block of an aura metal."
                }
                "end_flower" => "🌺 End flower: blooms where the end's aura gathers.",
                _ => return EventOutcome::Pass,
            };
            chat::show(text);
            EventOutcome::Consume
        }
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseInAir && event.action != ItemAction::UseOnBlock {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.item);
        if path == "book" {
            let _ = ui::open_screen(&guide_screen());
            return EventOutcome::Consume;
        }
        if let Some((_, name)) = TOKENS.iter().find(|(token, _)| *token == path) {
            chat::show(&format!(
                "🏅 Token of {name}: a feeling the world gave up, brewed into aura."
            ));
            return EventOutcome::Consume;
        }
        let message = match path {
            "aura_bottle" => "🍶 Aura bottle: holds 1,000 aura, collected from the world.",
            "vacuum_bottle" => "🍶 Vacuum bottle: sucks the aura out of the air around it.",
            "bottle_two_the_rebottling" => {
                "🍶 Bottle of rebottling: turns a filled aura bottle back into aura."
            }
            "aura_cache" => "🗄️ Aura cache: stores aura for the devices beside it.",
            "aura_trove" => "🗄️ Aura trove: a cache that passes aura on to its neighbours.",
            _ => return EventOutcome::Pass,
        };
        chat::show(message);
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "aura" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Nature's Aura: {} generators; aura stored in caches and troves up to {AURA_CAPACITY}.",
            GENERATORS.len()
        )))
    }
}

horizon_mod_sdk::export_mod!(NaturesAura);

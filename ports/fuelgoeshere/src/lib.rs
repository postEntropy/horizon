//! Fuel Goes Here: which items burn, and for how long.
//!
//! The mod's whole content is a forced-fuels tag; the port reports burn values
//! for that tag and for the vanilla fuels a furnace accepts, so a pack author
//! can check an item before adding it.
use horizon_mod_sdk::{Mod, Registration, inventory};

const FUELS: &[(&str, u32)] = &[
    ("minecraft:coal", 1600),
    ("minecraft:charcoal", 1600),
    ("minecraft:coal_block", 16000),
    ("minecraft:lava_bucket", 20000),
    ("minecraft:blaze_rod", 2400),
    ("minecraft:stick", 100),
    ("minecraft:bowl", 200),
    ("minecraft:crafting_table", 300),
    ("minecraft:bookshelf", 300),
    ("minecraft:chest", 300),
    ("minecraft:trapped_chest", 300),
    ("minecraft:barrel", 300),
    ("minecraft:ladder", 300),
    ("minecraft:note_block", 300),
    ("minecraft:jukebox", 300),
    ("minecraft:beehive", 300),
    ("minecraft:bee_nest", 300),
    ("minecraft:dead_bush", 100),
    ("minecraft:scaffolding", 400),
    ("minecraft:dried_kelp_block", 4001),
];

const WOODS: [&str; 11] = [
    "oak",
    "spruce",
    "birch",
    "jungle",
    "acacia",
    "dark_oak",
    "mangrove",
    "cherry",
    "bamboo",
    "crimson",
    "warped",
];

fn is_wood(path: &str) -> bool {
    WOODS
        .iter()
        .any(|wood| path.starts_with(wood) && path.as_bytes().get(wood.len()) == Some(&b'_'))
        || path == "bamboo_mosaic"
}

/// Ticks `id` burns for, as a furnace counts them (200 ticks per item smelted).
fn burn_time(id: &str) -> Option<u32> {
    if let Some((_, ticks)) = FUELS.iter().find(|(fuel, _)| *fuel == id) {
        return Some(*ticks);
    }
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    if namespace != "minecraft" {
        return None;
    }

    if is_wood(path) {
        if path.ends_with("_slab") {
            return Some(150);
        }
        if path.ends_with("_boat") {
            return Some(1200);
        }
        if path.ends_with("_sapling") {
            return Some(100);
        }
        if path.ends_with("_sign") || path.ends_with("_hanging_sign") {
            return Some(200);
        }
        return Some(300);
    }
    None
}

fn describe(id: &str) -> String {
    match burn_time(id) {
        Some(ticks) => format!(
            "🔥 {id} burns for {ticks} ticks ({:.1} items smelted).",
            ticks as f32 / 200.0
        ),
        None => format!("🧊 {id} is not a fuel."),
    }
}

struct FuelGoesHere;

impl Mod for FuelGoesHere {
    fn init() -> Result<(Self, Registration), String> {
        Ok((FuelGoesHere, Registration::new().command("fuel")))
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "fuel" {
            return Err(format!("Unknown command /{name}"));
        }
        let arg = args.trim();
        if arg == "list" {
            let fuels: Vec<String> = FUELS
                .iter()
                .map(|(id, ticks)| format!("{id} ({ticks})"))
                .collect();
            return Ok(Some(format!(
                "Forced fuels and vanilla fuels: {}",
                fuels.join(", ")
            )));
        }
        if arg.is_empty() {
            let Some(stack) = inventory::get_main_hand() else {
                return Ok(Some(
                    "Hold an item, or use /fuel <item> (for example /fuel coal).".into(),
                ));
            };
            return Ok(Some(describe(&stack.id)));
        }
        let id = if arg.contains(':') {
            arg.to_string()
        } else {
            format!("minecraft:{arg}")
        };
        Ok(Some(describe(&id)))
    }
}

horizon_mod_sdk::export_mod!(FuelGoesHere);

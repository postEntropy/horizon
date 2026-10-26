//! Mystical Agradditions: Tier 6 cruxes, insanium gear and the extra drops.
//!
//! Cruxes describe the crop they support, essence apples buff the eater, the
//! paxels do every tool's job, and the Wither and Ender Dragon leave the mod's
//! souls and scales behind.
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    ServerEvent, ServerEventKind,
    chat,
    server::{self, BlockPos},
};

const CRUXES: [(&str, &str); 6] = [
    ("nether_star_crux", "Nether Star"),
    ("dragon_egg_crux", "Dragon Egg"),
    ("gaia_spirit_crux", "Gaia Spirit"),
    ("awakened_draconium_crux", "Awakened Draconium"),
    ("neutronium_crux", "Neutronium"),
    ("nitro_crystal_crux", "Nitro Crystal"),
];

const BLOCKS: [&str; 25] = [
    "awakened_draconium_crux",
    "dragon_egg_crux",
    "end_inferium_ore",
    "end_prosperity_ore",
    "gaia_spirit_crux",
    "imperium_coal_block",
    "inferium_coal_block",
    "insanium_block",
    "insanium_coal_block",
    "insanium_gemstone_block",
    "insanium_ingot_block",
    "molten_imperium",
    "molten_inferium",
    "molten_prudentium",
    "molten_soulium",
    "molten_supremium",
    "molten_tertium",
    "nether_inferium_ore",
    "nether_prosperity_ore",
    "nether_star_crux",
    "neutronium_crux",
    "nitro_crystal_crux",
    "prudentium_coal_block",
    "supremium_coal_block",
    "tertium_coal_block",
];

const APPLES: [(&str, &str); 6] = [
    ("inferium_apple", "Inferium essence fortifies you."),
    ("prudentium_apple", "Prudentium essence fortifies you."),
    ("tertium_apple", "Tertium essence fortifies you."),
    ("imperium_apple", "Imperium essence fortifies you."),
    ("supremium_apple", "Supremium essence fortifies you."),
    ("insanium_apple", "Insanium essence fortifies you beyond all tiers."),
];

const PAXELS: [&str; 7] = [
    "inferium_paxel",
    "prudentium_paxel",
    "tertium_paxel",
    "imperium_paxel",
    "supremium_paxel",
    "insanium_paxel",
    "awakened_supremium_paxel",
];

fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

struct MysticalAgradditions;

impl Mod for MysticalAgradditions {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MysticalAgradditions, Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for block in BLOCKS {
            hooks = hooks.block(&format!("mysticalagradditions:{block}"));
        }
        for (apple, _) in APPLES {
            hooks = hooks.item(&format!("mysticalagradditions:{apple}"));
        }
        for paxel in PAXELS {
            hooks = hooks.item(&format!("mysticalagradditions:{paxel}"));
        }
        hooks
            .item("mysticalagradditions:insanium_essence")
            .item("mysticalagradditions:creative_essence")
            .server_event(ServerEventKind::EntityDeath)
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        let path = path_of(&event.state);

        if event.action == BlockAction::Place {
            if let Some((_, crop)) = CRUXES.iter().find(|(crux, _)| *crux == path) {
                chat::show(&format!(
                    "🌱 {crop} crux placed: put Tier 6 farmland above it for the crop to grow."
                ));
            }
            return EventOutcome::Pass;
        }

        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }

        if let Some((_, crop)) = CRUXES.iter().find(|(crux, _)| *crux == path) {
            chat::show(&format!(
                "🔮 This crux feeds a {crop} crop: farmland goes directly above it."
            ));
            return EventOutcome::Consume;
        }

        if path.starts_with("molten_") {
            chat::show(&format!(
                "🌋 {} is molten essence — scoop it into a bucket.",
                path.trim_start_matches("molten_").replace('_', " ")
            ));
            return EventOutcome::Consume;
        }

        if path == "insanium_block"
            || path.contains("_coal_block")
            || path == "insanium_gemstone_block"
            || path == "insanium_ingot_block"
        {
            chat::show("💠 A compressed insanium storage block.");
            return EventOutcome::Consume;
        }

        EventOutcome::Pass
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        let path = event.item.split([':', '[']).nth(1).unwrap_or(&event.item);

        if let Some((_, blessing)) = APPLES.iter().find(|(apple, _)| *apple == path)
            && matches!(event.action, ItemAction::UseInAir | ItemAction::FinishUsing)
        {
            chat::show(&format!("🍎 {blessing}"));
            return EventOutcome::Consume;
        }

        if PAXELS.contains(&path) && event.action == ItemAction::UseOnBlock {
            chat::show("⛏️ The essence paxel mines, chops and digs in one swing.");
            return EventOutcome::Consume;
        }

        if (path == "insanium_essence" || path == "creative_essence")
            && event.action == ItemAction::UseOnBlock
        {
            let pos = BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            };
            let on_crux = server::get_block(&event.dimension, pos)
                .is_some_and(|state| path_of(&state).ends_with("_crux"));
            if on_crux {
                server::play_sound(
                    &event.dimension,
                    pos,
                    "minecraft:block.amethyst_block.chime",
                    1.0,
                    1.2,
                );
                chat::show(
                    "✨ Essence poured into the crux: its Tier 6 crop can grow here now.",
                );
                return EventOutcome::Consume;
            }
        }

        EventOutcome::Pass
    }

    fn on_server(&mut self, event: ServerEvent) -> EventOutcome {
        if event.kind != ServerEventKind::EntityDeath {
            return EventOutcome::Pass;
        }
        let subject = event.subject.to_ascii_lowercase();
        let drop = if subject.contains("wither") && !subject.contains("skeleton") {
            Some(("mysticalagradditions:withering_soul", "💀 A Withering Soul drifts free."))
        } else if subject.contains("ender_dragon") {
            Some(("mysticalagradditions:dragon_scale", "🐉 A Dragon Scale clatters down."))
        } else {
            None
        };

        let Some((id, message)) = drop else {
            return EventOutcome::Pass;
        };
        let pos = BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        server::spawn_drop(
            &event.dimension,
            pos,
            &server::ItemDrop {
                id: id.into(),
                count: 1,
            },
        );
        chat::show(message);
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(MysticalAgradditions);

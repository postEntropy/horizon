use horizon_mod_sdk::{
    server::{self, BlockPos},
    storage, vanilla_hooks, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    VanillaHookEvent, VanillaHookKind,
};

pub struct JagsMod {
    tick: u64,
    event_nonce: u64,
    seeds_planted: u64,
}

impl Mod for JagsMod {
    fn init() -> Result<(Self, Registration), String> {
        let seeds_planted = storage::get("seeds_planted")
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(0);
        let _ = vanilla_hooks::subscribe("loot-drop");
        Ok((
            Self {
                tick: 0,
                event_nonce: 0,
                seeds_planted,
            },
            Registration::new().command("jags").on_tick(),
        ))
    }

    fn tick(&mut self, tick: u64) {
        self.tick = tick;
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new().item("jags:jags")
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action != ItemAction::UseOnBlock || block_id(&event.target) != "minecraft:dirt" {
            return EventOutcome::Pass;
        }

        let pos = BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        };
        if server::set_block(&event.dimension, pos, "minecraft:grass_block").is_err() {
            return EventOutcome::Pass;
        }

        self.seeds_planted = self.seeds_planted.saturating_add(1);
        let _ = storage::set("seeds_planted", self.seeds_planted.to_string().as_bytes());
        EventOutcome::Consume
    }

    fn on_vanilla_hook(&mut self, event: VanillaHookEvent) -> EventOutcome {
        if event.kind != VanillaHookKind::LootDrop
            || !matches!(
                block_id(&event.subject),
                "minecraft:grass" | "minecraft:short_grass"
            )
        {
            return EventOutcome::Pass;
        }

        // The carried Forge loot modifier adds one JAGS seed with a 10% chance
        // to grass-block loot. The current hook API has no access to the
        // world's loot RNG, so use a position/tick/event stream rather than
        // making every grass drop deterministic for a given block position.
        self.event_nonce = self.event_nonce.wrapping_add(1);
        let position = (event.x as u32 as u64)
            ^ (event.y as u32 as u64).rotate_left(21)
            ^ (event.z as u32 as u64).rotate_left(42);
        let roll = mix64(position ^ self.tick.rotate_left(13) ^ self.event_nonce);
        if roll % 10 == 0 {
            vanilla_hooks::add_loot_drop("jags:jags", 1);
        }
        EventOutcome::Pass
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        match args.split_whitespace().next().unwrap_or("help") {
            "stats" => Ok(Some(format!(
                "JAGS: {} grass seeds used",
                self.seeds_planted
            ))),
            _ => Err("Usage: /jags stats".into()),
        }
    }
}

fn block_id(state: &str) -> &str {
    state.split('[').next().unwrap_or(state)
}

fn mix64(mut value: u64) -> u64 {
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}

horizon_mod_sdk::export_mod!(JagsMod);

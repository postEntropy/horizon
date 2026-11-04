use horizon_mod_sdk::{
    server::{self, BlockPos},
    storage, vanilla_hooks, EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration,
    VanillaHookEvent, VanillaHookKind,
};

pub struct JagsMod {
    seeds_planted: u64,
    grass_loot_checks: u64,
    grass_seeds_dropped: u64,
    grass_loot_samples: u64,
    last_grass_roll: f32,
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
                seeds_planted,
                grass_loot_checks: 0,
                grass_seeds_dropped: 0,
                grass_loot_samples: 0,
                last_grass_roll: -1.0,
            },
            Registration::new().command("jags"),
        ))
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
        // The host validates and applies the block replacement before consuming
        // the seed, as one authoritative operation.
        if !server::replace_block_and_consume_active_item(
            &event.dimension,
            pos,
            "minecraft:dirt",
            "minecraft:grass_block",
        ) {
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

        self.grass_loot_checks = self.grass_loot_checks.saturating_add(1);
        if (0.0..1.0).contains(&event.value) {
            self.grass_loot_samples = self.grass_loot_samples.saturating_add(1);
            self.last_grass_roll = event.value;
        }
        if (0.0..0.1).contains(&event.value) {
            vanilla_hooks::add_loot_drop("jags:jags", 1);
            self.grass_seeds_dropped = self.grass_seeds_dropped.saturating_add(1);
        }
        EventOutcome::Pass
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        match args.split_whitespace().next().unwrap_or("help") {
            "stats" => Ok(Some(format!(
                "JAGS: {} grass seeds used; {} grass loot checks; {} RNG samples (last {}); {} JAGS seeds added",
                self.seeds_planted,
                self.grass_loot_checks,
                self.grass_loot_samples,
                self.last_grass_roll,
                self.grass_seeds_dropped
            ))),
            _ => Err("Usage: /jags stats".into()),
        }
    }
}

fn block_id(state: &str) -> &str {
    state.split('[').next().unwrap_or(state)
}

horizon_mod_sdk::export_mod!(JagsMod);

//! `ComposterBlock`: compostable items go in by hand (`useItemOn`) or
//! through its input container (hoppers, droppers, crafters above), and the
//! fill feedback of level event 1500 (`handleFill`). Emptying a full
//! composter by hand is `useWithoutItem` in horizon-world.
use super::{Game, commands::compute::ComputeContext};
use horizon_client::{
    items::ItemStack,
    particles::{ParticleOptions, ParticleType},
};
use horizon_core::{position::BlockPos, random::RandomSource};

fn compostable_chance_layers(chance: f64, random: &mut impl RandomSource) -> i32 {
    let roll = f64::from(random.next_float());
    i32::from(chance.is_finite() && (0.0..=1.0).contains(&chance) && roll < chance)
}

impl Game<'_> {
    /// The composter's `level` at `pos`, None when it isn't a composter.
    pub(super) fn composter_level(&self, pos: [i32; 3]) -> Option<i32> {
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let s = self.pack.assets.registry.state(state)?;
        if &*s.name != "minecraft:composter" {
            return None;
        }
        s.properties.get("level")?.parse().ok()
    }
    /// The item's vanilla layers provider or a NeoForge composting chance.
    fn compostable(&self, stack: &ItemStack) -> Option<serde_json::Value> {
        self.res.items().get(&stack.item)?.compostable.clone()
    }
    /// `ComposterBlock.addLayer` and level event 1500: vanilla evaluates its
    /// layers provider, while NeoForge's data map rolls once against chance.
    /// True when the composter's state changed.
    fn composter_add(&mut self, pos: [i32; 3], component: &serde_json::Value) -> bool {
        let layers = if let Some(chance) = component.get("chance").and_then(|v| v.as_f64()) {
            compostable_chance_layers(chance, &mut self.world.random)
        } else {
            let context = ComputeContext {
                block: Some(pos),
                target: None,
            };
            let layers = component.get("layers").unwrap_or(component);
            self.provider_int(layers, &context, 0).unwrap_or(0)
        };
        let before = self.world.block(pos[0], pos[1], pos[2]);
        let at = BlockPos::new(pos[0], pos[1], pos[2]);
        let after = self.with_level(|b, level, ticks| b.composter_insert(level, ticks, at, layers));
        self.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
        // The game event and level event 1500 happen here, before the caller goes on.
        let _ = self.process_level_events();
        after != before
    }
    /// `ComposterBlock.useItemOn` with the main-hand stack: true when it
    /// consumed the action (SUCCESS; a full composter falls through to
    /// `useWithoutItem`, which empties it).
    pub(super) fn use_composter(&mut self, pos: [i32; 3], held: Option<&ItemStack>) -> bool {
        let Some(level) = self.composter_level(pos) else {
            return false;
        };
        let Some(stack) = held else {
            return false;
        };
        let Some(layers) = self.compostable(stack) else {
            return false;
        };
        if level >= 8 {
            return false;
        }
        if level < 7 {
            self.composter_add(pos, &layers);
            self.stats.add("used", &stack.item, 1);
            self.consume_held();
        }
        // ServerPlayerGameMode.useItemOn: the action was consumed.
        self.trigger_block("minecraft:item_used_on_block", pos, stack);
        true
    }
    /// `HopperBlockEntity.addItem` into the composter's `InputContainer`
    /// (levels 0 to 6; one item, compostable, from above): the remainder.
    pub(super) fn composter_container_insert(
        &mut self,
        pos: [i32; 3],
        mut stack: ItemStack,
        from_above: bool,
    ) -> Option<ItemStack> {
        let level = self.composter_level(pos)?;
        if !from_above || level >= 7 {
            return Some(stack);
        }
        let Some(layers) = self.compostable(&stack) else {
            return Some(stack);
        };
        // InputContainer.getMaxStackSize is 1: one item goes in, and its
        // setChanged composts it at once.
        stack.count -= 1;
        self.composter_add(pos, &layers);
        (stack.count > 0).then_some(stack)
    }
    /// `ComposterBlock.handleFill` (level event 1500) on the client: the
    /// fill sound and ten composter particles above the compost.
    pub(super) fn composter_fill_event(&mut self, pos: BlockPos, success: bool) {
        let center = [
            f64::from(pos.x) + 0.5,
            f64::from(pos.y) + 0.5,
            f64::from(pos.z) + 0.5,
        ];
        let sound = if success {
            "minecraft:block.composter.fill_success"
        } else {
            "minecraft:block.composter.fill"
        };
        self.audio.play(sound, Some(center), 1.0, 1.0);
        // getShape(..).max(Y, 0.5, 0.5): the compost surface (SHAPES[8] = SHAPES[7]).
        let level = self.composter_level([pos.x, pos.y, pos.z]).unwrap_or(0);
        let height = f64::from((1 + level.min(7) * 2).clamp(2, 16)) / 16.0 + 0.03125;
        let options = ParticleOptions::simple(ParticleType::Composter);
        for _ in 0..10 {
            let r = self.particles.random();
            let v = [
                r.next_gaussian() * 0.02,
                r.next_gaussian() * 0.02,
                r.next_gaussian() * 0.02,
            ];
            let p = [
                f64::from(pos.x) + 0.1875 + 0.625 * f64::from(r.next_float()),
                f64::from(pos.y) + height + f64::from(r.next_float()) * (1.0 - height),
                f64::from(pos.z) + 0.1875 + 0.625 * f64::from(r.next_float()),
            ];
            self.particles.add(&options, p, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::compostable_chance_layers;
    use horizon_core::random::{LegacyRandom, RandomSource};

    #[test]
    fn compostable_chance_consumes_one_vanilla_float_and_sets_zero_or_one_layer() {
        let mut failure = LegacyRandom::new(0);
        assert_eq!(compostable_chance_layers(0.0, &mut failure), 0);
        let after_failure = failure.next_float();
        let mut expected = LegacyRandom::new(0);
        expected.next_float();
        assert_eq!(after_failure, expected.next_float());

        let mut success = LegacyRandom::new(0);
        assert_eq!(compostable_chance_layers(1.0, &mut success), 1);
    }

    #[test]
    fn invalid_compostable_chance_adds_no_layer() {
        assert_eq!(compostable_chance_layers(1.1, &mut LegacyRandom::new(0)), 0);
        assert_eq!(
            compostable_chance_layers(f64::NAN, &mut LegacyRandom::new(0)),
            0
        );
    }
}

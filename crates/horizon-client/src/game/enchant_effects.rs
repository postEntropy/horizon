//! Enchantment effects on gameplay, read from the data-driven effect
//! components: efficiency and aqua affinity (`Player.getDestroySpeed`),
//! unbreaking (`minecraft:item_damage`), sharpness/smite/bane and power
//! (`minecraft:damage`), knockback, fire aspect, infinity (`minecraft:ammo_use`),
//! thorns (`minecraft:post_attack`), mending (`minecraft:repair_with_xp`) and
//! the armor attribute effects (respiration, depth strider, ...).
use super::Game;
use horizon_client::{enchant::EffectContext, items::ItemStack};
use horizon_core::random::RandomSource;

/// Where Mending found a damaged item (`EnchantedItemInUse.inSlot`).
#[derive(Clone, Copy)]
enum MendSlot {
    Main,
    Off,
    /// An `armor` index (head first).
    Armor(usize),
}

impl Game<'_> {
    /// `Player.getDestroySpeed` after the tool speed: mining efficiency, haste
    /// and mining fatigue.
    pub(super) fn modify_destroy_speed(&mut self, mut speed: f32) -> f32 {
        if speed > 1.0 {
            let held = self.inventory[self.selected]
                .clone()
                .map(|s| self.effective_stack(&s, &["minecraft:efficiency"]));
            speed += self
                .res
                .stations()
                .enchantments
                .attribute_add(held.as_ref(), "minecraft:mining_efficiency")
                as f32;
        }
        speed * self.potions.effects.mining_speed_factor()
    }
    /// `Attributes.SUBMERGED_MINING_SPEED` (0.2, raised by aqua affinity).
    pub(super) fn submerged_mining_speed(&mut self) -> f32 {
        let mut total = 0.0;
        let armor = self.armor.clone();
        for stack in armor.iter().flatten() {
            let stack = self.effective_stack(stack, &["minecraft:aqua_affinity"]);
            for (attr, _, amount, op) in self.res.stations().enchantments.attributes(&stack) {
                if attr == "minecraft:submerged_mining_speed" && op == "add_multiplied_total" {
                    total += amount;
                }
            }
        }
        (0.2 * (1.0 + total)) as f32
    }
    /// `EnchantmentHelper.processDurabilityChange` (unbreaking) for `amount` points.
    pub(super) fn durability_loss(&mut self, stack: &ItemStack, amount: u32) -> u32 {
        let stack = self.effective_stack(stack, &["minecraft:unbreaking"]);
        let e = &self.res.stations().enchantments;
        if !e.has_component(&stack, "minecraft:item_damage") {
            return amount;
        }
        let ctx = EffectContext {
            tool: Some(&stack.item),
            items: Some(self.res.items()),
            ..Default::default()
        };
        e.modify_value(
            Some(&stack),
            "minecraft:item_damage",
            &ctx,
            &mut self.world.random,
            amount as f32,
        )
        .max(0.0) as u32
    }
    /// `getEnchantedDamage - base`: damage bonus of the held weapon against a mob type.
    pub(super) fn enchant_damage_bonus(&mut self, target: &str) -> f32 {
        let Some(held) = self.inventory[self.selected].clone() else {
            return 0.0;
        };
        let held = self.effective_stack(
            &held,
            &[
                "minecraft:sharpness",
                "minecraft:smite",
                "minecraft:bane_of_arthropods",
                "minecraft:impaling",
                "minecraft:density",
                "minecraft:breach",
            ],
        );
        let ctx = EffectContext {
            this: Some(target),
            attacker: Some("minecraft:player"),
            direct_attacker: Some("minecraft:player"),
            is_direct: true,
            damage_type: Some("minecraft:player_attack"),
            items: Some(self.res.items()),
            ..Default::default()
        };
        self.res.stations().enchantments.modify_value(
            Some(&held),
            "minecraft:damage",
            &ctx,
            &mut self.world.random,
            0.0,
        )
    }
    /// Strength / weakness on the attack damage attribute.
    pub(super) fn effect_attack_bonus(&self) -> f32 {
        let fx = &self.potions.effects;
        fx.amplifier("strength")
            .map_or(0.0, |a| 3.0 * (a + 1) as f32)
            - fx.amplifier("weakness")
                .map_or(0.0, |a| 4.0 * (a + 1) as f32)
    }
    /// `Player.attack`'s extra knockback: `getKnockback` (the weapon's knockback
    /// enchantment, halved) plus 0.5 for a sprinting full-strength hit, and
    /// whether the hit is that knockback attack.
    pub(super) fn extra_knockback(&mut self, target_kind: &str, scale: f32) -> (f32, bool) {
        let knockback_attack = scale > 0.9 && self.player.as_ref().is_some_and(|p| p.sprinting);
        let enchanted = match self.inventory[self.selected].clone() {
            Some(held) => {
                let held = self.effective_stack(&held, &["minecraft:knockback"]);
                let ctx = EffectContext {
                    this: Some(target_kind),
                    attacker: Some("minecraft:player"),
                    direct_attacker: Some("minecraft:player"),
                    is_direct: true,
                    items: Some(self.res.items()),
                    ..Default::default()
                };
                self.res.stations().enchantments.modify_value(
                    Some(&held),
                    "minecraft:knockback",
                    &ctx,
                    &mut self.world.random,
                    0.0,
                )
            }
            None => 0.0,
        };
        let bonus = if knockback_attack { 0.5 } else { 0.0 };
        (enchanted / 2.0 + bonus, knockback_attack)
    }
    /// `causeExtraKnockback`'s direction, `(sin yRot, -cos yRot)` from Mth's
    /// sine table: the target flies where the attacker looks, not away from
    /// it (straight along an axis when the attacker faces one).
    pub(super) fn extra_knockback_direction(&self) -> (f64, f64) {
        use horizon_core::geometry::trig;
        let yaw = f64::from(self.yaw * trig::DEG_TO_RAD);
        (f64::from(trig::sin(yaw)), -f64::from(trig::cos(yaw)))
    }
    /// The attacker's side of `causeExtraKnockback` as the attacking client
    /// sees it: the server's player stops sprinting, and its flag reaches the
    /// client. (The server's 0.6 speed cut never does: the client's own
    /// `Player.attack` simulates no hurt, so it skips the knockback.)
    pub(super) fn stop_sprint_after_extra_knockback(&mut self) {
        if let Some(p) = self.player.as_mut() {
            let _ = p.set_sprinting(false);
        }
    }
    /// After a successful melee hit: fire aspect and wind burst.
    pub(super) fn post_attack(&mut self, mob: usize) {
        let Some(held) = self.inventory[self.selected].clone() else {
            return;
        };
        let held = self.effective_stack(&held, &["minecraft:fire_aspect", "minecraft:wind_burst"]);
        let Some(m) = self.mobs.get(mob) else { return };
        let kind = m.kind.id.clone();
        let ctx = EffectContext {
            this: Some(&kind),
            attacker: Some("minecraft:player"),
            direct_attacker: Some("minecraft:player"),
            is_direct: true,
            items: Some(self.res.items()),
            ..Default::default()
        };
        // minecraft:post_attack ignite effects of the attacker's weapon (fire aspect).
        let mut ignite = 0.0f32;
        for (id, level) in held.enchantments() {
            let Some(def) = self.res.stations().enchantments.get(id) else {
                continue;
            };
            for effect in def.effects["minecraft:post_attack"]
                .as_array()
                .into_iter()
                .flatten()
            {
                if effect["enchanted"] == "attacker"
                    && effect["affected"] == "victim"
                    && effect["effect"]["type"] == "minecraft:ignite"
                {
                    let mut random = self.world.random.clone();
                    let ok = effect.get("requirements").is_none_or(|r| {
                        self.res
                            .stations()
                            .enchantments
                            .condition(r, *level, &ctx, &mut random)
                    });
                    self.world.random = random;
                    if ok {
                        ignite = ignite.max(horizon_client::enchant::level_value(
                            &effect["effect"]["duration"],
                            *level,
                        ));
                    }
                }
            }
        }
        if ignite > 0.0
            && let Some(m) = self.mobs.get_mut(mob)
            && !m.kind.flags.fire_immune
        {
            m.ignite_for_ticks((ignite * 20.0) as i32);
        }
        self.post_attack_bursts(&held, &kind);
    }
    /// `minecraft:post_attack` explode effects on the attacker (wind burst):
    /// a burst at the player's feet that only knocks back and triggers
    /// blocks (`ExplodeEffect` with `block_interaction: trigger`).
    fn post_attack_bursts(&mut self, held: &ItemStack, victim: &str) {
        let Some(p) = self.player.as_ref() else {
            return;
        };
        // EntityFlagsPredicate.isFlying: gliding or flying.
        let motion = (p.fall_distance, p.fall_flying || p.flying);
        let feet = [p.position.x, p.position.y, p.position.z];
        let ctx = EffectContext {
            this: Some(victim),
            attacker: Some("minecraft:player"),
            direct_attacker: Some("minecraft:player"),
            is_direct: true,
            items: Some(self.res.items()),
            direct_motion: Some(motion),
            ..Default::default()
        };
        let mut bursts = Vec::new();
        for (id, level) in held.enchantments() {
            let Some(def) = self.res.stations().enchantments.get(id) else {
                continue;
            };
            for effect in def.effects["minecraft:post_attack"]
                .as_array()
                .into_iter()
                .flatten()
            {
                let e = &effect["effect"];
                if effect["enchanted"] != "attacker"
                    || effect["affected"] != "attacker"
                    || e["type"] != "minecraft:explode"
                    || e["block_interaction"] != "trigger"
                {
                    continue;
                }
                let ok = effect.get("requirements").is_none_or(|r| {
                    self.res.stations().enchantments.condition(
                        r,
                        *level,
                        &ctx,
                        &mut self.world.random,
                    )
                });
                if !ok {
                    continue;
                }
                let radius = horizon_client::enchant::level_value(&e["radius"], *level).max(0.0);
                // ExplosionDamageCalculator.getKnockbackMultiplier: 1 unless given.
                let knockback = if e["knockback_multiplier"].is_null() {
                    1.0
                } else {
                    horizon_client::enchant::level_value(&e["knockback_multiplier"], *level)
                };
                bursts.push((radius, knockback));
            }
        }
        for (radius, knockback) in bursts {
            let _ = self.wind_burst(feet, radius, knockback, false, false);
        }
    }
    /// Thorns on worn armor when a mob at `from` hits the player.
    pub(super) fn thorns(&mut self, from: [f64; 3]) {
        let attacker = self
            .mobs
            .iter()
            .enumerate()
            .filter(|(_, m)| m.alive())
            .min_by(|a, b| {
                let d = |m: &horizon_client::mobs::Mob| {
                    (m.position.x - from[0]).powi(2)
                        + (m.position.y - from[1]).powi(2)
                        + (m.position.z - from[2]).powi(2)
                };
                d(a.1).total_cmp(&d(b.1))
            })
            .map(|(i, _)| i);
        let Some(attacker) = attacker else { return };
        for slot in 0..4 {
            let Some(stack) = self.armor[slot].clone() else {
                continue;
            };
            let level = self.effective_enchantment_level(&stack, "minecraft:thorns");
            if level == 0 {
                continue;
            }
            let Some(def) = self.res.stations().enchantments.get("minecraft:thorns") else {
                continue;
            };
            let effect = def.effects["minecraft:post_attack"][0].clone();
            let ctx = EffectContext {
                items: Some(self.res.items()),
                ..Default::default()
            };
            let mut random = self.world.random.clone();
            let ok = self.res.stations().enchantments.condition(
                &effect["requirements"],
                level,
                &ctx,
                &mut random,
            );
            self.world.random = random;
            if !ok {
                continue;
            }
            // DamageEntity(min 1, max 5) and ChangeItemDamage(2).
            let damage = 1.0 + self.world.random.next_float() * 4.0;
            let center = {
                let m = &mut self.mobs[attacker];
                m.hurt(damage, None, &mut self.world.random);
                [m.position.x, m.position.y, m.position.z]
            };
            self.audio
                .play("minecraft:enchant.thorns.hit", Some(center), 1.0, 1.0);
            if let Some(s) = self.armor[slot].as_mut() {
                s.damage += 2;
                if self
                    .res
                    .items()
                    .get(&s.item)
                    .is_some_and(|i| s.damage >= i.max_damage)
                {
                    self.armor[slot] = None;
                }
            }
        }
    }
    /// `ExperienceOrb.repairPlayerItems`: returns the experience left over.
    pub(super) fn mending(&mut self, amount: u32) -> u32 {
        let mut amount = amount;
        loop {
            // EnchantmentHelper.getRandomItemWith(REPAIR_WITH_XP, damaged), in
            // EquipmentSlot order: the main hand, the off hand, then feet, legs,
            // chest and head (`armor` holds head first).
            let selected = self.selected;
            let dimension = self.dim.id();
            let position = self.mod_hook_position();
            let (inventory, offhand, armor, mods) =
                (&self.inventory, &self.offhand, &self.armor, &mut self.mods);
            let has_hook = mods.has_enchantment_level_hook();
            let mut candidates = Vec::new();
            let mut add_candidate = |slot, stack: &Option<ItemStack>| {
                if let Some(stack) = stack.as_ref()
                    && stack.damage > 0
                {
                    let base = stack.enchantment_level("minecraft:mending");
                    let level = if has_hook {
                        mods.effective_enchantment_level(
                            &stack.item,
                            "minecraft:mending",
                            base,
                            dimension,
                            position,
                        )
                    } else {
                        base
                    };
                    if level > 0 {
                        candidates.push(slot);
                    }
                }
            };
            add_candidate(MendSlot::Main, &inventory[selected]);
            add_candidate(MendSlot::Off, offhand);
            for i in (0..armor.len()).rev() {
                add_candidate(MendSlot::Armor(i), &armor[i]);
            }
            if candidates.is_empty() {
                return amount;
            }
            let slot = candidates[self
                .world
                .random
                .next_int_bound(candidates.len() as i32)
                .unwrap_or(0) as usize];
            let Some(stack) = self.mend_slot(slot).clone() else {
                return amount;
            };
            let effective_stack = self.effective_stack(&stack, &["minecraft:mending"]);
            let ctx = EffectContext {
                items: Some(self.res.items()),
                ..Default::default()
            };
            let to_repair_xp = self.res.stations().enchantments.modify_value(
                Some(&effective_stack),
                "minecraft:repair_with_xp",
                &ctx,
                &mut self.world.random,
                amount as f32,
            ) as u32;
            let repaired = to_repair_xp.min(stack.damage);
            if let Some(s) = self.mend_slot_mut(slot).as_mut() {
                s.damage -= repaired;
            }
            if repaired == 0 || to_repair_xp == 0 {
                return 0;
            }
            let remaining = amount.saturating_sub(repaired * amount / to_repair_xp);
            if remaining == 0 {
                return 0;
            }
            amount = remaining;
        }
    }
    fn mend_slot(&self, slot: MendSlot) -> &Option<ItemStack> {
        match slot {
            MendSlot::Main => &self.inventory[self.selected],
            MendSlot::Off => &self.offhand,
            MendSlot::Armor(i) => &self.armor[i],
        }
    }
    fn mend_slot_mut(&mut self, slot: MendSlot) -> &mut Option<ItemStack> {
        match slot {
            MendSlot::Main => &mut self.inventory[self.selected],
            MendSlot::Off => &mut self.offhand,
            MendSlot::Armor(i) => &mut self.armor[i],
        }
    }
    /// Bow enchantments at release: (extra base damage, infinity keeps the arrow).
    pub(super) fn bow_enchantments(&mut self) -> (f64, bool) {
        let Some(bow) = self.inventory[self.selected].clone() else {
            return (0.0, false);
        };
        let bow = self.effective_stack(&bow, &["minecraft:power", "minecraft:infinity"]);
        let e = &self.res.stations().enchantments;
        let ctx = EffectContext {
            direct_attacker: Some("minecraft:arrow"),
            attacker: Some("minecraft:player"),
            items: Some(self.res.items()),
            ..Default::default()
        };
        let power = e.modify_value(
            Some(&bow),
            "minecraft:damage",
            &ctx,
            &mut self.world.random,
            0.0,
        );
        let ammo_ctx = EffectContext {
            tool: Some("minecraft:arrow"),
            items: Some(self.res.items()),
            ..Default::default()
        };
        let infinity = e.has_component(&bow, "minecraft:ammo_use")
            && e.modify_value(
                Some(&bow),
                "minecraft:ammo_use",
                &ammo_ctx,
                &mut self.world.random,
                1.0,
            ) == 0.0;
        (f64::from(power), infinity)
    }
    /// `EnchantmentHelper.onProjectileSpawned` ignite effects of the held bow
    /// (flame): the seconds the new arrow burns for.
    pub(super) fn bow_projectile_ignite(&mut self) -> Option<f32> {
        let bow = self.inventory[self.selected].clone()?;
        let bow = self.effective_stack(&bow, &["minecraft:flame"]);
        let ctx = EffectContext {
            this: Some("minecraft:arrow"),
            items: Some(self.res.items()),
            ..Default::default()
        };
        let mut ignite: Option<f32> = None;
        for (id, level) in bow.enchantments() {
            let Some(def) = self.res.stations().enchantments.get(id) else {
                continue;
            };
            for effect in def.effects["minecraft:projectile_spawned"]
                .as_array()
                .into_iter()
                .flatten()
            {
                if effect["effect"]["type"] != "minecraft:ignite" {
                    continue;
                }
                let ok = effect.get("requirements").is_none_or(|r| {
                    self.res.stations().enchantments.condition(
                        r,
                        *level,
                        &ctx,
                        &mut self.world.random,
                    )
                });
                if ok {
                    let seconds =
                        horizon_client::enchant::level_value(&effect["effect"]["duration"], *level);
                    ignite = Some(ignite.map_or(seconds, |s: f32| s.max(seconds)));
                }
            }
        }
        ignite
    }
    /// Enchantment attribute modifiers of worn armor (respiration, depth strider...).
    /// Each piece's modifier id ends with its slot (`EnchantmentAttributeEffect.idForSlot`),
    /// so the same enchantment on two pieces stacks instead of replacing itself.
    pub(super) fn apply_enchantment_attributes(&mut self) {
        let mut wanted: Vec<(String, String, f64, String)> = Vec::new();
        let armor = self.armor.clone();
        for (stack, slot) in armor.iter().zip(["head", "chest", "legs", "feet"]) {
            let Some(stack) = stack else { continue };
            let stack = self.effective_stack(
                stack,
                &[
                    "minecraft:aqua_affinity",
                    "minecraft:respiration",
                    "minecraft:depth_strider",
                    "minecraft:swift_sneak",
                    "minecraft:efficiency",
                ],
            );
            for (attr, id, amount, op) in self.res.stations().enchantments.attributes(&stack) {
                wanted.push((attr, format!("{id}/{slot}"), amount, op));
            }
        }
        let Some(p) = self.player.as_mut() else {
            return;
        };
        for (attr, id, _, op) in std::mem::take(&mut self.potions.enchant_modifiers) {
            let _ = p.set_modifier(&attr, &id, 0.0, &op, false);
        }
        for (attr, id, amount, op) in &wanted {
            let _ = p.set_modifier(attr, id, *amount, op, true);
        }
        self.potions.enchant_modifiers = wanted;
    }
}

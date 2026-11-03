//! Elemental armour sets, bangles and foci, kept apart from the game calls so
//! the rules can be tested on the host.
//!
//! The mod's own lang files describe what the gear does: each school is two of
//! Ars Elemental's four elements (cinder is fire and air, lava is fire and
//! earth, vapour is fire and water, tempest is water and air, silt is earth and
//! air, mire is water and earth, elemancer is all four), an armour family bonus
//! needs two or four pieces, a school bonus needs four pieces of one school,
//! and the set absorbs damage of its schools. The descriptions are the source
//! of the rules here; the numbers are this port's tuning, since the jar's code
//! is off limits.

/// The four elemental schools of Ars Elemental.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Element {
    Fire,
    Water,
    Air,
    Earth,
}

impl Element {
    pub const ALL: [Element; 4] = [Element::Fire, Element::Water, Element::Air, Element::Earth];

    pub fn id(self) -> &'static str {
        match self {
            Element::Fire => "fire",
            Element::Water => "water",
            Element::Air => "air",
            Element::Earth => "earth",
        }
    }

    /// The element a vanilla damage type belongs to, if any. Ars spell damage
    /// has no analogue here, so only the vanilla sources that map cleanly onto
    /// an element count.
    pub fn of_damage(damage: &str) -> Option<Element> {
        let id = damage.trim_start_matches("minecraft:");
        Some(match id {
            "in_fire" | "on_fire" | "lava" | "hot_floor" | "campfire" | "unattended_campfire"
            | "fireball" | "fireworks" => Element::Fire,
            "drown" | "freeze" => Element::Water,
            "fall" | "fly_into_wall" | "wind_charge" | "sonic_boom" | "lightning_bolt" => {
                Element::Air
            }
            "cactus" | "sweet_berry_bush" | "stalagmite" | "falling_block" | "falling_stalactite" => {
                Element::Earth
            }
            _ => return None,
        })
    }
}

/// The seven schools the mod adds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum School {
    Cinder,
    Lava,
    Vapor,
    Tempest,
    Silt,
    Mire,
    Elemancer,
}

impl School {
    pub const ALL: [School; 7] = [
        School::Cinder,
        School::Lava,
        School::Vapor,
        School::Tempest,
        School::Silt,
        School::Mire,
        School::Elemancer,
    ];

    pub fn id(self) -> &'static str {
        match self {
            School::Cinder => "cinder",
            School::Lava => "lava",
            School::Vapor => "vapor",
            School::Tempest => "tempest",
            School::Silt => "silt",
            School::Mire => "mire",
            School::Elemancer => "elemancer",
        }
    }

    /// The school's name in the mod's lang (`ars_elemancy.armor_set.<school>`).
    pub fn set_name(self) -> &'static str {
        match self {
            School::Cinder => "Cindermancer's Set",
            School::Lava => "Lavamancer's Set",
            School::Vapor => "Vapormancer's Set",
            School::Tempest => "Tempestmancer's Set",
            School::Silt => "Siltmancer's Set",
            School::Mire => "Miremancer's Set",
            School::Elemancer => "Elemancer's Set",
        }
    }

    /// `ars_elemental.armor_set.<school>.desc`: the two elements (four for
    /// elemancer) whose damage the set absorbs.
    pub fn elements(self) -> &'static [Element] {
        match self {
            School::Cinder => &[Element::Fire, Element::Air],
            School::Lava => &[Element::Fire, Element::Earth],
            School::Vapor => &[Element::Fire, Element::Water],
            School::Tempest => &[Element::Water, Element::Air],
            School::Silt => &[Element::Earth, Element::Air],
            School::Mire => &[Element::Water, Element::Earth],
            School::Elemancer => &Element::ALL,
        }
    }

    pub fn covers(self, element: Element) -> bool {
        self.elements().contains(&element)
    }

    /// The school an item belongs to, from its path (`cinder_boots_heavy`).
    pub fn of_item(item: &str) -> Option<School> {
        let path = item.split_once(':').map_or(item, |(_, path)| path);
        School::ALL
            .into_iter()
            .find(|school| path == school.id() || path.starts_with(&format!("{}_", school.id())))
    }

    pub fn parse(text: &str) -> Option<School> {
        let text = text.trim().to_ascii_lowercase();
        School::ALL.into_iter().find(|school| school.id() == text)
    }
}

/// The three armour families: the medium `mancer` set, the heavy `guard` set
/// and the light `mancer` variant (`ars_elemental.armor_set.{medium,heavy,light}`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Family {
    Medium,
    Heavy,
    Light,
}

impl Family {
    pub const ALL: [Family; 3] = [Family::Medium, Family::Heavy, Family::Light];

    pub fn id(self) -> &'static str {
        match self {
            Family::Medium => "medium",
            Family::Heavy => "heavy",
            Family::Light => "light",
        }
    }

    /// `ars_elemental.armor_set.<family>.name`.
    pub fn bonus_name(self) -> &'static str {
        match self {
            Family::Medium => "Arcanist's Wisdom",
            Family::Heavy => "Battlemage's Resilience",
            Family::Light => "Sorcerer's Fury",
        }
    }

    /// The family from an armour path: `_helmet`/`_chestplate`/`_leggings_heavy`/
    /// `_boots_heavy` are heavy, `_hood`/`_tunic`/`_pants`/`_shoes` light, and
    /// `_hat`/`_robes`/`_leggings`/`_boots` the medium set.
    pub fn of_path(path: &str) -> Option<Family> {
        let heavy = ["_helmet", "_chestplate", "_leggings_heavy", "_boots_heavy"];
        let light = ["_hood", "_tunic", "_pants", "_shoes"];
        let medium = ["_hat", "_robes", "_leggings", "_boots"];
        if heavy.iter().any(|suffix| path.ends_with(suffix)) {
            Some(Family::Heavy)
        } else if light.iter().any(|suffix| path.ends_with(suffix)) {
            Some(Family::Light)
        } else if medium.iter().any(|suffix| path.ends_with(suffix)) {
            Some(Family::Medium)
        } else {
            None
        }
    }

    /// The armour slot an item of this family fills: 39 helmet, 38 chestplate,
    /// 37 leggings, 36 boots (the ids `inventory` reports).
    pub fn slot_of_path(path: &str) -> Option<u32> {
        if path.ends_with("_helmet") || path.ends_with("_hat") || path.ends_with("_hood") {
            Some(39)
        } else if path.ends_with("_chestplate") || path.ends_with("_robes") || path.ends_with("_tunic")
        {
            Some(38)
        } else if path.ends_with("_leggings")
            || path.ends_with("_leggings_heavy")
            || path.ends_with("_pants")
        {
            Some(37)
        } else if path.ends_with("_boots") || path.ends_with("_boots_heavy") || path.ends_with("_shoes")
        {
            Some(36)
        } else {
            None
        }
    }
}

/// One worn armour piece.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub slot: u32,
    pub school: School,
    pub family: Family,
}

impl Piece {
    /// The piece an inventory entry is, if it is armour of this mod in the
    /// armour slot its kind belongs to.
    pub fn of(slot: u32, item: &str) -> Option<Piece> {
        let path = item.split_once(':').map_or(item, |(_, path)| path);
        if !item.starts_with("ars_elemancy:") {
            return None;
        }
        let school = School::of_item(path)?;
        let family = Family::of_path(path)?;
        if Family::slot_of_path(path)? != slot {
            return None;
        }
        Some(Piece {
            slot,
            school,
            family,
        })
    }
}

/// How much of a school's damage the base set absorbs with four pieces.
pub const SCHOOL_ABSORPTION: f32 = 0.30;
/// How much a two-piece family set absorbs.
pub const FAMILY_ABSORPTION_TWO: f32 = 0.10;
/// How much a four-piece family set absorbs.
pub const FAMILY_ABSORPTION_FOUR: f32 = 0.20;
/// A focus amplifies its schools by this much, and weakens the others.
pub const FOCUS_SHIFT: f32 = 0.10;
/// A bangle amplifies its schools by this much.
pub const BANGLE_SHIFT: f32 = 0.15;
/// No combination absorbs more than half a hit: the set, focus and bangle
/// together reach the cap, so gear never makes the player untouchable.
pub const MAX_ABSORPTION: f32 = 0.50;

/// What the player is wearing, from the armour slots of the inventory.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Loadout {
    pieces: Vec<Piece>,
}

impl Loadout {
    /// `inventory` as `(slot, item id)` pairs. Slots outside the armour range
    /// and items that are not this mod's armour are ignored.
    pub fn detect(inventory: &[(u32, &str)]) -> Loadout {
        let mut pieces: Vec<Piece> = inventory
            .iter()
            .filter_map(|(slot, item)| Piece::of(*slot, item))
            .collect();
        pieces.sort_by_key(|piece| piece.slot);
        Loadout { pieces }
    }

    pub fn pieces(&self) -> &[Piece] {
        &self.pieces
    }

    pub fn is_empty(&self) -> bool {
        self.pieces.is_empty()
    }

    pub fn school_count(&self, school: School) -> usize {
        self.pieces
            .iter()
            .filter(|piece| piece.school == school)
            .count()
    }

    pub fn family_count(&self, family: Family) -> usize {
        self.pieces
            .iter()
            .filter(|piece| piece.family == family)
            .count()
    }

    /// The school whose four-piece bonus is active. Four armour slots can only
    /// complete one school, so the first match is the only match.
    pub fn school_bonus(&self) -> Option<School> {
        School::ALL
            .into_iter()
            .find(|school| self.school_count(*school) >= 4)
    }

    /// The family whose bonus is active, at four pieces before two.
    pub fn family_bonus(&self) -> Option<Family> {
        Family::ALL
            .into_iter()
            .find(|family| self.family_count(*family) >= 4)
            .or_else(|| {
                Family::ALL
                    .into_iter()
                    .find(|family| self.family_count(*family) >= 2)
            })
    }

    /// The fraction of `element` damage the armour absorbs, before a focus or
    /// bangle. A school bonus and a family bonus do not stack: the better one
    /// for that element wins, as both describe the same "absorb some damage".
    pub fn absorption(&self, element: Element) -> f32 {
        let mut best: f32 = 0.0;
        if let Some(school) = self.school_bonus()
            && school.covers(element)
        {
            best = best.max(SCHOOL_ABSORPTION);
        }
        for family in Family::ALL {
            let pieces = self.family_count(family);
            let share = match pieces {
                0 | 1 => continue,
                2 | 3 => FAMILY_ABSORPTION_TWO,
                _ => FAMILY_ABSORPTION_FOUR,
            };
            if self
                .pieces
                .iter()
                .any(|piece| piece.family == family && piece.school.covers(element))
            {
                best = best.max(share);
            }
        }
        best
    }

    /// The active bonuses as the HUD shows them: `Cindermancer's Set`,
    /// `Battlemage's Resilience`, or both.
    pub fn bonus_names(&self) -> Vec<&'static str> {
        let mut names = Vec::new();
        if let Some(school) = self.school_bonus() {
            names.push(school.set_name());
        }
        if let Some(family) = self.family_bonus() {
            names.push(family.bonus_name());
        }
        names
    }
}

/// A focus or bangle in the player's hands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gear {
    /// The focus the player attuned to and is holding.
    pub focus: Option<School>,
    /// The bangle in either hand.
    pub bangle: Option<School>,
}

impl Gear {
    /// Reads the hands: `main` and `off` are item ids or empty.
    pub fn of_hands(main: &str, off: &str, attuned: Option<School>) -> Gear {
        let focus = [main, off]
            .into_iter()
            .filter_map(School::of_item)
            .find(|school| attuned == Some(*school) && is_focus_item(main, off, *school));
        let bangle = [main, off]
            .into_iter()
            .filter_map(School::of_item)
            .find(|school| is_bangle_item(main, off, *school));
        Gear { focus, bangle }
    }
}

fn is_focus_item(main: &str, off: &str, school: School) -> bool {
    [main, off]
        .into_iter()
        .any(|item| item == format!("ars_elemancy:{}_focus", school.id()))
}

fn is_bangle_item(main: &str, off: &str, school: School) -> bool {
    [main, off]
        .into_iter()
        .any(|item| item == format!("ars_elemancy:{}_bangle", school.id()))
}

/// The damage multiplier a hit takes: `1.0` untouched, lower when absorbed.
///
/// `focus` amplifies the schools it is attuned to and weakens the other two,
/// which is the port's reading of "amplifies and discounts glyphs of this
/// school; the lesser focus weakens glyphs of the other elemental schools".
/// `bangle` amplifies its two schools, and an earth bangle also nullifies
/// cactus and berry-bush damage, as Ars Elemental's Earth Bangle does.
pub fn damage_multiplier(loadout: &Loadout, gear: Gear, damage: &str) -> f32 {
    let Some(element) = Element::of_damage(damage) else {
        return 1.0;
    };
    let mut absorption = loadout.absorption(element);
    if let Some(focus) = gear.focus {
        absorption += if focus.covers(element) {
            FOCUS_SHIFT
        } else {
            -FOCUS_SHIFT
        };
    }
    if let Some(bangle) = gear.bangle
        && bangle.covers(element)
    {
        absorption += BANGLE_SHIFT;
    }
    // Earth's passive: berries and cacti cannot hurt an earth bangle's wearer
    // at all, so it is not subject to the absorption cap.
    let bare = damage.trim_start_matches("minecraft:");
    if gear.bangle.is_some_and(|bangle| bangle.covers(Element::Earth))
        && matches!(bare, "cactus" | "sweet_berry_bush")
    {
        return 0.0;
    }
    (1.0 - absorption.clamp(0.0, MAX_ABSORPTION)).max(0.0)
}

/// A school set that absorbs fire also puts out the fire the player stands in
/// (`ars_elemental.armor_set.<school>.desc`: "instantly clears fire").
pub fn clears_fire(loadout: &Loadout) -> bool {
    loadout.school_bonus().is_some_and(|school| {
        school.covers(Element::Fire) && loadout.absorption(Element::Fire) >= SCHOOL_ABSORPTION
    })
}

/// A school set that absorbs water also refills air when about to drown
/// ("refill air if about to drown"). Horizon has no air meter, so the port
/// turns the drowning damage itself off instead.
pub fn stops_drowning(loadout: &Loadout) -> bool {
    loadout.school_bonus().is_some_and(|school| {
        school.covers(Element::Water) && loadout.absorption(Element::Water) >= SCHOOL_ABSORPTION
    })
}

/// A school set that absorbs earth also feeds the wearer deep underground
/// ("gives food deep underground if about to starve"). Horizon cannot write
/// the hunger bar, so the port absorbs the starvation damage instead.
pub fn stops_starvation(loadout: &Loadout) -> bool {
    loadout.school_bonus().is_some_and(|school| {
        school.covers(Element::Earth) && loadout.absorption(Element::Earth) >= SCHOOL_ABSORPTION
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worn(pieces: &[&str]) -> Loadout {
        let inventory: Vec<(u32, &str)> = pieces
            .iter()
            .map(|item| {
                let path = item.split_once(':').unwrap().1;
                (Family::slot_of_path(path).unwrap(), *item)
            })
            .collect();
        Loadout::detect(&inventory)
    }

    #[test]
    fn reads_the_school_of_every_item_path() {
        assert_eq!(School::of_item("ars_elemancy:cinder_hat"), Some(School::Cinder));
        assert_eq!(School::of_item("ars_elemancy:tempest_boots_heavy"), Some(School::Tempest));
        assert_eq!(School::of_item("ars_elemancy:elemancer_bangle"), Some(School::Elemancer));
        assert_eq!(School::of_item("ars_elemancy:cinder_essence"), Some(School::Cinder));
        assert_eq!(School::of_item("ars_nouveau:fire_bangle"), None);
    }

    #[test]
    fn reads_the_family_of_every_armour_path() {
        assert_eq!(Family::of_path("cinder_hat"), Some(Family::Medium));
        assert_eq!(Family::of_path("cinder_boots"), Some(Family::Medium));
        assert_eq!(Family::of_path("cinder_leggings_heavy"), Some(Family::Heavy));
        assert_eq!(Family::of_path("cinder_boots_heavy"), Some(Family::Heavy));
        assert_eq!(Family::of_path("cinder_shoes"), Some(Family::Light));
        assert_eq!(Family::of_path("cinder_tunic"), Some(Family::Light));
        assert_eq!(Family::of_path("cinder_focus"), None);
    }

    #[test]
    fn armour_only_counts_in_its_own_slot() {
        // A cinder hat belongs on the head (39), not in the boots slot (36).
        assert_eq!(
            Piece::of(39, "ars_elemancy:cinder_hat").map(|p| p.family),
            Some(Family::Medium)
        );
        assert!(Piece::of(36, "ars_elemancy:cinder_hat").is_none());
        assert!(Piece::of(39, "minecraft:diamond_helmet").is_none());
        assert!(Piece::of(39, "ars_elemancy:cinder_focus").is_none());
    }

    #[test]
    fn four_pieces_of_one_school_give_the_school_bonus() {
        let loadout = worn(&[
            "ars_elemancy:cinder_hat",
            "ars_elemancy:cinder_robes",
            "ars_elemancy:cinder_leggings",
            "ars_elemancy:cinder_boots",
        ]);
        assert_eq!(loadout.school_bonus(), Some(School::Cinder));
        assert_eq!(loadout.family_bonus(), Some(Family::Medium));
        assert_eq!(loadout.absorption(Element::Fire), SCHOOL_ABSORPTION);
        assert_eq!(loadout.absorption(Element::Air), SCHOOL_ABSORPTION);
        assert_eq!(loadout.absorption(Element::Water), 0.0);
    }

    #[test]
    fn three_pieces_keep_only_the_family_bonus() {
        let loadout = worn(&[
            "ars_elemancy:cinder_hat",
            "ars_elemancy:cinder_robes",
            "ars_elemancy:cinder_leggings",
        ]);
        assert_eq!(loadout.school_bonus(), None);
        assert_eq!(loadout.family_bonus(), Some(Family::Medium));
        assert_eq!(loadout.absorption(Element::Fire), FAMILY_ABSORPTION_TWO);
    }

    #[test]
    fn two_pieces_of_a_family_are_enough() {
        let loadout = worn(&["ars_elemancy:tempest_helmet", "ars_elemancy:tempest_boots_heavy"]);
        assert_eq!(loadout.family_bonus(), Some(Family::Heavy));
        assert_eq!(loadout.absorption(Element::Water), FAMILY_ABSORPTION_TWO);
        assert_eq!(loadout.absorption(Element::Air), FAMILY_ABSORPTION_TWO);
        assert_eq!(loadout.absorption(Element::Fire), 0.0);
    }

    #[test]
    fn four_pieces_across_families_complete_both_bonuses() {
        // Four cinder pieces across the three families: the school set is
        // complete, and the two heavy pieces also complete that family.
        let loadout = worn(&[
            "ars_elemancy:cinder_hat",
            "ars_elemancy:cinder_chestplate",
            "ars_elemancy:cinder_leggings_heavy",
            "ars_elemancy:cinder_boots_heavy",
        ]);
        assert_eq!(loadout.school_bonus(), Some(School::Cinder));
        assert_eq!(loadout.family_bonus(), Some(Family::Heavy));
        // The school bonus is the stronger of the two and does not stack.
        assert_eq!(loadout.absorption(Element::Air), SCHOOL_ABSORPTION);
    }

    #[test]
    fn elemancer_covers_every_element() {
        let loadout = worn(&[
            "ars_elemancy:elemancer_helmet",
            "ars_elemancy:elemancer_chestplate",
            "ars_elemancy:elemancer_leggings_heavy",
            "ars_elemancy:elemancer_boots_heavy",
        ]);
        assert_eq!(loadout.school_bonus(), Some(School::Elemancer));
        for element in Element::ALL {
            assert_eq!(loadout.absorption(element), SCHOOL_ABSORPTION);
        }
    }

    #[test]
    fn the_better_bonus_wins_instead_of_stacking() {
        let loadout = worn(&[
            "ars_elemancy:mire_helmet",
            "ars_elemancy:mire_chestplate",
            "ars_elemancy:mire_leggings_heavy",
            "ars_elemancy:mire_boots_heavy",
        ]);
        assert_eq!(loadout.family_bonus(), Some(Family::Heavy));
        // Both the school set (30%) and the family set (20%) cover water and
        // earth; the school set is the stronger one and does not add up.
        assert_eq!(loadout.absorption(Element::Water), SCHOOL_ABSORPTION);
        assert_eq!(loadout.absorption(Element::Earth), SCHOOL_ABSORPTION);
    }

    #[test]
    fn maps_vanilla_damage_onto_the_four_elements() {
        assert_eq!(Element::of_damage("minecraft:in_fire"), Some(Element::Fire));
        assert_eq!(Element::of_damage("minecraft:lava"), Some(Element::Fire));
        assert_eq!(Element::of_damage("minecraft:drown"), Some(Element::Water));
        assert_eq!(Element::of_damage("minecraft:fall"), Some(Element::Air));
        assert_eq!(Element::of_damage("minecraft:cactus"), Some(Element::Earth));
        assert_eq!(Element::of_damage("minecraft:mob_attack"), None);
        assert_eq!(Element::of_damage("minecraft:player_attack"), None);
    }

    #[test]
    fn an_unmapped_damage_source_is_untouched() {
        let loadout = worn(&[
            "ars_elemancy:cinder_hat",
            "ars_elemancy:cinder_robes",
            "ars_elemancy:cinder_leggings",
            "ars_elemancy:cinder_boots",
        ]);
        assert_eq!(
            damage_multiplier(&loadout, Gear::default(), "minecraft:mob_attack"),
            1.0
        );
        assert!(
            (damage_multiplier(&loadout, Gear::default(), "minecraft:in_fire") - 0.7).abs()
                < 1e-6
        );
    }

    #[test]
    fn a_focus_amplifies_its_schools_and_weakens_the_others() {
        let loadout = worn(&[
            "ars_elemancy:cinder_hat",
            "ars_elemancy:cinder_robes",
            "ars_elemancy:cinder_leggings",
            "ars_elemancy:cinder_boots",
        ]);
        let gear = Gear {
            focus: Some(School::Cinder),
            bangle: None,
        };
        assert!(
            (damage_multiplier(&loadout, gear, "minecraft:in_fire") - (1.0 - 0.40)).abs() < 1e-6
        );
        // The lesser focus weakens the other schools: water goes from 0 to
        // -10%, which is no absorption at all.
        assert!((damage_multiplier(&loadout, gear, "minecraft:drown") - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_bangle_amplifies_its_two_schools() {
        let loadout = worn(&[
            "ars_elemancy:tempest_helmet",
            "ars_elemancy:tempest_chestplate",
            "ars_elemancy:tempest_leggings_heavy",
            "ars_elemancy:tempest_boots_heavy",
        ]);
        let gear = Gear {
            focus: None,
            bangle: Some(School::Tempest),
        };
        assert!(
            (damage_multiplier(&loadout, gear, "minecraft:drown") - (1.0 - 0.45)).abs() < 1e-6
        );
        assert!(
            (damage_multiplier(&loadout, gear, "minecraft:fall") - (1.0 - 0.45)).abs() < 1e-6
        );
    }

    #[test]
    fn an_earth_bangle_nullifies_cactus_and_berries() {
        let loadout = Loadout::default();
        let gear = Gear {
            focus: None,
            bangle: Some(School::Silt),
        };
        assert_eq!(damage_multiplier(&loadout, gear, "minecraft:cactus"), 0.0);
        assert_eq!(
            damage_multiplier(&loadout, gear, "minecraft:sweet_berry_bush"),
            0.0
        );
        assert_eq!(damage_multiplier(&loadout, gear, "minecraft:mob_attack"), 1.0);
    }

    #[test]
    fn absorption_never_hides_a_hit_completely() {
        let loadout = worn(&[
            "ars_elemancy:cinder_hat",
            "ars_elemancy:cinder_robes",
            "ars_elemancy:cinder_leggings",
            "ars_elemancy:cinder_boots",
        ]);
        let gear = Gear {
            focus: Some(School::Cinder),
            bangle: Some(School::Cinder),
        };
        // School, focus and bangle add up (0.30 + 0.10 + 0.15 = 0.55) and the
        // cap stops the hit from being absorbed further.
        assert!(
            (damage_multiplier(&loadout, gear, "minecraft:in_fire") - (1.0 - MAX_ABSORPTION))
                .abs()
                < 1e-6
        );
        assert!(SCHOOL_ABSORPTION + FOCUS_SHIFT + BANGLE_SHIFT > MAX_ABSORPTION);
    }

    #[test]
    fn fire_water_and_earth_sets_keep_their_utility() {
        let cinder = worn(&[
            "ars_elemancy:cinder_hat",
            "ars_elemancy:cinder_robes",
            "ars_elemancy:cinder_leggings",
            "ars_elemancy:cinder_boots",
        ]);
        assert!(clears_fire(&cinder));
        assert!(!stops_drowning(&cinder));
        let tempest = worn(&[
            "ars_elemancy:tempest_hat",
            "ars_elemancy:tempest_robes",
            "ars_elemancy:tempest_leggings",
            "ars_elemancy:tempest_boots",
        ]);
        assert!(stops_drowning(&tempest));
        let silt = worn(&[
            "ars_elemancy:silt_hat",
            "ars_elemancy:silt_robes",
            "ars_elemancy:silt_leggings",
            "ars_elemancy:silt_boots",
        ]);
        assert!(stops_starvation(&silt));
        assert!(!clears_fire(&silt));
    }

    #[test]
    fn the_hands_decide_which_focus_is_live() {
        let gear = Gear::of_hands(
            "ars_elemancy:lava_focus",
            "",
            Some(School::Lava),
        );
        assert_eq!(gear.focus, Some(School::Lava));
        assert_eq!(gear.bangle, None);
        // Holding a focus the player never attuned does nothing.
        let gear = Gear::of_hands("ars_elemancy:lava_focus", "", Some(School::Mire));
        assert_eq!(gear.focus, None);
        let gear = Gear::of_hands("ars_elemancy:cinder_bangle", "", None);
        assert_eq!(gear.bangle, Some(School::Cinder));
        assert_eq!(gear.focus, None);
    }

    #[test]
    fn bonus_names_match_the_mod_lang() {
        let loadout = worn(&[
            "ars_elemancy:lava_helmet",
            "ars_elemancy:lava_chestplate",
            "ars_elemancy:lava_leggings_heavy",
            "ars_elemancy:lava_boots_heavy",
        ]);
        assert_eq!(
            loadout.bonus_names(),
            vec!["Lavamancer's Set", "Battlemage's Resilience"]
        );
    }

    #[test]
    fn no_armour_absorbs_nothing() {
        let loadout = Loadout::default();
        for element in Element::ALL {
            assert_eq!(loadout.absorption(element), 0.0);
        }
        assert!(loadout.bonus_names().is_empty());
        assert!(!clears_fire(&loadout));
    }
}

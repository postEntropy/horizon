//! Ars Elemancy: dual-element foci, bangles and armour built on Ars Elemental.
//!
//! The mod's lang files say what the gear does: every school is two of the four
//! elements, an armour family bonus needs two or four pieces, a school bonus
//! needs four pieces of one school, and the set absorbs damage of its schools
//! (`ars_elemental.armor_set.<school>.desc`). The port reproduces that with the
//! game's `entity-damage` hook, which is the only place Horizon lets a mod see
//! an incoming hit and its source:
//!
//! * the worn armour is read from slots 36..40 every tick, and the school and
//!   family bonuses become a damage multiplier for the hit's element;
//! * a fire set puts out the fire the player stands in, a water set stops
//!   drowning damage (Horizon has no air meter to refill) and an earth set
//!   stops starvation damage (the hunger bar is read-only);
//! * a focus attunes on use and, while held, amplifies its two schools and
//!   weakens the other two, the port's reading of "amplifies and discounts
//!   glyphs of this school" on the only school-scaled effect it has;
//! * a bangle in either hand amplifies its two schools, and an earth bangle
//!   also nullifies cactus and berry-bush damage, as Ars Elemental's Earth
//!   Bangle does.
//!
//! Ars Nouveau's glyphs, mana and potion effects have no Horizon API, so the
//! focus's spell side cannot be reproduced; what is left is the gear's
//! elemental defence, and the HUD and `/elemancy` show what is active.
mod items;
mod sets;

use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, VanillaHookEvent,
    VanillaHookKind, chat, game, hud,
    hud::{Corner, Line},
    inventory, server,
    server::BlockPos,
    storage, vanilla_hooks,
};
use sets::{Element, Gear, Loadout, School};

/// `storage` key holding the attuned school (one byte, 0 for none).
const STORAGE_KEY: &str = "attunement";
/// Colour of the set-name line.
const SET_COLOR: u32 = 0xFFD45E;
/// Colour of the attunement line.
const FOCUS_COLOR: u32 = 0xA8D8FF;

struct ArsElemancy {
    /// The school the last used focus attuned the player to.
    attuned: Option<School>,
    /// The armour the HUD currently describes, so it is only rewritten when
    /// the player's gear changes.
    shown: Loadout,
    /// Whether the HUD is showing anything.
    panel: bool,
}

impl ArsElemancy {
    fn load(&mut self) {
        self.attuned = storage::get(STORAGE_KEY)
            .and_then(|bytes| bytes.first().copied())
            .filter(|byte| *byte != 0)
            .and_then(|byte| School::ALL.get(usize::from(byte) - 1).copied());
    }

    fn save(&self) {
        let byte = self.attuned.map_or(0, |school| {
            School::ALL
                .iter()
                .position(|candidate| *candidate == school)
                .unwrap_or(0) as u8
                + 1
        });
        let _ = storage::set(STORAGE_KEY, &[byte]);
    }

    /// The armour the player wears, from the inventory's armour slots.
    fn loadout(&self) -> Loadout {
        let stacks = inventory::get_player_inventory();
        let slots: Vec<(u32, &str)> = stacks
            .iter()
            .map(|stack| (stack.slot, stack.id.as_str()))
            .collect();
        Loadout::detect(&slots)
    }

    /// The focus and bangle in the player's hands.
    fn gear(&self) -> Gear {
        let main = inventory::get_main_hand().map_or_else(String::new, |stack| stack.id);
        let off = inventory::get_off_hand().map_or_else(String::new, |stack| stack.id);
        Gear::of_hands(&main, &off, self.attuned)
    }

    fn refresh_hud(&mut self, loadout: &Loadout) {
        let names = loadout.bonus_names();
        if names.is_empty() && self.attuned.is_none() {
            if self.panel {
                hud::clear();
                self.panel = false;
            }
            return;
        }
        let mut lines = Vec::new();
        if !names.is_empty() {
            lines.push(Line {
                text: names.join(" · "),
                color: SET_COLOR,
            });
        }
        if let Some(school) = self.attuned {
            lines.push(Line {
                text: format!("Attuned: {} ({})", school.set_name(), element_list(school)),
                color: FOCUS_COLOR,
            });
        }
        hud::set_panel(Corner::TopRight, &lines);
        self.panel = true;
    }

    /// The fire set's "instantly clears fire": put out the block the player
    /// stands in while the set is worn.
    fn extinguish(&self, loadout: &Loadout) {
        if !sets::clears_fire(loadout) {
            return;
        }
        let Some(player) = game::get_player() else {
            return;
        };
        let at = BlockPos {
            x: player.x.floor() as i32,
            y: player.y.floor() as i32,
            z: player.z.floor() as i32,
        };
        let Some(block) = server::get_block(&player.dimension, at) else {
            return;
        };
        if block.starts_with("minecraft:fire") || block.starts_with("minecraft:soul_fire") {
            let _ = server::set_block(&player.dimension, at, "minecraft:air");
        }
    }

    /// The school a used item attunes to.
    fn attunement_of(item: &str) -> Option<School> {
        if !items::FOCI.contains(&item) {
            return None;
        }
        School::of_item(item)
    }
}

fn element_list(school: School) -> String {
    school
        .elements()
        .iter()
        .map(|element| element.id())
        .collect::<Vec<_>>()
        .join(" + ")
}

impl Mod for ArsElemancy {
    fn init() -> Result<(Self, Registration), String> {
        let _ = vanilla_hooks::subscribe("entity-damage");
        Ok((
            ArsElemancy {
                attuned: None,
                shown: Loadout::default(),
                panel: false,
            },
            Registration::new().command("elemancy").on_tick(),
        ))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        self.load();
        let mut hooks = LogicHooks::new();
        for item in items::ITEMS {
            hooks = hooks.item(item);
        }
        hooks
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if !matches!(event.action, ItemAction::UseInAir | ItemAction::UseOnBlock) {
            return EventOutcome::Pass;
        }
        let Some(school) = Self::attunement_of(&event.item) else {
            return EventOutcome::Pass;
        };
        self.attuned = Some(school);
        self.save();
        let loadout = self.loadout();
        self.refresh_hud(&loadout);
        chat::show(&format!(
            "Attuned to {} ({}). Hold it to amplify those schools.",
            school.set_name(),
            element_list(school)
        ));
        EventOutcome::Consume
    }

    fn tick(&mut self, _tick: u64) {
        let loadout = self.loadout();
        if loadout != self.shown {
            self.shown = loadout.clone();
            self.refresh_hud(&loadout);
        }
        self.extinguish(&loadout);
    }

    fn on_vanilla_hook(&mut self, event: VanillaHookEvent) -> EventOutcome {
        if event.kind != VanillaHookKind::EntityDamage || event.subject != "minecraft:player" {
            return EventOutcome::Pass;
        }
        let loadout = self.loadout();
        let damage = event.context.as_str();
        // "Refill air if about to drown" and "gives food deep underground if
        // about to starve": Horizon cannot write the air or hunger meters, so
        // the damage those meters would deal is what the set removes.
        if sets::stops_drowning(&loadout) && damage == "minecraft:drown" {
            return EventOutcome::Cancel;
        }
        if sets::stops_starvation(&loadout) && damage == "minecraft:starve" {
            return EventOutcome::Cancel;
        }
        let multiplier = sets::damage_multiplier(&loadout, self.gear(), damage);
        if multiplier < 1.0 {
            vanilla_hooks::set_damage_multiplier(multiplier);
        }
        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "elemancy" {
            return Err(format!("unknown command {name:?}"));
        }
        let mut parts = args.split_whitespace();
        let verb = parts.next().unwrap_or("info");
        match verb {
            "info" => {
                let loadout = self.loadout();
                let mut text = String::from("Ars Elemancy");
                if loadout.is_empty() {
                    text.push_str("\nNo elemental armour worn.");
                } else {
                    for piece in loadout.pieces() {
                        text.push_str(&format!(
                            "\n slot {}: {} {} ({})",
                            piece.slot,
                            piece.school.id(),
                            piece.family.id(),
                            piece.family.bonus_name()
                        ));
                    }
                    for name in loadout.bonus_names() {
                        text.push_str(&format!("\n bonus: {name}"));
                    }
                    text.push_str("\n absorption:");
                    for element in Element::ALL {
                        text.push_str(&format!(
                            " {} {:.0}%",
                            element.id(),
                            loadout.absorption(element) * 100.0
                        ));
                    }
                }
                text.push_str(&format!(
                    "\n attunement: {}",
                    self.attuned.map_or("none", |school| school.id())
                ));
                Ok(Some(text))
            }
            "attune" => {
                let school = parts.next().and_then(School::parse).ok_or_else(|| {
                    format!(
                        "usage: /elemancy attune <{}>",
                        School::ALL
                            .iter()
                            .map(|school| school.id())
                            .collect::<Vec<_>>()
                            .join("|")
                    )
                })?;
                self.attuned = Some(school);
                self.save();
                let loadout = self.loadout();
                self.refresh_hud(&loadout);
                Ok(Some(format!(
                    "Attuned to {} ({}).",
                    school.set_name(),
                    element_list(school)
                )))
            }
            "clear" => {
                self.attuned = None;
                self.save();
                let loadout = self.loadout();
                self.refresh_hud(&loadout);
                Ok(Some("Attunement cleared.".into()))
            }
            other => Err(format!(
                "unknown /elemancy subcommand {other:?}; use info, attune <school> or clear"
            )),
        }
    }
}

horizon_mod_sdk::export_mod!(ArsElemancy);

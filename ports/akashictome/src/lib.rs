//! Akashic Tome: one item for every mod's guide book.
//!
//! The Java mod stores the guides on the item's NBT and turns the Tome into the
//! real guide, which Horizon cannot do: no capability rewrites the player's
//! held stack. The library therefore lives in the mod's `storage` for the
//! world, and the Tome keeps its identity while it remembers the active guide.
//! The rest follows the mod:
//!
//! * using the Tome on a block of a mod whose guide is stored morphs it to the
//!   first guide of that mod (the mod's sneak-right-click shortcut);
//! * using it in the air, or pressing the cycle key while holding it, steps to
//!   the next stored guide (the mod's selection menu, which needs a GUI the
//!   game only opens for block entities);
//! * `/akashictome eject` throws the active guide out as a real item drop;
//! * `/akashictome add` stores a guide; the crafting-table route has no
//!   equivalent because mod recipes are data-driven here.
mod items;
mod library;

use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, config, game, hud,
    hud::{Corner, Line},
    inventory, keys,
    keys::KeyBinding,
    server,
    server::{BlockPos, ItemDrop},
    storage,
};
use library::{DEFAULT_NAME_FRAGMENTS, Library};

/// `storage` key holding the encoded library.
const STORAGE_KEY: &str = "library";
/// Keybinding that steps through the stored guides.
const CYCLE_KEY: &str = "akashictome.cycle";
/// Colour of the HUD title line.
const TITLE_COLOR: u32 = 0xFFD45E;
/// Colour of the active-guide line.
const GUIDE_COLOR: u32 = 0xE8E8E8;

struct AkashicTome {
    library: Library,
    loaded: bool,
}

impl AkashicTome {
    /// Reads the library once, on the first callback that needs it.
    fn ensure_loaded(&mut self) {
        if self.loaded {
            return;
        }
        self.loaded = true;
        if let Some(bytes) = storage::get(STORAGE_KEY) {
            self.library = Library::decode(&bytes);
        }
    }

    fn persist(&self) {
        let _ = storage::set(STORAGE_KEY, &self.library.encode());
    }

    /// The mod's two settings, re-read so a change applies without a reload.
    fn accepts(&self, item: &str) -> bool {
        let allow_any = config::get_bool("allow_any_item").unwrap_or(false);
        let fragments = config::get_string("whitelisted_names")
            .unwrap_or_else(|| DEFAULT_NAME_FRAGMENTS.to_owned());
        library::looks_like_guide(item, allow_any, &fragments)
    }

    /// The HUD panel is the Tome's own state, not a notification: it shows
    /// which guide the item is currently morphed to.
    fn refresh_hud(&self) {
        match self.library.active_book() {
            Some(book) => hud::set_panel(
                Corner::TopRight,
                &[
                    Line {
                        text: "Akashic Tome".into(),
                        color: TITLE_COLOR,
                    },
                    Line {
                        text: library::display_name(book),
                        color: GUIDE_COLOR,
                    },
                ],
            ),
            None => hud::clear(),
        }
    }

    fn holding_tome(&self) -> bool {
        inventory::get_main_hand().is_some_and(|stack| stack.id == items::TOME)
            || inventory::get_off_hand().is_some_and(|stack| stack.id == items::TOME)
    }

    /// Drops `book` at the player's feet, the mod's sneak-drop removal.
    fn drop_book(&self, book: &str) -> Result<(), String> {
        let Some(player) = game::get_player() else {
            return Err("no player in a world".into());
        };
        let at = BlockPos {
            x: player.x.floor() as i32,
            y: player.y.floor() as i32,
            z: player.z.floor() as i32,
        };
        server::spawn_drop(
            &player.dimension,
            at,
            &ItemDrop {
                id: book.to_owned(),
                count: 1,
            },
        );
        Ok(())
    }

    /// 1-based index argument, defaulting to the active guide.
    fn index_argument(&self, argument: Option<&str>) -> Result<usize, String> {
        match argument {
            Some(text) => text
                .parse::<usize>()
                .ok()
                .filter(|index| (1..=self.library.len()).contains(index))
                .map(|index| index - 1)
                .ok_or_else(|| {
                    format!(
                        "{text:?} is not a stored guide number (1..{})",
                        self.library.len()
                    )
                }),
            None => self
                .library
                .active()
                .ok_or_else(|| "no guide is active; give a guide number".to_owned()),
        }
    }
}

impl Mod for AkashicTome {
    fn init() -> Result<(Self, Registration), String> {
        let _ = keys::register(&KeyBinding {
            id: CYCLE_KEY.into(),
            label: "Akashic Tome: next guide".into(),
            default_key: "KeyK".into(),
        });
        Ok((
            AkashicTome {
                library: Library::default(),
                loaded: false,
            },
            Registration::new().command("akashictome").on_tick(),
        ))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        self.ensure_loaded();
        LogicHooks::new().item(items::TOME)
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        self.ensure_loaded();
        match event.action {
            ItemAction::UseInAir => {
                if self.library.cycle(1).is_none() {
                    return EventOutcome::Pass;
                }
                self.persist();
                self.refresh_hud();
                EventOutcome::Consume
            }
            // The mod's shortcut: the block's mod decides which guide is used.
            // A block whose own use was accepted never reaches this callback,
            // so interactive blocks keep working.
            ItemAction::UseOnBlock => {
                let Some(namespace) = library::namespace_of_state(&event.target) else {
                    return EventOutcome::Pass;
                };
                if self.library.morph(namespace).is_none() {
                    return EventOutcome::Pass;
                }
                self.persist();
                self.refresh_hud();
                EventOutcome::Consume
            }
            _ => EventOutcome::Pass,
        }
    }

    fn tick(&mut self, _tick: u64) {
        if !keys::was_pressed(CYCLE_KEY) {
            return;
        }
        self.ensure_loaded();
        if !self.holding_tome() || self.library.cycle(1).is_none() {
            return;
        }
        self.persist();
        self.refresh_hud();
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "akashictome" {
            return Err(format!("unknown command {name:?}"));
        }
        self.ensure_loaded();
        let mut parts = args.split_whitespace();
        let verb = parts.next().unwrap_or("list");
        match verb {
            "list" => {
                if self.library.is_empty() {
                    return Ok(Some("The Akashic Tome holds no guides.".into()));
                }
                let mut text = String::from("Stored guides:");
                for (index, book) in self.library.books().iter().enumerate() {
                    let marker = if self.library.active() == Some(index) {
                        '*'
                    } else {
                        ' '
                    };
                    text.push_str(&format!("\n {marker} {}. {book}", index + 1));
                }
                Ok(Some(text))
            }
            "add" => {
                let item = parts
                    .next()
                    .ok_or_else(|| "usage: /akashictome add <item>".to_owned())?;
                if !self.accepts(item) {
                    return Err(format!(
                        "{item} does not look like a guide book (its name needs one of the whitelisted names; see the mod settings)"
                    ));
                }
                let index = self.library.add(item)?;
                self.persist();
                self.refresh_hud();
                Ok(Some(format!("Stored {item} as guide {}.", index + 1)))
            }
            "select" | "next" | "prev" => {
                let index = match verb {
                    "select" => {
                        let index = self.index_argument(parts.next())?;
                        self.library.select(index);
                        index
                    }
                    "next" => self.library.cycle(1).unwrap_or(0),
                    _ => self.library.cycle(-1).unwrap_or(0),
                };
                self.persist();
                self.refresh_hud();
                let book = self.library.active_book().unwrap_or_default();
                Ok(Some(format!(
                    "Morphed to {} ({}/{}).",
                    library::display_name(book),
                    index + 1,
                    self.library.len()
                )))
            }
            "eject" | "remove" => {
                let index = self.index_argument(parts.next())?;
                let Some(book) = self.library.remove(index) else {
                    return Err(format!("there is no guide {}", index + 1));
                };
                self.drop_book(&book)?;
                self.persist();
                self.refresh_hud();
                Ok(Some(format!("Dropped {book}.")))
            }
            "clear" => {
                let count = self.library.len();
                self.library.clear();
                self.persist();
                self.refresh_hud();
                Ok(Some(format!("Emptied {count} guide(s) from the Tome.")))
            }
            other => Err(format!(
                "unknown /akashictome subcommand {other:?}; use list, add, select, next, prev, eject or clear"
            )),
        }
    }
}

horizon_mod_sdk::export_mod!(AkashicTome);

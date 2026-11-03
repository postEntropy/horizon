//! The Tome's stored-guide library, kept apart from the game calls so the
//! rules can be tested on the host.
//!
//! The Java mod keeps the guides on the item's own NBT and turns the item into
//! the real guide. Horizon gives a mod no way to rewrite the player's held
//! stack, so the library lives in the mod's `storage` and the Tome remembers
//! which guide is active instead of changing identity.

/// The mod's `Whitelisted Names` default: a stored item's path has to contain
/// one of these fragments, which is how the mod decides what counts as a guide.
pub const DEFAULT_NAME_FRAGMENTS: &str = "book,tome,lexicon,guide,manual,codex,compendium,journal";

/// The mod has no limit; the store is bounded so a runaway command cannot
/// fill the mod's storage quota.
pub const MAX_BOOKS: usize = 64;

/// A stored-guide payload starts with this so a foreign or old value is
/// ignored instead of decoded into garbage.
const MAGIC: &[u8; 4] = b"AKT1";

/// `namespace:path`, the only shape an item id may have.
pub fn valid_item(id: &str) -> bool {
    let Some((namespace, path)) = id.split_once(':') else {
        return false;
    };
    !namespace.is_empty()
        && !path.is_empty()
        && namespace
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-'))
        && path
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '_' | '.' | '-' | '/'))
}

/// The mod id an item id belongs to.
pub fn namespace_of(item: &str) -> Option<&str> {
    if !valid_item(item) {
        return None;
    }
    item.split_once(':').map(|(namespace, _)| namespace)
}

/// The mod id of a block-state string (`minecraft:chest[facing=north]`).
pub fn namespace_of_state(state: &str) -> Option<&str> {
    let id = state.split_once('[').map_or(state, |(id, _)| id);
    namespace_of(id)
}

/// `Whitelisted Names` / `Blacklisted Mods` / `Allow all items to be added`,
/// as the mod applies them to a book offered to the Tome.
///
/// Vanilla is a blacklisted mod on purpose: the Tome is for *mod* guides, so
/// `minecraft:book` and `minecraft:written_book` never go in.
pub fn looks_like_guide(item: &str, allow_any: bool, fragments: &str) -> bool {
    let Some((namespace, path)) = item.split_once(':') else {
        return false;
    };
    if !valid_item(item) || namespace == "minecraft" {
        return false;
    }
    if allow_any {
        return true;
    }
    fragments
        .split(',')
        .map(str::trim)
        .filter(|fragment| !fragment.is_empty())
        .any(|fragment| path.contains(&fragment.to_ascii_lowercase()))
}

/// A display name for a guide: `ars_nouveau:novice_spell_book` reads as
/// `novice spell book`, which is what the HUD shows without the guide's lang.
pub fn display_name(item: &str) -> String {
    let path = item.split_once(':').map_or(item, |(_, path)| path);
    path.rsplit('/').next().unwrap_or(path).replace('_', " ")
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Library {
    books: Vec<String>,
    active: Option<usize>,
}

impl Library {
    pub fn books(&self) -> &[String] {
        &self.books
    }

    pub fn len(&self) -> usize {
        self.books.len()
    }

    pub fn is_empty(&self) -> bool {
        self.books.is_empty()
    }

    pub fn active(&self) -> Option<usize> {
        self.active
    }

    pub fn active_book(&self) -> Option<&str> {
        self.active
            .and_then(|index| self.books.get(index))
            .map(String::as_str)
    }

    /// Stores another guide. Duplicates are allowed, as in the mod: adding the
    /// same book twice keeps two entries and the first one wins a morph.
    pub fn add(&mut self, item: &str) -> Result<usize, String> {
        if !valid_item(item) {
            return Err(format!("{item:?} is not an item id"));
        }
        if self.books.len() >= MAX_BOOKS {
            return Err(format!("the Tome holds at most {MAX_BOOKS} guides"));
        }
        self.books.push(item.to_owned());
        Ok(self.books.len() - 1)
    }

    /// Takes a guide out of the library; the caller drops it in the world.
    pub fn remove(&mut self, index: usize) -> Option<String> {
        if index >= self.books.len() {
            return None;
        }
        let book = self.books.remove(index);
        match self.active {
            Some(active) if active == index => self.active = None,
            Some(active) if active > index => self.active = Some(active - 1),
            _ => {}
        }
        Some(book)
    }

    /// The first guide added for `namespace`, which is the one the mod's
    /// sneak-right-click shortcut picks.
    pub fn first_of_namespace(&self, namespace: &str) -> Option<usize> {
        self.books
            .iter()
            .position(|book| namespace_of(book) == Some(namespace))
    }

    /// Morphs to the first guide of the block's mod, if one is stored.
    pub fn morph(&mut self, namespace: &str) -> Option<usize> {
        let index = self.first_of_namespace(namespace)?;
        self.active = Some(index);
        Some(index)
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index < self.books.len() {
            self.active = Some(index);
            true
        } else {
            false
        }
    }

    /// Steps `delta` places through the guides, wrapping. With no guide active
    /// a positive step starts at the first and a negative one at the last.
    pub fn cycle(&mut self, delta: isize) -> Option<usize> {
        if self.books.is_empty() {
            self.active = None;
            return None;
        }
        let len = self.books.len() as isize;
        let next = match self.active {
            Some(active) => (active as isize + delta).rem_euclid(len),
            None if delta < 0 => len - 1,
            None => 0,
        };
        self.active = Some(next as usize);
        self.active
    }

    pub fn clear(&mut self) {
        self.books.clear();
        self.active = None;
    }

    /// The `storage` value: magic, count, length-prefixed ids, active index
    /// (`-1` for none).
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(10 + self.books.iter().map(String::len).sum::<usize>());
        out.extend_from_slice(MAGIC);
        out.extend_from_slice(&(self.books.len() as u16).to_le_bytes());
        for book in &self.books {
            let bytes = book.as_bytes();
            let len = bytes.len().min(u16::MAX as usize);
            out.extend_from_slice(&(len as u16).to_le_bytes());
            out.extend_from_slice(&bytes[..len]);
        }
        let active = self.active.map_or(-1i32, |index| index as i32);
        out.extend_from_slice(&active.to_le_bytes());
        out
    }

    /// Anything unreadable decodes to an empty library rather than a panic: a
    /// half-written or older value must not stop the mod.
    pub fn decode(bytes: &[u8]) -> Library {
        let mut library = Library::default();
        if bytes.len() < 10 || &bytes[..4] != MAGIC {
            return library;
        }
        let count = u16::from_le_bytes([bytes[4], bytes[5]]) as usize;
        let mut at = 6;
        for _ in 0..count.min(MAX_BOOKS) {
            if at + 2 > bytes.len() {
                return library;
            }
            let len = u16::from_le_bytes([bytes[at], bytes[at + 1]]) as usize;
            at += 2;
            if at + len > bytes.len() {
                return library;
            }
            let Ok(book) = std::str::from_utf8(&bytes[at..at + len]) else {
                return library;
            };
            at += len;
            library.books.push(book.to_owned());
        }
        if at + 4 <= bytes.len() {
            let active = i32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
            if active >= 0 && (active as usize) < library.books.len() {
                library.active = Some(active as usize);
            }
        }
        library
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn library(books: &[&str]) -> Library {
        let mut library = Library::default();
        for book in books {
            library.add(book).expect("valid guide");
        }
        library
    }

    #[test]
    fn stores_guides_in_order_and_keeps_duplicates() {
        let library = library(&["ars_nouveau:novice_spell_book", "botania:lexicon", "botania:lexicon"]);
        assert_eq!(library.len(), 3);
        assert_eq!(library.books()[0], "ars_nouveau:novice_spell_book");
        assert_eq!(library.books()[2], "botania:lexicon");
    }

    #[test]
    fn rejects_ids_that_are_not_item_ids() {
        let mut library = Library::default();
        assert!(library.add("botania").is_err());
        assert!(library.add("botania:").is_err());
        assert!(library.add("Botania:Lexicon").is_err());
        assert!(library.add("botania:lexicon extra").is_err());
        assert_eq!(library.len(), 0);
    }

    #[test]
    fn vanilla_books_are_ignored_on_purpose() {
        assert!(!looks_like_guide("minecraft:book", false, DEFAULT_NAME_FRAGMENTS));
        assert!(!looks_like_guide("minecraft:written_book", true, DEFAULT_NAME_FRAGMENTS));
        assert!(looks_like_guide("botania:lexicon", false, DEFAULT_NAME_FRAGMENTS));
        assert!(!looks_like_guide("botania:mana_petal", false, DEFAULT_NAME_FRAGMENTS));
    }

    #[test]
    fn allow_any_item_accepts_any_mod_item() {
        assert!(looks_like_guide("botania:mana_petal", true, DEFAULT_NAME_FRAGMENTS));
        assert!(!looks_like_guide("botania:mana_petal", false, DEFAULT_NAME_FRAGMENTS));
    }

    #[test]
    fn whitelisted_names_are_comma_separated_fragments() {
        assert!(looks_like_guide("mod:ancient_codex", false, " codex , almanac "));
        assert!(looks_like_guide("mod:field_almanac", false, "codex,almanac"));
        assert!(!looks_like_guide("mod:ancient_codex", false, "almanac"));
    }

    #[test]
    fn morph_picks_the_first_guide_of_the_blocks_mod() {
        let mut library = library(&["botania:lexicon", "ars_nouveau:novice_spell_book", "botania:lexicon"]);
        assert_eq!(library.morph("botania"), Some(0));
        assert_eq!(library.active_book(), Some("botania:lexicon"));
        assert_eq!(library.morph("ars_nouveau"), Some(1));
    }

    #[test]
    fn morph_without_a_guide_leaves_the_active_one_alone() {
        let mut library = library(&["botania:lexicon"]);
        assert!(library.select(0));
        assert_eq!(library.morph("create"), None);
        assert_eq!(library.active(), Some(0));
    }

    #[test]
    fn namespace_of_state_strips_block_state_properties() {
        assert_eq!(namespace_of_state("minecraft:chest[facing=north]"), Some("minecraft"));
        assert_eq!(namespace_of_state("create:mechanical_press"), Some("create"));
        assert_eq!(namespace_of_state("not a state"), None);
    }

    #[test]
    fn cycle_wraps_both_ways() {
        let mut library = library(&["a:one", "b:two", "c:three"]);
        assert_eq!(library.cycle(1), Some(0));
        assert_eq!(library.cycle(1), Some(1));
        assert_eq!(library.cycle(-1), Some(0));
        assert_eq!(library.cycle(-1), Some(2));
    }

    #[test]
    fn cycle_on_an_empty_library_has_no_active_guide() {
        let mut library = Library::default();
        assert_eq!(library.cycle(1), None);
        assert_eq!(library.active(), None);
    }

    #[test]
    fn removing_a_guide_keeps_the_active_index_on_the_same_book() {
        let mut library = library(&["a:one", "b:two", "c:three"]);
        assert!(library.select(2));
        assert_eq!(library.remove(0), Some("a:one".into()));
        assert_eq!(library.active_book(), Some("c:three"));
        assert_eq!(library.remove(1), Some("c:three".into()));
        assert_eq!(library.active(), None);
        assert_eq!(library.remove(9), None);
    }

    #[test]
    fn round_trips_through_the_storage_payload() {
        let mut library = library(&["botania:lexicon", "ars_nouveau:novice_spell_book"]);
        assert!(library.select(1));
        let decoded = Library::decode(&library.encode());
        assert_eq!(decoded, library);
    }

    #[test]
    fn decode_ignores_foreign_and_truncated_payloads() {
        assert_eq!(Library::decode(b""), Library::default());
        assert_eq!(Library::decode(b"NOPE\x00\x00"), Library::default());
        let mut payload = Library::default().encode();
        payload.truncate(payload.len() - 2);
        assert_eq!(Library::decode(&payload), Library::default());
    }

    #[test]
    fn a_full_library_refuses_another_guide() {
        let mut library = Library::default();
        for index in 0..MAX_BOOKS {
            library.add(&format!("mod:guide_{index}")).expect("room");
        }
        assert!(library.add("mod:one_too_many").is_err());
        assert_eq!(library.len(), MAX_BOOKS);
    }

    #[test]
    fn display_name_reads_like_the_guide() {
        assert_eq!(display_name("ars_nouveau:novice_spell_book"), "novice spell book");
        assert_eq!(display_name("mod:books/field_notes"), "field notes");
    }
}

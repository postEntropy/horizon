//! Write Horizon mods in Rust (`docs/mods.md`).
//!
//! A mod is a type implementing [`Mod`], exported with [`export_mod!`] from
//! a `cdylib` crate built for `wasm32-unknown-unknown`;
//! `cargo horizon-mod build` turns it into the component the game loads.
//!
//! ```ignore
//! use horizon_mod_sdk::{Mod, Registration, chat};
//!
//! struct Hello;
//! impl Mod for Hello {
//!     fn init() -> Result<(Self, Registration), String> {
//!         Ok((Hello, Registration::new().command("hello")))
//!     }
//!     fn command(&mut self, _name: &str, _args: &str) -> Result<Option<String>, String> {
//!         chat::show("Hello from a mod!");
//!         Ok(None)
//!     }
//! }
//! horizon_mod_sdk::export_mod!(Hello);
//! ```
//!
//! Every interface but `log` is a capability: list the ones the mod uses in
//! `horizon-mod.toml` (`game-read`, `chat`, `hud`, `storage`, and
//! `block-entities` for [`machine`] with `api = "0.4"`). Machine mods may also
//! use `item-transfer` to define the sides from which hoppers move items.
//!
//! Block entities (API 0.4): a mod whose blocks are machines returns their
//! types from [`Mod::block_entity_types`], keeps each one's state by its
//! handle, and reads and writes its slots and block state through [`machine`]
//! during [`Mod::tick_block_entities`]. The game owns the slots (hoppers,
//! menus, drops and saves see them); the mod saves the rest.

wit_bindgen::generate!({
    path: "wit",
    world: "horizon-machine-mod",
    pub_export_macro: true,
    export_macro_name: "__export_horizon_mod",
    default_bindings_module: "horizon_mod_sdk",
});

pub use exports::horizon::mods::block_entities::{BlockEntityType, Guest as __BlockEntitiesGuest};
pub use exports::horizon::mods::events::{Guest as __Guest, Registration};
pub use exports::horizon::mods::logic::{
    BlockAction, BlockEvent, EventOutcome, Guest as __LogicGuest, ItemAction, ItemEvent,
    LogicHooks, ServerEvent, ServerEventKind, VanillaHookEvent, VanillaHookKind,
};
pub use exports::horizon::mods::mod_entities::{
    AiGoal, EntityType, Guest as __ModEntitiesGuest, SpawnCategory,
};
pub use horizon::mods::{
    capabilities, chat, config, dimensions, entities, game, hud, inventory, item_data,
    item_transfer, keys, log, machine, models, net, recipes, render, server, storage, target, ui,
    vanilla_hooks, world_read as world,
};

/// The game's calls into a mod. Only the hooks [`Registration`] or
/// [`LogicHooks`] asks for are called; the others may stay as their defaults.
pub trait Mod: Sized + 'static {
    /// Once, after loading: the mod's state and the hooks it wants. An error
    /// keeps the mod off and is shown to the player.
    fn init() -> Result<(Self, Registration), String>;
    /// Every game tick, when [`Registration::tick`] is set.
    fn tick(&mut self, _tick: u64) {}
    /// Every rendered frame, when [`Registration::frame`] is set.
    fn frame(&mut self, _seconds: f32) {}
    /// `/name args` for a command the mod registered. `Ok(Some(text))` is
    /// shown as feedback, `Err(text)` in red.
    fn command(&mut self, _name: &str, _args: &str) -> Result<Option<String>, String> {
        Err("This command does nothing".into())
    }
    /// A chat line the player received, when [`Registration::chat`] is set.
    fn chat(&mut self, _text: &str) {}

    /// The mod's block entity types, once after [`Mod::init`]: each for one
    /// of its own blocks. Needs capability `block-entities`.
    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        Vec::new()
    }
    /// A block entity was placed (`saved` empty) or loaded (`saved` is what
    /// [`Mod::save_block_entity`] returned).
    fn create_block_entity(&mut self, _handle: u32, _block: &str, _saved: &[u8]) {}
    /// One server tick of every loaded ticking block entity of the mod; its
    /// slots and block state are readable and writable through [`machine`]
    /// during this call only.
    fn tick_block_entities(&mut self, _handles: &[u32]) {}
    /// Its state for the save, beside the slots the game saves.
    fn save_block_entity(&mut self, _handle: u32) -> Vec<u8> {
        Vec::new()
    }
    /// Broken or unloaded.
    fn remove_block_entity(&mut self, _handle: u32) {}
    /// The values its menu shows (progress bars, fuel left).
    fn block_entity_data(&mut self, _handle: u32) -> Vec<i32> {
        Vec::new()
    }

    /// Which blocks, items, server events and network channels the mod
    /// handles (API 0.3 and 0.4), called once after [`Mod::init`].
    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new()
    }
    /// A block callback on one of [`LogicHooks::blocks`] (API 0.3).
    fn on_block(&mut self, _event: BlockEvent) -> EventOutcome {
        EventOutcome::Pass
    }
    /// An item callback on one of [`LogicHooks::items`] (API 0.3).
    fn on_item(&mut self, _event: ItemEvent) -> EventOutcome {
        EventOutcome::Pass
    }
    /// A server lifecycle or world event in [`LogicHooks::server_events`] (API 0.3).
    fn on_server(&mut self, _event: ServerEvent) -> EventOutcome {
        EventOutcome::Pass
    }
    /// A slot click in a screen opened via [`ui::open_screen`] (API 0.4).
    fn on_menu_click(&mut self, _screen_id: &str, _slot: u32, _button: u8) {}
    /// A button click in a screen opened via [`ui::open_screen`] (API 0.4).
    fn on_menu_button(&mut self, _screen_id: &str, _button: u8) {}
    /// A packet received on one of [`LogicHooks::channels`] (API 0.4).
    fn on_packet(&mut self, _channel: &str, _payload: &[u8], _from_player: Option<&str>) {}
    /// A named vanilla hook subscribed via [`vanilla_hooks::subscribe`] (API 0.5).
    fn on_vanilla_hook(&mut self, _event: VanillaHookEvent) -> EventOutcome {
        EventOutcome::Pass
    }

    /// The mod's custom entity types, once after [`Mod::init`] (API 0.5).
    /// Needs capability `entities`.
    fn entity_types(&mut self) -> Vec<EntityType> {
        Vec::new()
    }
    /// A custom entity was spawned (`saved` empty) or loaded from disk (API 0.5).
    fn create_entity(&mut self, _handle: u32, _kind: &str, _saved: &[u8]) {}
    /// One server tick of every loaded custom entity of the mod (API 0.5).
    fn tick_entities(&mut self, _handles: &[u32]) {}
    /// Serializes the mod's private state for `handle` (API 0.5).
    fn save_entity(&mut self, _handle: u32) -> Vec<u8> {
        Vec::new()
    }
    /// Called when `handle` is despawned or unloaded (API 0.5).
    fn remove_entity(&mut self, _handle: u32) {}
    /// A player interacted (right-clicked) with `handle` holding `held_item` (API 0.5).
    fn on_entity_interact(&mut self, _handle: u32, _player: &str, _held_item: &str) -> bool {
        false
    }
    /// `handle` took `amount` damage from `source`; returns true to allow the damage (API 0.5).
    fn on_entity_hurt(&mut self, _handle: u32, _amount: f32, _source: &str) -> bool {
        true
    }
}

impl Registration {
    /// No commands and no hooks.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            tick: false,
            frame: false,
            chat: false,
        }
    }
    /// Adds `/name`.
    pub fn command(mut self, name: &str) -> Self {
        self.commands.push(name.into());
        self
    }
    pub fn on_tick(mut self) -> Self {
        self.tick = true;
        self
    }
    pub fn on_frame(mut self) -> Self {
        self.frame = true;
        self
    }
    pub fn on_chat(mut self) -> Self {
        self.chat = true;
        self
    }
}
impl Default for Registration {
    fn default() -> Self {
        Self::new()
    }
}

impl LogicHooks {
    /// No block, item, server-event or packet hooks.
    pub fn new() -> Self {
        Self {
            blocks: Vec::new(),
            items: Vec::new(),
            server_events: Vec::new(),
            channels: Vec::new(),
        }
    }
    /// Registers block callbacks (`on_block`) for `block_id` (`<mod>:<block>`).
    pub fn block(mut self, block_id: &str) -> Self {
        self.blocks.push(block_id.into());
        self
    }
    /// Registers item callbacks (`on_item`) for `item_id` (`<mod>:<item>`).
    pub fn item(mut self, item_id: &str) -> Self {
        self.items.push(item_id.into());
        self
    }
    /// Subscribes to a server lifecycle or world event (`on_server`).
    pub fn server_event(mut self, kind: ServerEventKind) -> Self {
        if !self.server_events.contains(&kind) {
            self.server_events.push(kind);
        }
        self
    }
    /// Listens on packet channel `channel` (`on_packet`).
    pub fn channel(mut self, channel: &str) -> Self {
        self.channels.push(channel.into());
        self
    }
}
impl Default for LogicHooks {
    fn default() -> Self {
        Self::new()
    }
}

/// `log::write` at info level.
pub fn info(message: &str) {
    log::write(log::Level::Info, message);
}
/// `log::write` at warn level.
pub fn warn(message: &str) {
    log::write(log::Level::Warn, message);
}

/// Exports a [`Mod`] as the component's `events`, `block-entities` and
/// `logic` interfaces. The state lives in a thread local: a mod instance is
/// single-threaded.
#[macro_export]
macro_rules! export_mod {
    ($t:ty) => {
        const _: () = {
            use ::std::cell::RefCell;
            ::std::thread_local! {
                static STATE: RefCell<Option<$t>> = const { RefCell::new(None) };
            }
            fn with(f: impl FnOnce(&mut $t)) {
                STATE.with(|s| {
                    if let Some(m) = s.borrow_mut().as_mut() {
                        f(m)
                    }
                })
            }
            struct Export;
            impl $crate::__Guest for Export {
                fn init() -> Result<$crate::Registration, String> {
                    let (m, registration) = <$t as $crate::Mod>::init()?;
                    STATE.with(|s| *s.borrow_mut() = Some(m));
                    Ok(registration)
                }
                fn tick(tick: u64) {
                    with(|m| $crate::Mod::tick(m, tick))
                }
                fn frame(seconds: f32) {
                    with(|m| $crate::Mod::frame(m, seconds))
                }
                fn command(name: String, args: String) -> Result<Option<String>, String> {
                    let mut out = Err("The mod is not running".to_string());
                    with(|m| out = $crate::Mod::command(m, &name, &args));
                    out
                }
                fn chat(text: String) {
                    with(|m| $crate::Mod::chat(m, &text))
                }
            }
            impl $crate::__BlockEntitiesGuest for Export {
                fn types() -> Vec<$crate::BlockEntityType> {
                    let mut out = Vec::new();
                    with(|m| out = $crate::Mod::block_entity_types(m));
                    out
                }
                fn create(handle: u32, block: String, saved: Vec<u8>) {
                    with(|m| $crate::Mod::create_block_entity(m, handle, &block, &saved))
                }
                fn tick(handles: Vec<u32>) {
                    with(|m| $crate::Mod::tick_block_entities(m, &handles))
                }
                fn save(handle: u32) -> Vec<u8> {
                    let mut out = Vec::new();
                    with(|m| out = $crate::Mod::save_block_entity(m, handle));
                    out
                }
                fn remove(handle: u32) {
                    with(|m| $crate::Mod::remove_block_entity(m, handle))
                }
                fn data(handle: u32) -> Vec<i32> {
                    let mut out = Vec::new();
                    with(|m| out = $crate::Mod::block_entity_data(m, handle));
                    out
                }
            }
            impl $crate::__LogicGuest for Export {
                fn hooks() -> $crate::LogicHooks {
                    let mut out = $crate::LogicHooks::new();
                    with(|m| out = $crate::Mod::logic_hooks(m));
                    out
                }
                fn on_block(event: $crate::BlockEvent) -> $crate::EventOutcome {
                    let mut out = $crate::EventOutcome::Pass;
                    with(|m| out = $crate::Mod::on_block(m, event));
                    out
                }
                fn on_item(event: $crate::ItemEvent) -> $crate::EventOutcome {
                    let mut out = $crate::EventOutcome::Pass;
                    with(|m| out = $crate::Mod::on_item(m, event));
                    out
                }
                fn on_server(event: $crate::ServerEvent) -> $crate::EventOutcome {
                    let mut out = $crate::EventOutcome::Pass;
                    with(|m| out = $crate::Mod::on_server(m, event));
                    out
                }
                fn on_menu_click(screen_id: String, slot: u32, button: u8) {
                    with(|m| $crate::Mod::on_menu_click(m, &screen_id, slot, button))
                }
                fn on_menu_button(screen_id: String, button: u8) {
                    with(|m| $crate::Mod::on_menu_button(m, &screen_id, button))
                }
                fn on_packet(channel: String, payload: Vec<u8>, from_player: Option<String>) {
                    with(|m| $crate::Mod::on_packet(m, &channel, &payload, from_player.as_deref()))
                }
                fn on_vanilla_hook(event: $crate::VanillaHookEvent) -> $crate::EventOutcome {
                    let mut out = $crate::EventOutcome::Pass;
                    with(|m| out = $crate::Mod::on_vanilla_hook(m, event));
                    out
                }
            }
            impl $crate::__ModEntitiesGuest for Export {
                fn types() -> Vec<$crate::EntityType> {
                    let mut out = Vec::new();
                    with(|m| out = $crate::Mod::entity_types(m));
                    out
                }
                fn create(handle: u32, kind: String, saved: Vec<u8>) {
                    with(|m| $crate::Mod::create_entity(m, handle, &kind, &saved))
                }
                fn tick(handles: Vec<u32>) {
                    with(|m| $crate::Mod::tick_entities(m, &handles))
                }
                fn save(handle: u32) -> Vec<u8> {
                    let mut out = Vec::new();
                    with(|m| out = $crate::Mod::save_entity(m, handle));
                    out
                }
                fn remove(handle: u32) {
                    with(|m| $crate::Mod::remove_entity(m, handle))
                }
                fn on_interact(handle: u32, player: String, held_item: String) -> bool {
                    let mut out = false;
                    with(|m| out = $crate::Mod::on_entity_interact(m, handle, &player, &held_item));
                    out
                }
                fn on_hurt(handle: u32, amount: f32, source: String) -> bool {
                    let mut out = true;
                    with(|m| out = $crate::Mod::on_entity_hurt(m, handle, amount, &source));
                    out
                }
            }
            // Only WebAssembly has the component's exports: a host build (cargo test or
            // clippy on the workspace) checks the mod without them, which macOS
            // couldn't link (its exported-symbols list cuts names at the `#`).
            #[cfg(target_arch = "wasm32")]
            $crate::__export_horizon_mod!(Export);
            #[cfg(not(target_arch = "wasm32"))]
            let _ = (
                <Export as $crate::__Guest>::init,
                <Export as $crate::__BlockEntitiesGuest>::types,
                <Export as $crate::__LogicGuest>::hooks,
                <Export as $crate::__ModEntitiesGuest>::types,
            );
        };
    };
}

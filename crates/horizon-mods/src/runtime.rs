//! Loading and calling mods with wasmtime's component model.
use crate::manifest::{Capability, ConfigSchema, ConfigValue, Manifest, Package};
use horizon_world::nbt::{Limits as NbtLimits, Tag as NbtTag, read_network};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{Duration, Instant, SystemTime},
};
use wasmtime::{
    Config, Engine, Store, StoreLimits, StoreLimitsBuilder, Trap,
    component::{Component, HasSelf, Linker},
};

mod bindings {
    wasmtime::component::bindgen!({
        path: "../horizon-mod-sdk/wit",
        world: "horizon-mod",
    });
}
/// API 0.3 and 0.4 extensions: a second world, so mods built before it (no
/// `block-entities` or `logic` exports) still instantiate as `horizon-mod`.
mod machine_bindings {
    wasmtime::component::bindgen!({
        path: "../horizon-mod-sdk/wit",
        world: "machine-host",
        additional_derives: [PartialEq],
    });
}
/// The typed logic bindings intentionally omit `on-block`: early preview
/// components used both the old and new record shape under the same WIT id.
mod logic_bindings {
    wasmtime::component::bindgen!({
        path: "wit-compat",
        world: "logic-host",
        additional_derives: [PartialEq],
    });
}
pub use bindings::horizon::mods::{
    game::{Clock, Player},
    hud::{Corner, Line},
};
use bindings::{
    HorizonMod,
    exports::horizon::mods::events::Registration,
    horizon::mods::{chat, game, hud, log as mod_log, storage},
};
use logic_bindings::LogicHost;
pub use logic_bindings::exports::horizon::mods::logic::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, ServerEvent, ServerEventKind,
    VanillaHookEvent, VanillaHookKind,
};
use machine_bindings::{
    MachineHost,
    horizon::mods::{
        capabilities, config, dimensions, entities, inventory, item_data, keys, machine, models,
        net, recipes, render, server, target, ui, vanilla_hooks, world_read as world,
    },
};
use wasmtime::component::{Func, Val};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockAction {
    UseWithoutItem,
    UseItemOn,
    Place,
    Broken,
    Attack,
    RandomTick,
    ScheduledTick,
    NeighborChanged,
    EntityInside,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockEvent {
    pub action: BlockAction,
    pub dimension: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub state: String,
    pub actor: String,
    pub held_item: String,
}
pub use machine_bindings::{
    exports::horizon::mods::{
        block_entities::BlockEntityType,
        mod_entities::{AiGoal, EntityType, SpawnCategory},
    },
    horizon::mods::{
        capabilities::{EnergyView, FluidView, ItemSlot},
        dimensions::{DimensionSpec, GeneratorKind, PortalSpec},
        entities::EntityState,
        inventory::StackView,
        keys::KeyBinding,
        machine::{CookingKind, CookingRecipe, ItemStack as Stack},
        models::{BonePose, DrawPart, DrawPartKind},
        recipes::{IngredientSlot, RecipeEntry},
        render::{BoxShape, GhostBlock},
        server::{BlockPos, ItemDrop},
        target::{BlockHit, EntityHit},
        ui::{ScreenBackground, ScreenSpec, Widget, WidgetKind},
    },
};

/// A mod's linear memory cap.
const MEMORY: usize = 64 << 20;
/// Fuel (about one unit per WebAssembly instruction) per call: a bound on
/// runaway loops, far above the hooks' time budgets.
const FUEL_INIT: u64 = 2_000_000_000;
const FUEL_TICK: u64 = 50_000_000;
const FUEL_FRAME: u64 = 10_000_000;
const FUEL_EVENT: u64 = 100_000_000;
/// Chat lines a mod may show per call, HUD lines and their length.
const CHAT_PER_CALL: usize = 8;
const HUD_LINES: usize = 16;
const LINE_CHARS: usize = 256;
/// Total bytes a mod's `storage` may hold on disk, across all its keys.
const STORAGE_QUOTA: u64 = 4 << 20;
/// Distinct keys a mod's `storage` may hold.
const STORAGE_MAX_KEYS: usize = 64;
/// Bytes `storage::set` may write across one hook call.
const STORAGE_WRITE_BUDGET: u64 = 1 << 20;
/// A mod's data, beside its `horizon-mods` folder.
const STORAGE_DIR: &str = ".horizon-mod-data";

/// The export a block-entity/logic mod adds (`horizon:mods/block-entities`).
const BLOCK_ENTITIES_EXPORT: &str = "horizon:mods/block-entities@0.1.0";
/// Slots a mod's block entity may ask the game to keep.
const MAX_SLOTS: u32 = 256;
/// Max packet payload on `net` channels (32 KiB).
const MAX_PACKET_BYTES: usize = 32 << 10;
/// Opaque data components on one machine-slot stack.
const MAX_ITEM_COMPONENT_BYTES: usize = 64 << 10;
/// Max attachment payload in `capabilities` (64 KiB).
const MAX_ATTACHMENT_BYTES: usize = 64 << 10;
/// Max 3D wireframe boxes and ghost blocks per mod.
const MAX_RENDER_SHAPES: usize = 256;
const MAX_GHOST_BLOCKS: usize = 512;

/// The game's item and recipe facts that block entities query (`machine`):
/// set once with [`Mods::set_item_facts`] after the data packs load.
pub trait ItemFacts: Send + Sync {
    /// 0 for an unknown item.
    fn max_stack_size(&self, id: &str) -> u32;
    /// Ticks the item burns as furnace fuel, 0 if it is no fuel.
    fn burn_time(&self, id: &str) -> u32;
    /// What is left once the item is used up (a bucket from a lava bucket).
    fn remainder(&self, id: &str) -> Option<String>;
    fn cooking_recipe(&self, kind: CookingKind, input: &str) -> Option<CookingRecipe>;
}

/// A block state property of a block entity's block, as `machine` sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct Property {
    pub name: String,
    pub value: String,
    /// The values the block allows for it; `set-property` refuses others.
    pub values: Vec<String>,
}

/// What the game lends a mod's block entity during a tick: its slots and its
/// block's properties. `changed` is set when the mod wrote either, so the
/// game saves the chunk and updates the block and open menus.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockEntityView {
    pub handle: u32,
    pub dimension: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
    pub slots: Vec<Option<Stack>>,
    /// Opaque binary network-NBT components aligned with `slots`.
    pub slot_components: Vec<Vec<u8>>,
    pub properties: Vec<Property>,
    pub changed: bool,
}

/// What mods may read of the game, copied once per tick by the game.
#[derive(Clone, Debug, Default)]
pub struct Snapshot {
    pub player: Option<Player>,
    pub clock: Option<Clock>,
}

/// A server-side mutation requested by a mod via capability `server`.
#[derive(Clone, Debug, PartialEq)]
pub enum ServerAction {
    SetBlock {
        dimension: String,
        x: i32,
        y: i32,
        z: i32,
        state: String,
    },
    ScheduleTick {
        dimension: String,
        x: i32,
        y: i32,
        z: i32,
        block: String,
        delay: u32,
    },
    SpawnDrop {
        dimension: String,
        x: i32,
        y: i32,
        z: i32,
        id: String,
        count: u32,
    },
    PlaySound {
        dimension: String,
        x: i32,
        y: i32,
        z: i32,
        sound: String,
        volume: f32,
        pitch: f32,
    },
    SetRule {
        name: String,
        value: String,
    },
}

/// What mods asked the game to show or apply since the last [`Mods::drain`].
#[derive(Clone, Debug, PartialEq)]
pub enum Output {
    /// `chat.show`: a line after the mod's name.
    Chat { name: String, text: String },
    /// A mod command's feedback, red when `error`.
    Feedback { text: String, error: bool },
    /// The mod was turned off: it trapped, overran or failed to start.
    Stopped { name: String, reason: String },
    /// A server mutation requested via capability `server` (API 0.3).
    Server(ServerAction),
    /// A declarative container screen opened via capability `ui` (API 0.4).
    ScreenOpened { mod_id: String, spec: ScreenSpec },
    /// The mod closed its declarative screen (API 0.4).
    ScreenClosed { mod_id: String },
    /// A packet sent from client to server via `net.send-to-server` (API 0.4).
    PacketToServer {
        mod_id: String,
        channel: String,
        payload: Vec<u8>,
    },
    /// A packet sent from server to client via `net.send-to-client` (API 0.4).
    PacketToClient {
        mod_id: String,
        player: String,
        channel: String,
        payload: Vec<u8>,
    },
    /// A player or entity teleported via `dimensions.teleport` (API 0.5).
    Teleported {
        subject: String,
        dimension: String,
        x: f64,
        y: f64,
        z: f64,
    },
}

/// A mod's text panel, drawn every frame until it changes.
#[derive(Clone, Debug)]
pub struct Panel {
    pub corner: Corner,
    pub lines: Vec<Line>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Status {
    /// Installed and turned off by the player.
    Off,
    Running,
    /// Could not load or start, or stopped while running.
    Failed(String),
}

/// Shared state across mods for API 0.3/0.4 world, capabilities, and recipes.
#[derive(Default)]
struct SharedWorld {
    blocks: HashMap<(String, i32, i32, i32), String>,
    biomes: HashMap<(i32, i32, i32), String>,
    lights: HashMap<(i32, i32, i32), u8>,
    rules: HashMap<String, String>,
    target_block: Option<BlockHit>,
    target_entity: Option<EntityHit>,
    main_hand: Option<StackView>,
    off_hand: Option<StackView>,
    player_inventory: Vec<StackView>,
    open_container: Vec<StackView>,
    energy: HashMap<String, EnergyView>,
    fluids: HashMap<(String, u32), FluidView>,
    item_slots: HashMap<String, BTreeMap<u32, (String, u32)>>,
    attachments: HashMap<(String, String), Vec<u8>>,
    accessories: HashMap<(String, String, u32), (String, u32)>,
    recipes: Vec<RecipeEntry>,
    entities: HashMap<u32, EntityState>,
    next_entity_handle: u32,
    animations: HashMap<String, (String, bool, f32)>,
    bone_poses: HashMap<String, Vec<BonePose>>,
    draw_lists: HashMap<String, Vec<DrawPart>>,
    dimensions: Vec<DimensionSpec>,
    portals: Vec<PortalSpec>,
    damage_multiplier: f32,
    xp_multiplier: f32,
    enchantment_bonus: i32,
    extra_loot: Vec<ItemDrop>,
}

/// One installed mod, for the Mods page.
pub struct Entry {
    pub path: PathBuf,
    /// None when the manifest could not be read (`status` says why).
    pub manifest: Option<Manifest>,
    pub status: Status,
    /// The mod's average time per call of its busiest hook.
    pub cost: Duration,
    run: Option<Running>,
    modified: Option<SystemTime>,
}

struct Running {
    store: Store<State>,
    exports: Exports,
    hooks: Registration,
    /// Its block entity types (API 0.4), checked against its namespaces.
    types: Vec<BlockEntityType>,
    /// Its block, item, server-event and packet hooks (API 0.3 & 0.4).
    logic: LogicHooks,
    /// Its custom entity types (API 0.5), checked against its namespaces.
    entity_types: Vec<EntityType>,
}

struct Exports {
    events: HorizonMod,
    /// Present when the component exports block entities.
    machine: Option<MachineHost>,
    logic: Option<LogicHost>,
    on_block: Option<BlockCallback>,
}

struct BlockCallback {
    func: Func,
    fields: Vec<String>,
}

struct State {
    name: String,
    id: String,
    owned_namespaces: Vec<String>,
    snapshot: Arc<Snapshot>,
    shared: Arc<Mutex<SharedWorld>>,
    out: Vec<Output>,
    panel: Option<Panel>,
    /// Capability `storage`'s folder; made on first `set`.
    data_dir: PathBuf,
    /// Bytes `storage::set` may still write before this hook call ends
    /// (reset per call in [`Mods::call`]): fuel bounds a mod's CPU time but
    /// not disk I/O time, so a loop of small-fuel, large-payload `set`s
    /// needs its own budget.
    storage_budget: u64,
    limits: StoreLimits,
    /// Capability `block-entities`: the game's facts, and the block entities
    /// lent to the current `tick` call (none outside it).
    facts: Option<Arc<dyn ItemFacts>>,
    views: Vec<BlockEntityView>,
    /// Position retained between callbacks so create/save hooks can address capabilities.
    positions: HashMap<u32, (String, i32, i32, i32)>,
    /// Capability `config`: typed schema and values.
    config_schemas: BTreeMap<String, ConfigSchema>,
    configs: BTreeMap<String, ConfigValue>,
    /// Capability `keys`: registered bindings and current key states.
    key_bindings: Vec<KeyBinding>,
    keys_down: HashSet<String>,
    keys_pressed: HashSet<String>,
    /// Capability `ui`: open declarative screen and its slot contents.
    open_screen: Option<ScreenSpec>,
    screen_backgrounds: Vec<ScreenBackground>,
    screen_slots: BTreeMap<u32, (String, u32)>,
    /// Hit face exposed only while dispatching a block callback.
    active_block_face: Option<String>,
    /// Capability `render`: declarative 3D boxes and ghost blocks.
    shapes: Vec<BoxShape>,
    ghost_blocks: Vec<GhostBlock>,
    /// Capability `vanilla-hooks` (API 0.5): subscribed named hooks.
    vanilla_hooks: Vec<VanillaHookKind>,
}

/// The installed mods and the engine running them. `Mods::default()` is
/// the state with none: no engine, and every hook a no-op.
#[derive(Default)]
pub struct Mods {
    engine: Option<(Engine, Linker<State>)>,
    entries: Vec<Entry>,
    snapshot: Arc<Snapshot>,
    shared: Arc<Mutex<SharedWorld>>,
    out: Vec<Output>,
    facts: Option<Arc<dyn ItemFacts>>,
    /// Each live block entity's handle: the entry of the mod that owns it.
    handles: HashMap<u32, usize>,
    next_handle: u32,
    /// The block entities a mod held when it stopped mid-call, saved from
    /// its dropped store so the game gets their slots back.
    stranded: Vec<BlockEntityView>,
}

impl Mods {
    /// Reads every mod at `paths` and starts those `enabled` says are on.
    /// A command that `reserved` (vanilla's commands) or an earlier mod
    /// already uses is refused.
    pub fn load(
        paths: &[PathBuf],
        enabled: impl Fn(&str) -> bool,
        reserved: impl Fn(&str) -> bool,
    ) -> Mods {
        let mut mods = Mods::default();
        for path in paths {
            let modified = modified(path);
            let mut entry = Entry {
                path: path.clone(),
                manifest: None,
                status: Status::Off,
                cost: Duration::ZERO,
                run: None,
                modified,
            };
            match Package::read(path) {
                Err(e) => entry.status = Status::Failed(e),
                Ok(package) => {
                    let on = enabled(&package.manifest.id);
                    entry.manifest = Some(package.manifest.clone());
                    if on {
                        // Content is read before the world loads; a mod whose
                        // content is refused stays off, with the reason.
                        let content = crate::Content::read(path, &package.manifest);
                        entry.status =
                            match content.and_then(|_| mods.start(path, &package, &reserved)) {
                                Ok(run) => {
                                    entry.run = run;
                                    Status::Running
                                }
                                Err(e) => Status::Failed(e),
                            };
                    }
                }
            }
            if let (Status::Failed(reason), Some(m)) = (&entry.status, &entry.manifest) {
                log::warn!("Horizon: mod {} did not start: {reason}", m.id);
            }
            mods.entries.push(entry);
        }
        mods
    }

    fn engine(&mut self) -> Result<&(Engine, Linker<State>), String> {
        if self.engine.is_none() {
            let mut config = Config::new();
            config.wasm_component_model(true).consume_fuel(true);
            let engine = Engine::new(&config).map_err(|e| e.to_string())?;
            let mut linker = Linker::new(&engine);
            HorizonMod::add_to_linker::<State, HasSelf<State>>(&mut linker, |s| s)
                .map_err(|e| e.to_string())?;
            MachineHost::add_to_linker::<State, HasSelf<State>>(&mut linker, |s| s)
                .map_err(|e| e.to_string())?;
            self.engine = Some((engine, linker));
        }
        Ok(self.engine.as_ref().expect("made above"))
    }

    /// Starts a mod's code; `Ok(None)` for a content-only mod, which has
    /// nothing to run (the game reads its content before the world loads).
    /// `path` is the mod's own `.hmod` or folder, for capability `storage`'s
    /// per-mod directory.
    fn start(
        &mut self,
        path: &Path,
        package: &Package,
        reserved: &dyn Fn(&str) -> bool,
    ) -> Result<Option<Running>, String> {
        let m = &package.manifest;
        let Some(wasm) = &package.wasm else {
            return Ok(None);
        };
        // Two running mods with the same id would share one `data_dir`
        // (capability `storage`): the first loaded keeps the id.
        if self
            .entries
            .iter()
            .any(|e| e.run.is_some() && e.manifest.as_ref().is_some_and(|other| other.id == m.id))
        {
            return Err(format!("another running mod already uses id {}", m.id));
        }
        let taken: Vec<String> = self
            .entries
            .iter()
            .filter_map(|e| e.run.as_ref())
            .flat_map(|r| r.hooks.commands.iter().cloned())
            .collect();
        let snapshot = self.snapshot.clone();
        let shared = self.shared.clone();
        let facts = self.facts.clone();
        let (engine, linker) = self.engine()?;
        let component =
            Component::new(engine, wasm).map_err(|e| format!("not a component: {e}"))?;
        let ty = component.component_type();
        let imports: Vec<String> = ty
            .imports(engine)
            .map(|(name, _)| name.to_string())
            .collect();
        m.check_imports(imports.iter().map(String::as_str))?;
        let has_machine = ty
            .exports(engine)
            .any(|(name, _)| name == BLOCK_ENTITIES_EXPORT);
        let data_dir = path
            .parent()
            .unwrap_or(Path::new("."))
            .join(STORAGE_DIR)
            .join(&m.id);
        let configs = m
            .config
            .iter()
            .map(|(k, v)| (k.clone(), v.default.clone()))
            .collect();
        let state = State {
            name: m.name.clone(),
            id: m.id.clone(),
            owned_namespaces: m.owned_namespaces().map(str::to_owned).collect(),
            snapshot,
            shared,
            out: Vec::new(),
            panel: None,
            data_dir,
            storage_budget: STORAGE_WRITE_BUDGET,
            limits: StoreLimitsBuilder::new()
                .memory_size(MEMORY)
                .instances(16)
                .build(),
            facts,
            views: Vec::new(),
            positions: HashMap::new(),
            config_schemas: m.config.clone(),
            configs,
            key_bindings: Vec::new(),
            keys_down: HashSet::new(),
            keys_pressed: HashSet::new(),
            open_screen: None,
            screen_backgrounds: Vec::new(),
            screen_slots: BTreeMap::new(),
            active_block_face: None,
            shapes: Vec::new(),
            ghost_blocks: Vec::new(),
            vanilla_hooks: Vec::new(),
        };
        let mut store = Store::new(engine, state);
        store.limiter(|s| &mut s.limits);
        store.set_fuel(FUEL_INIT).map_err(|e| e.to_string())?;
        let instance = linker
            .instantiate(&mut store, &component)
            .map_err(|e| reason(&e))?;
        let events = HorizonMod::new(&mut store, &instance).map_err(|e| reason(&e))?;
        let machine = match has_machine {
            true => Some(MachineHost::new(&mut store, &instance).map_err(|e| reason(&e))?),
            false => None,
        };
        let (logic_exports, on_block) = if has_machine {
            let logic = LogicHost::new(&mut store, &instance).map_err(|e| reason(&e))?;
            let logic_index =
                instance.get_export_index(&mut store, None, "horizon:mods/logic@0.1.0");
            let callback = logic_index
                .as_ref()
                .and_then(|index| instance.get_export_index(&mut store, Some(index), "on-block"))
                .and_then(|index| instance.get_func(&mut store, &index))
                .map(|func| block_callback(func, &store))
                .transpose()
                .map_err(|e| reason(&e))?;
            (Some(logic), callback)
        } else {
            (None, None)
        };
        let mut hooks = events
            .horizon_mods_events()
            .call_init(&mut store)
            .map_err(|e| reason(&e))??;
        let mut types = Vec::new();
        let mut logic = LogicHooks {
            blocks: Vec::new(),
            items: Vec::new(),
            server_events: Vec::new(),
            channels: Vec::new(),
        };
        let mut entity_types = Vec::new();
        if let Some(machine) = &machine {
            let declared = machine
                .horizon_mods_block_entities()
                .call_types(&mut store)
                .map_err(|e| reason(&e))?;
            if !declared.is_empty() && !m.capabilities.contains(&Capability::BlockEntities) {
                return Err(format!(
                    "has block entities but not capability block-entities in {}",
                    crate::manifest::MANIFEST
                ));
            }
            types = check_types(m, declared)?;
            let declared_logic = logic_exports
                .as_ref()
                .expect("machine logic binding was created")
                .horizon_mods_logic()
                .call_hooks(&mut store)
                .map_err(|e| reason(&e))?;
            logic = check_logic_hooks(m, declared_logic)?;
            if !logic.blocks.is_empty() && on_block.is_none() {
                return Err("registers block callbacks but exports no on-block function".into());
            }
            let declared_entities = machine
                .horizon_mods_mod_entities()
                .call_types(&mut store)
                .map_err(|e| reason(&e))?;
            if !declared_entities.is_empty() && !m.capabilities.contains(&Capability::Entities) {
                return Err(format!(
                    "has custom entities but not capability entities in {}",
                    crate::manifest::MANIFEST
                ));
            }
            entity_types = check_entity_types(m, declared_entities)?;
        }
        hooks.commands.retain(|c| {
            let free = !reserved(c) && !taken.contains(c);
            if !free {
                log::warn!("Horizon: mod {} cannot take /{c}: already a command", m.id);
            }
            free
        });
        self.out.append(&mut store.data_mut().out);
        Ok(Some(Running {
            store,
            exports: Exports {
                events,
                machine,
                logic: logic_exports,
                on_block,
            },
            hooks,
            types,
            logic,
            entity_types,
        }))
    }

    /// True when at least one mod runs: the game skips its mod work otherwise.
    pub fn is_running(&self) -> bool {
        self.entries.iter().any(|e| e.run.is_some())
    }
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }
    /// What mods read until the next snapshot.
    pub fn set_snapshot(&mut self, snapshot: Snapshot) {
        self.snapshot = Arc::new(snapshot);
    }

    /// Updates the player's crosshair block/entity target (capability `target`).
    pub fn set_target(&mut self, block: Option<BlockHit>, entity: Option<EntityHit>) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.target_block = block;
            sh.target_entity = entity;
        }
    }

    /// Updates the read-only inventory snapshot (capability `inventory-read`).
    pub fn set_inventory(
        &mut self,
        main_hand: Option<StackView>,
        off_hand: Option<StackView>,
        player_inventory: Vec<StackView>,
        open_container: Vec<StackView>,
    ) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.main_hand = main_hand;
            sh.off_hand = off_hand;
            sh.player_inventory = player_inventory;
            sh.open_container = open_container;
        }
    }

    /// Sets a game-owned energy buffer. The game uses this to expose block
    /// inventories and energy networks to mod callbacks; machine probes use it
    /// to give a scenario a known starting charge.
    pub fn set_energy_buffer(&self, pos: &str, mut view: EnergyView) {
        view.stored = view.stored.min(view.capacity);
        if let Ok(mut sh) = self.shared.lock() {
            sh.energy.insert(pos.to_owned(), view);
        }
    }

    /// Reads a game-owned energy buffer after mod callbacks have run.
    pub fn energy_buffer(&self, pos: &str) -> Option<EnergyView> {
        self.shared.lock().ok()?.energy.get(pos).copied()
    }

    /// Clears transient energy and fluid buffers for a machine position. Game
    /// save/load probes use this to ensure the reloaded block entity restores
    /// its contents from its serialized state.
    pub fn clear_machine_buffers(&self, pos: &str) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.energy.remove(pos);
            sh.fluids.retain(|(at, _), _| at != pos);
        }
    }

    /// Reads a capability tank from game logic (for item containers and pipes).
    pub fn fluid_tank(&self, pos: &str, tank: u32) -> Option<FluidView> {
        self.shared
            .lock()
            .ok()?
            .fluids
            .get(&(pos.to_owned(), tank))
            .cloned()
    }

    /// Declares a game-owned tank for vanilla interactions before the first mod tick.
    pub fn declare_fluid_tank(&self, pos: &str, tank: u32, capacity: u64) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.fluids
                .entry((pos.to_owned(), tank))
                .or_insert(FluidView {
                    fluid: String::new(),
                    amount: 0,
                    capacity,
                });
        }
    }

    /// Fills a capability tank from game logic; returns the accepted amount.
    pub fn fill_fluid_tank(&self, pos: &str, tank: u32, fluid: &str, amount: u64) -> u64 {
        if fluid.is_empty() || amount == 0 {
            return 0;
        }
        let Ok(mut sh) = self.shared.lock() else {
            return 0;
        };
        let Some(view) = sh.fluids.get_mut(&(pos.to_owned(), tank)) else {
            return 0;
        };
        if view.amount > 0 && view.fluid != fluid {
            return 0;
        }
        let accepted = amount.min(view.capacity.saturating_sub(view.amount));
        if accepted > 0 {
            view.fluid = fluid.to_owned();
            view.amount += accepted;
        }
        accepted
    }

    /// Drains a capability tank from game logic; returns the amount removed.
    pub fn drain_fluid_tank(&self, pos: &str, tank: u32, amount: u64) -> Option<FluidView> {
        let mut sh = self.shared.lock().ok()?;
        let view = sh.fluids.get_mut(&(pos.to_owned(), tank))?;
        if view.amount < amount || amount == 0 {
            return None;
        }
        let drained = FluidView {
            fluid: view.fluid.clone(),
            amount,
            capacity: view.capacity,
        };
        view.amount -= amount;
        if view.amount == 0 {
            view.fluid.clear();
        }
        Some(drained)
    }

    /// Updates a key's down/pressed state for capability `keys`.
    pub fn set_key_state(&mut self, id: &str, down: bool, pressed: bool) {
        for entry in &mut self.entries {
            if let Some(run) = entry.run.as_mut() {
                let s = run.store.data_mut();
                if s.key_bindings.iter().any(|b| b.id == id) {
                    if down {
                        s.keys_down.insert(id.to_owned());
                    } else {
                        s.keys_down.remove(id);
                    }
                    if pressed {
                        s.keys_pressed.insert(id.to_owned());
                    }
                }
            }
        }
    }

    /// Registered keybindings across all running mods.
    pub fn key_bindings(&self) -> impl Iterator<Item = (&str, &KeyBinding)> {
        self.entries
            .iter()
            .filter_map(|e| e.run.as_ref())
            .flat_map(|r| {
                let id = r.store.data().id.as_str();
                r.store.data().key_bindings.iter().map(move |b| (id, b))
            })
    }

    /// Sets a block in the shared world cache (for `server.get-block` and `world.get-block`).
    pub fn set_world_block(&mut self, dimension: &str, x: i32, y: i32, z: i32, state: &str) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.blocks
                .insert((dimension.to_owned(), x, y, z), state.to_owned());
        }
    }

    /// Sets a biome in the shared world cache (for `world.get-biome`).
    pub fn set_world_biome(&mut self, x: i32, y: i32, z: i32, biome: &str) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.biomes.insert((x, y, z), biome.to_owned());
        }
    }

    /// Sets packed light in the shared world cache (for `world.get-light`).
    pub fn set_world_light(&mut self, x: i32, y: i32, z: i32, light: u8) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.lights.insert((x, y, z), light);
        }
    }

    /// Sets a game rule in the shared cache (for `server.get-rule`).
    pub fn set_rule(&mut self, name: &str, value: &str) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.rules.insert(name.to_owned(), value.to_owned());
        }
    }

    /// Registers a recipe in the shared recipe graph (for capability `recipes`).
    pub fn register_recipe(&mut self, entry: RecipeEntry) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.recipes.retain(|r| r.id != entry.id);
            sh.recipes.push(entry);
        }
    }

    /// Adds recipes loaded from the active resource data to the read-only
    /// machine recipe graph. A mod registering the same id can replace it.
    pub fn add_resource_recipes(&mut self, entries: impl IntoIterator<Item = RecipeEntry>) {
        if let Ok(mut sh) = self.shared.lock() {
            for entry in entries {
                sh.recipes.retain(|r| r.id != entry.id);
                sh.recipes.push(entry);
            }
        }
    }

    /// Calls `f` on every running mod whose hooks `wants`, with `fuel`.
    fn each(
        &mut self,
        fuel: u64,
        wants: impl Fn(&Registration) -> bool,
        mut f: impl FnMut(&Exports, &mut Store<State>) -> wasmtime::Result<()>,
    ) {
        for i in 0..self.entries.len() {
            if self.entries[i]
                .run
                .as_ref()
                .is_some_and(|r| wants(&r.hooks))
            {
                self.call(i, fuel, &mut f);
            }
        }
    }

    /// One call into mod `i`; a trap or an overrun turns the mod off.
    fn call<R>(
        &mut self,
        i: usize,
        fuel: u64,
        f: impl FnOnce(&Exports, &mut Store<State>) -> wasmtime::Result<R>,
    ) -> Option<R> {
        let entry = &mut self.entries[i];
        let run = entry.run.as_mut()?;
        run.store.data_mut().snapshot = self.snapshot.clone();
        run.store.data_mut().storage_budget = STORAGE_WRITE_BUDGET;
        run.store.data_mut().facts = self.facts.clone();
        let started = Instant::now();
        let result = run
            .store
            .set_fuel(fuel)
            .and_then(|()| f(&run.exports, &mut run.store));
        // An average over about the last 64 calls.
        entry.cost = (entry.cost * 63 + started.elapsed()) / 64;
        self.out.append(&mut run.store.data_mut().out);
        match result {
            Ok(r) => Some(r),
            Err(e) => {
                let reason = reason(&e);
                log::warn!("Horizon: mod {} stopped: {reason}", run.store.data().id);
                let name = run.store.data().name.clone();
                self.out.push(Output::Stopped {
                    name,
                    reason: reason.clone(),
                });
                entry.status = Status::Failed(reason);
                self.stranded = std::mem::take(&mut run.store.data_mut().views);
                entry.run = None;
                None
            }
        }
    }

    /// A game tick.
    pub fn tick(&mut self, tick: u64) {
        self.each(
            FUEL_TICK,
            |h| h.tick,
            |m, s| m.events.horizon_mods_events().call_tick(s, tick),
        );
    }
    /// A rendered frame, `seconds` after the last.
    pub fn frame(&mut self, seconds: f32) {
        self.each(
            FUEL_FRAME,
            |h| h.frame,
            |m, s| m.events.horizon_mods_events().call_frame(s, seconds),
        );
    }
    /// A chat line the player received, as plain text.
    pub fn chat(&mut self, text: &str) {
        self.each(
            FUEL_EVENT,
            |h| h.chat,
            |m, s| m.events.horizon_mods_events().call_chat(s, text),
        );
    }

    /// Dispatches a block callback (API 0.3) to the mod that registered the block.
    pub fn block_event(&mut self, event: BlockEvent, face: String) -> EventOutcome {
        let block_id = event
            .state
            .split_once('[')
            .map_or(event.state.as_str(), |(id, _)| id);
        let Some(i) = self.entries.iter().position(|e| {
            e.run
                .as_ref()
                .is_some_and(|r| r.logic.blocks.iter().any(|b| b == block_id))
        }) else {
            return EventOutcome::Pass;
        };
        if let Some(run) = self.entries[i].run.as_mut() {
            run.store.data_mut().active_block_face = (!face.is_empty()).then_some(face);
        }
        let outcome = self
            .call(i, FUEL_EVENT, |m, s| {
                let callback = m
                    .on_block
                    .as_ref()
                    .ok_or_else(|| wasmtime::Error::msg("no on-block export"))?;
                call_on_block(callback, s, &event)
            })
            .unwrap_or(EventOutcome::Pass);
        if let Some(run) = self.entries[i].run.as_mut() {
            run.store.data_mut().active_block_face = None;
        }
        outcome
    }

    /// Dispatches an item callback (API 0.3) to the mod that registered the item.
    pub fn item_event(&mut self, event: ItemEvent) -> EventOutcome {
        let Some(i) = self.entries.iter().position(|e| {
            e.run
                .as_ref()
                .is_some_and(|r| r.logic.items.iter().any(|it| it == &event.item))
        }) else {
            return EventOutcome::Pass;
        };
        self.call(i, FUEL_EVENT, |m, s| {
            logic_of(m)?.horizon_mods_logic().call_on_item(s, &event)
        })
        .unwrap_or(EventOutcome::Pass)
    }

    /// Dispatches a server lifecycle or world event (API 0.3) to every subscribed mod.
    pub fn server_event(&mut self, event: ServerEvent) -> EventOutcome {
        let mut outcome = EventOutcome::Pass;
        for i in 0..self.entries.len() {
            let wants = self.entries[i]
                .run
                .as_ref()
                .is_some_and(|r| r.logic.server_events.contains(&event.kind));
            if !wants {
                continue;
            }
            if let Some(res) = self.call(i, FUEL_EVENT, |m, s| {
                logic_of(m)?.horizon_mods_logic().call_on_server(s, &event)
            }) {
                if res == EventOutcome::Cancel {
                    return EventOutcome::Cancel;
                }
                if res == EventOutcome::Consume {
                    outcome = EventOutcome::Consume;
                }
            }
        }
        outcome
    }

    /// Dispatches a slot click in a declarative mod screen (API 0.4).
    pub fn menu_click(&mut self, screen_id: &str, slot: u32, button: u8) {
        let Some(i) = self.entries.iter().position(|e| {
            e.run.as_ref().is_some_and(|r| {
                r.store
                    .data()
                    .open_screen
                    .as_ref()
                    .is_some_and(|sc| sc.id == screen_id)
            })
        }) else {
            return;
        };
        self.call(i, FUEL_EVENT, |m, s| {
            logic_of(m)?
                .horizon_mods_logic()
                .call_on_menu_click(s, screen_id, slot, button)
        });
    }

    /// Dispatches a button click in a declarative mod screen (API 0.4).
    pub fn menu_button(&mut self, screen_id: &str, button: u8) {
        let Some(i) = self.entries.iter().position(|e| {
            e.run.as_ref().is_some_and(|r| {
                r.store
                    .data()
                    .open_screen
                    .as_ref()
                    .is_some_and(|sc| sc.id == screen_id)
            })
        }) else {
            return;
        };
        self.call(i, FUEL_EVENT, |m, s| {
            logic_of(m)?
                .horizon_mods_logic()
                .call_on_menu_button(s, screen_id, button)
        });
    }

    /// Delivers a packet on `channel` to every mod listening on it (API 0.4).
    pub fn deliver_packet(&mut self, channel: &str, payload: &[u8], from_player: Option<&str>) {
        for i in 0..self.entries.len() {
            let wants = self.entries[i]
                .run
                .as_ref()
                .is_some_and(|r| r.logic.channels.iter().any(|c| c == channel));
            if wants {
                self.call(i, FUEL_EVENT, |m, s| {
                    logic_of(m)?.horizon_mods_logic().call_on_packet(
                        s,
                        channel,
                        payload,
                        from_player,
                    )
                });
            }
        }
    }

    /// Whether a running mod owns `/name`.
    pub fn has_command(&self, name: &str) -> bool {
        self.command_owner(name).is_some()
    }
    fn command_owner(&self, name: &str) -> Option<usize> {
        self.entries.iter().position(|e| {
            e.run
                .as_ref()
                .is_some_and(|r| r.hooks.commands.iter().any(|c| c == name))
        })
    }
    /// Runs `line` (without its slash) if a mod owns its command; false
    /// when none does.
    pub fn command(&mut self, line: &str) -> bool {
        let (name, args) = line.split_once(' ').unwrap_or((line, ""));
        let Some(i) = self.command_owner(name) else {
            return false;
        };
        let result = self.call(i, FUEL_EVENT, |m, s| {
            m.events.horizon_mods_events().call_command(s, name, args)
        });
        match result {
            Some(Ok(Some(text))) => self.out.push(Output::Feedback { text, error: false }),
            Some(Err(text)) => self.out.push(Output::Feedback { text, error: true }),
            Some(Ok(None)) | None => {}
        }
        true
    }
    /// The commands running mods own, for suggestions.
    pub fn commands(&self) -> impl Iterator<Item = &str> {
        self.entries
            .iter()
            .filter_map(|e| e.run.as_ref())
            .flat_map(|r| r.hooks.commands.iter().map(String::as_str))
    }

    /// The running mods' HUD panels.
    pub fn panels(&self) -> impl Iterator<Item = &Panel> {
        self.entries
            .iter()
            .filter_map(|e| e.run.as_ref()?.store.data().panel.as_ref())
    }
    /// The running mods' 3D wireframe boxes (capability `render`).
    pub fn shapes(&self) -> impl Iterator<Item = &BoxShape> {
        self.entries
            .iter()
            .filter_map(|e| e.run.as_ref())
            .flat_map(|r| r.store.data().shapes.iter())
    }
    /// The running mods' 3D ghost-block previews (capability `render`).
    pub fn ghost_blocks(&self) -> impl Iterator<Item = &GhostBlock> {
        self.entries
            .iter()
            .filter_map(|e| e.run.as_ref())
            .flat_map(|r| r.store.data().ghost_blocks.iter())
    }
    /// The running mods' currently open declarative screens (capability `ui`).
    pub fn open_screens(&self) -> impl Iterator<Item = (&str, &ScreenSpec)> {
        self.entries.iter().filter_map(|e| {
            let run = e.run.as_ref()?;
            let s = run.store.data();
            Some((s.id.as_str(), s.open_screen.as_ref()?))
        })
    }
    /// Textured regions for each mod's currently open declarative screen.
    pub fn open_screen_backgrounds(&self) -> impl Iterator<Item = (&str, &[ScreenBackground])> {
        self.entries.iter().filter_map(|e| {
            let run = e.run.as_ref()?;
            let s = run.store.data();
            s.open_screen.as_ref()?;
            Some((s.id.as_str(), s.screen_backgrounds.as_slice()))
        })
    }
    /// The game's container closed a mod-owned screen.
    pub fn close_screen(&mut self, screen_id: &str) {
        let Some(entry) = self.entries.iter_mut().find(|e| {
            e.run.as_ref().is_some_and(|run| {
                run.store
                    .data()
                    .open_screen
                    .as_ref()
                    .is_some_and(|spec| spec.id == screen_id)
            })
        }) else {
            return;
        };
        if let Some(run) = entry.run.as_mut() {
            let state = run.store.data_mut();
            state.open_screen = None;
            state.out.push(Output::ScreenClosed {
                mod_id: state.id.clone(),
            });
        }
    }
    /// What mods asked to show since the last call.
    pub fn drain(&mut self) -> Vec<Output> {
        std::mem::take(&mut self.out)
    }

    /// The game's item and recipe facts, for capability `block-entities`.
    pub fn set_item_facts(&mut self, facts: Arc<dyn ItemFacts>) {
        self.facts = Some(facts);
    }

    /// The block entity types of the running mods (API 0.4).
    pub fn block_entity_types(&self) -> impl Iterator<Item = &BlockEntityType> {
        self.entries
            .iter()
            .filter_map(|e| e.run.as_ref())
            .flat_map(|r| r.types.iter())
    }

    /// The running mod whose block entity type is `block`'s, and the type.
    fn block_entity_type(&self, block: &str) -> Option<(usize, &BlockEntityType)> {
        self.entries.iter().enumerate().find_map(|(i, e)| {
            let t = e.run.as_ref()?.types.iter().find(|t| t.block == block)?;
            Some((i, t))
        })
    }

    /// A block entity for `block`, placed (`saved` empty) or loaded (what
    /// [`Mods::save_block_entity`] returned): its handle, and the slots the
    /// game must keep for it. None when no running mod has `block`, or the
    /// mod stopped.
    pub fn create_block_entity(&mut self, block: &str, saved: &[u8]) -> Option<(u32, u32)> {
        self.create_block_entity_at(block, saved, None)
    }

    /// Creates a block entity with its world position available during restore.
    pub fn create_block_entity_at(
        &mut self,
        block: &str,
        saved: &[u8],
        position: Option<(String, i32, i32, i32)>,
    ) -> Option<(u32, u32)> {
        let (i, ty) = self.block_entity_type(block)?;
        let slots = ty.slots;
        let handle = self.next_handle;
        self.next_handle = self.next_handle.wrapping_add(1);
        if let Some(position) = position {
            if let Some(run) = self.entries[i].run.as_mut() {
                run.store.data_mut().positions.insert(handle, position);
            }
        }
        self.call(i, FUEL_EVENT, |m, s| {
            machine_of(m)?
                .horizon_mods_block_entities()
                .call_create(s, handle, block, saved)
        })?;
        self.handles.insert(handle, i);
        Some((handle, slots))
    }

    /// One server tick of the ticking block entities in `views`, one call
    /// per mod. Each mod sees only its own; their slots and properties come
    /// back changed. Views of unknown handles or stopped mods are left alone.
    pub fn tick_block_entities(&mut self, views: &mut [BlockEntityView]) {
        for i in 0..self.entries.len() {
            let Some(run) = self.entries[i].run.as_ref() else {
                continue;
            };
            let ticking = |v: &BlockEntityView| self.handles.get(&v.handle) == Some(&i);
            if !run.types.iter().any(|t| t.ticks) || !views.iter().any(ticking) {
                continue;
            }
            let mine: Vec<usize> = (0..views.len()).filter(|&v| ticking(&views[v])).collect();
            let lent: Vec<BlockEntityView> = mine
                .iter()
                .map(|&v| std::mem::take(&mut views[v]))
                .collect();
            let handles: Vec<u32> = lent.iter().map(|v| v.handle).collect();
            if let Some(run) = self.entries[i].run.as_mut() {
                run.store.data_mut().views = lent;
            }
            self.call(i, FUEL_TICK, |m, s| {
                machine_of(m)?
                    .horizon_mods_block_entities()
                    .call_tick(s, &handles)
            });
            // A mod that stopped in the call left its views in `stranded`,
            // with what it wrote before it stopped.
            let returned = match self.entries[i].run.as_mut() {
                Some(run) => std::mem::take(&mut run.store.data_mut().views),
                None => std::mem::take(&mut self.stranded),
            };
            for (slot, view) in mine.into_iter().zip(returned) {
                views[slot] = view;
            }
        }
    }

    /// What the game saves for the block entity beside its slots.
    pub fn save_block_entity(&mut self, handle: u32) -> Option<Vec<u8>> {
        let i = *self.handles.get(&handle)?;
        self.call(i, FUEL_EVENT, |m, s| {
            machine_of(m)?
                .horizon_mods_block_entities()
                .call_save(s, handle)
        })
    }

    /// The values the block entity's menu shows (progress, fuel).
    pub fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        let Some(&i) = self.handles.get(&handle) else {
            return Vec::new();
        };
        self.call(i, FUEL_EVENT, |m, s| {
            machine_of(m)?
                .horizon_mods_block_entities()
                .call_data(s, handle)
        })
        .unwrap_or_default()
    }

    /// The block entity was broken or unloaded.
    pub fn remove_block_entity(&mut self, handle: u32) {
        if let Some(i) = self.handles.remove(&handle) {
            self.call(i, FUEL_EVENT, |m, s| {
                machine_of(m)?
                    .horizon_mods_block_entities()
                    .call_remove(s, handle)
            });
            if let Some(run) = self.entries[i].run.as_mut() {
                if let Some((dimension, x, y, z)) = run.store.data_mut().positions.remove(&handle) {
                    let pos = format!("{dimension}:{x},{y},{z}");
                    if let Ok(mut shared) = self.shared.lock() {
                        shared.energy.remove(&pos);
                        shared.fluids.retain(|(at, _), _| at != &pos);
                        shared.item_slots.remove(&pos);
                    }
                }
            }
        }
    }

    /// Every running mod's custom entity types (API 0.5).
    pub fn entity_types(&self) -> impl Iterator<Item = (&str, &EntityType)> {
        self.entries
            .iter()
            .filter_map(|e| Some((e.manifest.as_ref()?.id.as_str(), e.run.as_ref()?)))
            .flat_map(|(id, r)| r.entity_types.iter().map(move |t| (id, t)))
    }

    /// Creates a custom entity of `kind` (`<mod>:<entity>`) and returns its handle (API 0.5).
    pub fn create_entity(&mut self, kind: &str, saved: &[u8]) -> Option<u32> {
        let i = self.entries.iter().position(|e| {
            e.run
                .as_ref()
                .is_some_and(|r| r.entity_types.iter().any(|t| t.id == kind))
        })?;
        self.next_handle += 1;
        let handle = self.next_handle;
        self.call(i, FUEL_EVENT, |m, s| {
            machine_of(m)?
                .horizon_mods_mod_entities()
                .call_create(s, handle, kind, saved)
        })?;
        self.handles.insert(handle, i);
        Some(handle)
    }

    /// Ticks custom entities in one batched call per mod (API 0.5).
    pub fn tick_entities(&mut self, handles: &[u32]) {
        for i in 0..self.entries.len() {
            let mine: Vec<u32> = handles
                .iter()
                .copied()
                .filter(|h| self.handles.get(h) == Some(&i))
                .collect();
            if mine.is_empty() {
                continue;
            }
            self.call(i, FUEL_TICK, |m, s| {
                machine_of(m)?
                    .horizon_mods_mod_entities()
                    .call_tick(s, &mine)
            });
        }
    }

    /// Serializes a custom entity's mod state (API 0.5).
    pub fn save_entity(&mut self, handle: u32) -> Option<Vec<u8>> {
        let i = *self.handles.get(&handle)?;
        self.call(i, FUEL_EVENT, |m, s| {
            machine_of(m)?
                .horizon_mods_mod_entities()
                .call_save(s, handle)
        })
    }

    /// Removes a custom entity (API 0.5).
    pub fn remove_entity(&mut self, handle: u32) {
        if let Some(i) = self.handles.remove(&handle) {
            self.call(i, FUEL_EVENT, |m, s| {
                machine_of(m)?
                    .horizon_mods_mod_entities()
                    .call_remove(s, handle)
            });
        }
    }

    /// Dispatches a player right-click interaction on a custom entity (API 0.5).
    pub fn entity_interact(&mut self, handle: u32, player: &str, held_item: &str) -> bool {
        let Some(&i) = self.handles.get(&handle) else {
            return false;
        };
        self.call(i, FUEL_EVENT, |m, s| {
            machine_of(m)?
                .horizon_mods_mod_entities()
                .call_on_interact(s, handle, player, held_item)
        })
        .unwrap_or(false)
    }

    /// Dispatches damage taken by a custom entity; returns true to allow the damage (API 0.5).
    pub fn entity_hurt(&mut self, handle: u32, amount: f32, source: &str) -> bool {
        let Some(&i) = self.handles.get(&handle) else {
            return true;
        };
        self.call(i, FUEL_EVENT, |m, s| {
            machine_of(m)?
                .horizon_mods_mod_entities()
                .call_on_hurt(s, handle, amount, source)
        })
        .unwrap_or(true)
    }

    /// Dispatches a named vanilla hook (`vanilla-hooks`, API 0.5) and returns
    /// `(outcome, damage_multiplier, xp_multiplier, enchantment_bonus, extra_loot)`.
    pub fn vanilla_hook(
        &mut self,
        event: &VanillaHookEvent,
    ) -> (EventOutcome, f32, f32, i32, Vec<ItemDrop>) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.damage_multiplier = 1.0;
            sh.xp_multiplier = 1.0;
            sh.enchantment_bonus = 0;
            sh.extra_loot.clear();
        }
        let mut outcome = EventOutcome::Pass;
        for i in 0..self.entries.len() {
            let subscribed = self.entries[i]
                .run
                .as_ref()
                .is_some_and(|r| r.store.data().vanilla_hooks.contains(&event.kind));
            if !subscribed {
                continue;
            }
            if let Some(res) = self.call(i, FUEL_EVENT, |m, s| {
                logic_of(m)?
                    .horizon_mods_logic()
                    .call_on_vanilla_hook(s, event)
            }) && res != EventOutcome::Pass
            {
                outcome = res;
                if outcome == EventOutcome::Cancel {
                    break;
                }
            }
        }
        if let Ok(mut sh) = self.shared.lock() {
            (
                outcome,
                sh.damage_multiplier,
                sh.xp_multiplier,
                sh.enchantment_bonus,
                std::mem::take(&mut sh.extra_loot),
            )
        } else {
            (outcome, 1.0, 1.0, 0, Vec::new())
        }
    }

    /// Custom dimensions registered by running mods (API 0.5).
    pub fn dimensions(&self) -> Vec<DimensionSpec> {
        self.shared
            .lock()
            .map(|sh| sh.dimensions.clone())
            .unwrap_or_default()
    }

    /// Custom portals registered by running mods (API 0.5).
    pub fn portals(&self) -> Vec<PortalSpec> {
        self.shared
            .lock()
            .map(|sh| sh.portals.clone())
            .unwrap_or_default()
    }

    /// Active model animation on `target` (`block:x,y,z` or `entity:handle`, API 0.5).
    pub fn model_animation(&self, target: &str) -> Option<(String, bool, f32)> {
        self.shared.lock().ok()?.animations.get(target).cloned()
    }

    /// Bone pose overrides on `target` (API 0.5).
    pub fn model_bones(&self, target: &str) -> Vec<BonePose> {
        self.shared
            .lock()
            .ok()
            .and_then(|sh| sh.bone_poses.get(target).cloned())
            .unwrap_or_default()
    }

    /// Declarative draw list on `target` (API 0.5).
    pub fn model_draw_list(&self, target: &str) -> Vec<DrawPart> {
        self.shared
            .lock()
            .ok()
            .and_then(|sh| sh.draw_lists.get(target).cloned())
            .unwrap_or_default()
    }

    /// Restarts the mods whose file changed since they loaded (mod
    /// development: rebuild, and the game picks it up). True when any did.
    pub fn reload_changed(&mut self, reserved: impl Fn(&str) -> bool) -> bool {
        let mut any = false;
        for i in 0..self.entries.len() {
            let entry = &self.entries[i];
            let now = modified(&entry.path);
            let was_on = entry.status != Status::Off;
            if now == entry.modified || !was_on {
                continue;
            }
            any = true;
            self.entries[i].modified = now;
            self.entries[i].run = None;
            // The new instance knows none of the old one's block entities:
            // the game creates them again from their saves.
            self.handles.retain(|_, owner| *owner != i);
            let path = self.entries[i].path.clone();
            let started = Package::read(&path).and_then(|p| {
                self.entries[i].manifest = Some(p.manifest.clone());
                self.start(&path, &p, &reserved)
            });
            let entry = &mut self.entries[i];
            let id = entry.manifest.as_ref().map_or("?", |m| m.id.as_str());
            match started {
                Ok(run) => {
                    log::info!("Horizon: mod {id} reloaded");
                    entry.run = run;
                    entry.status = Status::Running;
                }
                Err(e) => {
                    log::warn!("Horizon: mod {id} did not reload: {e}");
                    entry.status = Status::Failed(e);
                }
            }
        }
        any
    }
}

/// A mod's `block-entities` / `logic` export; an error (the call fails, the
/// mod goes on) for a component without one.
fn machine_of(m: &Exports) -> wasmtime::Result<&MachineHost> {
    m.machine
        .as_ref()
        .ok_or_else(|| wasmtime::Error::msg("no block-entities export"))
}

fn logic_of(m: &Exports) -> wasmtime::Result<&LogicHost> {
    m.logic
        .as_ref()
        .ok_or_else(|| wasmtime::Error::msg("no logic export"))
}

fn call_on_block(
    callback: &BlockCallback,
    store: &mut Store<State>,
    event: &BlockEvent,
) -> wasmtime::Result<EventOutcome> {
    let face = store.data().active_block_face.clone().unwrap_or_default();
    let fields = callback
        .fields
        .iter()
        .map(|field| {
            let value = match field.as_str() {
                "action" => Val::Enum(
                    match event.action {
                        BlockAction::UseWithoutItem => "use-without-item",
                        BlockAction::UseItemOn => "use-item-on",
                        BlockAction::Place => "place",
                        BlockAction::Broken => "broken",
                        BlockAction::Attack => "attack",
                        BlockAction::RandomTick => "random-tick",
                        BlockAction::ScheduledTick => "scheduled-tick",
                        BlockAction::NeighborChanged => "neighbor-changed",
                        BlockAction::EntityInside => "entity-inside",
                    }
                    .into(),
                ),
                "dimension" => Val::String(event.dimension.clone()),
                "x" => Val::S32(event.x),
                "y" => Val::S32(event.y),
                "z" => Val::S32(event.z),
                "face" => Val::String(face.clone()),
                "state" => Val::String(event.state.clone()),
                "actor" => Val::String(event.actor.clone()),
                "held-item" => Val::String(event.held_item.clone()),
                other => {
                    return Err(wasmtime::Error::msg(format!(
                        "unknown on-block field {other}"
                    )));
                }
            };
            Ok((field.clone(), value))
        })
        .collect::<wasmtime::Result<Vec<_>>>()?;
    let mut results = [Val::Enum("pass".into())];
    callback
        .func
        .call(&mut *store, &[Val::Record(fields)], &mut results)?;
    callback.func.post_return(store)?;
    match &results[0] {
        Val::Enum(value) if value == "pass" => Ok(EventOutcome::Pass),
        Val::Enum(value) if value == "consume" => Ok(EventOutcome::Consume),
        Val::Enum(value) if value == "cancel" => Ok(EventOutcome::Cancel),
        other => Err(wasmtime::Error::msg(format!(
            "invalid on-block outcome {other:?}"
        ))),
    }
}

fn block_callback(func: Func, store: &Store<State>) -> wasmtime::Result<BlockCallback> {
    let params = func.params(store);
    let Some((_, wasmtime::component::Type::Record(record))) = params.first() else {
        return Err(wasmtime::Error::msg("on-block has no event record"));
    };
    let fields: Vec<String> = record.fields().map(|field| field.name.to_owned()).collect();
    let required = [
        "action",
        "dimension",
        "x",
        "y",
        "z",
        "state",
        "actor",
        "held-item",
    ];
    if required
        .iter()
        .any(|name| !fields.iter().any(|field| field == name))
        || fields
            .iter()
            .any(|field| !required.contains(&field.as_str()) && field != "face")
    {
        return Err(wasmtime::Error::msg(format!(
            "unsupported on-block record fields: {}",
            fields.join(", ")
        )));
    }
    Ok(BlockCallback { func, fields })
}

/// Checks that `id` is `<namespace>:<path>` where `namespace` belongs to `m`.
fn owns_id(m: &Manifest, id: &str) -> bool {
    id.split_once(':')
        .is_some_and(|(ns, path)| !path.is_empty() && m.owned_namespaces().any(|o| o == ns))
}

fn valid_id(id: &str) -> bool {
    id.split_once(':')
        .is_some_and(|(ns, path)| !ns.is_empty() && !path.is_empty())
}

/// The block entity types a mod declared, if every one is for a block in its
/// own namespaces, declared once, with at most [`MAX_SLOTS`] slots.
fn check_types(m: &Manifest, types: Vec<BlockEntityType>) -> Result<Vec<BlockEntityType>, String> {
    for (n, t) in types.iter().enumerate() {
        if !owns_id(m, &t.block) {
            return Err(format!(
                "declares a block entity for {}, which is not one of its blocks",
                t.block
            ));
        }
        if types[..n].iter().any(|o| o.block == t.block) {
            return Err(format!("declares {} twice", t.block));
        }
        if t.slots > MAX_SLOTS {
            return Err(format!("{} asks for over {MAX_SLOTS} slots", t.block));
        }
    }
    Ok(types)
}

/// Checks that a mod's `logic.hooks` only claims blocks and items in its own
/// namespaces, or valid block/item identifiers when it has the `server` capability.
fn check_logic_hooks(m: &Manifest, hooks: LogicHooks) -> Result<LogicHooks, String> {
    let can_hook_foreign = m.capabilities.contains(&Capability::Server);
    for block in &hooks.blocks {
        if !(owns_id(m, block) || (can_hook_foreign && valid_id(block))) {
            return Err(format!(
                "declares a block hook for {block}, which is not one of its blocks"
            ));
        }
    }
    for item in &hooks.items {
        if !(owns_id(m, item) || (can_hook_foreign && valid_id(item))) {
            return Err(format!(
                "declares an item hook for {item}, which is not one of its items"
            ));
        }
    }
    Ok(hooks)
}

fn check_entity_types(m: &Manifest, types: Vec<EntityType>) -> Result<Vec<EntityType>, String> {
    for (n, t) in types.iter().enumerate() {
        if !owns_id(m, &t.id) {
            return Err(format!(
                "declares an entity {}, which is not in its namespaces",
                t.id
            ));
        }
        if types[..n].iter().any(|o| o.id == t.id) {
            return Err(format!("declares entity {} twice", t.id));
        }
    }
    Ok(types)
}

fn modified(path: &Path) -> Option<SystemTime> {
    let meta = std::fs::metadata(path).ok()?;
    if meta.is_dir() {
        // A folder mod changes when its component does.
        return std::fs::metadata(path.join(crate::manifest::WASM))
            .ok()?
            .modified()
            .ok();
    }
    meta.modified().ok()
}

/// A trap or error, as the player reads it.
fn reason(e: &wasmtime::Error) -> String {
    match e.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => "it took too long (an endless loop?)".into(),
        Some(Trap::UnreachableCodeReached) => "it crashed (a panic)".into(),
        Some(trap) => format!("it crashed: {trap}"),
        None => format!("{e:#}"),
    }
}

fn clip(text: String) -> String {
    match text.char_indices().nth(LINE_CHARS) {
        Some((at, _)) => text[..at].to_string(),
        None => text,
    }
}

impl mod_log::Host for State {
    fn write(&mut self, level: mod_log::Level, message: String) {
        let level = match level {
            mod_log::Level::Debug => log::Level::Debug,
            mod_log::Level::Info => log::Level::Info,
            mod_log::Level::Warn => log::Level::Warn,
            mod_log::Level::Error => log::Level::Error,
        };
        log::log!(level, "[mod {}] {}", self.id, clip(message));
    }
}
impl game::Host for State {
    fn get_player(&mut self) -> Option<Player> {
        self.snapshot.player.clone()
    }
    fn get_clock(&mut self) -> Option<Clock> {
        self.snapshot.clock
    }
}
impl chat::Host for State {
    fn show(&mut self, text: String) {
        let shown = self
            .out
            .iter()
            .filter(|o| matches!(o, Output::Chat { .. }))
            .count();
        if shown < CHAT_PER_CALL {
            let name = self.name.clone();
            self.out.push(Output::Chat {
                name,
                text: clip(text),
            });
        }
    }
}
impl hud::Host for State {
    fn set_panel(&mut self, corner: Corner, lines: Vec<Line>) {
        let lines = lines
            .into_iter()
            .take(HUD_LINES)
            .map(|l| Line {
                text: clip(l.text),
                color: l.color & 0xFF_FFFF,
            })
            .collect();
        self.panel = Some(Panel { corner, lines });
    }
    fn clear(&mut self) {
        self.panel = None;
    }
}
impl storage::Host for State {
    fn get(&mut self, key: String) -> Option<Vec<u8>> {
        storage_key_ok(&key)
            .then(|| std::fs::read(self.data_dir.join(&key)).ok())
            .flatten()
    }
    fn set(&mut self, key: String, value: Vec<u8>) -> Result<(), String> {
        if !storage_key_ok(&key) {
            return Err(format!("{key:?} is not [a-z0-9_.-]{{1,64}}"));
        }
        let len = value.len() as u64;
        if len > self.storage_budget {
            return Err("too much storage traffic in one call".into());
        }
        // Everything already on disk but the file this call replaces.
        let existing: Vec<std::fs::DirEntry> = std::fs::read_dir(&self.data_dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy() != key)
            .collect();
        if existing.len() >= STORAGE_MAX_KEYS {
            return Err(format!("would hold over {STORAGE_MAX_KEYS} keys"));
        }
        let used: u64 = existing
            .iter()
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum();
        if used + len > STORAGE_QUOTA {
            return Err(format!(
                "would use over {} MiB of storage",
                STORAGE_QUOTA >> 20
            ));
        }
        std::fs::create_dir_all(&self.data_dir).map_err(|e| e.to_string())?;
        std::fs::write(self.data_dir.join(&key), value).map_err(|e| e.to_string())?;
        self.storage_budget -= len;
        Ok(())
    }
    fn delete(&mut self, key: String) {
        if storage_key_ok(&key) {
            let _ = std::fs::remove_file(self.data_dir.join(&key));
        }
    }
}

/// A storage key: one path segment of `[a-z0-9_.-]{1,64}`, not `.` or `..`.
/// No `/`, so a key can never leave the mod's own directory.
fn storage_key_ok(key: &str) -> bool {
    let chars_ok = (1..=64).contains(&key.len())
        && key
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_.-".contains(&b));
    chars_ok && key != "." && key != ".."
}

impl State {
    /// The block entity `handle` lent to this call.
    fn view(&mut self, handle: u32) -> Option<&mut BlockEntityView> {
        self.views.iter_mut().find(|v| v.handle == handle)
    }
    fn owns_id(&self, id: &str) -> bool {
        id.split_once(':').is_some_and(|(ns, path)| {
            !path.is_empty() && self.owned_namespaces.iter().any(|o| o == ns)
        })
    }
}
impl machine::Host for State {
    fn get_position(&mut self, handle: u32) -> Option<machine::BlockEntityPosition> {
        if let Some(view) = self.view(handle) {
            return Some(machine::BlockEntityPosition {
                dimension: view.dimension.clone(),
                x: view.x,
                y: view.y,
                z: view.z,
            });
        }
        let (dimension, x, y, z) = self.positions.get(&handle)?.clone();
        Some(machine::BlockEntityPosition { dimension, x, y, z })
    }
    fn get_world_rule(&mut self, name: String) -> Option<String> {
        self.shared.lock().ok()?.rules.get(&name).cloned()
    }
    fn set_world_rule(&mut self, name: String, value: String) -> Result<(), String> {
        if !self.owns_id(&name) {
            return Err(format!("rule {name:?} is outside the mod's namespaces"));
        }
        let Ok(mut shared) = self.shared.lock() else {
            return Err("world state lock poisoned".into());
        };
        shared.rules.insert(name, value);
        Ok(())
    }
    fn get_slot(&mut self, handle: u32, slot: u32) -> Option<Stack> {
        self.view(handle)?
            .slots
            .get(slot as usize)
            .cloned()
            .flatten()
    }
    fn set_slot(&mut self, handle: u32, slot: u32, stack: Option<Stack>) -> bool {
        let stack = match stack {
            Some(s) if s.count > 0 => {
                // Unknown items are refused; with no facts set (a test host)
                // nothing is known about items, so any id passes uncut.
                let max = match &self.facts {
                    Some(facts) => match facts.max_stack_size(&s.id) {
                        0 => return false,
                        max => max,
                    },
                    None => u32::MAX,
                };
                Some(Stack {
                    count: s.count.min(max),
                    id: s.id,
                })
            }
            _ => None,
        };
        let Some(view) = self.view(handle) else {
            return false;
        };
        let Some(cell) = view.slots.get_mut(slot as usize) else {
            return false;
        };
        if *cell != stack {
            let item_changed = cell.as_ref().map(|current| &current.id)
                != stack.as_ref().map(|replacement| &replacement.id);
            *cell = stack;
            if item_changed && let Some(components) = view.slot_components.get_mut(slot as usize) {
                components.clear();
            }
            view.changed = true;
        }
        true
    }
    fn get_property(&mut self, handle: u32, name: String) -> Option<String> {
        let view = self.view(handle)?;
        let p = view.properties.iter().find(|p| p.name == name)?;
        Some(p.value.clone())
    }
    fn set_property(&mut self, handle: u32, name: String, value: String) -> bool {
        let Some(view) = self.view(handle) else {
            return false;
        };
        let Some(p) = view.properties.iter_mut().find(|p| p.name == name) else {
            return false;
        };
        if !p.values.contains(&value) {
            return false;
        }
        if p.value != value {
            p.value = value;
            view.changed = true;
        }
        true
    }
    fn max_stack_size(&mut self, id: String) -> u32 {
        self.facts.as_ref().map_or(0, |f| f.max_stack_size(&id))
    }
    fn burn_time(&mut self, id: String) -> u32 {
        self.facts.as_ref().map_or(0, |f| f.burn_time(&id))
    }
    fn remainder(&mut self, id: String) -> Option<String> {
        self.facts.as_ref()?.remainder(&id)
    }
    fn cooking(&mut self, kind: CookingKind, input: String) -> Option<CookingRecipe> {
        self.facts.as_ref()?.cooking_recipe(kind, &input)
    }
}

impl item_data::Host for State {
    fn get_slot_components(&mut self, handle: u32, slot: u32) -> Vec<u8> {
        let Some(view) = self.view(handle) else {
            return Vec::new();
        };
        if view.slots.get(slot as usize).is_none_or(Option::is_none) {
            return Vec::new();
        }
        view.slot_components
            .get(slot as usize)
            .cloned()
            .unwrap_or_default()
    }

    fn set_slot_components(&mut self, handle: u32, slot: u32, components: Vec<u8>) -> bool {
        if components.len() > MAX_ITEM_COMPONENT_BYTES {
            return false;
        }
        if !components.is_empty()
            && !read_network(&components, NbtLimits::default()).is_ok_and(|(tag, used)| {
                used == components.len() && matches!(tag, NbtTag::Compound(_))
            })
        {
            return false;
        }
        let Some(view) = self.view(handle) else {
            return false;
        };
        if view.slots.get(slot as usize).is_none_or(Option::is_none) {
            return false;
        }
        let Some(value) = view.slot_components.get_mut(slot as usize) else {
            return false;
        };
        if *value != components {
            *value = components;
            view.changed = true;
        }
        true
    }
}

impl config::Host for State {
    fn get_bool(&mut self, key: String) -> Option<bool> {
        match self.configs.get(&key)? {
            ConfigValue::Bool(v) => Some(*v),
            _ => None,
        }
    }
    fn get_int(&mut self, key: String) -> Option<i64> {
        match self.configs.get(&key)? {
            ConfigValue::Int(v) => Some(*v),
            _ => None,
        }
    }
    fn get_float(&mut self, key: String) -> Option<f64> {
        match self.configs.get(&key)? {
            ConfigValue::Float(v) => Some(*v),
            ConfigValue::Int(v) => Some(*v as f64),
            _ => None,
        }
    }
    fn get_string(&mut self, key: String) -> Option<String> {
        match self.configs.get(&key)? {
            ConfigValue::String(v) => Some(v.clone()),
            _ => None,
        }
    }
    fn set_bool(&mut self, key: String, value: bool) -> Result<(), String> {
        let schema = self
            .config_schemas
            .get(&key)
            .ok_or_else(|| format!("unknown config key {key}"))?;
        if schema.kind != "bool" {
            return Err(format!("config {key} is not a bool"));
        }
        self.configs.insert(key, ConfigValue::Bool(value));
        Ok(())
    }
    fn set_int(&mut self, key: String, value: i64) -> Result<(), String> {
        let schema = self
            .config_schemas
            .get(&key)
            .ok_or_else(|| format!("unknown config key {key}"))?;
        if schema.kind != "int" {
            return Err(format!("config {key} is not an int"));
        }
        if let Some(min) = schema.min
            && (value as f64) < min
        {
            return Err(format!("value {value} is below min {min}"));
        }
        if let Some(max) = schema.max
            && (value as f64) > max
        {
            return Err(format!("value {value} is above max {max}"));
        }
        self.configs.insert(key, ConfigValue::Int(value));
        Ok(())
    }
    fn set_float(&mut self, key: String, value: f64) -> Result<(), String> {
        let schema = self
            .config_schemas
            .get(&key)
            .ok_or_else(|| format!("unknown config key {key}"))?;
        if schema.kind != "float" {
            return Err(format!("config {key} is not a float"));
        }
        if let Some(min) = schema.min
            && value < min
        {
            return Err(format!("value {value} is below min {min}"));
        }
        if let Some(max) = schema.max
            && value > max
        {
            return Err(format!("value {value} is above max {max}"));
        }
        self.configs.insert(key, ConfigValue::Float(value));
        Ok(())
    }
    fn set_string(&mut self, key: String, value: String) -> Result<(), String> {
        let schema = self
            .config_schemas
            .get(&key)
            .ok_or_else(|| format!("unknown config key {key}"))?;
        if schema.kind != "string" {
            return Err(format!("config {key} is not a string"));
        }
        if !schema.choices.is_empty() && !schema.choices.contains(&value) {
            return Err(format!("value {value:?} is not in choices"));
        }
        self.configs.insert(key, ConfigValue::String(value));
        Ok(())
    }
}

impl server::Host for State {
    fn get_block_face(&mut self) -> Option<String> {
        self.active_block_face.clone()
    }
    fn get_block(&mut self, dimension: String, pos: BlockPos) -> Option<String> {
        self.shared
            .lock()
            .ok()?
            .blocks
            .get(&(dimension, pos.x, pos.y, pos.z))
            .cloned()
    }
    fn set_block(&mut self, dimension: String, pos: BlockPos, state: String) -> Result<(), String> {
        if state.is_empty() || !state.contains(':') {
            return Err(format!("invalid block state {state:?}"));
        }
        if let Ok(mut sh) = self.shared.lock() {
            sh.blocks
                .insert((dimension.clone(), pos.x, pos.y, pos.z), state.clone());
        }
        self.out.push(Output::Server(ServerAction::SetBlock {
            dimension,
            x: pos.x,
            y: pos.y,
            z: pos.z,
            state,
        }));
        Ok(())
    }
    fn schedule_tick(&mut self, dimension: String, pos: BlockPos, block: String, delay: u32) {
        self.out.push(Output::Server(ServerAction::ScheduleTick {
            dimension,
            x: pos.x,
            y: pos.y,
            z: pos.z,
            block,
            delay,
        }));
    }
    fn spawn_drop(&mut self, dimension: String, pos: BlockPos, drop: ItemDrop) {
        if drop.count > 0 {
            self.out.push(Output::Server(ServerAction::SpawnDrop {
                dimension,
                x: pos.x,
                y: pos.y,
                z: pos.z,
                id: drop.id,
                count: drop.count.min(99),
            }));
        }
    }
    fn play_sound(
        &mut self,
        dimension: String,
        pos: BlockPos,
        sound: String,
        volume: f32,
        pitch: f32,
    ) {
        self.out.push(Output::Server(ServerAction::PlaySound {
            dimension,
            x: pos.x,
            y: pos.y,
            z: pos.z,
            sound,
            volume,
            pitch,
        }));
    }
    fn get_rule(&mut self, name: String) -> Option<String> {
        self.shared.lock().ok()?.rules.get(&name).cloned()
    }
    fn set_rule(&mut self, name: String, value: String) -> Result<(), String> {
        if name.is_empty() {
            return Err("rule name cannot be empty".into());
        }
        if let Ok(mut sh) = self.shared.lock() {
            sh.rules.insert(name.clone(), value.clone());
        }
        self.out
            .push(Output::Server(ServerAction::SetRule { name, value }));
        Ok(())
    }
}

impl keys::Host for State {
    fn register(&mut self, binding: KeyBinding) -> Result<(), String> {
        if !storage_key_ok(&binding.id) {
            return Err(format!(
                "keybinding id {:?} is not [a-z0-9_.-]{{1,64}}",
                binding.id
            ));
        }
        if self.key_bindings.len() >= 32 {
            return Err("at most 32 keybindings per mod".into());
        }
        self.key_bindings.retain(|b| b.id != binding.id);
        self.key_bindings.push(binding);
        Ok(())
    }
    fn is_down(&mut self, id: String) -> bool {
        self.keys_down.contains(&id)
    }
    fn was_pressed(&mut self, id: String) -> bool {
        self.keys_pressed.remove(&id)
    }
}

impl target::Host for State {
    fn get_block(&mut self) -> Option<BlockHit> {
        self.shared.lock().ok()?.target_block.clone()
    }
    fn get_entity(&mut self) -> Option<EntityHit> {
        self.shared.lock().ok()?.target_entity.clone()
    }
}

impl inventory::Host for State {
    fn get_main_hand(&mut self) -> Option<StackView> {
        self.shared.lock().ok()?.main_hand.clone()
    }
    fn get_off_hand(&mut self) -> Option<StackView> {
        self.shared.lock().ok()?.off_hand.clone()
    }
    fn get_player_inventory(&mut self) -> Vec<StackView> {
        self.shared
            .lock()
            .map(|sh| sh.player_inventory.clone())
            .unwrap_or_default()
    }
    fn get_open_container(&mut self) -> Vec<StackView> {
        self.shared
            .lock()
            .map(|sh| sh.open_container.clone())
            .unwrap_or_default()
    }
}

impl capabilities::Host for State {
    fn set_energy_buffer(&mut self, pos: String, mut view: EnergyView) {
        view.stored = view.stored.min(view.capacity);
        if let Ok(mut sh) = self.shared.lock() {
            sh.energy.insert(pos, view);
        }
    }
    fn get_energy(&mut self, pos: String) -> Option<EnergyView> {
        self.shared.lock().ok()?.energy.get(&pos).copied()
    }
    fn receive_energy(&mut self, pos: String, amount: u64, simulate: bool) -> u64 {
        let Ok(mut sh) = self.shared.lock() else {
            return 0;
        };
        let Some(buf) = sh.energy.get_mut(&pos) else {
            return 0;
        };
        let room = buf.capacity.saturating_sub(buf.stored);
        let accepted = amount.min(buf.max_receive).min(room);
        if !simulate {
            buf.stored += accepted;
        }
        accepted
    }
    fn extract_energy(&mut self, pos: String, amount: u64, simulate: bool) -> u64 {
        let Ok(mut sh) = self.shared.lock() else {
            return 0;
        };
        let Some(buf) = sh.energy.get_mut(&pos) else {
            return 0;
        };
        let extracted = amount.min(buf.max_extract).min(buf.stored);
        if !simulate {
            buf.stored -= extracted;
        }
        extracted
    }
    fn set_fluid_tank(&mut self, pos: String, tank: u32, mut view: FluidView) {
        view.amount = view.amount.min(view.capacity);
        if view.amount == 0 {
            view.fluid.clear();
        }
        if let Ok(mut sh) = self.shared.lock() {
            sh.fluids.insert((pos, tank), view);
        }
    }
    fn get_fluid(&mut self, pos: String, tank: u32) -> Option<FluidView> {
        self.shared.lock().ok()?.fluids.get(&(pos, tank)).cloned()
    }
    fn fill_fluid(&mut self, pos: String, fluid: String, amount: u64, simulate: bool) -> u64 {
        self.fill_fluid_tank(pos, 0, fluid, amount, simulate)
    }
    fn fill_fluid_tank(
        &mut self,
        pos: String,
        tank: u32,
        fluid: String,
        amount: u64,
        simulate: bool,
    ) -> u64 {
        if fluid.is_empty() || amount == 0 {
            return 0;
        }
        let Ok(mut sh) = self.shared.lock() else {
            return 0;
        };
        let Some(view) = sh.fluids.get_mut(&(pos, tank)) else {
            return 0;
        };
        if !view.fluid.is_empty() && view.fluid != fluid && view.amount > 0 {
            return 0;
        }
        let room = view.capacity.saturating_sub(view.amount);
        let accepted = amount.min(room);
        if !simulate && accepted > 0 {
            view.fluid = fluid;
            view.amount += accepted;
        }
        accepted
    }
    fn drain_fluid(&mut self, pos: String, fluid: String, amount: u64, simulate: bool) -> u64 {
        self.drain_fluid_tank(pos, 0, fluid, amount, simulate)
    }
    fn drain_fluid_tank(
        &mut self,
        pos: String,
        tank: u32,
        fluid: String,
        amount: u64,
        simulate: bool,
    ) -> u64 {
        let Ok(mut sh) = self.shared.lock() else {
            return 0;
        };
        let Some(view) = sh.fluids.get_mut(&(pos, tank)) else {
            return 0;
        };
        if view.amount == 0 || (!fluid.is_empty() && view.fluid != fluid) {
            return 0;
        }
        let drained = amount.min(view.amount);
        if !simulate {
            view.amount -= drained;
            if view.amount == 0 {
                view.fluid.clear();
            }
        }
        drained
    }
    fn set_item_slot(&mut self, pos: String, slot: u32, id: String, count: u32) {
        if let Ok(mut sh) = self.shared.lock() {
            let map = sh.item_slots.entry(pos).or_default();
            if count == 0 || id.is_empty() {
                map.remove(&slot);
            } else {
                map.insert(slot, (id, count));
            }
        }
    }
    fn get_item_slots(&mut self, pos: String) -> Vec<ItemSlot> {
        let Ok(sh) = self.shared.lock() else {
            return Vec::new();
        };
        sh.item_slots
            .get(&pos)
            .map(|m| {
                m.iter()
                    .map(|(&slot, (id, count))| ItemSlot {
                        slot,
                        id: id.clone(),
                        count: *count,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
    fn get_attachment(&mut self, target: String, key: String) -> Option<Vec<u8>> {
        self.shared
            .lock()
            .ok()?
            .attachments
            .get(&(target, key))
            .cloned()
    }
    fn set_attachment(
        &mut self,
        target: String,
        key: String,
        value: Vec<u8>,
    ) -> Result<(), String> {
        if !self.owns_id(&key) {
            return Err(format!(
                "attachment key {key:?} is not in the mod's namespaces"
            ));
        }
        if value.len() > MAX_ATTACHMENT_BYTES {
            return Err(format!(
                "attachment is over {} KiB",
                MAX_ATTACHMENT_BYTES >> 10
            ));
        }
        if let Ok(mut sh) = self.shared.lock() {
            sh.attachments.insert((target, key), value);
        }
        Ok(())
    }
    fn delete_attachment(&mut self, target: String, key: String) {
        if self.owns_id(&key)
            && let Ok(mut sh) = self.shared.lock()
        {
            sh.attachments.remove(&(target, key));
        }
    }
    fn get_accessory(&mut self, entity: String, slot_kind: String, index: u32) -> Option<ItemSlot> {
        let sh = self.shared.lock().ok()?;
        let (id, count) = sh.accessories.get(&(entity, slot_kind, index))?;
        Some(ItemSlot {
            slot: index,
            id: id.clone(),
            count: *count,
        })
    }
    fn set_accessory(
        &mut self,
        entity: String,
        slot_kind: String,
        index: u32,
        id: String,
        count: u32,
    ) -> Result<(), String> {
        if index >= 16 {
            return Err("accessory slot index out of range (max 16)".into());
        }
        if let Ok(mut sh) = self.shared.lock() {
            if count == 0 || id.is_empty() {
                sh.accessories.remove(&(entity, slot_kind, index));
            } else {
                sh.accessories
                    .insert((entity, slot_kind, index), (id, count));
            }
        }
        Ok(())
    }
}

impl ui::Host for State {
    fn open_screen(&mut self, spec: ScreenSpec) -> Result<(), String> {
        if !self.owns_id(&spec.id) {
            return Err(format!(
                "screen id {:?} is not in the mod's namespaces",
                spec.id
            ));
        }
        self.open_screen = Some(spec.clone());
        self.screen_backgrounds.clear();
        self.out.push(Output::ScreenOpened {
            mod_id: self.id.clone(),
            spec,
        });
        Ok(())
    }
    fn close_screen(&mut self) {
        if self.open_screen.take().is_some() {
            self.screen_backgrounds.clear();
            self.out.push(Output::ScreenClosed {
                mod_id: self.id.clone(),
            });
        }
    }
    fn set_backgrounds(&mut self, screen_id: String, backgrounds: Vec<ScreenBackground>) {
        if self
            .open_screen
            .as_ref()
            .is_some_and(|sc| sc.id == screen_id)
        {
            self.screen_backgrounds = backgrounds.into_iter().take(16).collect();
        }
    }
    fn set_slot(&mut self, screen_id: String, slot: u32, id: String, count: u32) {
        if self
            .open_screen
            .as_ref()
            .is_some_and(|sc| sc.id == screen_id)
        {
            if count == 0 || id.is_empty() {
                self.screen_slots.remove(&slot);
            } else {
                self.screen_slots.insert(slot, (id, count));
            }
        }
    }
    fn set_gauge(
        &mut self,
        screen_id: String,
        widget_id: u32,
        value: u64,
        max_value: u64,
        text: String,
    ) {
        if let Some(sc) = self.open_screen.as_mut()
            && sc.id == screen_id
            && let Some(w) = sc.widgets.iter_mut().find(|w| w.id == widget_id)
        {
            w.value = value;
            w.max_value = max_value;
            w.text = clip(text);
        }
    }
}

impl net::Host for State {
    fn send_to_server(&mut self, channel: String, payload: Vec<u8>) -> Result<(), String> {
        if !self.owns_id(&channel) {
            return Err(format!(
                "channel {channel:?} is not in the mod's namespaces"
            ));
        }
        if payload.len() > MAX_PACKET_BYTES {
            return Err(format!("packet is over {} KiB", MAX_PACKET_BYTES >> 10));
        }
        self.out.push(Output::PacketToServer {
            mod_id: self.id.clone(),
            channel,
            payload,
        });
        Ok(())
    }
    fn send_to_client(
        &mut self,
        player: String,
        channel: String,
        payload: Vec<u8>,
    ) -> Result<(), String> {
        if !self.owns_id(&channel) {
            return Err(format!(
                "channel {channel:?} is not in the mod's namespaces"
            ));
        }
        if payload.len() > MAX_PACKET_BYTES {
            return Err(format!("packet is over {} KiB", MAX_PACKET_BYTES >> 10));
        }
        self.out.push(Output::PacketToClient {
            mod_id: self.id.clone(),
            player,
            channel,
            payload,
        });
        Ok(())
    }
}

impl recipes::Host for State {
    fn get(&mut self, id: String) -> Option<RecipeEntry> {
        self.shared
            .lock()
            .ok()?
            .recipes
            .iter()
            .find(|r| r.id == id)
            .cloned()
    }
    fn by_type(&mut self, kind: String) -> Vec<RecipeEntry> {
        self.shared
            .lock()
            .map(|sh| {
                sh.recipes
                    .iter()
                    .filter(|r| r.kind == kind)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    fn for_input(&mut self, item_id: String) -> Vec<RecipeEntry> {
        self.shared
            .lock()
            .map(|sh| {
                sh.recipes
                    .iter()
                    .filter(|r| {
                        r.inputs
                            .iter()
                            .any(|slot| slot.items.iter().any(|i| i == &item_id))
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    fn for_output(&mut self, item_id: String) -> Vec<RecipeEntry> {
        self.shared
            .lock()
            .map(|sh| {
                sh.recipes
                    .iter()
                    .filter(|r| {
                        r.outputs
                            .iter()
                            .any(|o| o.split_once('*').map_or(o.as_str(), |(id, _)| id) == item_id)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
    fn register(&mut self, entry: RecipeEntry) -> Result<(), String> {
        if !self.owns_id(&entry.id) {
            return Err(format!(
                "recipe id {:?} is not in the mod's namespaces",
                entry.id
            ));
        }
        if let Ok(mut sh) = self.shared.lock() {
            sh.recipes.retain(|r| r.id != entry.id);
            sh.recipes.push(entry);
        }
        Ok(())
    }
}

impl world::Host for State {
    fn get_block(&mut self, x: i32, y: i32, z: i32) -> Option<String> {
        let dim = self
            .snapshot
            .player
            .as_ref()
            .map_or("minecraft:overworld", |p| p.dimension.as_str())
            .to_owned();
        self.shared
            .lock()
            .ok()?
            .blocks
            .get(&(dim, x, y, z))
            .cloned()
    }
    fn get_biome(&mut self, x: i32, y: i32, z: i32) -> Option<String> {
        self.shared.lock().ok()?.biomes.get(&(x, y, z)).cloned()
    }
    fn get_light(&mut self, x: i32, y: i32, z: i32) -> Option<u8> {
        self.shared.lock().ok()?.lights.get(&(x, y, z)).copied()
    }
}

impl render::Host for State {
    fn set_shapes(&mut self, boxes: Vec<BoxShape>) {
        self.shapes = boxes.into_iter().take(MAX_RENDER_SHAPES).collect();
    }
    fn set_ghost_blocks(&mut self, blocks: Vec<GhostBlock>) {
        self.ghost_blocks = blocks.into_iter().take(MAX_GHOST_BLOCKS).collect();
    }
    fn clear(&mut self) {
        self.shapes.clear();
        self.ghost_blocks.clear();
    }
}

impl entities::Host for State {
    fn spawn(
        &mut self,
        dimension: String,
        kind: String,
        x: f64,
        y: f64,
        z: f64,
    ) -> Result<u32, String> {
        if !self.owns_id(&kind) && !kind.starts_with("minecraft:") {
            return Err(format!(
                "entity kind {kind:?} is not a vanilla entity or in the mod's namespaces"
            ));
        }
        let Ok(mut sh) = self.shared.lock() else {
            return Err("world lock poisoned".into());
        };
        sh.next_entity_handle += 1;
        let handle = sh.next_entity_handle;
        sh.entities.insert(
            handle,
            EntityState {
                handle,
                kind,
                dimension,
                x,
                y,
                z,
                vx: 0.0,
                vy: 0.0,
                vz: 0.0,
                yaw: 0.0,
                pitch: 0.0,
                health: 20.0,
                max_health: 20.0,
            },
        );
        Ok(handle)
    }
    fn despawn(&mut self, handle: u32) -> bool {
        self.shared
            .lock()
            .ok()
            .is_some_and(|mut sh| sh.entities.remove(&handle).is_some())
    }
    fn get_state(&mut self, handle: u32) -> Option<EntityState> {
        self.shared.lock().ok()?.entities.get(&handle).cloned()
    }
    fn set_position(&mut self, handle: u32, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) -> bool {
        let Ok(mut sh) = self.shared.lock() else {
            return false;
        };
        let Some(ent) = sh.entities.get_mut(&handle) else {
            return false;
        };
        ent.x = x;
        ent.y = y;
        ent.z = z;
        ent.yaw = yaw;
        ent.pitch = pitch;
        true
    }
    fn set_velocity(&mut self, handle: u32, vx: f64, vy: f64, vz: f64) -> bool {
        let Ok(mut sh) = self.shared.lock() else {
            return false;
        };
        let Some(ent) = sh.entities.get_mut(&handle) else {
            return false;
        };
        ent.vx = vx;
        ent.vy = vy;
        ent.vz = vz;
        true
    }
    fn set_health(&mut self, handle: u32, health: f32) -> bool {
        let Ok(mut sh) = self.shared.lock() else {
            return false;
        };
        let Some(ent) = sh.entities.get_mut(&handle) else {
            return false;
        };
        ent.health = health.clamp(0.0, ent.max_health);
        true
    }
    fn navigate_to(&mut self, handle: u32, x: f64, y: f64, z: f64, speed: f32) -> bool {
        let Ok(mut sh) = self.shared.lock() else {
            return false;
        };
        let Some(ent) = sh.entities.get_mut(&handle) else {
            return false;
        };
        let dx = x - ent.x;
        let dy = y - ent.y;
        let dz = z - ent.z;
        let dist = (dx * dx + dy * dy + dz * dz).sqrt();
        if dist > 1e-5 {
            let step = (speed as f64).min(dist);
            ent.vx = dx / dist * step;
            ent.vy = dy / dist * step;
            ent.vz = dz / dist * step;
        }
        true
    }
    fn nearby(
        &mut self,
        dimension: String,
        x: f64,
        y: f64,
        z: f64,
        radius: f64,
    ) -> Vec<EntityState> {
        let r2 = radius * radius;
        self.shared
            .lock()
            .map(|sh| {
                sh.entities
                    .values()
                    .filter(|e| {
                        e.dimension == dimension && {
                            let dx = e.x - x;
                            let dy = e.y - y;
                            let dz = e.z - z;
                            dx * dx + dy * dy + dz * dz <= r2
                        }
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl models::Host for State {
    fn set_animation(&mut self, target: String, animation: String, looping: bool, speed: f32) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.animations
                .insert(target, (clip(animation), looping, speed));
        }
    }
    fn set_bone_poses(&mut self, target: String, poses: Vec<BonePose>) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.bone_poses
                .insert(target, poses.into_iter().take(64).collect());
        }
    }
    fn set_draw_list(&mut self, target: String, parts: Vec<DrawPart>) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.draw_lists
                .insert(target, parts.into_iter().take(32).collect());
        }
    }
}

impl dimensions::Host for State {
    fn register_dimension(&mut self, spec: DimensionSpec) -> Result<(), String> {
        if !self.owns_id(&spec.id) {
            return Err(format!(
                "dimension id {:?} is not in the mod's namespaces",
                spec.id
            ));
        }
        if spec.height == 0 || (spec.height & 15) != 0 || (spec.min_y & 15) != 0 {
            return Err("dimension min_y and height must be multiples of 16".into());
        }
        if let Ok(mut sh) = self.shared.lock() {
            sh.dimensions.retain(|d| d.id != spec.id);
            sh.dimensions.push(spec);
        }
        Ok(())
    }
    fn register_portal(&mut self, spec: PortalSpec) -> Result<(), String> {
        if !self.owns_id(&spec.id) {
            return Err(format!(
                "portal id {:?} is not in the mod's namespaces",
                spec.id
            ));
        }
        if let Ok(mut sh) = self.shared.lock() {
            sh.portals.retain(|p| p.id != spec.id);
            sh.portals.push(spec);
        }
        Ok(())
    }
    fn teleport(
        &mut self,
        subject: String,
        dimension: String,
        x: f64,
        y: f64,
        z: f64,
    ) -> Result<(), String> {
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err("teleport coordinates must be finite".into());
        }
        self.out.push(Output::Teleported {
            subject,
            dimension,
            x,
            y,
            z,
        });
        Ok(())
    }
    fn list_dimensions(&mut self) -> Vec<String> {
        let mut out = vec![
            "minecraft:overworld".to_owned(),
            "minecraft:the_nether".to_owned(),
            "minecraft:the_end".to_owned(),
        ];
        if let Ok(sh) = self.shared.lock() {
            for d in &sh.dimensions {
                if !out.contains(&d.id) {
                    out.push(d.id.clone());
                }
            }
        }
        out
    }
}

impl vanilla_hooks::Host for State {
    fn subscribe(&mut self, hook: String) -> Result<(), String> {
        let kind = match hook.as_str() {
            "loot-drop" => VanillaHookKind::LootDrop,
            "entity-damage" => VanillaHookKind::EntityDamage,
            "xp-gain" => VanillaHookKind::XpGain,
            "crop-grow" => VanillaHookKind::CropGrow,
            "explosion-detonate" => VanillaHookKind::ExplosionDetonate,
            "enchantment-level" => VanillaHookKind::EnchantmentLevel,
            _ => return Err(format!("unknown vanilla hook {hook:?}")),
        };
        if !self.vanilla_hooks.contains(&kind) {
            self.vanilla_hooks.push(kind);
        }
        Ok(())
    }
    fn set_damage_multiplier(&mut self, factor: f32) {
        if factor.is_finite()
            && factor >= 0.0
            && let Ok(mut sh) = self.shared.lock()
        {
            sh.damage_multiplier *= factor;
        }
    }
    fn add_loot_drop(&mut self, item: String, count: u32) {
        if count > 0
            && !item.is_empty()
            && let Ok(mut sh) = self.shared.lock()
            && sh.extra_loot.len() < 32
        {
            sh.extra_loot.push(ItemDrop { id: item, count });
        }
    }
    fn set_xp_multiplier(&mut self, factor: f32) {
        if factor.is_finite()
            && factor >= 0.0
            && let Ok(mut sh) = self.shared.lock()
        {
            sh.xp_multiplier *= factor;
        }
    }
    fn add_enchantment_bonus(&mut self, bonus: i32) {
        if let Ok(mut sh) = self.shared.lock() {
            sh.enchantment_bonus = sh.enchantment_bonus.saturating_add(bonus);
        }
    }
}

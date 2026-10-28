//! Horizon mods in the game (`docs/mods.md`): loading the ones turned on,
//! their tick, frame, chat and command hooks, their HUD panels and what
//! they print. With no mod on, `horizon_mods::Mods` holds no engine and
//! every hook here returns after one check.
//!
//! Code mods run only in builds with the `mods` feature (release builds):
//! without it the game links no wasmtime, and every hook does nothing.
//! Content mods (`content_root`) load in every build.
use super::Game;
#[cfg(feature = "mods")]
use horizon_client::hud::rgba;
use horizon_client::{
    command::{Dispatcher, vanilla},
    extras_hud::Corners,
    hud::HudBatch,
    mod_list,
};
use std::{path::PathBuf, time::Duration};

#[derive(Clone, Debug)]
#[cfg_attr(not(feature = "mods"), allow(dead_code))]
pub(super) struct ModScreenView {
    pub(super) title: String,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) backgrounds: Vec<ModScreenBackground>,
    pub(super) widgets: Vec<ModWidgetView>,
    pub(super) slot_items: Vec<(u32, String, u32)>,
}
#[derive(Clone, Debug)]
#[cfg_attr(not(feature = "mods"), allow(dead_code))]
pub(super) struct ModScreenBackground {
    pub(super) texture: String,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: u32,
    pub(super) height: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(not(feature = "mods"), allow(dead_code))]
pub(super) enum ModWidgetKind {
    Slot,
    ProgressArrow,
    Flame,
    EnergyBar,
    FluidGauge,
    Label,
    Button,
}
#[derive(Clone, Debug)]
#[cfg_attr(not(feature = "mods"), allow(dead_code))]
pub(super) struct ModWidgetView {
    pub(super) id: u32,
    pub(super) kind: ModWidgetKind,
    pub(super) x: i32,
    pub(super) y: i32,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) text: String,
    pub(super) value: u64,
    pub(super) max_value: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ModHookOutcome {
    Pass,
    #[cfg_attr(not(feature = "mods"), allow(dead_code))]
    Consume,
    Cancel,
}
impl ModHookOutcome {
    pub(super) fn keeps_vanilla_drops(self) -> bool {
        self == Self::Pass
    }
    pub(super) fn keeps_mod_drops(self) -> bool {
        self != Self::Cancel
    }
}

#[derive(Clone, Debug)]
pub(super) struct ModItemDrop {
    pub(super) id: String,
    pub(super) count: u32,
}

#[cfg(feature = "mods")]
use {
    super::{Screen, integrated::ModPacket},
    horizon_client::{
        extras::Corner,
        extras_hud,
        items::ItemStack,
        mod_list::Installed,
        text::{self, Text},
    },
    horizon_core::ticks::ScheduleResult,
    horizon_mods::{
        BlockAction, BlockEntityView, BlockEvent, BlockHit, Clock, CookingKind, CookingRecipe,
        EventOutcome, ItemAction, ItemDrop, ItemEvent, ItemFacts, Mods, Output, Player,
        ServerAction, ServerEvent, ServerEventKind, Snapshot, Stack, StackView, Status,
        VanillaHookEvent, VanillaHookKind, WorldCell,
    },
    std::{
        collections::{HashMap, HashSet},
        sync::Arc,
        time::Instant,
    },
};

#[cfg(feature = "mods")]
struct GameFacts(Arc<super::Resources>);
#[cfg(feature = "mods")]
impl ItemFacts for GameFacts {
    fn max_stack_size(&self, id: &str) -> u32 {
        self.0
            .rest
            .get()
            .and_then(|r| r.items.get(id))
            .map_or(0, |i| i.max_stack)
    }
    fn burn_time(&self, id: &str) -> u32 {
        let Some(r) = self.0.rest.get() else {
            return 0;
        };
        let stack = ItemStack::new(id, 1);
        r.providers.burn_time(&r.items, &stack, false).max(0) as u32
    }
    fn remainder(&self, id: &str) -> Option<String> {
        self.0.rest.get()?.items.get(id)?.remainder.clone()
    }
    fn cooking_recipe(&self, kind: CookingKind, input: &str) -> Option<CookingRecipe> {
        let r = self.0.rest.get()?;
        let kind = match kind {
            CookingKind::Smelting => "minecraft:smelting",
            CookingKind::Blasting => "minecraft:blasting",
            CookingKind::Smoking => "minecraft:smoking",
            CookingKind::Campfire => "minecraft:campfire_cooking",
        };
        let c = r.recipes.cook(kind, &ItemStack::new(input, 1))?;
        Some(CookingRecipe {
            output: Stack {
                id: c.result.item.to_string(),
                count: c.result.count,
            },
            cooking_time: c.time,
            experience: c.experience,
        })
    }
    fn item_in_tag(&self, id: &str, tag: &str) -> bool {
        self.0
            .rest
            .get()
            .and_then(|r| r.items.item_tags.get(tag))
            .is_some_and(|items| items.contains(id))
    }
}

#[cfg(feature = "mods")]
fn item_components(stack: &ItemStack) -> Vec<u8> {
    horizon_client::item_data::encode_components(stack)
        .and_then(|tag| {
            horizon_world::nbt::write_network(&tag, horizon_world::nbt::Limits::default()).ok()
        })
        .unwrap_or_default()
}

#[cfg(feature = "mods")]
fn mod_block_state(text: &str) -> Option<horizon_world::chunk::BlockState> {
    let (name, properties) = match text.split_once('[') {
        Some((name, rest)) if rest.ends_with(']') => (name, &rest[..rest.len() - 1]),
        Some(_) => return None,
        None => (text, ""),
    };
    if name.is_empty() {
        return None;
    }
    let name = if name.contains(':') {
        name.to_owned()
    } else {
        format!("minecraft:{name}")
    };
    let mut parsed = std::collections::BTreeMap::new();
    for pair in properties.split(',').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=')?;
        if key.is_empty()
            || value.is_empty()
            || parsed.insert(key.to_owned(), value.to_owned()).is_some()
        {
            return None;
        }
    }
    Some(horizon_world::chunk::BlockState {
        name,
        properties: parsed,
    })
}

/// Runs the API damage hook from inside the Mob simulation tick without
/// borrowing the whole Game while its world snapshot is borrowed.
pub(super) fn mob_tick_damage_hook(
    mods: &mut ModsState,
    dimension: &str,
    mob: &horizon_client::mobs::Mob,
    amount: f32,
    source: horizon_client::mobs::Damage,
) -> Option<f32> {
    #[cfg(feature = "mods")]
    {
        if !mods.mods.is_running() {
            return Some(amount);
        }
        let kind = &mob.kind.id;
        let subject = if kind.contains(':') {
            kind.clone()
        } else {
            format!("minecraft:{kind}")
        };
        let position = [mob.position.x, mob.position.y, mob.position.z].map(|v| v.floor() as i32);
        let context = match source.kind {
            horizon_client::mobs::DamageKind::PlayerAttack => "minecraft:player_attack",
            horizon_client::mobs::DamageKind::MaceSmash => "minecraft:mace_smash",
            horizon_client::mobs::DamageKind::MobAttack => "minecraft:mob_attack",
            horizon_client::mobs::DamageKind::Arrow => "minecraft:arrow",
            horizon_client::mobs::DamageKind::Thrown => "minecraft:thrown",
            horizon_client::mobs::DamageKind::Fireball => "minecraft:fireball",
            horizon_client::mobs::DamageKind::Explosion => "minecraft:explosion",
            horizon_client::mobs::DamageKind::Fall => "minecraft:fall",
            horizon_client::mobs::DamageKind::FallingBlock => "minecraft:falling_block",
            horizon_client::mobs::DamageKind::OnFire => "minecraft:on_fire",
            horizon_client::mobs::DamageKind::InFire => "minecraft:in_fire",
            horizon_client::mobs::DamageKind::Lava => "minecraft:lava",
            horizon_client::mobs::DamageKind::Drown => "minecraft:drown",
            horizon_client::mobs::DamageKind::Magic => "minecraft:magic",
            horizon_client::mobs::DamageKind::Sting => "minecraft:sting",
            horizon_client::mobs::DamageKind::Cactus => "minecraft:cactus",
            horizon_client::mobs::DamageKind::Generic => "minecraft:generic",
            horizon_client::mobs::DamageKind::InWall => "minecraft:in_wall",
            horizon_client::mobs::DamageKind::Thorns => "minecraft:thorns",
            horizon_client::mobs::DamageKind::SonicBoom => "minecraft:sonic_boom",
            horizon_client::mobs::DamageKind::OutOfWorld => "minecraft:out_of_world",
        };
        let (outcome, multiplier, _, _, _) = mods.mods.vanilla_hook(&VanillaHookEvent {
            kind: VanillaHookKind::EntityDamage,
            subject,
            dimension: dimension.to_owned(),
            x: position[0],
            y: position[1],
            z: position[2],
            value: amount,
            context: context.to_owned(),
        });
        if outcome == EventOutcome::Cancel {
            None
        } else {
            Some(amount * multiplier)
        }
    }
    #[cfg(not(feature = "mods"))]
    {
        let _ = (mods, dimension, mob, source);
        Some(amount)
    }
}

#[cfg(all(test, feature = "mods"))]
mod block_state_tests {
    use super::mod_block_state;

    #[test]
    fn server_block_states_keep_properties_and_reject_malformed_values() {
        let state = mod_block_state("test:machine[active=true,facing=north]").unwrap();
        assert_eq!(state.name, "test:machine");
        assert_eq!(state.properties["active"], "true");
        assert_eq!(state.properties["facing"], "north");
        assert_eq!(mod_block_state("stone").unwrap().name, "minecraft:stone");
        assert!(mod_block_state("test:machine[active").is_none());
        assert!(mod_block_state("test:machine[active=true,active=false]").is_none());
    }
}

#[cfg(feature = "mods")]
fn restore_item_components(stack: &mut ItemStack, encoded: &[u8]) {
    if encoded.is_empty() {
        return;
    }
    let Ok((tag, used)) =
        horizon_world::nbt::read_network(encoded, horizon_world::nbt::Limits::default())
    else {
        return;
    };
    if used != encoded.len() {
        return;
    }
    let (damage, extra) = horizon_client::item_data::decode_components(Some(&tag));
    stack.damage = damage;
    stack.extra = extra;
}

#[derive(Default)]
pub(super) struct ModsState {
    pub(super) local_player_session_active: bool,
    #[cfg(feature = "mods")]
    pub(super) dispatching_neighbor_hooks: bool,
    #[cfg(feature = "mods")]
    mods: Mods,
    /// Server mutations waiting for destination chunks to finish loading.
    #[cfg(feature = "mods")]
    pub(super) pending_server_actions: Vec<(ServerAction, i64)>,
    /// Temporary forced tickets owned by queued mod actions or scheduled ticks.
    #[cfg(feature = "mods")]
    mod_action_tickets: HashSet<(String, i32, i32)>,
    /// Mod scheduled ticks that still need their chunk's temporary ticket.
    #[cfg(feature = "mods")]
    scheduled_mod_action_tickets: HashMap<(String, i32, i32, i32), (i32, i32, i64)>,
    /// `--dev-mod PATH`: always on, reloaded when rebuilt.
    pub(super) dev: Vec<PathBuf>,
    /// When dev mods were last checked for a rebuild.
    #[cfg(feature = "mods")]
    checked: Option<Instant>,
    /// The chat's command tree while a running mod owns commands.
    #[cfg(feature = "mods")]
    tree: Option<Dispatcher>,
    /// Active mod block entities in the loaded world, keyed by `[x, y, z]`.
    #[cfg(feature = "mods")]
    block_entities: HashMap<[i32; 3], BlockEntityView>,
    #[cfg(feature = "mods")]
    resource_recipes_loaded: bool,
    #[cfg(feature = "mods")]
    thermal_evaporation_plants: HashMap<[i32; 3], Vec<[i32; 3]>>,
    #[cfg(feature = "mods")]
    thermal_evaporation_checked_tick: u64,
    #[cfg(feature = "mods")]
    induction_matrix_ports: HashMap<[i32; 3], String>,
    #[cfg(feature = "mods")]
    dynamic_tank_valves: HashMap<[i32; 3], String>,
    #[cfg(feature = "mods")]
    turbine_valves: HashMap<[i32; 3], String>,
    #[cfg(feature = "mods")]
    fission_reactor_ports: HashMap<[i32; 3], String>,
    #[cfg(feature = "mods")]
    fusion_reactor_components: HashMap<[i32; 3], String>,
}

#[cfg(feature = "mods")]
struct InductionMatrixInfo {
    min: [i32; 3],
    capacity: u64,
    transfer: u64,
    ports: Vec<[i32; 3]>,
}

#[cfg(feature = "mods")]
struct DynamicTankInfo {
    min: [i32; 3],
    volume: u64,
    valves: Vec<[i32; 3]>,
}

#[cfg(feature = "mods")]
struct TurbineInfo {
    min: [i32; 3],
    volume: u64,
    max_rate: u64,
    rotor_height: u64,
    rotors: u64,
    valves: Vec<[i32; 3]>,
}

#[cfg(feature = "mods")]
struct FissionReactorInfo {
    min: [i32; 3],
    size: [i32; 3],
    ports: Vec<[i32; 3]>,
    fuel_assemblies: u64,
}

#[cfg(feature = "mods")]
struct FusionReactorInfo {
    min: [i32; 3],
    ports: Vec<[i32; 3]>,
    controller: [i32; 3],
}
impl ModsState {
    /// The chat's command tree: vanilla's, plus the running mods' commands.
    pub(super) fn dispatcher(&self) -> &Dispatcher {
        #[cfg(feature = "mods")]
        if let Some(tree) = &self.tree {
            return tree;
        }
        vanilla::game_dispatcher()
    }
    /// Whether a mod runs: the chat keeps its lines for them.
    pub(super) fn running(&self) -> bool {
        #[cfg(feature = "mods")]
        if self.mods.is_running() {
            return true;
        }
        false
    }
    /// Whether a mod draws a HUD panel.
    pub(super) fn has_panels(&self) -> bool {
        #[cfg(feature = "mods")]
        if self.mods.panels().next().is_some() {
            return true;
        }
        false
    }
}
/// How often dev mods are checked for a rebuild.
#[cfg(feature = "mods")]
const RELOAD_CHECK: Duration = Duration::from_secs(1);

/// A vanilla command name: mods may not take it.
#[cfg(feature = "mods")]
fn vanilla_command(name: &str) -> bool {
    let d = vanilla::game_dispatcher();
    d.nodes[0]
        .children
        .iter()
        .any(|&c| d.nodes[c].name() == name)
}

/// `root` with the content mods that are on layered over it
/// (docs/modpacks.md): their blocks, items, data and assets. Read once at
/// start, before the resources load; a mod whose content is refused is left
/// out here and its row on the Mods page says why.
pub(super) fn content_root(
    root: &std::path::Path,
    enabled: &std::collections::BTreeSet<String>,
    dev: &[std::path::PathBuf],
) -> std::path::PathBuf {
    use horizon_client::mod_content::{self, ContentMod};
    use horizon_mods::{Capability, Content, Package};
    let started = std::time::Instant::now();
    let here = std::env::current_dir().unwrap_or_default();
    let mut paths = horizon_mods::find(&here.join(mod_list::DIR));
    paths.retain(|p| !dev.contains(p));
    paths.extend(dev.iter().cloned());
    let mut mods: Vec<ContentMod> = Vec::new();
    for path in &paths {
        let Ok(package) = Package::read(path) else {
            continue;
        };
        let m = &package.manifest;
        let on = enabled.contains(&m.id) || dev.contains(path);
        if !on
            || !m.capabilities.contains(&Capability::Registries)
            || mods.iter().any(|c| c.id == m.id)
        {
            continue;
        }
        let content = Content::read(path, m).and_then(|content| {
            // The first of two mods owning one namespace stays on.
            let own = (m.id.as_str(), content.namespaces.as_slice());
            match mods
                .iter()
                .find_map(|c| horizon_mods::content::clash((&c.id, &c.content.namespaces), own))
            {
                Some(e) => Err(e),
                None => Ok(content),
            }
        });
        match content {
            Ok(content) => mods.push(ContentMod {
                id: m.id.clone(),
                content,
            }),
            Err(e) => log::warn!("Horizon: mod {} content not loaded: {e}", m.id),
        }
    }
    match mod_content::resource_root(root, &mods, &here.join(".horizon-cache")) {
        Ok(out) => {
            log::info!(
                "Horizon: content root ready for {} mod(s) in {:.1}s",
                mods.len(),
                started.elapsed().as_secs_f32()
            );
            if out != root {
                log::info!(
                    "Horizon: content from {} mod(s) in {}",
                    mods.len(),
                    out.display()
                );
            }
            out
        }
        Err(e) => {
            log::warn!("Horizon: content mods not loaded: {e}");
            root.to_owned()
        }
    }
}

/// The Mods page's rows while [`StartupMenu`] shows before [`Game::start_mods`] runs.
pub(super) fn list_installed_early(dev: &[PathBuf]) -> Vec<horizon_client::mod_list::Installed> {
    let dir = Game::options_path().with_file_name(mod_list::DIR);
    let mut paths = horizon_mods::find(&dir);
    paths.retain(|p| !dev.contains(p));
    paths.extend(dev.iter().cloned());
    paths
        .iter()
        .map(|path| {
            let file = path.file_name().unwrap_or_default().to_string_lossy();
            match horizon_mods::Package::read(path) {
                Ok(pkg) => {
                    let m = pkg.manifest;
                    let may: Vec<&str> = m.capabilities.iter().map(|c| c.describe()).collect();
                    let may = if may.is_empty() {
                        "Asks for nothing.".to_owned()
                    } else {
                        format!("May {}.", may.join(", "))
                    };
                    horizon_client::mod_list::Installed {
                        id: m.id,
                        title: format!("{} {}", m.name, m.version),
                        description: format!("{may} {}", m.description).trim().to_owned(),
                        problem: None,
                        dev: dev.contains(path),
                    }
                }
                Err(e) => horizon_client::mod_list::Installed {
                    id: String::new(),
                    title: file.into_owned(),
                    description: String::new(),
                    problem: Some(format!("Stopped: {e}")),
                    dev: false,
                },
            }
        })
        .collect()
}

/// A build without the `mods` feature: no mod runs.
#[cfg(not(feature = "mods"))]
impl Game<'_> {
    pub(super) fn mod_use_item_in_air(&mut self, _item: &str) -> bool {
        false
    }

    pub(super) fn mod_use_item_on_block(&mut self, _item: &str, _pos: [i32; 3]) -> bool {
        false
    }

    pub(super) fn mod_attack_entity(&mut self, _item: &str, _target: &str, _pos: [i32; 3]) -> bool {
        false
    }

    pub(super) fn mod_attack_block(&mut self, _pos: [i32; 3]) -> bool {
        false
    }

    pub(super) fn mod_has_block_hook(&self, _block: &str) -> bool {
        false
    }

    pub(super) fn mod_block_broken(&mut self, _pos: [i32; 3]) -> bool {
        false
    }

    pub(super) fn mod_block_placed(&mut self, _pos: [i32; 3], _face: String) {}

    pub(super) fn mod_scheduled_block_tick(&mut self, _pos: [i32; 3]) {}

    pub(super) fn mod_entity_inside(&mut self, _pos: [i32; 3], _actor: String) {}

    pub(super) fn mod_server_event(
        &mut self,
        _kind: &str,
        _subject: String,
        _pos: [i32; 3],
        _detail: String,
    ) {
    }

    pub(super) fn mod_server_event_at(
        &mut self,
        _kind: &str,
        _subject: String,
        _dimension: String,
        _pos: [i32; 3],
        _detail: String,
    ) {
    }

    pub(super) fn mod_local_player_joined(&mut self, _pos: [i32; 3]) {}

    pub(super) fn mod_local_player_left(&mut self) {}

    pub(super) fn reset_mod_player_session(&mut self) {
        self.mods.local_player_session_active = false;
    }

    pub(super) fn dispatch_mod_neighbor_events(&mut self) {
        self.block_ticks.neighbor_changes.clear();
    }

    pub(super) fn dispatch_mod_random_tick_events(&mut self) {
        self.block_ticks.random_block_ticks.clear();
    }

    pub(super) fn mod_finish_using(&mut self, _item: &str) -> bool {
        false
    }

    /// Says why the installed and dev mods don't run.
    pub(super) fn start_mods(&mut self) {
        let dir = Self::options_path().with_file_name(mod_list::DIR);
        let found = horizon_mods::find(&dir).len() + self.mods.dev.len();
        if found > 0 {
            log::warn!(
                "Horizon: {found} mod(s) not started: this build has no mod runtime (build with --features mods)"
            );
        }
    }
    pub(super) fn sync_mods(&mut self) {}
    pub(super) fn tick_mods(&mut self) {}
    pub(super) fn poll_mod_packets(&mut self) {}
    pub(super) fn mods_frame(&mut self, _elapsed: Duration) {}
    pub(super) fn show_mod_output(&mut self) {}
    pub(super) fn mod_use_block(
        &mut self,
        _pos: [i32; 3],
        _face: [i32; 3],
        _held_item: Option<&str>,
    ) -> bool {
        false
    }
    pub(super) fn mod_ticking_blocks(&self) -> Vec<String> {
        Vec::new()
    }
    pub(super) fn mod_menu_click(&mut self, _slot: u32, _button: u8) {}
    pub(super) fn mod_menu_button(&mut self, _button: u8) {}
    pub(super) fn mod_close_screen(&mut self) {}
    pub(super) fn mod_screen_view(&self) -> Option<ModScreenView> {
        None
    }
    pub(super) fn mod_slot_position(&self, _slot: usize) -> Option<(f32, f32)> {
        None
    }
    pub(super) fn mod_item_slot_access(
        &self,
        _handle: u32,
        _slot: usize,
        _face: &str,
    ) -> (bool, bool) {
        (true, true)
    }
    pub(super) fn mod_button_at(&self, _x: f32, _y: f32) -> Option<u8> {
        None
    }
    pub(super) fn mod_button_declared(&self, _button: u8) -> bool {
        false
    }
    pub(super) fn draw_mod_screen(&self, _batch: &mut HudBatch, _ox: f32, _oy: f32) {}
    pub(super) fn ensure_mod_block_entity(&mut self, _pos: [i32; 3], _block: &str) -> bool {
        false
    }
    pub(super) fn mod_save_block_entity(&mut self, _pos: [i32; 3]) {}
    pub(super) fn mod_save_attachments(&mut self) {}
    pub(super) fn mod_load_attachments(&mut self) {}
    pub(super) fn tick_mod_block_entities(&mut self, _positions: &[(i32, i32, i32)]) {}
    pub(super) fn mod_command(&mut self, _command: &str) -> bool {
        false
    }
    pub(super) fn mods_chat(&mut self, _line: &str) {}
    pub(super) fn mod_entity_damage(
        &mut self,
        _subject: &str,
        _position: [i32; 3],
        amount: f32,
        _context: &str,
    ) -> Option<f32> {
        Some(amount)
    }
    pub(super) fn mod_entity_death(&mut self, _id: u32, _kind: &str, _pos: [f64; 3]) {}
    pub(super) fn mod_mob_damage(
        &mut self,
        id: u32,
        amount: f32,
        source: horizon_client::mobs::Damage,
    ) -> bool {
        self.mobs
            .iter_mut()
            .find(|m| m.id == id)
            .is_some_and(|m| m.damage(amount, source))
    }
    pub(super) fn mod_xp_gain(&mut self, amount: u32, _context: &str) -> u32 {
        amount
    }
    pub(super) fn mod_loot_drop(
        &mut self,
        _subject: &str,
        _position: [i32; 3],
        _value: f32,
        _context: &str,
    ) -> (ModHookOutcome, Vec<ModItemDrop>) {
        (ModHookOutcome::Pass, Vec::new())
    }
    pub(super) fn draw_mod_panels(&self, _batch: &mut HudBatch, _corners: &mut Corners) {}
}

#[cfg(feature = "mods")]
impl Game<'_> {
    pub(super) fn mod_block_placed(&mut self, pos: [i32; 3], face: String) {
        let _ = self.mod_block_event(pos, BlockAction::Place, face);
    }

    pub(super) fn mod_scheduled_block_tick(&mut self, pos: [i32; 3]) {
        let key = (self.dim.id().to_owned(), pos[0], pos[1], pos[2]);
        if self
            .mods
            .scheduled_mod_action_tickets
            .remove(&key)
            .is_some()
        {
            self.release_mod_action_tickets();
        }
        let _ = self.mod_block_event_actor(
            pos,
            BlockAction::ScheduledTick,
            String::new(),
            String::new(),
        );
    }

    /// Releases only tickets with no pending action or scheduled tick left in
    /// their chunk. Scheduled ticks need a block-ticking ticket, not just a
    /// loaded chunk, until the engine actually runs them.
    fn release_mod_action_tickets(&mut self) {
        let now = self.world.game_time;
        let expired: Vec<_> = self
            .mods
            .scheduled_mod_action_tickets
            .iter()
            .filter_map(|(tick, &(_, _, deadline))| (now >= deadline).then(|| tick.clone()))
            .collect();
        for tick in expired {
            if self
                .mods
                .scheduled_mod_action_tickets
                .remove(&tick)
                .is_some()
            {
                log::warn!(
                    "Horizon: released expired mod scheduled-tick ticket for {:?} at ({}, {}, {})",
                    tick.0,
                    tick.1,
                    tick.2,
                    tick.3
                );
            }
        }
        let release: Vec<_> = self
            .mods
            .mod_action_tickets
            .iter()
            .filter(|key| {
                let pending_action = self.mods.pending_server_actions.iter().any(|(action, _)| {
                    mod_action_chunk(action).is_some_and(|(dimension, x, z)| {
                        dimension == key.0 && (x >> 4, z >> 4) == (key.1, key.2)
                    })
                });
                let pending_tick = self.mods.scheduled_mod_action_tickets.iter().any(
                    |((dimension, x, _, z), _)| {
                        dimension == &key.0 && (*x >> 4, *z >> 4) == (key.1, key.2)
                    },
                );
                !pending_action && !pending_tick
            })
            .cloned()
            .collect();
        for key in release {
            self.mods.mod_action_tickets.remove(&key);
            self.with_mod_dimension(&key.0, |game| {
                game.streaming.forced.remove(&(key.1, key.2));
            });
            log::info!(
                "Horizon: released mod action chunk ticket for {:?} [{}, {}]",
                key.0,
                key.1,
                key.2
            );
        }
    }

    pub(super) fn dispatch_mod_random_tick_events(&mut self) {
        const MAX_HOOKS_PER_DRAIN: usize = 65_536;
        let mut events = std::mem::take(&mut self.block_ticks.random_block_ticks);
        if events.len() > MAX_HOOKS_PER_DRAIN {
            log::warn!(
                "Horizon: limiting mod random-tick callbacks to {MAX_HOOKS_PER_DRAIN} ({} sampled blocks)",
                events.len()
            );
            events.truncate(MAX_HOOKS_PER_DRAIN);
        }
        for pos in events {
            let block = self
                .res
                .facts()
                .get(self.world.block(pos.x, pos.y, pos.z))
                .name
                .to_string();
            if self.mod_has_block_hook(&block) {
                let _ = self.mod_block_event_actor(
                    [pos.x, pos.y, pos.z],
                    BlockAction::RandomTick,
                    String::new(),
                    String::new(),
                );
            }
        }
    }

    pub(super) fn mod_entity_inside(&mut self, pos: [i32; 3], actor: String) {
        let _ = self.mod_block_event_actor(pos, BlockAction::EntityInside, String::new(), actor);
    }

    pub(super) fn mod_server_event(
        &mut self,
        kind: &str,
        subject: String,
        pos: [i32; 3],
        detail: String,
    ) {
        self.mod_server_event_at(kind, subject, self.dim.id().to_owned(), pos, detail);
    }

    pub(super) fn mod_server_event_at(
        &mut self,
        kind: &str,
        subject: String,
        dimension: String,
        pos: [i32; 3],
        detail: String,
    ) {
        if self.remote.is_some() || !self.mods.mods.is_running() {
            return;
        }
        let kind = match kind {
            "player-join" => ServerEventKind::PlayerJoin,
            "player-leave" => ServerEventKind::PlayerLeave,
            "block-break" => ServerEventKind::BlockBreak,
            "block-place" => ServerEventKind::BlockPlace,
            "entity-spawn" => ServerEventKind::EntitySpawn,
            "entity-death" => ServerEventKind::EntityDeath,
            _ => return,
        };
        self.mods.mods.server_event(ServerEvent {
            kind,
            subject,
            dimension,
            x: pos[0],
            y: pos[1],
            z: pos[2],
            detail,
        });
        self.show_mod_output();
    }

    /// Dispatches a death event for a game entity removed from the world.
    /// Entity positions use block coordinates, matching the other server hooks.
    pub(super) fn mod_entity_death(&mut self, id: u32, kind: &str, pos: [f64; 3]) {
        self.mod_server_event(
            "entity-death",
            id.to_string(),
            pos.map(|v| v.floor() as i32),
            kind.to_owned(),
        );
    }

    pub(super) fn mod_local_player_joined(&mut self, pos: [i32; 3]) {
        if self.remote.is_some() || self.mods.local_player_session_active {
            return;
        }
        self.mods.local_player_session_active = true;
        self.mod_server_event(
            "player-join",
            self.acting_player_name().to_owned(),
            pos,
            String::new(),
        );
    }

    pub(super) fn mod_local_player_left(&mut self) {
        if self.remote.is_some() || !self.mods.local_player_session_active {
            return;
        }
        self.mods.local_player_session_active = false;
        let pos = self.player.as_ref().map_or([0, 0, 0], |player| {
            [
                player.position.x.floor() as i32,
                player.position.y.floor() as i32,
                player.position.z.floor() as i32,
            ]
        });
        self.mod_server_event(
            "player-leave",
            self.acting_player_name().to_owned(),
            pos,
            String::new(),
        );
    }

    pub(super) fn reset_mod_player_session(&mut self) {
        self.mods.local_player_session_active = false;
    }

    pub(super) fn dispatch_mod_neighbor_events(&mut self) {
        if self.mods.dispatching_neighbor_hooks {
            return;
        }
        const MAX_HOOKS_PER_DRAIN: usize = 65_536;
        self.mods.dispatching_neighbor_hooks = true;
        let mut dispatched = 0;
        loop {
            let events = std::mem::take(&mut self.block_ticks.neighbor_changes);
            if events.is_empty() {
                break;
            }
            for pos in events {
                if dispatched == MAX_HOOKS_PER_DRAIN {
                    self.block_ticks.neighbor_changes.clear();
                    log::warn!(
                        "Horizon: stopped mod neighbor callbacks after {dispatched} events in one drain"
                    );
                    break;
                }
                dispatched += 1;
                let block = self
                    .res
                    .facts()
                    .get(self.world.block(pos.x, pos.y, pos.z))
                    .name
                    .to_string();
                if self.mod_has_block_hook(&block) {
                    let _ = self.mod_block_event_actor(
                        [pos.x, pos.y, pos.z],
                        BlockAction::NeighborChanged,
                        String::new(),
                        String::new(),
                    );
                }
            }
            if dispatched == MAX_HOOKS_PER_DRAIN {
                break;
            }
        }
        self.mods.dispatching_neighbor_hooks = false;
    }

    pub(super) fn mod_has_block_hook(&self, block: &str) -> bool {
        self.mods.mods.has_block_hook(block)
    }

    fn mod_block_state_at(&self, pos: [i32; 3]) -> String {
        let state = self.world.block(pos[0], pos[1], pos[2]);
        let fallback = self.res.facts().get(state).name.to_string();
        let Some(state) = self.pack.assets.registry.state(state) else {
            return fallback;
        };
        if state.properties.is_empty() {
            return state.name.clone();
        }
        let properties = state
            .properties
            .iter()
            .map(|(key, value)| format!("{key}={value}"))
            .collect::<Vec<_>>()
            .join(",");
        format!("{}[{properties}]", state.name)
    }

    pub(super) fn mod_block_broken(&mut self, pos: [i32; 3]) -> bool {
        let state = self
            .res
            .facts()
            .get(self.world.block(pos[0], pos[1], pos[2]))
            .name
            .to_string();
        self.mod_has_block_hook(&state)
            && self.mod_block_event(pos, BlockAction::Broken, String::new()) != EventOutcome::Pass
    }

    /// Dispatches a use-in-air callback for a mod-owned item. `true` means
    /// the mod consumed or cancelled the use.
    pub(super) fn mod_use_item_in_air(&mut self, item: &str) -> bool {
        if self.remote.is_some() || !self.mods.mods.is_running() {
            return false;
        }
        let outcome = self.mods.mods.item_event(ItemEvent {
            action: ItemAction::UseInAir,
            item: item.to_owned(),
            player: self.acting_player_name().to_owned(),
            dimension: self.dim.id().to_owned(),
            x: 0,
            y: 0,
            z: 0,
            target: String::new(),
        });
        self.show_mod_output();
        outcome != EventOutcome::Pass
    }

    /// Dispatches an item's block-use callback after the block itself passed.
    pub(super) fn mod_use_item_on_block(&mut self, item: &str, pos: [i32; 3]) -> bool {
        if self.remote.is_some() || !self.mods.mods.is_running() {
            return false;
        }
        let outcome = self.mods.mods.item_event(ItemEvent {
            action: ItemAction::UseOnBlock,
            item: item.to_owned(),
            player: self.acting_player_name().to_owned(),
            dimension: self.dim.id().to_owned(),
            x: pos[0],
            y: pos[1],
            z: pos[2],
            target: self.mod_block_state_at(pos),
        });
        self.show_mod_output();
        outcome != EventOutcome::Pass
    }

    /// Dispatches the held item's callback before a player damages a mob.
    pub(super) fn mod_attack_entity(&mut self, item: &str, target: &str, pos: [i32; 3]) -> bool {
        if self.remote.is_some() || !self.mods.mods.is_running() {
            return false;
        }
        let outcome = self.mods.mods.item_event(ItemEvent {
            action: ItemAction::AttackEntity,
            item: item.to_owned(),
            player: self.acting_player_name().to_owned(),
            dimension: self.dim.id().to_owned(),
            x: pos[0],
            y: pos[1],
            z: pos[2],
            target: target.to_owned(),
        });
        self.show_mod_output();
        outcome != EventOutcome::Pass
    }

    /// Dispatches a left-click callback for a mod-owned block.
    pub(super) fn mod_attack_block(&mut self, pos: [i32; 3]) -> bool {
        if self.remote.is_some() || !self.mods.mods.is_running() {
            return false;
        }
        self.mod_block_event(pos, BlockAction::Attack, String::new()) != EventOutcome::Pass
    }

    /// Dispatches an action to the mod that owns the block and applies its
    /// queued server actions.
    pub(super) fn mod_block_event(
        &mut self,
        pos: [i32; 3],
        action: BlockAction,
        face: String,
    ) -> EventOutcome {
        let actor = self.acting_player_name().to_owned();
        self.mod_block_event_actor(pos, action, face, actor)
    }

    fn mod_block_event_actor(
        &mut self,
        pos: [i32; 3],
        action: BlockAction,
        face: String,
        actor: String,
    ) -> EventOutcome {
        if self.remote.is_some() || !self.mods.mods.is_running() {
            return EventOutcome::Pass;
        }
        let state = self.mod_block_state_at(pos);
        let name = self
            .res
            .facts()
            .get(self.world.block(pos[0], pos[1], pos[2]))
            .name
            .to_string();
        self.ensure_mod_block_entity(pos, &name);
        let outcome = self.mods.mods.block_event(
            BlockEvent {
                action,
                dimension: self.dim.id().to_owned(),
                x: pos[0],
                y: pos[1],
                z: pos[2],
                state,
                actor,
                held_item: if matches!(
                    action,
                    BlockAction::NeighborChanged
                        | BlockAction::ScheduledTick
                        | BlockAction::RandomTick
                        | BlockAction::EntityInside
                ) {
                    String::new()
                } else {
                    self.held()
                        .map_or_else(String::new, |stack| stack.item.to_string())
                },
            },
            face,
        );
        self.show_mod_output();
        self.mod_save_block_entity(pos);
        outcome
    }

    /// Dispatches the item callback when a consumable completes.
    pub(super) fn mod_finish_using(&mut self, item: &str) -> bool {
        if self.remote.is_some() || !self.mods.mods.is_running() {
            return false;
        }
        let outcome = self.mods.mods.item_event(ItemEvent {
            action: ItemAction::FinishUsing,
            item: item.to_owned(),
            player: self.acting_player_name().to_owned(),
            dimension: self.dim.id().to_owned(),
            x: 0,
            y: 0,
            z: 0,
            target: String::new(),
        });
        self.show_mod_output();
        outcome != EventOutcome::Pass
    }

    /// Reads `horizon-mods/` and the dev mods, and starts the ones turned on.
    pub(super) fn start_mods(&mut self) {
        let dir = Self::options_path().with_file_name(mod_list::DIR);
        let dev = &self.mods.dev;
        let mut paths = horizon_mods::find(&dir);
        paths.retain(|p| !dev.contains(p));
        paths.extend(dev.iter().cloned());
        // A dev mod is on whatever the list says.
        let dev_ids: Vec<String> = dev
            .iter()
            .filter_map(|p| horizon_mods::Package::read(p).ok())
            .map(|p| p.manifest.id)
            .collect();
        let enabled = &self.settings.mods.enabled;
        let on = |id: &str| enabled.contains(id) || dev_ids.iter().any(|d| d == id);
        let attachments = self.mods.mods.attachment_snapshot();
        let mut mods = Mods::load(&paths, on, vanilla_command);
        mods.restore_attachments(attachments);
        let running = mods
            .entries()
            .iter()
            .filter(|entry| matches!(&entry.status, Status::Running))
            .count();
        let commands = mods.commands().count();
        log::info!(
            "Horizon: loaded {running} running mod(s) with {commands} command(s) from {} package(s)",
            mods.entries().len(),
        );
        mods.set_item_facts(Arc::new(GameFacts(self.res.clone())));
        self.mods.mods = mods;
        self.mods.resource_recipes_loaded = false;
        self.load_mod_resource_recipes();
        self.mods.block_entities.clear();
        self.world.block_entities.set_states(Vec::new());
        self.list_mods();
        self.sync_mod_commands();
        self.show_mod_output();
    }
    /// The Mods page's rows, from what loaded.
    fn list_mods(&mut self) {
        let dev = &self.mods.dev;
        self.settings.mods.installed = self
            .mods
            .mods
            .entries()
            .iter()
            .map(|e| {
                let file = e.path.file_name().unwrap_or_default().to_string_lossy();
                let problem = match &e.status {
                    Status::Failed(reason) => Some(format!("Stopped: {reason}")),
                    _ => None,
                };
                let Some(m) = &e.manifest else {
                    return Installed {
                        id: String::new(),
                        title: file.into_owned(),
                        description: String::new(),
                        problem,
                        dev: false,
                    };
                };
                // What it may do first: the row cuts long descriptions.
                let may: Vec<&str> = m.capabilities.iter().map(|c| c.describe()).collect();
                let may = if may.is_empty() {
                    "Asks for nothing.".to_owned()
                } else {
                    format!("May {}.", may.join(", "))
                };
                Installed {
                    id: m.id.clone(),
                    title: format!("{} {}", m.name, m.version),
                    description: format!("{may} {}", m.description).trim().to_owned(),
                    problem,
                    dev: dev.contains(&e.path),
                }
            })
            .collect();
    }
    /// The chat's tree after the running mods changed.
    fn sync_mod_commands(&mut self) {
        let mut names = self.mods.mods.commands().peekable();
        self.mods.tree = names
            .peek()
            .is_some()
            .then(|| vanilla::game_dispatcher().with_mod_commands(names));
    }
    /// After the options changed: restart the mods if the list of the ones
    /// turned on did.
    pub(super) fn sync_mods(&mut self) {
        let running: Vec<&str> = self
            .mods
            .mods
            .entries()
            .iter()
            .filter(|e| e.status != Status::Off)
            .filter_map(|e| Some(e.manifest.as_ref()?.id.as_str()))
            .collect();
        let wanted: Vec<&str> = self
            .settings
            .mods
            .installed
            .iter()
            .enumerate()
            .filter(|&(i, _)| self.settings.mods.is_on(i))
            .map(|(_, m)| m.id.as_str())
            .collect();
        if running != wanted {
            self.start_mods();
        }
    }
    /// What mods read this tick.
    fn mod_snapshot(&self) -> Snapshot {
        let player = self.player.as_ref().map(|p| Player {
            x: p.position.x,
            y: p.position.y,
            z: p.position.z,
            yaw: self.yaw,
            pitch: self.pitch,
            health: p.health.health(),
            food: p.health.food.food.max(0) as u32,
            dimension: self.dim.id().to_owned(),
        });
        let clock = player.is_some().then(|| Clock {
            game_time: self.world.game_time.max(0) as u64,
            day_time: self.world.day_time.max(0) as u64,
        });
        Snapshot { player, clock }
    }
    #[cfg(feature = "mods")]
    fn mod_world_view(&self) -> Option<Vec<WorldCell>> {
        let player = self.player.as_ref()?;
        let center = [
            player.position.x.floor() as i32,
            player.position.y.floor() as i32,
            player.position.z.floor() as i32,
        ];
        const RADIUS: i32 = 4;
        let (min_y, max_y) = if self.dim.id() == "minecraft:overworld" {
            (
                horizon_client::chunk::MIN_Y,
                horizon_client::chunk::MIN_Y + horizon_client::chunk::HEIGHT - 1,
            )
        } else {
            (0, 255)
        };
        let mut cells = Vec::with_capacity(9 * 9 * 9);
        for x in center[0] - RADIUS..=center[0] + RADIUS {
            for y in (center[1] - RADIUS).max(min_y)..=(center[1] + RADIUS).min(max_y) {
                for z in center[2] - RADIUS..=center[2] + RADIUS {
                    if !self.world.loaded(x, z) {
                        continue;
                    }
                    let state = self.world.block(x, y, z);
                    let name = self.res.facts().get(state).name.to_string();
                    let block = self
                        .pack
                        .assets
                        .registry
                        .state(state)
                        .map_or(name.clone(), |s| {
                            if s.properties.is_empty() {
                                name
                            } else {
                                let properties = s
                                    .properties
                                    .iter()
                                    .map(|(key, value)| format!("{key}={value}"))
                                    .collect::<Vec<_>>()
                                    .join(",");
                                format!("{name}[{properties}]")
                            }
                        });
                    let chunk = self.world.chunks.get(&(x >> 4, z >> 4))?;
                    let biome = self.res.core.biomes.get(chunk.biome(x, y, z)).name.clone();
                    cells.push(WorldCell {
                        x,
                        y,
                        z,
                        block,
                        biome,
                        light: self.world.light(x, y, z),
                    });
                }
            }
        }
        Some(cells)
    }
    /// A game tick.
    pub(super) fn tick_mods(&mut self) {
        self.load_mod_resource_recipes();
        if !self.mods.mods.is_running() {
            self.mods.block_entities.clear();
            return;
        }
        let stale: Vec<([i32; 3], u32)> = self
            .mods
            .block_entities
            .iter()
            .filter_map(|(&pos, view)| {
                let live = matches!(
                    self.world.block_entities.get(&(pos[0], pos[1], pos[2])),
                    Some(horizon_client::containers::BlockEntity::Mod { handle, .. })
                        if *handle == view.handle
                );
                (!live).then_some((pos, view.handle))
            })
            .collect();
        for (pos, handle) in stale {
            self.mods.block_entities.remove(&pos);
            self.mods.mods.remove_block_entity(handle);
        }
        let snapshot = self.mod_snapshot();
        self.mods.mods.set_snapshot(snapshot);
        if self.mods.mods.wants_world_read()
            && let Some(cells) = self.mod_world_view()
        {
            self.mods.mods.set_world_view(self.dim.id(), cells);
        }
        let block_hit = self.pick().map(|hit| {
            let state_id = self.world.block(hit.pos[0], hit.pos[1], hit.pos[2]);
            let name = self.res.facts().get(state_id).name.to_string();
            let owner = name
                .split_once(':')
                .map_or("minecraft", |(ns, _)| ns)
                .to_owned();
            let face = match hit.face {
                [0, -1, 0] => "down",
                [0, 1, 0] => "up",
                [0, 0, -1] => "north",
                [0, 0, 1] => "south",
                [-1, 0, 0] => "west",
                [1, 0, 0] => "east",
                _ => "up",
            }
            .to_owned();
            BlockHit {
                x: hit.pos[0],
                y: hit.pos[1],
                z: hit.pos[2],
                state: name,
                face,
                owner,
            }
        });
        self.mods.mods.set_target(block_hit, None);
        if self.res.rest.get().is_some() {
            let to_view = |slot: usize, s: &ItemStack| {
                let max_damage = self.res.items().get(&s.item).map_or(0, |i| i.max_damage);
                StackView {
                    slot: slot as u32,
                    id: s.item.to_string(),
                    count: s.count,
                    damage: s.damage,
                    max_damage,
                }
            };
            let main_hand = self
                .inventory
                .get(self.selected)
                .and_then(|s| s.as_ref())
                .map(|s| to_view(self.selected, s));
            let off_hand = self.offhand.as_ref().map(|s| to_view(40, s));
            let mut inv = Vec::new();
            for (i, slot) in self.inventory.iter().enumerate() {
                if let Some(s) = slot {
                    inv.push(to_view(i, s));
                }
            }
            for (i, slot) in self.armor.iter().enumerate() {
                if let Some(s) = slot {
                    inv.push(to_view(36 + i, s));
                }
            }
            if let Some(ref s) = off_hand {
                inv.push(s.clone());
            }
            let container_slots = self.container_kind().map_or(0, |_| self.container_size());
            let mut container_stacks = Vec::new();
            for slot in 0..container_slots {
                if let Some(stack) = self.slot_view(300 + slot) {
                    container_stacks.push((slot, stack));
                }
            }
            let open_container = container_stacks
                .into_iter()
                .map(|(slot, stack)| {
                    let max_damage = self
                        .res
                        .items()
                        .get(&stack.item)
                        .map_or(0, |item| item.max_damage);
                    StackView {
                        slot: slot as u32,
                        id: stack.item.to_string(),
                        count: stack.count,
                        damage: stack.damage,
                        max_damage,
                    }
                })
                .collect();
            self.mods
                .mods
                .set_inventory(main_hand, off_hand, inv, open_container);
        }
        self.mods.mods.tick(self.ticks);
        self.show_mod_output();
    }
    fn load_mod_resource_recipes(&mut self) {
        if self.mods.resource_recipes_loaded {
            return;
        }
        let Some(rest) = self.res.rest.get() else {
            return;
        };
        self.mods
            .mods
            .add_resource_recipes(rest.recipes.machine.iter().map(|recipe| {
                horizon_mods::RecipeEntry {
                    id: recipe.id.clone(),
                    kind: recipe.kind.clone(),
                    inputs: recipe
                        .inputs
                        .iter()
                        .map(|input| match input {
                            horizon_client::crafting::MachineInput::Item { ingredient, count } => {
                                horizon_mods::IngredientSlot {
                                    items: ingredient.1.clone(),
                                    count: *count,
                                }
                            }
                            horizon_client::crafting::MachineInput::Resource {
                                kind,
                                tank,
                                accepted,
                                amount,
                            } => {
                                let kind = match kind {
                                    horizon_client::crafting::MachineResourceKind::Fluid => "fluid",
                                    horizon_client::crafting::MachineResourceKind::Chemical => {
                                        "chemical"
                                    }
                                };
                                let tank = *tank + if kind == "chemical" { 1_000 } else { 0 };
                                horizon_mods::IngredientSlot {
                                    items: accepted
                                        .iter()
                                        .map(|id| format!("@{kind}/{tank}/{id}"))
                                        .collect(),
                                    count: *amount,
                                }
                            }
                        })
                        .collect(),
                    outputs: recipe
                        .outputs
                        .iter()
                        .map(|output| match output {
                            horizon_client::crafting::MachineOutput::Item(stack) => {
                                format!("{}*{}", stack.item, stack.count)
                            }
                            horizon_client::crafting::MachineOutput::Resource {
                                kind,
                                tank,
                                id,
                                amount,
                            } => {
                                let kind = match kind {
                                    horizon_client::crafting::MachineResourceKind::Fluid => "fluid",
                                    horizon_client::crafting::MachineResourceKind::Chemical => {
                                        "chemical"
                                    }
                                };
                                let tank = *tank + if kind == "chemical" { 1_000 } else { 0 };
                                format!("@{kind}/{tank}/{id}*{amount}")
                            }
                        })
                        .collect(),
                    ticks: recipe.ticks,
                    energy: recipe.energy,
                }
            }));
        self.mods.resource_recipes_loaded = true;
    }
    /// Once per frame: the frame hook, and dev mods' rebuilds.
    pub(super) fn mods_frame(&mut self, elapsed: Duration) {
        if !self.mods.dev.is_empty()
            && self
                .mods
                .checked
                .is_none_or(|t| t.elapsed() >= RELOAD_CHECK)
        {
            self.mods.checked = Some(Instant::now());
            if self.mods.mods.reload_changed(vanilla_command) {
                self.list_mods();
                self.sync_mod_commands();
                self.system_message(Text::literal("Mods reloaded").with_color(text::GRAY));
            }
        }
        if !self.mods.mods.is_running() {
            return;
        }
        self.mods.mods.frame(elapsed.as_secs_f32());
        self.show_mod_output();
    }
    /// `/command` typed in chat: true when a mod owns it.
    pub(super) fn mod_command(&mut self, command: &str) -> bool {
        if !self.mods.mods.is_running() {
            return false;
        }
        // The command may follow a teleport in the same tick.
        let snapshot = self.mod_snapshot();
        self.mods.mods.set_snapshot(snapshot);
        if !self.mods.mods.command(command) {
            return false;
        }
        self.show_mod_output();
        true
    }
    /// A chat line the player received.
    pub(super) fn mods_chat(&mut self, line: &str) {
        self.mods.mods.chat(line);
        self.show_mod_output();
    }
    /// Runs the API 0.5 entity-damage hook. A cancelled hit never reaches the
    /// normal damage pipeline; accepted multipliers apply before vanilla armor
    /// and damage handling.
    pub(super) fn mod_entity_damage(
        &mut self,
        subject: &str,
        position: [i32; 3],
        amount: f32,
        context: &str,
    ) -> Option<f32> {
        #[cfg(feature = "mods")]
        {
            if !self.mods.mods.is_running() {
                return Some(amount);
            }
            let (outcome, multiplier, _, _, _) = self.mods.mods.vanilla_hook(&VanillaHookEvent {
                kind: VanillaHookKind::EntityDamage,
                subject: subject.to_owned(),
                dimension: self.dim.id().to_owned(),
                x: position[0],
                y: position[1],
                z: position[2],
                value: amount,
                context: context.to_owned(),
            });
            if outcome == EventOutcome::Cancel {
                None
            } else {
                Some(amount * multiplier)
            }
        }
        #[cfg(not(feature = "mods"))]
        {
            let _ = (subject, position, context);
            Some(amount)
        }
    }
    /// Applies an entity-damage hook before ordinary mob damage. Keep the
    /// vanilla Damage source intact; the hook only cancels or scales its value.
    pub(super) fn mod_mob_damage(
        &mut self,
        id: u32,
        amount: f32,
        source: horizon_client::mobs::Damage,
    ) -> bool {
        let Some((subject, position)) = self.mobs.iter().find(|m| m.id == id).map(|m| {
            (
                if m.kind.id.contains(':') {
                    m.kind.id.clone()
                } else {
                    format!("minecraft:{}", m.kind.id)
                },
                [m.position.x, m.position.y, m.position.z].map(|v| v.floor() as i32),
            )
        }) else {
            return false;
        };
        let context = match source.kind {
            horizon_client::mobs::DamageKind::PlayerAttack => "minecraft:player_attack",
            horizon_client::mobs::DamageKind::MaceSmash => "minecraft:mace_smash",
            horizon_client::mobs::DamageKind::MobAttack => "minecraft:mob_attack",
            horizon_client::mobs::DamageKind::Arrow => "minecraft:arrow",
            horizon_client::mobs::DamageKind::Thrown => "minecraft:thrown",
            horizon_client::mobs::DamageKind::Fireball => "minecraft:fireball",
            horizon_client::mobs::DamageKind::Explosion => "minecraft:explosion",
            horizon_client::mobs::DamageKind::Fall => "minecraft:fall",
            horizon_client::mobs::DamageKind::FallingBlock => "minecraft:falling_block",
            horizon_client::mobs::DamageKind::OnFire => "minecraft:on_fire",
            horizon_client::mobs::DamageKind::InFire => "minecraft:in_fire",
            horizon_client::mobs::DamageKind::Lava => "minecraft:lava",
            horizon_client::mobs::DamageKind::Drown => "minecraft:drown",
            horizon_client::mobs::DamageKind::Magic => "minecraft:magic",
            horizon_client::mobs::DamageKind::Sting => "minecraft:sting",
            horizon_client::mobs::DamageKind::Cactus => "minecraft:cactus",
            horizon_client::mobs::DamageKind::Generic => "minecraft:generic",
            horizon_client::mobs::DamageKind::InWall => "minecraft:in_wall",
            horizon_client::mobs::DamageKind::Thorns => "minecraft:thorns",
            horizon_client::mobs::DamageKind::SonicBoom => "minecraft:sonic_boom",
            horizon_client::mobs::DamageKind::OutOfWorld => "minecraft:out_of_world",
        };
        let Some(amount) = self.mod_entity_damage(&subject, position, amount, context) else {
            return false;
        };
        self.mobs
            .iter_mut()
            .find(|m| m.id == id)
            .is_some_and(|m| m.damage(amount, source))
    }
    pub(super) fn mod_xp_gain(&mut self, amount: u32, context: &str) -> u32 {
        #[cfg(feature = "mods")]
        {
            if !self.mods.mods.is_running() {
                return amount;
            }
            let position = self.player.as_ref().map_or([0; 3], |p| {
                [
                    p.position.x.floor() as i32,
                    p.position.y.floor() as i32,
                    p.position.z.floor() as i32,
                ]
            });
            let (outcome, _, multiplier, _, _) = self.mods.mods.vanilla_hook(&VanillaHookEvent {
                kind: VanillaHookKind::XpGain,
                subject: "minecraft:player".into(),
                dimension: self.dim.id().to_owned(),
                x: position[0],
                y: position[1],
                z: position[2],
                value: amount as f32,
                context: context.to_owned(),
            });
            if outcome == EventOutcome::Cancel {
                0
            } else {
                scaled_xp(amount, multiplier)
            }
        }
        #[cfg(not(feature = "mods"))]
        {
            let _ = context;
            amount
        }
    }
    pub(super) fn mod_loot_drop(
        &mut self,
        subject: &str,
        position: [i32; 3],
        value: f32,
        context: &str,
    ) -> (ModHookOutcome, Vec<ModItemDrop>) {
        if !self.mods.mods.is_running() {
            return (ModHookOutcome::Pass, Vec::new());
        }
        let (outcome, _, _, _, drops) = self.mods.mods.vanilla_hook(&VanillaHookEvent {
            kind: VanillaHookKind::LootDrop,
            subject: subject.to_owned(),
            dimension: self.dim.id().to_owned(),
            x: position[0],
            y: position[1],
            z: position[2],
            value,
            context: context.to_owned(),
        });
        let outcome = match outcome {
            EventOutcome::Pass => ModHookOutcome::Pass,
            EventOutcome::Consume => ModHookOutcome::Consume,
            EventOutcome::Cancel => ModHookOutcome::Cancel,
        };
        let mut validated = Vec::new();
        for ItemDrop { id, count } in drops {
            let Some(max_stack) = self.res.items().get(&id).map(|item| item.max_stack) else {
                log::warn!("Horizon: mod loot hook returned unknown item {id}");
                continue;
            };
            if count == 0 || max_stack == 0 {
                continue;
            }
            let mut remaining = count;
            while remaining > 0 {
                let stack = remaining.min(max_stack);
                validated.push(ModItemDrop {
                    id: id.clone(),
                    count: stack,
                });
                remaining -= stack;
            }
        }
        (outcome, validated)
    }
    /// A right-click on a mod-owned block. `true` means the mod handled it.
    pub(super) fn mod_use_block(
        &mut self,
        pos: [i32; 3],
        face: [i32; 3],
        held_item: Option<&str>,
    ) -> bool {
        if !self.mods.mods.is_running() {
            return false;
        }
        let name = self
            .res
            .facts()
            .get(self.world.block(pos[0], pos[1], pos[2]))
            .name
            .to_string();
        self.ensure_mod_block_entity(pos, &name);
        // Vanilla buckets can fill and drain Mekanism's fluid tanks. The mod
        // capability state is shared with machine ticks and persists on save.
        if (name.starts_with("mekanism:") && name.ends_with("_fluid_tank"))
            || name == "mekanism:dynamic_valve"
        {
            if let Some(item) = held_item {
                let is_bucket = matches!(
                    item,
                    "minecraft:bucket" | "minecraft:water_bucket" | "minecraft:lava_bucket"
                );
                let key = format!("{}:{},{},{}", self.dim.id(), pos[0], pos[1], pos[2]);
                self.mods.mods.declare_fluid_tank(&key, 0, 64_000);
                let transfer = match item {
                    "minecraft:water_bucket" => Some(("minecraft:water", "minecraft:bucket")),
                    "minecraft:lava_bucket" => Some(("minecraft:lava", "minecraft:bucket")),
                    "minecraft:bucket" => self.mods.mods.fluid_tank(&key, 0).and_then(|tank| {
                        let bucket = match tank.fluid.as_str() {
                            "minecraft:water" if tank.amount >= 1_000 => "minecraft:water_bucket",
                            "minecraft:lava" if tank.amount >= 1_000 => "minecraft:lava_bucket",
                            _ => return None,
                        };
                        Some(("", bucket))
                    }),
                    _ => None,
                };
                if let Some((fluid, result_item)) = transfer {
                    let moved = if item == "minecraft:bucket" {
                        self.creative || self.mods.mods.drain_fluid_tank(&key, 0, 1_000).is_some()
                    } else {
                        self.mods.mods.fill_fluid_tank(&key, 0, fluid, 1_000) == 1_000
                    };
                    if moved {
                        if item == "minecraft:bucket" {
                            if self.creative {
                                *self.held_mut() =
                                    Some(horizon_client::items::ItemStack::new(result_item, 1));
                            } else {
                                self.consume_held();
                                let result = horizon_client::items::ItemStack::new(result_item, 1);
                                if let Some(rest) = self.give(result)
                                    && let Some(eye) = self.eye_position()
                                {
                                    self.spawn_item(rest, eye, [0.0, 0.1, 0.0], 40);
                                }
                            }
                        } else if !self.creative {
                            *self.held_mut() =
                                Some(horizon_client::items::ItemStack::new(result_item, 1));
                        }
                        self.mod_save_block_entity(pos);
                        return true;
                    }
                }
                if is_bucket {
                    return true;
                }
            }
        }
        let face = match face {
            [0, -1, 0] => "down",
            [0, 1, 0] => "up",
            [0, 0, -1] => "north",
            [0, 0, 1] => "south",
            [-1, 0, 0] => "west",
            [1, 0, 0] => "east",
            _ => "",
        }
        .to_owned();
        let state = self.mod_block_state_at(pos);
        let outcome = self.mods.mods.block_event(
            BlockEvent {
                action: if held_item.is_some() {
                    BlockAction::UseItemOn
                } else {
                    BlockAction::UseWithoutItem
                },
                dimension: self.dim.id().to_owned(),
                x: pos[0],
                y: pos[1],
                z: pos[2],
                state,
                actor: self.acting_player_name().to_owned(),
                held_item: held_item.unwrap_or_default().to_owned(),
            },
            face,
        );
        let opened_screen = self.mods.mods.open_screens().next().is_some();
        self.show_mod_output();
        self.mod_save_block_entity(pos);
        if outcome != horizon_mods::EventOutcome::Pass && opened_screen {
            self.open_menu(pos);
        }
        outcome != horizon_mods::EventOutcome::Pass
    }
    /// The blocks whose API 0.4 block entities tick each server tick.
    pub(super) fn mod_ticking_blocks(&self) -> Vec<String> {
        self.mods
            .mods
            .block_entity_types()
            .filter(|ty| ty.ticks)
            .map(|ty| ty.block.clone())
            .collect()
    }
    /// A click in the mod's declared screen, paired with the game's slot menu.
    pub(super) fn mod_menu_click(&mut self, slot: u32, button: u8) {
        let Some(screen_id) = self
            .mods
            .mods
            .open_screens()
            .next()
            .map(|(_, spec)| spec.id.clone())
        else {
            return;
        };
        self.mods.mods.menu_click(&screen_id, slot, button);
        self.show_mod_output();
        if let Some(pos) = self.container {
            self.mod_save_block_entity([pos.0, pos.1, pos.2]);
        }
    }
    pub(super) fn mod_menu_button(&mut self, button: u8) {
        let Some(screen_id) = self
            .mods
            .mods
            .open_screens()
            .next()
            .map(|(_, spec)| spec.id.clone())
        else {
            return;
        };
        self.mods.mods.menu_button(&screen_id, button);
        self.show_mod_output();
        if let Some(pos) = self.container {
            self.mod_save_block_entity([pos.0, pos.1, pos.2]);
        }
    }
    pub(super) fn mod_close_screen(&mut self) {
        let Some(screen_id) = self
            .mods
            .mods
            .open_screens()
            .next()
            .map(|(_, spec)| spec.id.clone())
        else {
            return;
        };
        self.mods.mods.close_screen(&screen_id);
        self.show_mod_output();
    }
    pub(super) fn mod_screen_view(&self) -> Option<ModScreenView> {
        let (mod_id, spec) = self.mods.mods.open_screens().next()?;
        let backgrounds = self
            .mods
            .mods
            .open_screen_backgrounds()
            .find(|(owner, _)| *owner == mod_id)
            .map(|(_, backgrounds)| {
                backgrounds
                    .iter()
                    .map(|background| ModScreenBackground {
                        texture: background.texture.clone(),
                        x: background.x,
                        y: background.y,
                        width: background.width,
                        height: background.height,
                    })
                    .collect()
            })
            .unwrap_or_default();
        let widgets = spec
            .widgets
            .iter()
            .map(|w| ModWidgetView {
                id: w.id,
                kind: match w.kind {
                    horizon_mods::WidgetKind::Slot => ModWidgetKind::Slot,
                    horizon_mods::WidgetKind::ProgressArrow => ModWidgetKind::ProgressArrow,
                    horizon_mods::WidgetKind::Flame => ModWidgetKind::Flame,
                    horizon_mods::WidgetKind::EnergyBar => ModWidgetKind::EnergyBar,
                    horizon_mods::WidgetKind::FluidGauge => ModWidgetKind::FluidGauge,
                    horizon_mods::WidgetKind::Label => ModWidgetKind::Label,
                    horizon_mods::WidgetKind::Button => ModWidgetKind::Button,
                },
                x: w.x,
                y: w.y,
                width: w.width,
                height: w.height,
                text: w.text.clone(),
                value: w.value,
                max_value: w.max_value,
            })
            .collect();
        let slot_items = self
            .mods
            .mods
            .open_screen_slots()
            .find(|(owner, _)| *owner == mod_id)
            .map(|(_, slots)| {
                slots
                    .iter()
                    .map(|(&slot, (id, count))| (slot, id.clone(), *count))
                    .collect()
            })
            .unwrap_or_default();
        Some(ModScreenView {
            title: spec.title.clone(),
            width: spec.width,
            height: spec.height,
            backgrounds,
            widgets,
            slot_items,
        })
    }
    pub(super) fn mod_slot_position(&self, slot: usize) -> Option<(f32, f32)> {
        let slot = u32::try_from(slot).ok()?;
        let w = self
            .mod_screen_view()?
            .widgets
            .into_iter()
            .find(|w| w.kind == ModWidgetKind::Slot && w.id == slot)?;
        Some((w.x as f32, w.y as f32))
    }
    pub(super) fn mod_item_slot_access(
        &self,
        handle: u32,
        slot: usize,
        face: &str,
    ) -> (bool, bool) {
        u32::try_from(slot).ok().map_or((true, true), |slot| {
            self.mods.mods.slot_access(handle, slot, face)
        })
    }
    pub(super) fn mod_button_at(&self, x: f32, y: f32) -> Option<u8> {
        self.mod_screen_view()?
            .widgets
            .into_iter()
            .filter(|w| w.kind == ModWidgetKind::Button)
            .find(|w| {
                x >= w.x as f32
                    && y >= w.y as f32
                    && x < w.x as f32 + w.width as f32
                    && y < w.y as f32 + w.height as f32
            })
            .and_then(|w| u8::try_from(w.id).ok())
    }
    pub(super) fn mod_button_declared(&self, button: u8) -> bool {
        self.mod_screen_view().is_some_and(|screen| {
            screen.widgets.iter().any(|widget| {
                widget.kind == ModWidgetKind::Button && widget.id == u32::from(button)
            })
        })
    }
    pub(super) fn draw_mod_screen(&self, batch: &mut HudBatch, ox: f32, oy: f32) {
        let Some(screen) = self.mod_screen_view() else {
            return;
        };
        let atlas = &self.gui.atlas;
        let width = screen.width as f32;
        let height = screen.height as f32;
        let has_background = screen
            .backgrounds
            .iter()
            .any(|background| atlas.sprites.contains_key(&background.texture));
        if has_background {
            for background in screen
                .backgrounds
                .iter()
                .filter(|background| atlas.sprites.contains_key(&background.texture))
            {
                batch.sprite_scaled(
                    atlas,
                    &background.texture,
                    ox + background.x as f32,
                    oy + background.y as f32,
                    background.width as f32,
                    background.height as f32,
                    u32::MAX,
                );
            }
        } else {
            batch.fill(atlas, ox, oy, width, height, rgba(198, 198, 198, 255));
            batch.fill(
                atlas,
                ox + 2.0,
                oy + 2.0,
                width - 4.0,
                height - 4.0,
                rgba(55, 55, 55, 255),
            );
        }
        let heading_color = if has_background {
            rgba(48, 48, 48, 255)
        } else {
            u32::MAX
        };
        let heading_x = if has_background {
            ox + (width - HudBatch::text_width(atlas, &screen.title)) / 2.0
        } else {
            ox + 8.0
        };
        batch.text(
            atlas,
            &screen.title,
            heading_x,
            oy + 5.0,
            heading_color,
            false,
        );
        batch.text(
            atlas,
            "Inventory",
            ox + 8.0,
            oy + height - 92.0,
            heading_color,
            false,
        );
        for i in 0..36 {
            let (x, y) = if i < 9 {
                (8.0 + i as f32 * 18.0, height - 24.0)
            } else {
                (
                    8.0 + ((i - 9) % 9) as f32 * 18.0,
                    height - 82.0 + ((i - 9) / 9) as f32 * 18.0,
                )
            };
            batch.sprite(atlas, "gui/sprites/container/slot", ox + x, oy + y);
        }
        for w in screen.widgets {
            let (x, y, width, height) = (
                ox + w.x as f32,
                oy + w.y as f32,
                w.width as f32,
                w.height as f32,
            );
            let ratio = if w.max_value == 0 {
                0.0
            } else {
                (w.value as f32 / w.max_value as f32).clamp(0.0, 1.0)
            };
            match w.kind {
                ModWidgetKind::Slot => {
                    let mek_slot = match w.text.as_str() {
                        "Input" => Some("mekanism/gui/slot/input"),
                        "Second input" => Some("mekanism/gui/slot/input_2"),
                        "Output" => Some("mekanism/gui/slot/output"),
                        _ => None,
                    };
                    if let Some(sprite) = mek_slot.filter(|name| atlas.sprites.contains_key(*name))
                    {
                        batch.sprite_scaled(atlas, sprite, x, y, width, height, u32::MAX);
                    } else {
                        batch.sprite(atlas, "gui/sprites/container/slot", x, y);
                    }
                }
                ModWidgetKind::ProgressArrow => {
                    if atlas.sprites.contains_key("mekanism/gui/progress/right") {
                        batch.sprite_region(
                            atlas,
                            "mekanism/gui/progress/right",
                            [x, y, width, height],
                            32.0,
                            24.0,
                        );
                    } else {
                        batch.fill(atlas, x, y, width, height, rgba(34, 34, 34, 255));
                        batch.fill(atlas, x, y, width * ratio, height, rgba(230, 230, 230, 255));
                    }
                }
                ModWidgetKind::Flame => {
                    batch.fill(atlas, x, y, width, height, rgba(34, 34, 34, 255));
                    let fill = height * ratio;
                    batch.fill(
                        atlas,
                        x,
                        y + height - fill,
                        width,
                        fill,
                        rgba(240, 100, 20, 255),
                    );
                }
                ModWidgetKind::EnergyBar | ModWidgetKind::FluidGauge => {
                    batch.fill(atlas, x, y, width, height, rgba(24, 24, 24, 255));
                    let fill = height * ratio;
                    if w.kind == ModWidgetKind::EnergyBar
                        && atlas
                            .sprites
                            .contains_key("mekanism/gui/bar/vertical_power")
                    {
                        batch.sprite_scaled(
                            atlas,
                            "mekanism/gui/bar/vertical_power",
                            x + 1.0,
                            y + height - fill,
                            (width - 2.0).max(0.0),
                            fill,
                            u32::MAX,
                        );
                        continue;
                    }
                    let color = if w.kind == ModWidgetKind::EnergyBar {
                        rgba(60, 150, 240, 255)
                    } else {
                        rgba(50, 170, 220, 255)
                    };
                    batch.fill(
                        atlas,
                        x + 2.0,
                        y + height - fill,
                        (width - 4.0).max(0.0),
                        fill,
                        color,
                    );
                    if !w.text.is_empty() {
                        batch.text(atlas, &w.text, x + width + 4.0, y, u32::MAX, false);
                    }
                }
                ModWidgetKind::Label => {
                    batch.text(atlas, &w.text, x, y, u32::MAX, false);
                }
                ModWidgetKind::Button => {
                    batch.fill(atlas, x, y, width, height, rgba(80, 80, 80, 255));
                    let tw = HudBatch::text_width(atlas, &w.text);
                    batch.text(
                        atlas,
                        &w.text,
                        x + (width - tw) / 2.0,
                        y + (height - 8.0) / 2.0,
                        u32::MAX,
                        false,
                    );
                }
            }
        }
    }
    /// Creates the game-owned inventory and mod state on placement or first use.
    pub(super) fn ensure_mod_block_entity(&mut self, pos: [i32; 3], block: &str) -> bool {
        let key = (pos[0], pos[1], pos[2]);
        if let Some(horizon_client::containers::BlockEntity::Mod {
            block,
            handle: 0,
            saved,
            items,
            energy,
            fluids,
        }) = self.world.block_entities.get(&key)
        {
            let (block, saved, items, energy, fluids) = (
                block.clone(),
                saved.clone(),
                items.clone(),
                energy.clone(),
                fluids.clone(),
            );
            let Some((handle, slots)) = self.mods.mods.create_block_entity_at(
                &block,
                &saved,
                Some((self.dim.id().to_owned(), pos[0], pos[1], pos[2])),
            ) else {
                return false;
            };
            if let Some(energy) = &energy {
                self.mods.mods.set_energy_buffer(
                    &format!("{}:{},{},{}", self.dim.id(), pos[0], pos[1], pos[2]),
                    horizon_mods::EnergyView {
                        stored: energy.stored,
                        capacity: energy.capacity,
                        max_receive: energy.max_receive,
                        max_extract: energy.max_extract,
                    },
                );
            }
            for (tank, fluid) in &fluids {
                self.mods.mods.set_fluid_tank(
                    &format!("{}:{},{},{}", self.dim.id(), pos[0], pos[1], pos[2]),
                    *tank,
                    horizon_mods::FluidView {
                        fluid: fluid.fluid.clone(),
                        amount: fluid.amount,
                        capacity: fluid.capacity,
                    },
                );
            }
            if let Some(horizon_client::containers::BlockEntity::Mod {
                handle: current_handle,
                items: current_items,
                saved: current_saved,
                ..
            }) = self.world.block_entities.get_mut(&key)
            {
                *current_handle = handle;
                current_items.resize(slots as usize, None);
                *current_saved = self.mods.mods.save_block_entity(handle).unwrap_or(saved);
            }
            self.mods.block_entities.insert(
                pos,
                BlockEntityView {
                    handle,
                    dimension: self.dim.id().to_owned(),
                    x: pos[0],
                    y: pos[1],
                    z: pos[2],
                    slots: items
                        .iter()
                        .map(|slot| {
                            slot.as_ref().map(|s| Stack {
                                id: s.item.to_string(),
                                count: s.count,
                            })
                        })
                        .collect(),
                    slot_components: items
                        .iter()
                        .map(|slot| slot.as_ref().map_or_else(Vec::new, item_components))
                        .collect(),
                    energy: energy.as_ref().map(|e| horizon_mods::EnergyView {
                        stored: e.stored,
                        capacity: e.capacity,
                        max_receive: e.max_receive,
                        max_extract: e.max_extract,
                    }),
                    fluids: fluids
                        .iter()
                        .map(|(tank, f)| {
                            (
                                *tank,
                                horizon_mods::FluidView {
                                    fluid: f.fluid.clone(),
                                    amount: f.amount,
                                    capacity: f.capacity,
                                },
                            )
                        })
                        .collect(),
                    ..Default::default()
                },
            );
            return true;
        }
        if matches!(
            self.world.block_entities.get(&key),
            Some(horizon_client::containers::BlockEntity::Mod { .. })
        ) {
            return true;
        }
        let Some((handle, slots)) = self.mods.mods.create_block_entity_at(
            block,
            &[],
            Some((self.dim.id().to_owned(), pos[0], pos[1], pos[2])),
        ) else {
            return false;
        };
        self.world.block_entities.insert(
            key,
            horizon_client::containers::BlockEntity::Mod {
                block: block.to_owned(),
                handle,
                saved: self.mods.mods.save_block_entity(handle).unwrap_or_default(),
                items: vec![None; slots as usize],
                energy: None,
                fluids: Default::default(),
            },
        );
        self.mods.block_entities.insert(
            pos,
            BlockEntityView {
                handle,
                dimension: self.dim.id().to_owned(),
                x: pos[0],
                y: pos[1],
                z: pos[2],
                slots: vec![None; slots as usize],
                ..Default::default()
            },
        );
        self.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
        true
    }
    pub(super) fn mod_save_block_entity(&mut self, pos: [i32; 3]) {
        let key = (pos[0], pos[1], pos[2]);
        let Some(handle) = (match self.world.block_entities.get(&key) {
            Some(horizon_client::containers::BlockEntity::Mod { handle, .. }) => Some(*handle),
            _ => None,
        }) else {
            return;
        };
        let Some(saved) = self.mods.mods.save_block_entity(handle) else {
            return;
        };
        if let Some(horizon_client::containers::BlockEntity::Mod { saved: current, .. }) =
            self.world.block_entities.get_mut(&key)
            && *current != saved
        {
            *current = saved;
            self.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
        }
    }
    /// Restore capability attachments after the world root is known. The
    /// target strings are kept intact so dimensions and entity identities
    /// remain the mod's stable keys.
    pub(super) fn mod_load_attachments(&mut self) {
        const MAX_ATTACHMENT_SAVE_BYTES: u64 = 64 << 20;
        let path = self.world_root.join("data/horizon/mod-attachments.json");
        let attachments = match std::fs::metadata(&path) {
            Ok(metadata) if metadata.len() > MAX_ATTACHMENT_SAVE_BYTES => {
                log::warn!(
                    "Horizon: mod attachment file {} exceeds the 64 MiB save limit",
                    path.display()
                );
                Vec::new()
            }
            Ok(_) => match std::fs::read(&path) {
                Ok(bytes) => match serde_json::from_slice(&bytes) {
                    Ok(attachments) => attachments,
                    Err(error) => {
                        log::warn!(
                            "Horizon: could not read mod attachments {}: {error}",
                            path.display()
                        );
                        Vec::new()
                    }
                },
                Err(error) => {
                    log::warn!(
                        "Horizon: could not read mod attachments {}: {error}",
                        path.display()
                    );
                    Vec::new()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            Err(error) => {
                log::warn!(
                    "Horizon: could not inspect mod attachments {}: {error}",
                    path.display()
                );
                Vec::new()
            }
        };
        self.mods.mods.restore_attachments(attachments);
    }

    /// Persist world-scoped attachments beside level metadata. Write and sync
    /// a temporary file before replacing the previous snapshot.
    pub(super) fn mod_save_attachments(&mut self) {
        let path = self.world_root.join("data/horizon/mod-attachments.json");
        let result = (|| -> std::io::Result<()> {
            let bytes = serde_json::to_vec(&self.mods.mods.attachment_snapshot())
                .map_err(std::io::Error::other)?;
            let parent = path.parent().expect("world attachment file has a parent");
            std::fs::create_dir_all(parent)?;
            let temporary = path.with_extension("json.tmp");
            let mut file = std::fs::File::create(&temporary)?;
            use std::io::Write;
            file.write_all(&bytes)?;
            file.sync_all()?;
            std::fs::rename(&temporary, &path)
        })();
        if let Err(error) = result {
            log::warn!(
                "Horizon: could not save mod attachments {}: {error}",
                path.display()
            );
        }
    }
    fn thermal_evaporation_casing(&self, pos: [i32; 3]) -> bool {
        matches!(
            self.res
                .facts()
                .get(self.world.block(pos[0], pos[1], pos[2]))
                .name
                .as_ref(),
            "mekanism:thermal_evaporation_block"
                | "mekanism:structural_glass"
                | "mekanism:thermal_evaporation_controller"
                | "mekanism:thermal_evaporation_valve"
        )
    }
    fn induction_matrix_block(&self, pos: [i32; 3]) -> String {
        self.res
            .facts()
            .get(self.world.block(pos[0], pos[1], pos[2]))
            .name
            .to_string()
    }
    /// Finds the formed matrix containing an induction port. Its casing may
    /// be at most 18 blocks per side; the interior is at most 16 per side.
    fn induction_matrix_for_port(&self, port: [i32; 3]) -> Option<InductionMatrixInfo> {
        // Avoid the box search for a lone port or incomplete build.
        let mut nearby_cells = false;
        let mut nearby_providers = false;
        let mut nearby_casings = 0usize;
        for x in port[0] - 16..=port[0] + 16 {
            for y in port[1] - 16..=port[1] + 16 {
                for z in port[2] - 16..=port[2] + 16 {
                    let block = self.induction_matrix_block([x, y, z]);
                    nearby_cells |= block.ends_with("_induction_cell");
                    nearby_providers |= block.ends_with("_induction_provider");
                    nearby_casings += usize::from(block == "mekanism:induction_casing");
                }
            }
        }
        if !nearby_cells || !nearby_providers || nearby_casings < 20 {
            return None;
        }
        let mut sizes = Vec::with_capacity(16 * 16 * 16);
        for sx in 3..=18 {
            for sy in 3..=18 {
                for sz in 3..=18 {
                    sizes.push([sx, sy, sz]);
                }
            }
        }
        sizes.sort_by_key(|size| size[0] * size[1] * size[2]);
        for size in sizes {
            for rx in 0..size[0] {
                for ry in 0..size[1] {
                    for rz in 0..size[2] {
                        let port_edges = usize::from(rx == 0 || rx == size[0] - 1)
                            + usize::from(ry == 0 || ry == size[1] - 1)
                            + usize::from(rz == 0 || rz == size[2] - 1);
                        if port_edges != 1 {
                            continue;
                        }
                        let min = [port[0] - rx, port[1] - ry, port[2] - rz];
                        let max = [
                            min[0] + size[0] - 1,
                            min[1] + size[1] - 1,
                            min[2] + size[2] - 1,
                        ];
                        let corners = [
                            [min[0], min[1], min[2]],
                            [min[0], min[1], max[2]],
                            [min[0], max[1], min[2]],
                            [min[0], max[1], max[2]],
                            [max[0], min[1], min[2]],
                            [max[0], min[1], max[2]],
                            [max[0], max[1], min[2]],
                            [max[0], max[1], max[2]],
                        ];
                        if corners.iter().any(|&pos| {
                            self.induction_matrix_block(pos) != "mekanism:induction_casing"
                        }) {
                            continue;
                        }
                        let mut capacity = 0u64;
                        let mut transfer = 0u64;
                        let mut cells = 0u32;
                        let mut providers = 0u32;
                        let mut ports = Vec::new();
                        let mut valid = true;
                        'volume: for x in min[0]..=max[0] {
                            for y in min[1]..=max[1] {
                                for z in min[2]..=max[2] {
                                    let pos = [x, y, z];
                                    let edges = usize::from(x == min[0] || x == max[0])
                                        + usize::from(y == min[1] || y == max[1])
                                        + usize::from(z == min[2] || z == max[2]);
                                    let block = self.induction_matrix_block(pos);
                                    if edges >= 2 {
                                        if block != "mekanism:induction_casing" {
                                            valid = false;
                                            break 'volume;
                                        }
                                    } else if edges == 1 {
                                        if !matches!(
                                            block.as_str(),
                                            "mekanism:induction_casing"
                                                | "mekanism:induction_port"
                                                | "mekanism:structural_glass"
                                                | "mekanismgenerators:reactor_glass"
                                        ) {
                                            valid = false;
                                            break 'volume;
                                        }
                                        if block == "mekanism:induction_port" {
                                            ports.push(pos);
                                        }
                                    } else if !matches!(
                                        block.as_str(),
                                        "minecraft:air"
                                            | "minecraft:cave_air"
                                            | "minecraft:void_air"
                                    ) {
                                        if let Some(tier) = block
                                            .strip_prefix("mekanism:")
                                            .and_then(|n| n.strip_suffix("_induction_cell"))
                                        {
                                            capacity = capacity.saturating_add(match tier {
                                                "basic" => 3_200_000_000,
                                                "advanced" => 25_600_000_000,
                                                "elite" => 204_800_000_000,
                                                "ultimate" => 1_600_000_000_000,
                                                _ => 0,
                                            });
                                            cells += 1;
                                        } else if let Some(tier) = block
                                            .strip_prefix("mekanism:")
                                            .and_then(|n| n.strip_suffix("_induction_provider"))
                                        {
                                            transfer = transfer.saturating_add(match tier {
                                                "basic" => 25_600,
                                                "advanced" => 204_800,
                                                "elite" => 1_638_400,
                                                "ultimate" => 13_000_000,
                                                _ => 0,
                                            });
                                            providers += 1;
                                        } else {
                                            valid = false;
                                            break 'volume;
                                        }
                                    }
                                }
                            }
                        }
                        if valid && cells > 0 && providers > 0 && !ports.is_empty() {
                            return Some(InductionMatrixInfo {
                                min,
                                capacity,
                                transfer,
                                ports,
                            });
                        }
                    }
                }
            }
        }
        None
    }
    /// Finds the 3×3×3 through 18×18×18 Dynamic Tank around a valve.
    fn dynamic_tank_for_valve(&self, valve: [i32; 3]) -> Option<DynamicTankInfo> {
        let mut nearby_casing = 0usize;
        for x in valve[0] - 16..=valve[0] + 16 {
            for y in valve[1] - 16..=valve[1] + 16 {
                for z in valve[2] - 16..=valve[2] + 16 {
                    nearby_casing += usize::from(
                        self.induction_matrix_block([x, y, z]) == "mekanism:dynamic_tank",
                    );
                }
            }
        }
        if nearby_casing < 20 {
            return None;
        }
        let mut sizes = Vec::with_capacity(16 * 16 * 16);
        for sx in 3..=18 {
            for sy in 3..=18 {
                for sz in 3..=18 {
                    sizes.push([sx, sy, sz]);
                }
            }
        }
        sizes.sort_by_key(|size| size[0] * size[1] * size[2]);
        for size in sizes {
            for rx in 0..size[0] {
                for ry in 0..size[1] {
                    for rz in 0..size[2] {
                        let face = usize::from(rx == 0 || rx == size[0] - 1)
                            + usize::from(ry == 0 || ry == size[1] - 1)
                            + usize::from(rz == 0 || rz == size[2] - 1);
                        if face != 1 {
                            continue;
                        }
                        let min = [valve[0] - rx, valve[1] - ry, valve[2] - rz];
                        let max = [
                            min[0] + size[0] - 1,
                            min[1] + size[1] - 1,
                            min[2] + size[2] - 1,
                        ];
                        let corners = [
                            [min[0], min[1], min[2]],
                            [min[0], min[1], max[2]],
                            [min[0], max[1], min[2]],
                            [min[0], max[1], max[2]],
                            [max[0], min[1], min[2]],
                            [max[0], min[1], max[2]],
                            [max[0], max[1], min[2]],
                            [max[0], max[1], max[2]],
                        ];
                        if corners
                            .iter()
                            .any(|&pos| self.induction_matrix_block(pos) != "mekanism:dynamic_tank")
                        {
                            continue;
                        }
                        let mut valves = Vec::new();
                        let mut valid = true;
                        'volume: for x in min[0]..=max[0] {
                            for y in min[1]..=max[1] {
                                for z in min[2]..=max[2] {
                                    let edges = usize::from(x == min[0] || x == max[0])
                                        + usize::from(y == min[1] || y == max[1])
                                        + usize::from(z == min[2] || z == max[2]);
                                    let pos = [x, y, z];
                                    let block = self.induction_matrix_block(pos);
                                    if edges >= 2 {
                                        if block != "mekanism:dynamic_tank" {
                                            valid = false;
                                            break 'volume;
                                        }
                                    } else if edges == 1
                                        && !matches!(
                                            block.as_str(),
                                            "mekanism:dynamic_tank"
                                                | "mekanism:dynamic_valve"
                                                | "mekanism:structural_glass"
                                        )
                                    {
                                        valid = false;
                                        break 'volume;
                                    }
                                    if block == "mekanism:dynamic_valve" {
                                        valves.push(pos);
                                    }
                                }
                            }
                        }
                        if valid && !valves.is_empty() {
                            return Some(DynamicTankInfo {
                                min,
                                volume: (size[0] * size[1] * size[2]) as u64,
                                valves,
                            });
                        }
                    }
                }
            }
        }
        None
    }

    /// Finds a vertically oriented, square Industrial Turbine around a valve.
    fn turbine_for_valve(&self, valve: [i32; 3]) -> Option<TurbineInfo> {
        let mut nearby_casing = 0usize;
        for x in valve[0] - 18..=valve[0] + 18 {
            for y in valve[1] - 18..=valve[1] + 18 {
                for z in valve[2] - 18..=valve[2] + 18 {
                    nearby_casing += usize::from(
                        self.induction_matrix_block([x, y, z])
                            == "mekanismgenerators:turbine_casing",
                    );
                }
            }
        }
        if nearby_casing < 40 {
            return None;
        }

        // A valve must be on a vertical wall, away from the bottom, roof and corners.
        for width in 5..=17 {
            for height in 5..=18 {
                for (rx, rz) in (1..width - 1)
                    .map(|offset| (offset, 0))
                    .chain((1..width - 1).map(|offset| (offset, width - 1)))
                    .chain((1..width - 1).map(|offset| (0, offset)))
                    .chain((1..width - 1).map(|offset| (width - 1, offset)))
                {
                    for ry in 1..height - 1 {
                        let min = [valve[0] - rx, valve[1] - ry, valve[2] - rz];
                        let max = [min[0] + width - 1, min[1] + height - 1, min[2] + width - 1];
                        let mut valid = true;
                        let mut valves = Vec::new();
                        let mut vents = 0u64;
                        let mut dispersers = 0u64;
                        let mut blades = 0u64;
                        let mut coils = 0u64;
                        let mut rotational_complex = 0u64;
                        'structure: for x in min[0]..=max[0] {
                            for y in min[1]..=max[1] {
                                for z in min[2]..=max[2] {
                                    let on_x = x == min[0] || x == max[0];
                                    let on_y = y == min[1] || y == max[1];
                                    let on_z = z == min[2] || z == max[2];
                                    let face_count =
                                        usize::from(on_x) + usize::from(on_y) + usize::from(on_z);
                                    let block = self.induction_matrix_block([x, y, z]);
                                    if face_count == 3 || (face_count == 2 && on_y) {
                                        if block != "mekanismgenerators:turbine_casing" {
                                            valid = false;
                                            break 'structure;
                                        }
                                    } else if face_count == 1 {
                                        if y == min[1] {
                                            if !matches!(
                                                block.as_str(),
                                                "mekanismgenerators:turbine_casing"
                                                    | "mekanism:structural_glass"
                                            ) {
                                                valid = false;
                                                break 'structure;
                                            }
                                        } else if y == max[1] {
                                            if !matches!(
                                                block.as_str(),
                                                "mekanismgenerators:turbine_casing"
                                                    | "mekanismgenerators:turbine_vent"
                                            ) {
                                                valid = false;
                                                break 'structure;
                                            }
                                        } else if !matches!(
                                            block.as_str(),
                                            "mekanismgenerators:turbine_casing"
                                                | "mekanism:structural_glass"
                                                | "mekanismgenerators:turbine_valve"
                                                | "mekanismgenerators:turbine_vent"
                                        ) {
                                            valid = false;
                                            break 'structure;
                                        }
                                    } else {
                                        match block.as_str() {
                                            "mekanism:pressure_disperser" => dispersers += 1,
                                            "mekanismgenerators:turbine_vent" => vents += 1,
                                            "mekanismgenerators:turbine_rotor" => blades += 1,
                                            "mekanismgenerators:rotational_complex" => {
                                                rotational_complex += 1
                                            }
                                            "mekanismgenerators:electromagnetic_coil" => coils += 1,
                                            "mekanismgenerators:saturating_condenser"
                                            | "mekanismgenerators:turbine_casing"
                                            | "minecraft:air"
                                            | "minecraft:cave_air"
                                            | "minecraft:void_air" => {}
                                            _ => {
                                                valid = false;
                                                break 'structure;
                                            }
                                        }
                                    }
                                    if block == "mekanismgenerators:turbine_valve" {
                                        valves.push([x, y, z]);
                                    }
                                }
                            }
                        }
                        let rotor_height = blades;
                        let lower_volume = ((width - 2) * (width - 2)) as u64 * rotor_height;
                        let max_rate = (dispersers * 1_280 * lower_volume).min(vents * 32_000);
                        if valid
                            && valves.len() >= 2
                            && vents > 0
                            && dispersers > 0
                            && blades > 0
                            && coils > 0
                            && rotational_complex == 1
                            && rotor_height <= ((2 * width - 5).min(14) as u64)
                            && max_rate > 0
                        {
                            return Some(TurbineInfo {
                                min,
                                volume: ((width - 2) * (width - 2) * (height - 2)) as u64,
                                max_rate,
                                rotor_height,
                                rotors: 1,
                                valves,
                            });
                        }
                    }
                }
            }
        }
        None
    }

    /// Finds a closed Fission Reactor cuboid around one of its external ports.
    fn fission_reactor_for_port(&self, port: [i32; 3]) -> Option<FissionReactorInfo> {
        let mut nearby_casing = 0usize;
        for x in port[0] - 18..=port[0] + 18 {
            for y in port[1] - 18..=port[1] + 18 {
                for z in port[2] - 18..=port[2] + 18 {
                    nearby_casing += usize::from(
                        self.induction_matrix_block([x, y, z])
                            == "mekanismgenerators:fission_reactor_casing",
                    );
                }
            }
        }
        if nearby_casing < 20 {
            return None;
        }
        for sx in 3..=18 {
            for sy in 4..=18 {
                for sz in 3..=18 {
                    let size = [sx, sy, sz];
                    for rx in 0..sx {
                        for ry in 0..sy {
                            for rz in 0..sz {
                                let faces = usize::from(rx == 0 || rx == sx - 1)
                                    + usize::from(ry == 0 || ry == sy - 1)
                                    + usize::from(rz == 0 || rz == sz - 1);
                                if faces != 1 {
                                    continue;
                                }
                                let min = [port[0] - rx, port[1] - ry, port[2] - rz];
                                let max = [min[0] + sx - 1, min[1] + sy - 1, min[2] + sz - 1];
                                let mut valid = true;
                                let mut ports = Vec::new();
                                let mut fuel_assemblies = 0u64;
                                let mut fuel_columns = HashSet::new();
                                'shell: for x in min[0]..=max[0] {
                                    for y in min[1]..=max[1] {
                                        for z in min[2]..=max[2] {
                                            let faces = usize::from(x == min[0] || x == max[0])
                                                + usize::from(y == min[1] || y == max[1])
                                                + usize::from(z == min[2] || z == max[2]);
                                            let pos = [x, y, z];
                                            let block = self.induction_matrix_block(pos);
                                            if faces >= 2 {
                                                if block
                                                    != "mekanismgenerators:fission_reactor_casing"
                                                {
                                                    valid = false;
                                                    break 'shell;
                                                }
                                            } else if faces == 1 {
                                                if !matches!(
                                                    block.as_str(),
                                                    "mekanismgenerators:fission_reactor_casing"
                                                        | "mekanismgenerators:reactor_glass"
                                                        | "mekanismgenerators:fission_reactor_port"
                                                        | "mekanismgenerators:fission_reactor_logic_adapter"
                                                ) {
                                                    valid = false;
                                                    break 'shell;
                                                }
                                                if block
                                                    == "mekanismgenerators:fission_reactor_port"
                                                {
                                                    ports.push(pos);
                                                }
                                            } else {
                                                match block.as_str() {
                                                    "mekanismgenerators:fission_fuel_assembly" => {
                                                        fuel_assemblies += 1;
                                                        fuel_columns.insert((x, z));
                                                    }
                                                    "mekanismgenerators:control_rod_assembly"
                                                    | "minecraft:air"
                                                    | "minecraft:cave_air"
                                                    | "minecraft:void_air" => {}
                                                    _ => {
                                                        valid = false;
                                                        break 'shell;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if !valid || ports.len() < 4 || fuel_assemblies == 0 {
                                    continue;
                                }
                                // Fuel assemblies form separate vertical columns, capped by a control rod.
                                'columns: for &(x, z) in &fuel_columns {
                                    let mut found_fuel = false;
                                    let mut found_rod = false;
                                    let mut ended = false;
                                    for y in min[1] + 1..max[1] {
                                        let block = self.induction_matrix_block([x, y, z]);
                                        match block.as_str() {
                                            "mekanismgenerators:fission_fuel_assembly"
                                                if !ended =>
                                            {
                                                found_fuel = true;
                                            }
                                            "mekanismgenerators:control_rod_assembly"
                                                if found_fuel && !found_rod && !ended =>
                                            {
                                                found_rod = true;
                                                ended = true;
                                            }
                                            "minecraft:air" | "minecraft:cave_air"
                                            | "minecraft:void_air" => {
                                                if found_fuel {
                                                    ended = true;
                                                }
                                            }
                                            _ => {
                                                valid = false;
                                                break 'columns;
                                            }
                                        }
                                    }
                                    if !found_fuel || !found_rod {
                                        valid = false;
                                        break 'columns;
                                    }
                                }
                                if !valid {
                                    continue;
                                }
                                if fuel_columns.iter().any(|&(x, z)| {
                                    [(x + 1, z), (x - 1, z), (x, z + 1), (x, z - 1)]
                                        .iter()
                                        .any(|neighbor| fuel_columns.contains(neighbor))
                                }) {
                                    continue;
                                }
                                ports.sort_unstable();
                                return Some(FissionReactorInfo {
                                    min,
                                    size,
                                    ports,
                                    fuel_assemblies,
                                });
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Validates the fixed five-layer Fusion Reactor around its controller.
    fn fusion_reactor_for_controller(&self, controller: [i32; 3]) -> Option<FusionReactorInfo> {
        let min = [controller[0] - 2, controller[1] - 4, controller[2] - 2];
        let mut ports = Vec::new();
        let mut focus_matrices = 0u8;
        let mut valid = true;
        for layer in 0..5 {
            for x in 0..5 {
                for z in 0..5 {
                    let is_corner = matches!((x, z), (0, 0) | (0, 4) | (4, 0) | (4, 4));
                    let required = match layer {
                        0 => x == 2 || z == 2 || is_corner,
                        1 | 3 => (x == 0 || x == 4 || z == 0 || z == 4) && !is_corner,
                        2 => true,
                        4 => ((x == 2 || z == 2) && (x != 2 || z != 2)) || is_corner,
                        _ => false,
                    };
                    let at = [min[0] + x, min[1] + layer, min[2] + z];
                    let block = self.induction_matrix_block(at);
                    if layer == 4 && x == 2 && z == 2 {
                        if at != controller
                            || block != "mekanismgenerators:fusion_reactor_controller"
                        {
                            valid = false;
                            break;
                        }
                        continue;
                    }
                    if !required {
                        if block != "minecraft:air"
                            && block != "minecraft:cave_air"
                            && block != "minecraft:void_air"
                        {
                            valid = false;
                            break;
                        }
                        continue;
                    }
                    if is_corner {
                        if block != "mekanismgenerators:fusion_reactor_frame" {
                            valid = false;
                            break;
                        }
                        continue;
                    }
                    if !matches!(
                        block.as_str(),
                        "mekanismgenerators:fusion_reactor_frame"
                            | "mekanismgenerators:reactor_glass"
                            | "mekanismgenerators:fusion_reactor_port"
                            | "mekanismgenerators:fusion_reactor_logic_adapter"
                            | "mekanismgenerators:laser_focus_matrix"
                    ) {
                        valid = false;
                        break;
                    }
                    if block == "mekanismgenerators:fusion_reactor_port" {
                        ports.push(at);
                    }
                    if block == "mekanismgenerators:laser_focus_matrix" {
                        focus_matrices += 1;
                        if layer != 2 || x != 2 || z != 2 {
                            valid = false;
                            break;
                        }
                    }
                }
                if !valid {
                    break;
                }
            }
            if !valid {
                break;
            }
        }
        let center = [min[0] + 2, min[1] + 2, min[2] + 2];
        let external_heat_port =
            self.induction_matrix_block(center) == "mekanismgenerators:fusion_reactor_port";
        (valid && ports.len() >= 2 && (focus_matrices == 1 || external_heat_port)).then_some(
            FusionReactorInfo {
                min,
                ports,
                controller,
            },
        )
    }

    fn thermal_evaporation_air(&self, pos: [i32; 3]) -> bool {
        matches!(
            self.res
                .facts()
                .get(self.world.block(pos[0], pos[1], pos[2]))
                .name
                .as_ref(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    }
    /// Checks the 4×4, hollow, 3–18 block high Thermal Evaporation Plant.
    fn thermal_evaporation_valves(&self, controller: [i32; 3]) -> Option<Vec<[i32; 3]>> {
        for height in 3..=18 {
            for base_y in (controller[1] - height + 1)..=controller[1] {
                for base_x in (controller[0] - 3)..=controller[0] {
                    for base_z in (controller[2] - 3)..=controller[2] {
                        let rel_x = controller[0] - base_x;
                        let rel_z = controller[2] - base_z;
                        let rel_y = controller[1] - base_y;
                        if !(0..=3).contains(&rel_x)
                            || !(0..=3).contains(&rel_z)
                            || !(0..height).contains(&rel_y)
                        {
                            continue;
                        }
                        let border = rel_x == 0 || rel_x == 3 || rel_z == 0 || rel_z == 3;
                        let top_corner = rel_y == height - 1
                            && (rel_x == 0 || rel_x == 3)
                            && (rel_z == 0 || rel_z == 3);
                        if !border
                            || ((rel_x == 0 || rel_x == 3)
                                && (rel_z == 0 || rel_z == 3)
                                && !top_corner)
                        {
                            continue;
                        }
                        let mut valves = Vec::new();
                        let mut valid = true;
                        for y_layer in 0..height {
                            let y = base_y + y_layer;
                            for x in 0..4 {
                                for z in 0..4 {
                                    let at = [base_x + x, y, base_z + z];
                                    let edge = x == 0 || x == 3 || z == 0 || z == 3;
                                    if y_layer == 0 {
                                        if !self.thermal_evaporation_casing(at) {
                                            valid = false;
                                            break;
                                        }
                                    } else if edge {
                                        let corner = x != 1 && x != 2 && z != 1 && z != 2;
                                        if y_layer == height - 1
                                            && corner
                                            && self.thermal_evaporation_air(at)
                                        {
                                            continue;
                                        }
                                        if !self.thermal_evaporation_casing(at) {
                                            valid = false;
                                            break;
                                        }
                                    } else if !self.thermal_evaporation_air(at) {
                                        valid = false;
                                        break;
                                    }
                                    if self
                                        .res
                                        .facts()
                                        .get(self.world.block(at[0], at[1], at[2]))
                                        .name
                                        .as_ref()
                                        == "mekanism:thermal_evaporation_valve"
                                    {
                                        valves.push(at);
                                    }
                                }
                                if !valid {
                                    break;
                                }
                            }
                            if !valid {
                                break;
                            }
                        }
                        if valid && !valves.is_empty() {
                            return Some(valves);
                        }
                    }
                }
            }
        }
        None
    }
    /// Ticks all loaded mod block entities in block-entity order, in one
    /// fuel-budgeted call per mod, and returns their game-owned state.
    pub(super) fn tick_mod_block_entities(&mut self, positions: &[(i32, i32, i32)]) {
        let dimension = self.dim.id().to_owned();
        // The server capability is useful during a machine tick only for
        // positions the game has copied into its read cache. The pump needs
        // six adjacent cells, so refresh just that small neighborhood.
        for &(x, y, z) in positions {
            if !matches!(
                self.res
                    .facts()
                    .get(self.world.block(x, y, z))
                    .name
                    .as_ref(),
                "mekanism:electric_pump" | "mekanism:fluidic_plenisher"
            ) {
                continue;
            }
            for [dx, dy, dz] in [
                [0, 0, 0],
                [1, 0, 0],
                [-1, 0, 0],
                [0, 1, 0],
                [0, -1, 0],
                [0, 0, 1],
                [0, 0, -1],
            ] {
                let (nx, ny, nz) = (x + dx, y + dy, z + dz);
                let state = self.world.block(nx, ny, nz);
                let name = self.res.facts().get(state).name.to_string();
                let state_name = self
                    .pack
                    .assets
                    .registry
                    .state(state)
                    .map_or(name.clone(), |s| {
                        if s.properties.is_empty() {
                            name.clone()
                        } else {
                            let properties = s
                                .properties
                                .iter()
                                .map(|(key, value)| format!("{key}={value}"))
                                .collect::<Vec<_>>()
                                .join(",");
                            format!("{name}[{properties}]")
                        }
                    });
                self.mods
                    .mods
                    .set_world_block(&dimension, nx, ny, nz, &state_name);
            }
        }
        if self.ticks.is_multiple_of(20) {
            self.mods.induction_matrix_ports.clear();
            let mut checked = HashSet::new();
            for &(x, y, z) in positions {
                let port = [x, y, z];
                if self.induction_matrix_block(port) != "mekanism:induction_port"
                    || !checked.insert(port)
                    || self.mods.induction_matrix_ports.contains_key(&port)
                {
                    continue;
                }
                if let Some(matrix) = self.induction_matrix_for_port(port) {
                    let link = format!(
                        "{},{},{};{};{}",
                        matrix.min[0],
                        matrix.min[1],
                        matrix.min[2],
                        matrix.capacity,
                        matrix.transfer
                    );
                    for matrix_port in matrix.ports {
                        self.mods
                            .induction_matrix_ports
                            .insert(matrix_port, link.clone());
                    }
                }
            }
        }
        for &(x, y, z) in positions {
            if self.induction_matrix_block([x, y, z]) == "mekanism:induction_port" {
                let rule = format!("mekanism:induction/port/{dimension}/{x},{y},{z}");
                let value = self
                    .mods
                    .induction_matrix_ports
                    .get(&[x, y, z])
                    .map_or("", String::as_str);
                self.mods.mods.set_rule(&rule, value);
            }
        }
        if self.ticks.is_multiple_of(20) {
            self.mods.dynamic_tank_valves.clear();
            let mut checked = HashSet::new();
            for &(x, y, z) in positions {
                let valve = [x, y, z];
                if self.induction_matrix_block(valve) != "mekanism:dynamic_valve"
                    || !checked.insert(valve)
                    || self.mods.dynamic_tank_valves.contains_key(&valve)
                {
                    continue;
                }
                if let Some(tank) = self.dynamic_tank_for_valve(valve) {
                    let link = format!(
                        "{},{},{};{};{}",
                        tank.min[0],
                        tank.min[1],
                        tank.min[2],
                        tank.volume,
                        tank.valves.len()
                    );
                    for tank_valve in tank.valves {
                        self.mods
                            .dynamic_tank_valves
                            .insert(tank_valve, link.clone());
                    }
                }
            }
        }
        for &(x, y, z) in positions {
            if self.induction_matrix_block([x, y, z]) == "mekanism:dynamic_valve" {
                let rule = format!("mekanism:dynamic/valve/{dimension}/{x},{y},{z}");
                let value = self
                    .mods
                    .dynamic_tank_valves
                    .get(&[x, y, z])
                    .map_or("", String::as_str);
                self.mods.mods.set_rule(&rule, value);
            }
        }
        if self.ticks.is_multiple_of(20) {
            self.mods.turbine_valves.clear();
            let mut checked = HashSet::new();
            for &(x, y, z) in positions {
                let valve = [x, y, z];
                if self.induction_matrix_block(valve) != "mekanismgenerators:turbine_valve"
                    || !checked.insert(valve)
                    || self.mods.turbine_valves.contains_key(&valve)
                {
                    continue;
                }
                if let Some(turbine) = self.turbine_for_valve(valve) {
                    let link = format!(
                        "{},{},{};{};{};{};{};{}",
                        turbine.min[0],
                        turbine.min[1],
                        turbine.min[2],
                        turbine.volume,
                        turbine.max_rate,
                        turbine.rotor_height,
                        turbine.rotors,
                        turbine.valves.len()
                    );
                    for turbine_valve in turbine.valves {
                        self.mods.turbine_valves.insert(turbine_valve, link.clone());
                    }
                }
            }
        }
        for &(x, y, z) in positions {
            if self.induction_matrix_block([x, y, z]) == "mekanismgenerators:turbine_valve" {
                let rule = format!("mekanismgenerators:turbine/valve/{dimension}/{x},{y},{z}");
                let value = self
                    .mods
                    .turbine_valves
                    .get(&[x, y, z])
                    .map_or("", String::as_str);
                self.mods.mods.set_rule(&rule, value);
            }
        }
        if self.ticks.is_multiple_of(20) {
            self.mods.fission_reactor_ports.clear();
            let mut checked = HashSet::new();
            for &(x, y, z) in positions {
                let port = [x, y, z];
                if self.induction_matrix_block(port) != "mekanismgenerators:fission_reactor_port"
                    || !checked.insert(port)
                    || self.mods.fission_reactor_ports.contains_key(&port)
                {
                    continue;
                }
                if let Some(reactor) = self.fission_reactor_for_port(port) {
                    let link = format!(
                        "{},{},{};{},{},{};{};{};{},{},{}",
                        reactor.min[0],
                        reactor.min[1],
                        reactor.min[2],
                        reactor.size[0],
                        reactor.size[1],
                        reactor.size[2],
                        reactor.fuel_assemblies,
                        reactor.ports.len(),
                        reactor.ports[0][0],
                        reactor.ports[0][1],
                        reactor.ports[0][2]
                    );
                    for reactor_port in reactor.ports {
                        self.mods
                            .fission_reactor_ports
                            .insert(reactor_port, link.clone());
                    }
                }
            }
        }
        for &(x, y, z) in positions {
            if self.induction_matrix_block([x, y, z]) == "mekanismgenerators:fission_reactor_port" {
                let rule = format!("mekanismgenerators:fission/port/{dimension}/{x},{y},{z}");
                let value = self
                    .mods
                    .fission_reactor_ports
                    .get(&[x, y, z])
                    .map_or("", String::as_str);
                self.mods.mods.set_rule(&rule, value);
            }
        }
        if self.ticks.is_multiple_of(20) {
            self.mods.fusion_reactor_components.clear();
            for &(x, y, z) in positions {
                let controller = [x, y, z];
                if self.induction_matrix_block(controller)
                    != "mekanismgenerators:fusion_reactor_controller"
                {
                    continue;
                }
                let Some(reactor) = self.fusion_reactor_for_controller(controller) else {
                    continue;
                };
                let link = format!(
                    "{},{},{};{};{},{},{}",
                    reactor.min[0],
                    reactor.min[1],
                    reactor.min[2],
                    reactor.ports.len(),
                    reactor.controller[0],
                    reactor.controller[1],
                    reactor.controller[2]
                );
                self.mods
                    .fusion_reactor_components
                    .insert(controller, link.clone());
                for port in reactor.ports {
                    self.mods
                        .fusion_reactor_components
                        .insert(port, link.clone());
                }
            }
        }
        for &(x, y, z) in positions {
            let pos = [x, y, z];
            let block = self.induction_matrix_block(pos);
            if block == "mekanismgenerators:fusion_reactor_controller" {
                let rule = format!("mekanismgenerators:fusion/controller/{dimension}/{x},{y},{z}");
                let value = self
                    .mods
                    .fusion_reactor_components
                    .get(&pos)
                    .map_or("", String::as_str);
                self.mods.mods.set_rule(&rule, value);
            } else if block == "mekanismgenerators:fusion_reactor_port" {
                let rule = format!("mekanismgenerators:fusion/port/{dimension}/{x},{y},{z}");
                let value = self
                    .mods
                    .fusion_reactor_components
                    .get(&pos)
                    .map_or("", String::as_str);
                self.mods.mods.set_rule(&rule, value);
            }
        }
        let mut controllers = Vec::new();
        let mut valves = Vec::new();
        for &(x, y, z) in positions {
            let name = &self.res.facts().get(self.world.block(x, y, z)).name;
            if name.as_ref() == "mekanism:thermal_evaporation_controller" {
                controllers.push([x, y, z]);
                self.mods.mods.set_rule(
                    &format!("mekanism:tep/controller/{dimension}/{x},{y},{z}"),
                    "false",
                );
            } else if name.as_ref() == "mekanism:thermal_evaporation_valve" {
                valves.push([x, y, z]);
                self.mods
                    .mods
                    .set_rule(&format!("mekanism:tep/valve/{dimension}/{x},{y},{z}"), "");
            }
        }
        if self.ticks.is_multiple_of(20) {
            self.mods.thermal_evaporation_plants.clear();
            for &controller in &controllers {
                self.mods.mods.set_rule(
                    &format!(
                        "mekanism:tep/heat/{dimension}/{},{},{}",
                        controller[0], controller[1], controller[2]
                    ),
                    "false",
                );
                if let Some(plant_valves) = self.thermal_evaporation_valves(controller) {
                    self.mods
                        .thermal_evaporation_plants
                        .insert(controller, plant_valves);
                }
            }
            self.mods.thermal_evaporation_checked_tick = self.ticks;
        }
        if self.mods.thermal_evaporation_checked_tick == self.ticks
            || !self.mods.thermal_evaporation_plants.is_empty()
        {
            for controller in controllers {
                let Some(plant_valves) = self.mods.thermal_evaporation_plants.get(&controller)
                else {
                    continue;
                };
                self.mods.mods.set_rule(
                    &format!(
                        "mekanism:tep/controller/{dimension}/{},{},{}",
                        controller[0], controller[1], controller[2]
                    ),
                    "true",
                );
                for &valve in plant_valves {
                    self.mods.mods.set_rule(
                        &format!(
                            "mekanism:tep/valve/{dimension}/{},{},{}",
                            valve[0], valve[1], valve[2]
                        ),
                        &format!("{},{},{}", controller[0], controller[1], controller[2]),
                    );
                }
            }
        }
        let mut entries = Vec::new();
        let mut views = Vec::new();
        for &pos in positions {
            if !matches!(
                self.world.block_entities.get(&pos),
                Some(horizon_client::containers::BlockEntity::Mod { .. })
            ) {
                let state = self.world.block(pos.0, pos.1, pos.2);
                let block = self.res.facts().get(state).name.to_string();
                self.ensure_mod_block_entity([pos.0, pos.1, pos.2], &block);
            }
            let Some(horizon_client::containers::BlockEntity::Mod {
                block,
                mut handle,
                saved,
                mut items,
                energy,
                fluids,
            }) = self.world.block_entities.take(&pos)
            else {
                continue;
            };
            if handle == 0 {
                let Some((new_handle, slots)) = self.mods.mods.create_block_entity_at(
                    &block,
                    &saved,
                    Some((self.dim.id().to_owned(), pos.0, pos.1, pos.2)),
                ) else {
                    self.world.block_entities.put(
                        pos,
                        horizon_client::containers::BlockEntity::Mod {
                            block,
                            handle,
                            saved,
                            items,
                            energy,
                            fluids,
                        },
                    );
                    continue;
                };
                handle = new_handle;
                items.resize(slots as usize, None);
            }
            if let Some(energy) = &energy {
                self.mods.mods.set_energy_buffer(
                    &format!("{}:{},{},{}", self.dim.id(), pos.0, pos.1, pos.2),
                    horizon_mods::EnergyView {
                        stored: energy.stored,
                        capacity: energy.capacity,
                        max_receive: energy.max_receive,
                        max_extract: energy.max_extract,
                    },
                );
            }
            for (tank, fluid) in &fluids {
                self.mods.mods.set_fluid_tank(
                    &format!("{}:{},{},{}", self.dim.id(), pos.0, pos.1, pos.2),
                    *tank,
                    horizon_mods::FluidView {
                        fluid: fluid.fluid.clone(),
                        amount: fluid.amount,
                        capacity: fluid.capacity,
                    },
                );
            }
            let state = self.world.block(pos.0, pos.1, pos.2);
            let mut properties = Vec::new();
            if let Some(current) = self.pack.assets.registry.state(state) {
                for (name, value) in &current.properties {
                    let mut values: Vec<String> = (0..self.pack.assets.registry.len())
                        .map(|i| horizon_world::registry::BlockStateId(i as u32))
                        .filter(|&id| self.res.facts().get(id).name.as_ref() == block)
                        .filter_map(|id| self.pack.assets.registry.state(id))
                        .filter_map(|s| s.properties.get(name).cloned())
                        .collect();
                    values.sort();
                    values.dedup();
                    properties.push(horizon_mods::Property {
                        name: name.clone(),
                        value: value.clone(),
                        values,
                    });
                }
            }
            views.push(BlockEntityView {
                handle,
                dimension: self.dim.id().to_owned(),
                x: pos.0,
                y: pos.1,
                z: pos.2,
                slots: items
                    .iter()
                    .map(|slot| {
                        slot.as_ref().map(|s| Stack {
                            id: s.item.to_string(),
                            count: s.count,
                        })
                    })
                    .collect(),
                slot_components: items
                    .iter()
                    .map(|slot| slot.as_ref().map_or_else(Vec::new, item_components))
                    .collect(),
                energy: energy.as_ref().map(|e| horizon_mods::EnergyView {
                    stored: e.stored,
                    capacity: e.capacity,
                    max_receive: e.max_receive,
                    max_extract: e.max_extract,
                }),
                fluids: fluids
                    .iter()
                    .map(|(tank, f)| {
                        (
                            *tank,
                            horizon_mods::FluidView {
                                fluid: f.fluid.clone(),
                                amount: f.amount,
                                capacity: f.capacity,
                            },
                        )
                    })
                    .collect(),
                properties,
                changed: false,
            });
            entries.push((pos, block, handle, saved, items, state));
        }
        self.mods.mods.tick_block_entities(&mut views);
        for ((pos, block, handle, old_saved, mut items, state), view) in
            entries.into_iter().zip(views)
        {
            if view.changed {
                items = view
                    .slots
                    .iter()
                    .enumerate()
                    .map(|(index, slot)| {
                        slot.as_ref().map(|s| {
                            let mut stack = ItemStack::new(&s.id, s.count);
                            restore_item_components(
                                &mut stack,
                                view.slot_components.get(index).map_or(&[], Vec::as_slice),
                            );
                            stack
                        })
                    })
                    .collect();
                if let Some(mut block_state) = self.pack.assets.registry.state(state).cloned() {
                    for property in &view.properties {
                        block_state
                            .properties
                            .insert(property.name.clone(), property.value.clone());
                    }
                    if let Ok(id) = self.pack.assets.registry.resolve(&block_state) {
                        let _ = self.world.set(pos.0, pos.1, pos.2, id);
                    }
                }
            }
            let energy =
                view.energy
                    .as_ref()
                    .map(|e| horizon_client::containers::ModEnergyBuffer {
                        stored: e.stored,
                        capacity: e.capacity,
                        max_receive: e.max_receive,
                        max_extract: e.max_extract,
                    });
            let fluids: std::collections::HashMap<_, _> = view
                .fluids
                .iter()
                .map(|(tank, fluid)| {
                    (
                        *tank,
                        horizon_client::containers::ModFluidTank {
                            fluid: fluid.fluid.clone(),
                            amount: fluid.amount,
                            capacity: fluid.capacity,
                        },
                    )
                })
                .collect();
            let saved = self
                .mods
                .mods
                .save_block_entity(handle)
                .unwrap_or(old_saved.clone());
            if view.changed || saved != old_saved {
                self.world.modified.insert((pos.0 >> 4, pos.2 >> 4));
            }
            self.mods.block_entities.insert([pos.0, pos.1, pos.2], view);
            self.world.block_entities.put(
                pos,
                horizon_client::containers::BlockEntity::Mod {
                    block,
                    handle,
                    saved,
                    items,
                    energy,
                    fluids,
                },
            );
        }
        self.show_mod_output();
    }
    /// Run a server action in its requested level, returning to the level that
    /// emitted it. In-frame mode has only its current dimension loaded.
    fn with_mod_dimension<T>(
        &mut self,
        dimension: &str,
        f: impl FnOnce(&mut Self) -> T,
    ) -> Option<T> {
        let target = super::Dim::from_id(dimension);
        if matches!(target, super::Dim::Custom(_)) && super::dimension_info(dimension).is_none() {
            return None;
        }
        let back = self.enter_command_level(target);
        if self.dim != target {
            return None;
        }
        let out = f(self);
        if let Some(back) = back {
            self.enter_command_level(back);
        }
        Some(out)
    }

    /// The destination chunk for server actions that need a loaded world.
    pub(super) fn pending_mod_action_dimensions(&self) -> Vec<super::Dim> {
        self.mods
            .pending_server_actions
            .iter()
            .filter_map(|(action, _)| {
                mod_action_chunk(action).map(|(dim, _, _)| super::Dim::from_id(dim))
            })
            .collect()
    }

    /// Chat lines, command feedback and stopped mods, into the chat.
    pub(super) fn show_mod_output(&mut self) {
        let mut outputs: Vec<_> = self
            .mods
            .mods
            .drain()
            .into_iter()
            .map(|output| (output, None))
            .collect();
        outputs.extend(
            std::mem::take(&mut self.mods.pending_server_actions)
                .into_iter()
                .map(|(action, deadline)| (Output::Server(action), Some(deadline))),
        );
        for (output, pending_age) in outputs {
            let line = match output {
                Output::Chat { name, text } => {
                    let mut line = Text::literal(format!("[{name}] ")).with_color(text::GRAY);
                    line.push(Text::literal(text));
                    line
                }
                Output::Feedback { text, error: false } => Text::literal(text),
                Output::Feedback { text, error: true } => Text::literal(text).with_color(text::RED),
                Output::Stopped { name, reason } => {
                    self.list_mods();
                    self.sync_mod_commands();
                    Text::literal(format!("Mod {name} stopped: {reason}")).with_color(text::RED)
                }
                Output::Server(action) => {
                    let needs_tick_ticket = matches!(&action, ServerAction::ScheduleTick { .. });
                    if let Some((dimension, x, z)) = mod_action_chunk(&action) {
                        let dimension = dimension.to_owned();
                        let key = (dimension.clone(), x >> 4, z >> 4);
                        let loaded = self.with_mod_dimension(&dimension, |game| {
                            if needs_tick_ticket
                                && game.streaming.forced.insert((x >> 4, z >> 4))
                            {
                                game.mods.mod_action_tickets.insert(key.clone());
                                log::info!(
                                    "Horizon: mod scheduled-tick action requested a ticking ticket for {dimension:?} [{}, {}]",
                                    x >> 4,
                                    z >> 4
                                );
                            }
                            if game.world.loaded(x, z) {
                                true
                            } else {
                                if game.streaming.forced.insert((x >> 4, z >> 4)) {
                                    game.mods.mod_action_tickets.insert(key.clone());
                                    log::info!(
                                        "Horizon: mod action requested a chunk ticket for {dimension:?} [{}, {}]",
                                        x >> 4,
                                        z >> 4
                                    );
                                }
                                false
                            }
                        });
                        if loaded == Some(false) {
                            let deadline = pending_age
                                .unwrap_or_else(|| self.world.game_time.saturating_add(1_200));
                            if self.world.game_time < deadline
                                && self.mods.pending_server_actions.len() < 256
                            {
                                self.mods.pending_server_actions.push((action, deadline));
                            } else {
                                log::warn!(
                                    "Horizon: dropped mod action after waiting for chunk ({}, {}) in {dimension:?}",
                                    key.1,
                                    key.2
                                );
                            }
                            continue;
                        }
                        if loaded == Some(true) && pending_age.is_some() {
                            log::info!(
                                "Horizon: mod action chunk ready in {dimension:?} [{}, {}]",
                                key.1,
                                key.2
                            );
                        }
                    }
                    match action {
                        ServerAction::SetBlock {
                            dimension,
                            x,
                            y,
                            z,
                            state,
                        } => {
                            if self
                                .with_mod_dimension(&dimension, |game| {
                                    if let Some(id) = mod_block_state(&state).and_then(|state| {
                                        game.pack.assets.registry.resolve(&state).ok()
                                    }) {
                                        if let Err(error) = game.level_set([x, y, z], id) {
                                            log::warn!("Horizon: mod set-block failed: {error}");
                                        }
                                    }
                                })
                                .is_none()
                            {
                                log::warn!(
                                    "Horizon: mod set-block requested unavailable dimension {dimension:?}"
                                );
                            }
                        }
                        ServerAction::SpawnDrop {
                            dimension,
                            x,
                            y,
                            z,
                            id,
                            count,
                        } => {
                            if self
                                .with_mod_dimension(&dimension, |game| {
                                    game.pop_resource([x, y, z], ItemStack::new(&id, count));
                                })
                                .is_none()
                            {
                                log::warn!(
                                    "Horizon: mod spawn-drop requested unavailable dimension {dimension:?}"
                                );
                            }
                        }
                        ServerAction::SpawnMob {
                            dimension,
                            x,
                            y,
                            z,
                            entity,
                        } => {
                            if self
                                .with_mod_dimension(&dimension, |game| {
                                    let at = [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5];
                                    game.spawn_mob_for(&entity, at, true, true).is_some()
                                })
                                .is_none_or(|spawned| !spawned)
                            {
                                if self.dim.id() != dimension {
                                    log::warn!(
                                        "Horizon: mod spawn-mob requested unavailable dimension {dimension:?}"
                                    );
                                } else {
                                    log::warn!(
                                        "Horizon: mod requested unknown or unavailable mob {entity:?}"
                                    );
                                }
                            }
                        }
                        ServerAction::PlaySound {
                            dimension,
                            x,
                            y,
                            z,
                            sound,
                            volume,
                            pitch,
                        } => {
                            if self
                                .with_mod_dimension(&dimension, |game| {
                                    game.audio.play(
                                        &sound,
                                        Some([
                                            f64::from(x) + 0.5,
                                            f64::from(y) + 0.5,
                                            f64::from(z) + 0.5,
                                        ]),
                                        volume,
                                        pitch,
                                    );
                                })
                                .is_none()
                            {
                                log::warn!(
                                    "Horizon: mod play-sound requested unavailable dimension {dimension:?}"
                                );
                            }
                        }
                        ServerAction::ScheduleTick {
                            dimension,
                            x,
                            y,
                            z,
                            block,
                            delay,
                        } => {
                            let key = (dimension.clone(), x >> 4, z >> 4);
                            match self.with_mod_dimension(&dimension, |game| {
                                if game
                                    .res
                                    .facts()
                                    .get(game.world.block(x, y, z))
                                    .name
                                    .as_ref()
                                    != block
                                {
                                    return None;
                                }
                                let chunk = (x >> 4, z >> 4);
                                if game
                                    .block_ticks
                                    .blocks
                                    .chunk(horizon_core::position::ChunkPos::new(chunk.0, chunk.1))
                                    .is_none()
                                {
                                    game.register_chunk_ticks(chunk);
                                }
                                Some(game.with_level(|b, level, ticks| {
                                    b.schedule_tick(
                                        level,
                                        ticks,
                                        horizon_core::position::BlockPos::new(x, y, z),
                                        delay.min(i32::MAX as u32) as i32,
                                    )
                                }))
                            }) {
                                Some(Some(ScheduleResult::Added)) => {
                                    if self.mods.mod_action_tickets.contains(&key) {
                                        self.mods.scheduled_mod_action_tickets.insert(
                                            (dimension.clone(), x, y, z),
                                            (
                                                key.1,
                                                key.2,
                                                self.world
                                                    .game_time
                                                    .saturating_add(i64::from(delay))
                                                    .saturating_add(1_200),
                                            ),
                                        );
                                    }
                                    log::info!(
                                        "Horizon: mod scheduled {block} tick in {dimension:?} at ({x}, {y}, {z})"
                                    );
                                }
                                Some(Some(ScheduleResult::Duplicate)) => {
                                    if self.mods.mod_action_tickets.contains(&key) {
                                        self.mods.scheduled_mod_action_tickets.insert(
                                            (dimension.clone(), x, y, z),
                                            (
                                                key.1,
                                                key.2,
                                                self.world
                                                    .game_time
                                                    .saturating_add(i64::from(delay))
                                                    .saturating_add(1_200),
                                            ),
                                        );
                                    }
                                    log::debug!(
                                        "Horizon: mod schedule-tick already exists for {block} in {dimension:?} at ({x}, {y}, {z})"
                                    );
                                }
                                Some(Some(ScheduleResult::MissingChunk)) => log::warn!(
                                    "Horizon: mod could not schedule {block} tick because its chunk is not registered in {dimension:?} at ({x}, {y}, {z})"
                                ),
                                Some(None) => log::warn!(
                                    "Horizon: mod schedule-tick block mismatch in {dimension:?} at ({x}, {y}, {z}); expected {block}"
                                ),
                                None => log::warn!(
                                    "Horizon: mod schedule-tick requested unavailable dimension {dimension:?}"
                                ),
                            }
                        }
                        ServerAction::SetRule { name, value } => {
                            self.mods.mods.set_rule(&name, &value);
                        }
                    }
                    continue;
                }
                Output::Teleported {
                    dimension, x, y, z, ..
                } => {
                    if dimension == self.dim.id()
                        && let Some(p) = self.player.as_mut()
                    {
                        p.position.x = x;
                        p.position.y = y;
                        p.position.z = z;
                    }
                    continue;
                }
                Output::PacketToServer {
                    channel, payload, ..
                } => {
                    if let Some(server) = self.integrated.as_ref() {
                        if !server.send_mod_packet(ModPacket { channel, payload }) {
                            log::warn!("Horizon: dropped a mod packet to the integrated server");
                        }
                    } else {
                        log::warn!("Horizon: mod packets need an integrated server");
                    }
                    continue;
                }
                Output::PacketToClient {
                    player,
                    channel,
                    payload,
                    ..
                } => {
                    let delivered = self.serving.as_ref().is_some_and(|server| {
                        server.send_mod_packet(&player, ModPacket { channel, payload })
                    });
                    if !delivered {
                        log::warn!("Horizon: dropped a mod packet for {player:?}");
                    }
                    continue;
                }
                Output::ScreenOpened { .. } | Output::ScreenClosed { .. } => continue,
            };
            // Not through system_message: a mod must not hear its own lines.
            let focused = self.screen == Screen::Chat;
            self.ui
                .chat
                .add(&self.gui.atlas, line, None, self.ticks, focused);
        }
        self.release_mod_action_tickets();
    }
    /// Delivers packets the integrated server queued for the window.
    pub(super) fn poll_mod_packets(&mut self) {
        #[cfg(feature = "mods")]
        {
            let packets = self
                .integrated
                .as_ref()
                .map_or_else(Vec::new, |server| server.drain_mod_packets());
            if packets.is_empty() {
                return;
            }
            for packet in packets {
                self.deliver_mod_packet(&packet.channel, &packet.payload, None);
            }
        }
    }
    /// A packet received from the other half of the local integrated mod.
    #[cfg(feature = "mods")]
    pub(super) fn deliver_mod_packet(
        &mut self,
        channel: &str,
        payload: &[u8],
        from_player: Option<&str>,
    ) {
        self.mods.mods.deliver_packet(channel, payload, from_player);
        self.show_mod_output();
    }
    /// The mods' HUD panels, stacked with the extras in their corners.
    pub(super) fn draw_mod_panels(&self, batch: &mut HudBatch, corners: &mut Corners) {
        for panel in self.mods.mods.panels() {
            let corner = match panel.corner {
                horizon_mods::Corner::TopLeft => Corner::TopLeft,
                horizon_mods::Corner::TopRight => Corner::TopRight,
                horizon_mods::Corner::BottomLeft => Corner::BottomLeft,
                horizon_mods::Corner::BottomRight => Corner::BottomRight,
            };
            // F3 takes the top left, as it does from the extras.
            if self.debug && corner == Corner::TopLeft {
                continue;
            }
            let lines: Vec<(&str, u32)> = panel
                .lines
                .iter()
                .map(|l| (l.text.as_str(), l.color))
                .collect();
            extras_hud::draw_lines(batch, &self.gui.atlas, corners, corner, &lines);
        }
    }
}

#[cfg(feature = "mods")]
fn mod_action_chunk(action: &ServerAction) -> Option<(&str, i32, i32)> {
    match action {
        ServerAction::SetBlock {
            dimension, x, z, ..
        }
        | ServerAction::ScheduleTick {
            dimension, x, z, ..
        }
        | ServerAction::SpawnDrop {
            dimension, x, z, ..
        }
        | ServerAction::SpawnMob {
            dimension, x, z, ..
        } => Some((dimension, *x, *z)),
        ServerAction::PlaySound { .. } | ServerAction::SetRule { .. } => None,
    }
}

#[cfg(any(feature = "mods", test))]
fn scaled_xp(amount: u32, multiplier: f32) -> u32 {
    let scaled = f64::from(amount) * f64::from(multiplier);
    let maximum = f64::from(i32::MAX);
    if !scaled.is_finite() || scaled >= maximum {
        i32::MAX as u32
    } else if scaled <= 0.0 {
        0
    } else {
        scaled.round() as u32
    }
}

#[cfg(all(test, feature = "mods"))]
mod xp_tests {
    use super::scaled_xp;

    #[test]
    fn xp_multipliers_round_and_saturate() {
        assert_eq!(scaled_xp(7, 1.5), 11);
        assert_eq!(scaled_xp(7, 0.5), 4);
        assert_eq!(scaled_xp(7, 0.0), 0);
        assert_eq!(scaled_xp(u32::MAX, f32::MAX), i32::MAX as u32);
    }
}

#[cfg(test)]
mod loot_tests {
    use super::ModHookOutcome;

    #[test]
    fn loot_hook_outcomes_select_the_default_and_mod_drops() {
        assert!(ModHookOutcome::Pass.keeps_vanilla_drops());
        assert!(ModHookOutcome::Pass.keeps_mod_drops());
        assert!(!ModHookOutcome::Consume.keeps_vanilla_drops());
        assert!(ModHookOutcome::Consume.keeps_mod_drops());
        assert!(!ModHookOutcome::Cancel.keeps_vanilla_drops());
        assert!(!ModHookOutcome::Cancel.keeps_mod_drops());
    }
}

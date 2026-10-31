# Mod API completeness audit

Audit date: 2026-10-28. This records what is present in the API contract, SDK,
WASM runtime and game bridge. An interface declared in WIT, or a runtime unit
test, does not by itself mean a pack feature works in a playable world.

## Summary

| Line | Contract / SDK | Runtime | Game integration | Status |
|---|---|---|---|---|
| 0.3 logic | Present | Present | Present for the client and local game hooks inspected | Implemented in parts; full multiplayer and per-capability gameplay coverage is not established |
| 0.4 machines | Present | Present | Machine lifecycle, saves, menus, recipes and capability plumbing exist; mod packets bridge the integrated single-player client and server | Usable foundation; directional transfer, broad recipe behavior, LAN/dedicated networking and some persistence cases remain incomplete |
| 0.5 deep | Present | Present, including runtime tests | Custom entity lifecycle (`mod-entities` + `entities`), model animations/bones/draw-lists (`models`), custom dimensions/portals/travel/worldgen (`dimensions`), crop growth, explosion filtering, enchantment reads, damage, XP and loot hooks have live single-player integration | Substantial single-player live bridge; LAN/dedicated parity remains deferred |

## Scope for the current ATM10 core milestone

**Active:** single-player API readiness across 0.3–0.5, including the machine
path and gameplay hooks. The [feature matrix](feature-matrix.md) is the current
item-by-item backlog and should be updated alongside this evidence log.

**Deferred:** LAN guest, remote multiplayer and dedicated-server parity for mod
APIs. Their interfaces remain available where implemented, but single-player
coverage is the current scope.

**None of the three lines should be described as complete for arbitrary mods.**
Progress and remaining work are tracked by capability in the feature matrix;
this audit records concrete evidence and its limits.

## Evidence and limits

### 0.3 — logic and hooks

- The WIT contract and SDK expose config, server mutations, keybindings, target,
  inventory snapshots, block/item callbacks and server events.
- `horizon-mods` implements the host interfaces and has sandbox coverage for
  logic hooks, manifest capability checks, storage and validated config
  persistence (`.horizon-mod-data/<mod_id>.config.json`) across mod reload and
  game restart (`tools/mod_api_e2e.py --mode both`).
- `horizon-client/src/game/mods.rs` dispatches block interactions, commands,
  chat, ticks, frames, target and inventory snapshots. This is evidence of an
  in-game path, but does not verify every action in both in-frame and integrated
  server modes, LAN, and dedicated-server mode.
- The game refreshes `world-read` with loaded block states, biomes and packed
  light in a four-block neighborhood around the player. Queries are scoped to
  the active dimension; runtime sandbox coverage checks two dimensions sharing
  the same coordinates.
- The `server` capability can spawn vanilla mobs and mod-declared custom
  entities at a position. In integrated single-player, server actions enter
  their requested registered level; in-frame mode has only the current
  dimension. Unknown ids and unavailable dimensions are logged by the game
  bridge. A live single-player check set a block in an already-loaded Nether
  chunk from the Overworld and read it back through `horizon-ctl`.
- World mutations to unloaded chunks now request a temporary chunk ticket and
  remain queued until the chunk loads. The queue is capped at 256 actions and
  each action has a deadline 1,200 world ticks after it was queued; overflow
  and timeout are logged. Pending dimensions are included in the integrated
  server's tick list, and tickets owned by this queue are removed after
  application. A fresh isolated single-player world verified unloaded
  `set-block`, `spawn-drop` and `spawn-mob`: the mod's player-join callback
  targeted the Nether while the player stayed in the Overworld; after 100
  ticks, `execute in minecraft:the_nether` found the gold block, dropped
  diamond and cow before travel. A separate in-frame run requested a block at
  `(240,64,0)` through a temporary ticket; the game logged ticket creation and
  readiness, and after a force-load/reload `execute` found the block, diamond
  drop and cow at x=240–242. A separate integrated run on an initially
  unloaded Nether chunk logged ticket request, readiness and a scheduled tick
  at `(1,64,1)`; an `execute in minecraft:the_nether` check found its gold
  block while the player stayed in the Overworld. Thus set-block/drop/mob have
  live evidence in both single-player paths. Scheduled-tick actions now request
  a block-ticking ticket, retain it until execution and release it after the
  callback. An effectful composter fixture changed to emerald through the mod's
  scheduled-tick callback in both in-frame and integrated mode; a separate
  world-level test confirms vanilla's composter tick advances level 7 to level
  8. The bridge records whether scheduling was added, already present, rejected
  for a missing tick chunk, or rejected because the block id did not match.
- Status: **implemented, integration coverage incomplete**.

### 0.4 — machines

- WIT and SDK expose block entities, slots/properties, energy/fluid/item
  capabilities, UI, networking, recipes, world reads and render descriptions.
- `horizon-mods` hosts those interfaces. Sandbox tests cover machine create,
  tick, save/load, UI/menu callbacks and the capability checks.
- Game code creates and ticks mod block entities, saves their private bytes and
  slots, exposes menu interactions, and bridges energy/fluid/item-slot
  capabilities to game-owned block-entity state. Energy buffers and fluid tanks
  are serialized in the block entity's `horizon_energy` and `horizon_fluids`
  fields, flushed before chunk/world saves and chunk unloads, and re-bound when
  a chunk reloads or a mod hot-reloads (`reloaded_saves`). The E2E fixture
  changes both through the runtime, saves and reopens the world, and verifies
  the energy has not decreased and the fluid amount and identity remain. This
  runs in both integrated-server and in-frame single-player.
  This persistence check by itself doesn't cover neighboring transfers; the
  separate two-mod scenario below covers FE and fluid transfer.
- Before machine callbacks, the runtime now seeds capability state (energy,
  fluid and item slots) from all loaded mod block entities, then writes
  capability mutations back to all of their owners after callbacks. This removes
  callback/load-order dependence for cross-mod energy, fluid and item-slot
  access. A runtime test verifies a change to a neighbor's energy and tank is
  reconciled into that neighbor's owning view and marked dirty, and that
  `capabilities::set-item-slot` and `machine::set-slot` / `insert-slot` stay
  synchronized across in-tick and out-of-tick calls. `tools/mod_api_e2e.py`
  loads separate `test_machine` and `test_peer` mods; the smelter sends FE into
  the peer cell and the script verifies the receiver's stored energy survives
  save/reopen in integrated and in-frame single-player. The same scenario
  transfers water into a peer tank and verifies that it survives reopen. This
  proves basic cross-mod FE and fluid transfer. FE receive/extract quotas are
  now accumulated per buffer per game tick across API calls and mods; simulated
  calls do not spend the quota. Sandbox coverage exercises repeated calls and
  verifies that a new tick restores the budget. Rate limits and saturated-buffer
  behavior now have live coverage: the peer accepts only 1 FE per tick while the
  donor attempts two transfers, and the E2E verifies it remains below the
  buffer's capacity across the run. The peer fluid tank also fills to its 16 mB
  capacity; the donor recovers to a full 1,000 mB tank without overfilling the
  peer. Directional transfer rules remain uncovered. Capability definitions are
  now owner-scoped: `set-energy-buffer`, `set-fluid-tank` and `set-item-slot`
  only affect positions with a block entity registered by the calling mod (and
  respect slot count and item `max_stack_size` bounds); cross-mod transfer
  remains available through receive/extract/fill/drain. A malicious peer fixture
  tries to replace the donor's 10,000 FE capacity and rate limits, change its
  water tank to lava, and overwrite slot 0 with bedrock; a sandbox test and both
  live single-player paths confirm those writes are ignored while normal
  transfer still works. Target-scoped attachments (`entity:<handle>` and
  `block:<x>,<y>,<z>`) are cleaned up when entities despawn/die or blocks are
  broken/replaced, and accessory slots validate stack/slot bounds and persist in
  `data/horizon/mod-accessories.json` across world save/reopen in both
  single-player modes.
- `tools/mod_api_e2e.py` now verifies block creation through the mod's
  `server::set_block` action, direct `/item` access to mod slots, hopper input,
  vanilla cooking and hopper output in both integrated-server and in-frame
  single-player. It saves and reopens each world and confirms the output
  remains. The test also verifies denied top-side insertion and bottom-side
  extraction. API 0.4 now offers atomic `machine::insert_slot`: it adds the
  whole stack only when the slot can hold it. The test mod uses it for recipe
  output, and runtime coverage proves a full or mismatched output leaves the
  input untouched while a stack that exactly fits succeeds; a componentful
  same-id stack is also left untouched rather than merged. The live test
  verifies a full 64-item output blocks recipe consumption until room is made.
  `machine::remainder` now uses the full item remainder (including `use_remainder`);
  runtime coverage checks the milk-bucket remainder, while the E2E exercises two
  honey bottles producing glass bottles, including output-full backpressure and
  extraction through a vanilla hopper. Other container types still need
  automated coverage. It also verifies persisted machine energy and fluid
  state after save/reopen in both single-player paths.
- The game now routes bounded `net` packets between the local window and its
  integrated server: 256 packet queue entries per direction, at most 64
  delivered per server/client tick. This covers the built-in single-player
  server thread only. LAN guests, remote servers and dedicated servers are not
  connected to mod packet dispatch yet; packet routing has not been exercised
  in a live multiplayer session.
- The general machine API's direct recipe lookup is limited to vanilla cooking
  recipes; custom machine recipes must use the recipe registry. The new
  `item-data` capability reads and writes opaque item components on machine
  slots; the game also preserves them when older mods use the original slot
  API. Both paths have sandbox coverage, while live compatibility across a
  broad set of mods remains to be established.
- Custom recipe registration now validates namespaced ids, item/tag/resource
  selectors, positive counts, field sizes, and a 4 MiB per-mod data budget.
  Runtime unit tests cover valid mixed item/resource recipes and rejected
  malformed or oversized fields.
- The Mekanism port has recipe-processing code. The Enrichment Chamber has
  been exercised in a live in-frame world: a creative energy cube powered a
  clay recipe, the machine produced four clay balls, and the output remained
  after saving and reopening the world. The machine-screen restart bug noted
  in the original audit was subsequently fixed: `tools/mod_api_e2e.py --mode
  both` now verifies screen contents, slots, background, callbacks, clicks and
  reopen after restart in both single-player paths. Visual pixel/layout QA and
  broader menu shapes remain. LAN and dedicated-server behavior remain
  unverified.
- The Horizon probe now loads custom machine recipes from its resource root and
  can seed energy/fluid state per scenario. `ports/mekanism/scenarios/` contains
  an Enrichment Chamber assertion. The Java oracle currently does not seed
  machine energy, so a Java-versus-Horizon powered recipe comparison is still
  open.
- Status: **substantial foundation; not validated as complete**.

### 0.5 — deep systems

- WIT, SDK and runtime expose custom entities, model animation/draw data,
  dimension registration/teleport, and named vanilla hooks.
- A runtime sandbox test exercises these host-side operations.
- `Game::attack_mob` dispatches `entity-damage` before player melee damage is
  applied, and incoming player damage (`mob_hits_player`, `hurt_player_by`,
  `PlayerMotion::hurt_blocking` environmental hazards/fall/cactus/drowning/fire/lava/void,
  starvation, poison/instant-damage effects, explosions, falling blocks/anvils,
  and `/damage @s`) also dispatches `entity-damage` with cancellation and
  scaling in both in-frame and integrated-server single-player. `tools/mod_api_e2e.py
  --mode both` verifies cancelled cactus damage and halved generic damage on both
  the local player and custom entities.
- Orb experience, advancement rewards and positive `/experience add ... points`
  awards dispatch `xp-gain`. A live fixture scales a 50-point command award to
  100 points (7 levels) in both integrated-server and in-frame single-player.
  Negative point adjustments and direct `set` operations keep their vanilla
  semantics; other direct XP sources still need an audit against this event.
- Player-mined block loot, non-player block breaks (`drop_resources_now`,
  explosion block drops, mob block breaks), vanilla container contents on break
  (chests, barrels, hoppers, dispensers, droppers, furnaces, decorated pots, etc.
  through both live and saved block entity removal paths), mob-death item drops,
  shearing/brushing/carving/harvesting interact loot (player and dispenser),
  fishing catches, Piglin bartering, villager gifts, advancement reward loot,
  vault and trial-spawner reward ejections, and `/loot` commands dispatch
  `loot-drop` before item entities spawn or items are granted. `tools/mod_api_e2e.py
  --mode both` verifies `cancel`, `consume`, and `pass` on broken container
  contents as well as `/loot` interception in both single-player modes.
  Fishing keeps vanilla XP rolls in their prior order. Table-generated vanilla
  loot rolls before hook dispatch. Container breaks draw all vanilla per-slot
  drop positions before filtering them. Opening/populating structure container
  loot tables at chunk generation remains unwired. `pass` preserves vanilla drops
  and adds mod drops; `consume` uses the mod drops instead; `cancel` suppresses
  item drops. XP orbs remain separate from item loot.
- Custom entity declarations (`mod-entities.types`) and host entity mutations
  (`entities`: `spawn`, `despawn`, `get-state`, `set-position`, `set-velocity`,
  `set-health`, `navigate-to`, `nearby`) are now connected to live world
  simulation (`Game::mobs`) in both integrated-server and in-frame single-player.
  Mod entities can be spawned via `/summon`, `entities::spawn`, and
  `server::spawn_mob`, targeted by selectors (`@e[type=...]`), ticked in batch
  via `mod-entities.tick`, interacted with via right-click (`on-interact`),
  damaged or damage-cancelled via `on-hurt` (including environmental damage like
  cactus and `/damage`), driven by custom AI goal weights (`AiGoal`), and
  persisted across world save/reopen and `/data get entity` via the
  `horizon_mod_data` NBT byte array. Custom model animations (`set-animation`),
  bone poses (`set-bone-pose` / `set-bones`), and declarative draw lists
  (`set-draw-list`: `ItemDisplay`, `FluidCuboid`, `ModelPart`, `FloatingText`)
  for `entity:<handle>` and `block:<dimension>:<x>,<y>,<z>` / `block:<x>,<y>,<z>`
  targets are sanitized in the runtime, cleaned up when entities or block
  entities are removed or cleared with empty strings/lists, synchronized across
  the integrated single-player server thread (`SyncedModModels`), and connected
  to mob pose overrides (`Poses::apply_named_animation` / `apply_bone_pose`),
  world-space draw-list geometry (`mod_draw_list_geometry`), and floating-text
  name tags (`append_mod_floating_text_tags`) in both `--in-frame` and
  integrated-server single-player (`tools/mod_api_e2e.py --mode both`). Custom
  dimensions and portals (`dimensions`: `register-dimension`, `register-portal`,
  `list-dimensions`, `teleport`) are now connected to live world registration,
  chunk generation (`flat`/`custom` with mod-specified `surface_block`,
  `void-world` with a 5×5 spawn platform, and `noise` falling back to custom
  flat generation when no datapack noise settings exist), dimension properties
  (`min_y`, `height`, `coordinate_scale`, `has_skylight`, `has_ceiling`,
  `ultrawarm`, `fixed_time`), portal frame activation with `activation_item` and
  return travel to Overworld, `/execute in <custom_dim>` coordinate scaling and
  tab completion, and direct `dimensions::teleport` in both `--in-frame` and
  integrated-server single-player (`tools/mod_api_e2e.py --mode both`). `crop-grow`
  now dispatches for crop age stages, hanging mangrove propagule ages,
  bamboo/cactus/sugar-cane heights, sapling-tree attempts, mushroom/vine/chorus
  spreading and growing-plant-head segments after vanilla random-tick conditions
  succeed, as well as bone meal growth on crops, stems, cocoa and sweet berries
  (verified in `tools/mod_api_e2e.py --mode both`). `explosion-detonate` now
  filters individual non-air blast targets before block damage, drops and fire;
  `tools/mod_api_e2e.py --mode both` verifies a protected stone and destroyed
  neighboring glass in both single-player paths. `enchantment-level` now affects
  live gameplay reads for respiration, player/armor-stand vanishing-curse filters,
  binding curse on equip, silk touch, crossbow Quick Charge/Multishot, trident
  Riptide/Loyalty/Impaling, fishing Lure/Luck, Thorns, Mending, Fire Aspect on
  minecarts, EnchantmentHelper effects (`effective_stack` for Efficiency, Aqua
  Affinity, Unbreaking, Sharpness/Smite/Bane of Arthropods/Impaling/Density/Breach,
  Knockback, Fire Aspect/Wind Burst, Mending, Power/Infinity/Flame, and equipment
  attribute modifiers), and loot-table tool evaluation (`effective_loot_tool` for
  Fortune, Looting, Silk Touch, Fire Aspect, and Flame across block mining, mob
  deaths, dispensers, and `/loot`), with an unsubscribed fast path and support
  for virtual enchantments on unenchanted items.
- Status: **partial live integration; API coverage remains incomplete**.

## Verification run

`cargo test -p horizon-mods --features runtime,pack` passes: 18 unit tests,
3 API-reference tests, and 24 sandbox tests (one ignored). The live
`python3 tools/mod_api_e2e.py --mode both` run also passes in integrated-server
and in-frame single-player. It covers positive XP command scaling, cross-mod
FE/fluid transfer, rejected cross-owner capability redefinitions (energy, fluid,
and item slots), config and accessory persistence across restart, item-slot
capability bridging to `BlockEntity::Mod`, custom entity lifecycle/persistence,
model animations/bone-poses/draw-lists, custom dimension worldgen/portals/travel,
simulation, per-tick limits, sided item movement, hopper input/output and
remainders, and save/reopen of machine inventory and capability state. These
checks validate the listed paths only; they do not establish complete
compatibility with arbitrary machine mods or every network topology.

## Completion gates

1. **Single-player coverage:** use the current matrix to select the next API
   gap; expand `tools/mod_api_e2e.py` where game behavior needs live evidence.
2. **Machines:** verify carried-mod recipes and directional energy/fluid
   networks.
3. Keep the generated API reference synchronized with WIT and this audit
   synchronized with actual end-to-end evidence.

LAN and dedicated-server parity remain deferred.

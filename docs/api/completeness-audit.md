# Mod API completeness audit

Audit date: 2026-10-28. This records what is present in the API contract, SDK,
WASM runtime and game bridge. An interface declared in WIT, or a runtime unit
test, does not by itself mean a pack feature works in a playable world.

## Summary

| Line | Contract / SDK | Runtime | Game integration | Status |
|---|---|---|---|---|
| 0.3 logic | Present | Present | Present for the client and local game hooks inspected | Implemented in parts; full multiplayer and per-capability gameplay coverage is not established |
| 0.4 machines | Present | Present | Machine lifecycle, saves, menus, recipes and capability plumbing exist; mod packets bridge the integrated single-player client and server | Usable foundation; neighboring FE/fluid transfer, broad recipe behavior, LAN/dedicated networking and some persistence cases remain incomplete |
| 0.5 deep | Present | Present, including runtime tests | Melee damage, XP awards and block/mob loot dispatch hooks; custom entity lifecycle, models and other hooks remain unwired | Partial live bridge; not a complete player-facing API |

## Scope for the current ATM10 core milestone

**Active:** API 0.4 machine behavior and client/server integration, starting
with the Enrichment Chamber and the local packet bridge.

**Deferred:** completing API 0.3 and 0.5 game integrations. Their current
interfaces and compatibility remain in the runtime; removing those versions
would prevent already-built mods from loading. The imported ATM10 workspace has
27 carried manifests on 0.3 and 9 on 0.5, so treating them as deferred preserves
those ports while we close the machine path.

**None of the three lines should be described as complete for arbitrary mods.**
For the current milestone, finish the 0.4 machine path first. Return to 0.3
bridge coverage and the missing 0.5 game integration in a later milestone.

## Evidence and limits

### 0.3 — logic and hooks

- The WIT contract and SDK expose config, server mutations, keybindings, target,
  inventory snapshots, block/item callbacks and server events.
- `horizon-mods` implements the host interfaces and has sandbox coverage for
  logic hooks, manifest capability checks and storage.
- `horizon-client/src/game/mods.rs` dispatches block interactions, commands,
  chat, ticks, frames, target and inventory snapshots. This is evidence of an
  in-game path, but does not verify every action in both in-frame and integrated
  server modes, LAN, and dedicated-server mode.
- The game refreshes `world-read` with loaded block states, biomes and packed
  light in a four-block neighborhood around the player. Queries are scoped to
  the active dimension; runtime sandbox coverage checks two dimensions sharing
  the same coordinates.
- The `server` capability can spawn vanilla mobs at a position. Registered
  custom entities are not handled by this action. In integrated single-player,
  server actions enter their requested registered level; in-frame mode has only
  the current dimension. Unknown ids and unavailable dimensions are logged by
  the game bridge. A live single-player check set a block in an already-loaded
  Nether chunk from the Overworld and read it back through `horizon-ctl`.
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
  slots, exposes menu interactions, and bridges energy/fluid capabilities to
  game-owned block-entity state. Energy buffers and fluid tanks are serialized
  in the block entity's `horizon_energy` and `horizon_fluids` fields. The E2E
  fixture changes both through the runtime, saves and reopens the world, and
  verifies the energy has not decreased and the fluid amount and identity
  remain. This runs in both integrated-server and in-frame single-player.
  This persistence check by itself doesn't cover neighboring transfers; the
  separate two-mod scenario below covers FE and fluid transfer. Chunk unload
  and mod reload are still not covered.
- Before machine callbacks, the runtime now seeds capability state from all
  loaded mod block entities, then writes capability mutations back to all of
  their owners after callbacks. This removes callback/load-order dependence for
  cross-mod energy and fluid access. A runtime test verifies a change to a
  neighbor's energy and tank is reconciled into that neighbor's owning view and
  marked dirty. `tools/mod_api_e2e.py` loads separate `test_machine` and
  `test_peer` mods; the smelter sends FE into the peer cell and the script
  verifies the receiver's stored energy survives save/reopen in integrated
  and in-frame single-player. The same scenario transfers water into a peer
  tank and verifies that it survives reopen. This proves basic cross-mod FE
  and fluid transfer. FE receive/extract quotas are now accumulated per buffer
  per game tick across API calls and mods; simulated calls do not spend the
  quota. Sandbox coverage exercises repeated calls and verifies that a new tick
  restores the budget. Rate limits and saturated-buffer behavior now have live
  coverage: the peer accepts only 1 FE per tick while the donor
  attempts two transfers, and the E2E verifies it remains below the buffer's
  capacity across the run. The peer fluid tank also fills to its 16 mB capacity;
  the donor recovers to a full 1,000 mB tank without overfilling the peer.
  Directional transfer rules remain uncovered. Capability definitions are now
  owner-scoped: `set-energy-buffer` and `set-fluid-tank` only affect positions
  with a block entity registered by the calling mod; cross-mod transfer remains
  available through receive/extract/fill/drain. A malicious peer fixture tries
  to replace the donor's 10,000 FE capacity and rate limits and change its water
  tank to lava; a sandbox test and both live single-player paths confirm those
  writes are ignored while normal transfer still works. This does not yet
  define ownership for the separate item-slot capability or attachments.
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
- The Mekanism port has recipe-processing code. The Enrichment Chamber has now
  been exercised in a live in-frame world: a creative energy cube powered a
  clay recipe, the machine produced four clay balls, and the output remained
  after saving and reopening the world. The declared machine screen opens when
  the player is holding an item. Its first-run layout rendered, but after a full
  game restart the screen opened with most of its contents missing; screen
  rendering across restart remains a bug. This verifies basic processing and
  saved output, not complete UI behavior. Integrated-server, LAN and dedicated-
  server behavior remain unverified.
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
  applied. A subscribed mod can multiply the hit or cancel it. This covers the
  local player's melee hit path; projectile, mob, environmental and other
  incoming damage paths do not dispatch it yet.
- Orb experience, advancement rewards and positive `/experience add ... points`
  awards dispatch `xp-gain`. A live fixture scales a 50-point command award to
  100 points (7 levels) in both integrated-server and in-frame single-player.
  Negative point adjustments and direct `set` operations keep their vanilla
  semantics; other direct XP sources still need an audit against this event.
- Player-mined block loot and mob-death item drops dispatch `loot-drop` before
  item entities spawn. `pass` preserves vanilla drops and adds mod drops;
  `consume` uses the mod drops instead; `cancel` suppresses item drops. XP orbs
  remain separate from item loot.
- No game-side dispatch was found for `tick_entities`, custom entity
  lifecycle/interactions, registered dimensions or model animation. `crop-grow`
  now dispatches for crop age stages, hanging mangrove propagule ages,
  bamboo/cactus/sugar-cane heights, sapling-tree attempts, mushroom/vine/chorus
  spreading and growing-plant-head segments after vanilla random-tick conditions
  succeed. `explosion-detonate` and
  `enchantment-level` are still not connected to gameplay. Container, fishing
  and structure loot paths still need review.
- Status: **partial live integration; API coverage remains incomplete**.

## Verification run

`cargo test -p horizon-mods --features runtime,pack` passes: 18 unit tests,
3 API-reference tests, and 20 sandbox tests (one ignored). The live
`python3 tools/mod_api_e2e.py --mode both` run also passes in integrated-server
and in-frame single-player. It covers positive XP command scaling, cross-mod
FE/fluid transfer, rejected cross-owner capability redefinitions, simulation,
per-tick limits, sided item movement, hopper input/output and remainders, and
save/reopen of machine inventory and capability state. These checks validate
the listed paths only; they do not establish complete compatibility with
arbitrary machine mods, chunk unload/reload, or every network topology.

## Completion gates

1. **0.4 machine pilot:** Horizon-side Enrichment Chamber processing is verified
   for a carried clay recipe in both the machine probe and a live in-frame world.
   The probe checks 200 ticks, 8,000 FE consumed, four clay balls, and a
   save/recreate boundary. The live world confirms powered output and output
   persistence through a world save/reopen. Still add Java-side power injection
   for direct parity and verify integrated-server client updates.
2. **0.4 generalization:** run the `net-bridge` example in an integrated world;
   extend `tools/mod_api_e2e.py` with chunk unload/mod reload and screen-restart
   scenarios; verify real carried-mod recipes, item components, directional
   energy/fluid networks, and ownership rules for item-slot capabilities and
   attachments. LAN and dedicated-server packet delivery remains deferred until
   after the single-player API work.
3. Keep the generated API reference synchronized with WIT and this audit
   synchronized with actual end-to-end evidence.

Deferred follow-up: complete 0.3 integration coverage, then add live game
dispatch and gameplay validation for 0.5 entities, models, dimensions and hooks.

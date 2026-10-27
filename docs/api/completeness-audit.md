# Mod API completeness audit

Audit date: 2026-10-26. This records what is present in the API contract, SDK,
WASM runtime and game bridge. An interface declared in WIT, or a runtime unit
test, does not by itself mean a pack feature works in a playable world.

## Summary

| Line | Contract / SDK | Runtime | Game integration | Status |
|---|---|---|---|---|
| 0.3 logic | Present | Present | Present for the client and local game hooks inspected | Implemented in parts; full multiplayer and per-capability gameplay coverage is not established |
| 0.4 machines | Present | Present | Machine lifecycle, saves, menus, recipes and capability plumbing exist | Usable foundation; feature behavior and multiplayer/persistence coverage are incomplete |
| 0.5 deep | Present | Present, including runtime tests | Player melee damage now dispatches `entity-damage`; custom entity lifecycle, models and other named hooks remain unwired | First live bridge; not a complete player-facing API |

## Scope for the current ATM10 core milestone

**Active:** API 0.4 machine behavior, starting with the Enrichment Chamber.

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
- Status: **implemented, integration coverage incomplete**.

### 0.4 — machines

- WIT and SDK expose block entities, slots/properties, energy/fluid/item
  capabilities, UI, networking, recipes, world reads and render descriptions.
- `horizon-mods` hosts those interfaces. Sandbox tests cover machine create,
  tick, save/load, UI/menu callbacks and the capability checks.
- Game code creates and ticks mod block entities, saves their private bytes and
  slots, exposes menu interactions, and connects energy/fluid operations.
- The general machine API's direct recipe lookup is limited to vanilla cooking
  recipes; custom machine recipes must use the recipe registry. Item-stack
  components are not carried by the machine slot contract. Those limits matter
  for broad mod compatibility.
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
  applied. A subscribed mod can multiply the hit or cancel it. This first
  bridge covers the local player's melee hit path; projectile, mob, environmental
  and other incoming damage paths do not dispatch it yet.
- No game-side dispatch was found for `tick_entities`, custom entity
  lifecycle/interactions, registered dimensions or model animation. The other
  named hooks (`loot-drop`, `xp-gain`, `crop-grow`, `explosion-detonate` and
  `enchantment-level`) are also not connected to gameplay.
- Status: **partial live integration; API coverage remains incomplete**.

## Verification run

`cargo test -p horizon-mods --features runtime,pack` passes: 14 unit tests,
3 API-reference tests, and 18 sandbox tests (one ignored). These validate the
contract/runtime, not the complete game integrations listed above. The first
run caught a stale generated `docs/api/mods-reference.md` after WIT changes; it
was regenerated with the documented command and the full run then passed.

## Completion gates

1. **0.4 machine pilot:** Horizon-side Enrichment Chamber processing is verified
   for a carried clay recipe in both the machine probe and a live in-frame world.
   The probe checks 200 ticks, 8,000 FE consumed, four clay balls, and a
   save/recreate boundary. The live world confirms powered output and output
   persistence through a world save/reopen. Still add Java-side power injection
   for direct parity and verify integrated-server client updates.
2. **0.4 generalization:** test representative item, fluid, energy, UI, network
   and recipe flows in a real game; document unsupported item components and
   recipe shapes; verify LAN/dedicated behavior before marking those capabilities
   complete.
3. Keep the generated API reference synchronized with WIT and this audit
   synchronized with actual end-to-end evidence.

Deferred follow-up: complete 0.3 integration coverage, then add live game
dispatch and gameplay validation for 0.5 entities, models, dimensions and hooks.

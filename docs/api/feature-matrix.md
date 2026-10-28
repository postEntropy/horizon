# Horizon Mod API — feature matrix

Snapshot: 2026-10-28 · Focus: single-player · Scope: what a Rust/WASM mod can
currently express and what the game actually runs. This is a detailed working
checklist maintained in the repository.

**Rough API-readiness estimate for ATM10 behavior in single-player: ~55%**
(wide range: 45–65%). This is an engineering judgment based on the current
contract/runtime/game-integration gaps below, not a measured percentage of mods
ported or features implemented.

**Legend:** [x] working in the stated scope · [~] partial or runtime-only · [ ] not
connected / not implemented. “Working” means there is a real game path or
automated runtime coverage as specified in the row; it does not mean every
Java mod behavior has an equivalent.

## Foundation and compatibility

| Status | Feature |
|---|---|
| [x] | Mods are WebAssembly components loaded by the Rust game runtime. |
| [x] | Manifest API versions 0.1–0.5 are recognized; capability imports are checked against the manifest. |
| [x] | WASM calls have fuel and memory limits; a trap/overrun stops that mod without stopping other mods. |
| [x] | Mods can be installed/enabled/disabled through the mod list; content-only mods load without the WASM engine. |
| [x] | Old components that import only their older WIT world continue to load when newer interfaces are added. |
| [~] | Most API behavior has sandbox tests; not every capability has an end-to-end single-player game scenario. |
| [ ] | Java/NeoForge mixins are not translated automatically. Their behavior needs a supported Horizon API or a Rust/WASM implementation. |

## API 0.1 — basic player-facing mods

| Status | Feature |
|---|---|
| [x] | `log`: write mod-scoped log messages. |
| [x] | `game-read`: read player position, rotation, health, food and current dimension. |
| [x] | `chat`: receive chat lines and show mod chat/feedback. |
| [x] | `hud`: display text panels at supported screen corners. |
| [x] | `storage`: save mod-owned key/value data with quotas and key limits. |
| [x] | Tick and frame callbacks run in-game for enabled mods. |

## API 0.2 — data-driven content

| Status | Feature |
|---|---|
| [x] | `.hmod` content can carry resource assets and data without executable code. |
| [x] | Content registration supports namespaced blocks/items and their data-driven definitions. |
| [x] | Existing pack-carry tooling imports supported assets, block/item declarations, recipes and loot data. |
| [x] | Shared `forge`/`neoforge` tag additions carry across mods; legacy `blocks/` and `items/` folders are normalized to Horizon's `block/` and `item/` tag paths. |
| [~] | NeoForge `furnace_fuels` and `compostables` maps update cooking-fuel and composter behavior for registered items, expanding direct and nested item-tag selectors against the merged tag registry. Unknown tags and invalid values are reported. Other data maps remain unsupported. |
| [~] | Re-carried 56 ATM10 mods on 2026-10-28: all 70 Forge/NeoForge loader-tag files now carry successfully. Of the 144 tag files still under `outside/`, 143 target namespaces absent from ATM10; the remaining CTOV tag is malformed JSON (EOF). |
| [~] | Carry coverage depends on templates and supported JSON shapes; reports list omitted blocks/files/recipes and `outside_reasons` classifies each excluded file as invalid, unsupported-format or unowned-namespace. |
| [~] | Large content packs may need unsupported model, loot, worldgen or registry formats added to the carry/runtime pipeline. |

## API 0.3 — logic and local world actions

| Status | Feature |
|---|---|
| [x] | Typed mod commands, config settings and keybinding registration. |
| [~] | Crosshair target, player-inventory, and open block-entity container snapshots. Other menu types and integrated-server ownership still need coverage. |
| [~] | Block `use-without-item`, `use-item-on`, placement, left-click `attack`, pre-break `broken`, scheduled block-tick, random-tick, neighbor-change and entity-inside callbacks dispatch. Entity-inside currently covers players, minecarts and LAN guests in the trigger scan; other entity types remain unwired. |
| [~] | Item `use-in-air`, `use-on-block`, `attack-entity` for mobs, dragon, boats, minecarts, paintings, frames and armor stands, and consumable `finish-using` callbacks dispatch during single-player gameplay. Player targets and non-consumable finish paths remain unwired. Server `block-break`/`block-place`, player join/leave and `entity-spawn` for mobs, dropped items, XP orbs, boats, minecarts, paintings, frames and armor stands dispatch. `entity-death` now covers mobs, boats, minecarts, paintings, item frames, armor stands and the ender dragon when removed; projectiles, dropped items, XP orbs and other removal paths still need coverage. |
| [~] | Server actions can set blocks/rules, schedule ticks, spawn drops/mobs and play sounds. In integrated single-player, dimension-targeted world actions enter the requested registered level and return to the prior level; in-frame mode only has its current dimension. Live tests found set-block, drop and mob after queuing them to an initially unloaded Nether chunk in integrated mode and to x=240–242 in in-frame mode, after force-loading the destination. Scheduled-tick actions now request a ticking ticket even for an already-loaded chunk and retain it until the due tick runs; a test mod changed `composter[level=7]` to emerald from its scheduled-tick callback in both in-frame and integrated single-player. The logs show the ticket request and release after execution, while `horizon-world` independently verifies the composter's vanilla tick advances level 7 to level 8. The host distinguishes added, duplicate, missing-chunk and block-mismatch scheduler results. Actions waiting for unloaded chunks are capped at 256 entries with a 1,200 world-tick deadline; timeout/capacity drops are logged. Unknown mobs and unavailable dimensions are logged. |
| [x] | `world-read` supplies loaded block states, biomes and packed light within four blocks of the player. |
| [x] | `world-read` queries are scoped to the active dimension. Same coordinates in different dimensions are tested independently. |
| [~] | Integration coverage does not establish every callback/action in both in-frame and integrated-server modes. |
| [~] | `world-read` is a bounded neighborhood snapshot; cells outside the sampled radius or outside loaded chunks return no value. |
| [ ] | LAN guest and dedicated-server execution of the full logic API is not considered complete. |

## API 0.4 — machines and integrations

| Status | Feature |
|---|---|
| [x] | Mods may declare block-entity types for their own content blocks. |
| [x] | In-game machine create/tick/save/remove lifecycle and private mod save bytes; `/setblock` and `server::set_block` create declared block entities. |
| [x] | Machine-owned slots and block-state properties are read/written through the machine interface. |
| [x] | Item components (custom names, damage, enchantments and other component NBT) survive machine ticks. |
| [x] | Older mods using only item id/count slots preserve components when the stack count changes. |
| [x] | `item-data` explicitly reads/writes a bounded binary network-NBT component compound. |
| [x] | Machine facts expose stack limits, fuel/remainders and vanilla cooking recipe lookup. |
| [x] | Recipe registry supports lookup by id/type/input/output and custom recipe registration. |
| [x] | `for-input` resolves item tags using the active content registry. |
| [x] | Custom recipe registration validates ids, selectors, counts and payload limits (64 KiB per entry, 4 MiB per mod). |
| [~] | Machine recipe lookup directly understands vanilla cooking recipes; other machine recipes are consumed through the recipe registry. |
| [~] | FE/fluid/item capability interfaces exist in the WASM host; mod item slots work with vanilla hoppers and `/item` slot commands. Generic sided access, full-stack backpressure and stacked `use_remainder` output through a hopper have live coverage. Game-owned energy buffers and fluid tanks persist in mod block-entity NBT and two distinct mods transfer FE/water between machines in both single-player paths. A two-call-per-tick FE donor is constrained to the peer's 1 FE/tick limit; a 16 mB peer tank fills and blocks further transfer. Only the owning mod can declare energy/fluid capacity and rates: an adversarial peer's attempts to rewrite a neighbor are rejected in sandbox and live tests. Attachments now save and restore across world reopen; target lifecycle cleanup, chunk unload, directional networks, item-slot ownership and real carried-mod recipes still need validation. |
| [~] | Declarative machine screens, widgets and menu callbacks exist. `tools/mod_api_e2e.py --mode both` verifies title, widgets, slot data, background, left/right/shift-click transfers, callbacks, button close, and full restart in both single-player paths; visual pixel/layout QA and broader menu shapes remain. |
| [x] | World-space boxes and ghost-block previews are available through the render capability. |
| [x] | Mod packet bridge works between the local window and the integrated single-player server with bounded queues. |
| [~] | Client/server packet routing has runtime coverage but has not been validated in a complete live gameplay scenario. |
| [ ] | Mod packet dispatch for LAN guests, remote servers and dedicated servers is deferred. |
| [~] | A Mekanism Enrichment Chamber scenario has processed a recipe and preserved output through save/reopen; this does not validate all machine types. |

## API 0.5 — deep systems

| Status | Feature |
|---|---|
| [x] | WIT/SDK/runtime expose custom entity declarations and create/tick/save/remove/interact/hurt callbacks. |
| [x] | Runtime tests exercise custom entity state, model, dimension and hook host calls. |
| [~] | `entity-damage` reaches the local player's melee attack path. |
| [~] | `xp-gain` reaches XP-orb pickup, advancement rewards and positive `/experience add ... points` commands. A live mod doubles 50 command points to 7 levels in both single-player paths; other direct XP sources still need an audit. |
| [~] | `loot-drop` reaches player-mined block drops and mob-death item drops. |
| [ ] | Custom entity lifecycle is not connected to actual game-world spawning, simulation, persistence and rendering. |
| [ ] | Custom model animation/draw data is not connected to game rendering. |
| [ ] | Registered custom dimensions and portals are not connected to actual world travel/generation. |
| [ ] | `crop-grow`, `explosion-detonate` and `enchantment-level` hooks are not connected to gameplay. |
| [ ] | Projectile, mob, environmental and other non-melee damage paths do not all dispatch `entity-damage`. |
| [ ] | XP from all sources and all loot contexts (containers, fishing and structures) are not covered. |

## Out of scope for this single-player snapshot

| Status | Feature |
|---|---|
| [ ] | Full LAN, remote multiplayer and dedicated-server parity for mod APIs. |
| [ ] | Automatic compatibility with arbitrary Java code, reflection or loader mixins. |
| [~] | ATM10-wide support is incomplete: this matrix measures API capability, not the number of pack mods already ported. |

## ATM10 workspace snapshot (not an API score)

Latest local coverage report inspected: `all-the-mods-10-atm10`, 2026-10-28.
These counts measure per-mod content/port status; they do not measure how much
of the API is implemented.

| Tier/class | Ported | Partial | Not yet | Not needed | Total |
|---|---:|---:|---:|---:|---:|
| Logic (0.3) | 31 | 35 | 0 | 0 | 66 |
| Machines (0.4) | 34 | 63 | 0 | 0 | 97 |
| Deep (0.5) | 11 | 81 | 0 | 0 | 92 |
| Data | 21 | 43 | 0 | 0 | 64 |
| Client | 0 | 0 | 35 | 0 | 35 |
| No code tier | 0 | 0 | 20 | 121 | 141 |
| **All classes** | **80** | **239** | **55** | **121** | **495** |

## Complete known API backlog

This is the consolidated list of known work to make more ATM10 behaviors
portable. “Complete” means all currently identified work in this audit; new
gaps will appear as representative mod ports exercise the API. Items below are
API/engine work, not a list of individual mods to port.

### 0.3 logic, local world access and client behavior

| Done? | Work remaining | Completion check |
|---|---|---|
| [ ] | Exercise each block action callback in a game harness. Placement, left-click attack, pre-break `broken`, scheduled and random block ticks, neighbor change and trigger-scan entity-inside callbacks now dispatch; remaining entity classes still need wiring and coverage. | Rust game and a test WASM mod agree on the callback and cancellation result. |
| [ ] | Finish auditing item callback routes: player targets and non-consumable `finish-using` paths remain; inventory tick, attack/damage and equipment/tool behavior need mapping against the WIT event set. | Every declared event has a named in-game dispatch path and a regression case; unsupported events are removed or documented. |
| [ ] | Complete entity spawn/death coverage for non-mob families. Block break/place, player join/leave, and spawn notifications for mobs, dropped items, XP orbs, boats, minecarts, paintings, frames and armor stands dispatch. Death notifications now cover mobs, boats, minecarts, paintings, frames, armor stands and the ender dragon. | Every event the WIT promises is dispatched on the single-player world simulation path, with removal reasons and positions checked in a game harness. |
| [ ] | Exercise server actions and their edge cases: block-state validation, scheduled ticks, item drops, vanilla mob spawning, sounds and game rules. | In-frame and integrated-server tests confirm mutation in the requested level, return to the previous level, persistence, visible results and invalid dimension/entity handling. |
| [ ] | Decide whether command arguments need typed trees, suggestions and permission metadata; current mod command callback receives the remaining arguments as text. | Port representative command-heavy mods or explicitly bound the supported command model. |
| [ ] | Extend `world-read` beyond its four-block neighborhood for minimaps, structure previews and larger machine multiblocks, with a strict query/fuel/memory bound. | A mod can request larger loaded areas without per-tick full-region copies or unbounded reads. |
| [ ] | Test `world-read` at dimension height limits, unloaded chunk borders, dimension changes and rapid movement. | Queries return correct states/biomes/light or `none`, never stale cells from the prior snapshot. |
| [ ] | Verify inventory snapshots for armor, offhand, all open-menu types, creative inventory and server-thread ownership. Open-container slots currently read the slots exposed by the active block entity; station menus without block-entity containers and cursor stack are not part of this snapshot. | Harness covers every slot source the WIT claims to expose. |
| [ ] | Verify config defaults, bounds, choices, persistence, reload and Mods-screen edits across restart. | Values round-trip and invalid updates are refused without stopping the mod. |
| [ ] | Define and implement richer client-only APIs for mods that need custom screens, mouse/cursor input, tooltips, overlays or visual settings beyond the current HUD/keys/UI surfaces. | Representative client-only ATM10 mods have a documented Rust/WASM equivalent or are explicitly unsupported. |
| [ ] | Audit client inventory/open-container snapshots for when the container changes while a mod is running. | No stale container view; updates are bounded and deterministic. |
| [ ] | Run the complete logic API in both `--in-frame` and default integrated-server single-player modes. | One parity suite covers the same API scenario in both paths. |

### 0.4 machines, recipes, menus and capabilities

| Done? | Work remaining | Completion check |
|---|---|---|
| [x] | Add per-side slot rules for insertion/extraction (vanilla `WorldlyContainer` behavior) to mod block entities. | `tools/mod_api_e2e.py` verifies allowed north-side insertion, denied top-side insertion and denied bottom-side extraction. Unset sides keep all-access behavior; pipe interoperability is separate. |
| [~] | Generalize item-handler interoperability between mod machines, vanilla containers and adjacent blocks. `tools/mod_api_e2e.py` automates `/item` slot access, hopper input/output, a stacked recipe remainder extracted by a hopper, a full 64-item output stack blocking consumption until room is made, and save/reopen in both game modes. API 0.4 also has atomic `machine::insert_slot` for complete recipe outputs. | Extend live coverage to other remainder behaviors and container types. |
| [~] | Connect FE capabilities to actual game-owned machine buffers and neighboring transfer paths. | `tools/mod_api_e2e.py` proves a distinct `test_machine` mod transfers FE into a `test_peer` block entity with a 1 FE/tick receive limit; two transfer attempts per tick do not exceed it, and the received charge survives save/reopen in both single-player paths. Runtime tests verify per-tick receive/extract limits, simulation without budget use and reset on the next tick. Directional networks remain. |
| [~] | Connect fluid tanks to actual machine storage and neighboring fluid containers/pipes, beyond the current bucket path for specific carried tanks. | `tools/mod_api_e2e.py` transfers water between two distinct mods, fills the peer's 16 mB tank to capacity, stops transfer there and verifies identity/persistence across save/reopen in both single-player paths. Neighboring vanilla containers/pipes, incompatible live fluid and directional access still need coverage; runtime tests cover incompatible fluid and simulation. |
| [~] | Define who owns and persists energy/fluid/item capability state across block-entity save, chunk unload, mod reload and world reopen. | Energy/fluid ownership and NBT round-trip are covered through world reopen in both SP paths; chunk unload and mod reload are not yet covered. |
| [~] | Persist mod attachments for player/entity/chunk/level targets through the corresponding single-player save files. | The whole attachment map now round-trips through a world-local JSON save; `tools/mod_api_e2e.py --mode both` uses a level target and checks it after restart before any callback can rewrite it. Target-specific lifecycle cleanup, including removing stale data when an entity/chunk/level is deleted, remains. |
| [ ] | Connect accessory-slot APIs to actual player equipment/accessory UI and item transfer, rather than only a shared runtime store. | Equip, inspect, replace and save an accessory in-game through the declared API. |
| [x] | Restore declarative machine-screen contents after a full game restart. | `tools/mod_api_e2e.py --mode both` opens, closes, saves, restarts and reopens the same screen, then verifies its widgets, slots and background in both single-player paths. |
| [x] | Validate menu click/button callbacks and slot synchronization in integrated-server single-player, including shift-click and cursor stack behavior. | `tools/mod_api_e2e.py --mode both` verifies left/right/shift-click cursor and slot agreement, server-side slot NBT, slot and button callbacks, and screen close in both modes. |
| [ ] | Add reusable matching/consumption helpers for multi-slot machine recipes, counts, tags and item components. | A guest can select a recipe and atomically consume inputs/produce outputs with no duplicate or partial craft. |
| [ ] | Expand custom recipe data coverage for supported ATM10 recipe JSON forms, item tags, multiple outputs, fluid/chemical ingredients and remainders. | Carry report identifies every dropped recipe shape and fixtures cover each supported form. |
| [ ] | Define recipe viewer data/layout support separately from machine processing. | JEI/EMI-style consumers can read ingredient alternatives, counts, resource amounts and outputs consistently. |
| [ ] | Exercise `get`, `by-type`, `for-input`, `for-output` and replacement precedence against loaded resource recipes and mod-registered recipes. | Queries return correct tags, resource selectors, item counts and namespace ownership. |
| [ ] | Run the `net-bridge` example in a real integrated single-player world and verify packet ordering, bounded queues, error handling and mod stop behavior. | In-game request/response succeeds across the local client/server bridge. |
| [ ] | Test block-entity behavior across chunk save/unload/reload, mod disable/reload and replacing/breaking its block. | No orphan handles, lost slots, duplicate callbacks or stale capability cache. |
| [~] | Define interoperability for capability data shared between two mods, including ownership and conflict rules. Energy/fluid definitions are owner-scoped and two mods transfer resources without changing each other's storage rules; item-slot ownership and attachment conflicts remain. | Two sandboxed mods can cooperate without silently replacing each other's buffers, tanks, or slots. |
| [ ] | Verify the machine UI/render/recipe/capability path with more than the Enrichment Chamber pilot. | At least one item-only, one fluid and one energy machine scenario pass. |

### 0.5 entities, models, dimensions and vanilla hooks

| Done? | Work remaining | Completion check |
|---|---|---|
| [ ] | Connect mod entity declarations to the game's actual entity registry and spawn path. | A mod entity can be spawned by its declared id in a single-player world. |
| [ ] | Connect general `entities` calls (spawn, despawn, state, position, velocity, health, navigation and nearby queries) to actual world entities. | Reads and writes refer to live entity handles, enforce loaded-dimension bounds and reject stale handles. |
| [ ] | Integrate custom entity tick/state with physics, collision, health, damage, AI goals, despawn, tracking and actual world persistence. | Entity survives save/reload, moves and interacts according to its declared behavior. |
| [ ] | Connect entity interaction and hurt callbacks to actual player and non-player entity interactions. | Right-click and every supported damage source reach the correct mod handle. |
| [ ] | Render custom entities and their models/animations in the Rust renderer. | A test model renders, animates and remains correct across resource reload and save/reload. |
| [ ] | Connect model bones, poses, draw lists and block-entity attachments to their world render targets. | Runtime model calls produce visible and bounded geometry in-game. |
| [ ] | Connect custom dimension/portal declarations to dimension registration, saved world metadata, travel, respawn and return portals. | Create/travel/save/reopen round-trip works without corrupting vanilla dimensions. |
| [ ] | Implement or explicitly constrain custom dimension biome sources, chunk generators, height/light rules and worldgen data. | A custom dimension generates and loads chunks using documented supported codecs. |
| [ ] | Wire `crop-grow` at the vanilla growth decision point, preserving vanilla RNG draw order and allowing pass/consume/cancel. | Parity harness compares random growth and cancellation against the vanilla oracle. |
| [ ] | Wire `explosion-detonate` before block/entity effects are applied. | Mods can safely pass, alter or cancel an explosion in a parity harness. |
| [ ] | Wire `enchantment-level` into enchantment selection/level calculation. | Vanilla result is unchanged for pass; bounded mod changes match test expectations. |
| [ ] | Expand `entity-damage` to projectile, mob, fall, fire, explosion, environmental and other damage sources. | Source-by-source harness confirms exactly one event and correct cancellation/scaling. |
| [~] | Expand `xp-gain` to every direct XP source and verify ordering with orb creation, pickup, advancement and commands. Orb pickup, advancements and positive command point grants are covered; audit any remaining direct awards. | XP amount and cancellation are consistent across a source matrix. |
| [ ] | Expand `loot-drop` to container, fishing, structure, block and entity loot contexts. | Pass/consume/cancel semantics preserve vanilla RNG and are tested per context. |
| [ ] | Add API hooks for other demonstrated pack needs that have no current WIT event (for example entity spawn/death, player lifecycle, inventory changes), after auditing representative mod requirements. | Every new hook has a named event, capability/version rule, security bound, and parity test. |

### After API work: pack-wide porting and integration

| Done? | Work remaining | Completion check |
|---|---|---|
| [ ] | Re-run ATM10 carry/report after each newly supported data format and review every omitted file/recipe/template. | Coverage report has no silent omissions; unsupported rows name an API/carry blocker or are explicitly not needed. |
| [ ] | Port the remaining ATM10 code mods individually in dependency order once their API blockers are cleared. | Every mod reaches ported, partial-with-named-blocker or not-needed; no unexplained status remains. |
| [ ] | Prioritize ports by pack progression and dependency fan-out (core storage/network/machine mods before add-ons and integrations). | Each batch has a working in-game scenario and dependent mods are not marked complete prematurely. |
| [ ] | Build representative cross-mod scenarios (item/fluid/energy transfer, recipes, menus, worldgen, entities, dimensions and progression). | Scenarios prove the APIs compose across multiple ports rather than only isolated test guests. |
| [ ] | Review licenses and redistribution requirements for every port before publishing or sharing its WASM. | No restricted or all-rights-reserved mod code is redistributed without permission. |

### Content carry/runtime gaps that block code ports

| Done? | Work remaining | Completion check |
|---|---|---|
| [ ] | Turn carry-report omissions (unsupported templates, blockstates, item models, files, recipes and loot) into a prioritized list of supported formats. | ATM10 carry report has no unexplained dropped entry; each unsupported shape names the missing codec/template. |
| [ ] | Expand block/item template coverage where ATM10 content has no suitable vanilla base. | Representative declarations load with correct shape, collision, sound, hardness and item form. |
| [ ] | Add missing model/state encodings needed by carried block/item assets, including unsupported multipart/selector forms when required. | Model fixtures render and item icons do not become invisible for supported carried content. |
| [ ] | Audit recipes, tags, loot and worldgen data that are carried but not yet honored by the runtime. | Each data registry has a format-support table plus fixtures for nested/merge cases. |
| [ ] | Keep content loading order, namespace ownership and cross-mod references deterministic when packs contain overlays/add-ons. | Repeated import/build yields the same registries; conflicts have actionable diagnostics. |
| [ ] | Build automated parity scenarios for the highest-impact carried content formats. | Carried recipes, loot, blockstates and worldgen match the vanilla oracle on selected cases. |

### Test, docs and release work

| Done? | Work remaining | Completion check |
|---|---|---|
| [~] | Run the API integration harness that launches a real modded single-player world in both simulation modes. | `tools/mod_api_e2e.py --mode both` covers logic, machine capabilities, screen lifecycle and save paths locally; wire the representative scenario into CI and add recipe coverage. |
| [ ] | Add missing parity harnesses for API effects that alter vanilla gameplay while preserving RNG order. | Each hook/action has a vanilla oracle and a Rust probe or equivalent fixture. |
| [ ] | Re-run the full crate suite and fix the existing `content::tests::refuses_content_outside_its_fence` acceptance of `assets/minecraft/...`. | Full `cargo test -p horizon-mods --features runtime,pack` passes. |
| [ ] | Refresh `docs/research/code-mods-plan.md`, whose original gap table predates APIs 0.3–0.5 and the current game bridges. | Historical research is labelled and current capability/backlog links point here. |
| [ ] | Keep WIT API reference, public mod guide, audit and this local matrix synchronized after each API change. | Generated API reference passes and statuses cite current code/tests. |
| [ ] | Recompute ATM10 API-readiness estimate after major integration gates instead of treating ~55% as measured truth. | Estimate has a dated basis and an explicit definition of what the percentage measures. |

### Explicitly deferred until single-player APIs advance

| Done? | Work remaining | Completion check |
|---|---|---|
| [ ] | Connect mod packet dispatch for LAN guests, remote servers and dedicated servers. | All peers receive and validate bounded mod payloads. |
| [ ] | Verify API behavior across multiplayer permissions, ownership and capability synchronization. | Host/guest/server parity suite passes for each supported API line. |
| [ ] | Add a sandboxed scripting VM for mods that fundamentally embed user programs (ComputerCraft-style systems). | Separate design, threat model, quotas and approved use cases before implementation. |

## Suggested order

1. Finish 0.4 single-player reliability: machine screen restart, sided inventory
   rules, generic energy/fluid/item integration and save/reload tests.
2. Finish the event and world-read audit for 0.3 and test both single-player
   simulation modes.
3. Add custom entity lifecycle and render integration, then dimensions.
4. Connect remaining high-value vanilla hooks and loot/damage/XP sources.
5. Expand content carry formats and use the updated ATM10 reports to select
   representative mod ports; LAN/dedicated remains deferred by request.

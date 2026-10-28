<!-- Generated from crates/horizon-mod-sdk/wit/horizon.wit and the manifest's capabilities by crates/horizon-mods/tests/api_reference.rs; a test fails when this file is out of date. Update it with: HORIZON_WRITE_DOCS=1 cargo test -p horizon-mods --features pack --test api_reference -->
# Mod API reference

The contract between Horizon and its mods: the WIT package `horizon:mods@0.1.0`, every interface, type, function and hook in it, and the capabilities a manifest may list. The guide is [mods.md](../mods.md); stability and versions: [stability.md](stability.md). Manifest `api` lines this game serves: `0.1`, `0.2`, `0.3`, `0.4`, `0.5`.

## Capabilities

| In `capabilities` | Grants | The player reads "May …" |
| --- | --- | --- |
| `game-read` | [`game`](#game) | see your position, health and the time |
| `chat` | [`chat`](#chat) | show you chat messages |
| `hud` | [`hud`](#hud) | draw text on your screen |
| `storage` | [`storage`](#storage) | save its own settings to disk |
| `registries` | data files (`assets/`, `data/`), no interface | add blocks and items to your worlds |
| `config` | [`config`](#config) | expose configurable settings in the Mods screen |
| `server` | [`server`](#server) | change blocks, drops, sounds and rules in your worlds |
| `keys` | [`keys`](#keys) | add keybindings to Controls |
| `target` | [`target`](#target) | see the block or entity you are looking at |
| `inventory-read` | [`inventory`](#inventory) | see the items in your inventory and open container |
| `block-entities` | [`machine`](#machine) | run its blocks' machines in your worlds |
| `item-transfer` | [`item-transfer`](#item-transfer) | control which sides allow item insertion and extraction |
| `item-data` | [`item-data`](#item-data) | preserve item data components in machine inventories |
| `capabilities` | [`capabilities`](#capabilities) | exchange energy, fluids, items, attachments and accessories |
| `ui` | [`ui`](#ui) | open custom machine and container screens |
| `net` | [`net`](#net) | send mod packets between client and server |
| `recipes` | [`recipes`](#recipes) | read recipes and register custom machine recipe types |
| `world-read` | [`world-read`](#world-read) | read blocks, biomes and light around you |
| `render` | [`render`](#render) | draw 3D boxes and ghost-block previews in the world |
| `entities` | [`entities`](#entities) | spawn and control custom entities and mobs |
| `models` | [`models`](#models) | animate custom 3D models and block-entity attachments |
| `dimensions` | [`dimensions`](#dimensions) | register custom dimensions and portals |
| `vanilla-hooks` | [`vanilla-hooks`](#vanilla-hooks) | hook into loot drops, combat damage, XP and plant growth |

## World `horizon-mod`

- Imports (what the game offers): `log`, `game`, `chat`, `hud`, `storage`
- Exports (what the mod provides): `events`

## World `horizon-machine-mod`

A mod with API 0.3 logic, API 0.4 machines and API 0.5 deep systems: the SDK builds every mod against this world. Mods built for `horizon-mod` alone still load.

- Imports (what the game offers): `config`, `server`, `keys`, `target`, `inventory`, `machine`, `item-transfer`, `item-data`, `capabilities`, `ui`, `net`, `recipes`, `world-read`, `render`, `entities`, `models`, `dimensions`, `vanilla-hooks`, `log`, `game`, `chat`, `hud`, `storage`
- Exports (what the mod provides): `block-entities`, `logic`, `mod-entities`, `events`

## World `machine-host`

The game's side of `horizon-machine-mod` beyond `horizon-mod`.

- Imports (what the game offers): `config`, `server`, `keys`, `target`, `inventory`, `machine`, `item-transfer`, `item-data`, `capabilities`, `ui`, `net`, `recipes`, `world-read`, `render`, `entities`, `models`, `dimensions`, `vanilla-hooks`
- Exports (what the mod provides): `block-entities`, `mod-entities`

## `log`

Access: always granted.

The mod's log, in the game's log under the mod's id. Always granted.

### enum `level`

| Case | |
| --- | --- |
| `debug` |  |
| `info` |  |
| `warn` |  |
| `error` |  |

| Function | Signature | What it does |
| --- | --- | --- |
| `write` | `func(level: level, message: string)` | One line in the game's log as `[mod <id>] message`, at that level. |

## `game`

Access: capability `game-read`.

Capability `game-read`: the local player and the world clock.

### record `player`

| Field | Type | |
| --- | --- | --- |
| `x` | `f64` | Feet position, in blocks. |
| `y` | `f64` |  |
| `z` | `f64` |  |
| `yaw` | `f32` | Degrees, as F3 shows them. |
| `pitch` | `f32` | Degrees: -90 looks straight up, 90 straight down. |
| `health` | `f32` | Hit points as vanilla counts them: 20 is full health without effects. |
| `food` | `u32` | 0 to 20, as the hunger bar. |
| `dimension` | `string` | `minecraft:overworld`, `minecraft:the_nether`, ... |

### record `clock`

| Field | Type | |
| --- | --- | --- |
| `game-time` | `u64` | Ticks since the world was created. |
| `day-time` | `u64` | 24000 ticks per day; 0 is 6:00 on the first day. |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-player` | `func() -> option<player>` | None on the title screen. |
| `get-clock` | `func() -> option<clock>` | None on the title screen. |

## `chat`

Access: capability `chat`.

Capability `chat`: messages only the local player sees.

| Function | Signature | What it does |
| --- | --- | --- |
| `show` | `func(text: string)` | Shown in chat after the mod's name, like a system message. |

## `hud`

Access: capability `hud`.

Capability `hud`: one panel of text lines per mod, drawn by the game every frame until the mod changes or clears it (no call per frame).

### enum `corner`

| Case | |
| --- | --- |
| `top-left` |  |
| `top-right` |  |
| `bottom-left` |  |
| `bottom-right` |  |

### record `line`

| Field | Type | |
| --- | --- | --- |
| `text` | `string` | Plain text, up to 256 characters. |
| `color` | `u32` | 0xRRGGBB. |

| Function | Signature | What it does |
| --- | --- | --- |
| `set-panel` | `func(corner: corner, lines: list<line>)` | Replaces the mod's panel: up to 16 lines in that corner of the screen. |
| `clear` | `func()` | Removes the mod's panel. |

## `storage`

Access: capability `storage`.

Capability `storage`: a per-mod folder of small key/value files, quota'd (docs/mods.md). Keys are `[a-z0-9_.-]{1,64}`, one path segment.

| Function | Signature | What it does |
| --- | --- | --- |
| `get` | `func(key: string) -> option<list<u8>>` | None if `key` has never been `set`, or the key is not well-formed. |
| `set` | `func(key: string, value: list<u8>) -> result<_, string>` | Errs on a malformed key or over quota (bytes for the whole mod). |
| `delete` | `func(key: string)` | No error if `key` was already absent. |

## `events`

Access: exported by the mod: the game calls it.

What the game calls. Only the hooks `init` asks for are ever called.

### record `registration`

| Field | Type | |
| --- | --- | --- |
| `commands` | `list<string>` | Chat commands, without the slash. A name vanilla already uses is refused (vanilla's command runs). |
| `tick` | `bool` | Call `tick` every game tick (20 per second while playing). |
| `frame` | `bool` | Call `frame` every rendered frame. Budget: 0.1 ms. |
| `chat` | `bool` | Call `chat` for each chat line the player receives. |

| Function | Signature | What it does |
| --- | --- | --- |
| `init` | `func() -> result<registration, string>` | Once, after loading. An error keeps the mod off and is shown. |
| `tick` | `func(tick: u64)` | Each game tick while in a world, with the game's tick counter. |
| `frame` | `func(seconds: f32)` | Each rendered frame, with the seconds since the last one. |
| `command` | `func(name: string, args: string) -> result<option<string>, string>` | `/name args`: ok(some(text)) is shown as feedback, err(text) in red. |
| `chat` | `func(text: string)` | Each chat line the player receives, as plain text. |

## `machine`

Access: capability `block-entities`.

Capability `block-entities` (API 0.4): what the code of a mod's block entities may read and change while the game ticks them. A handle names one block entity and is valid only during the `block-entities` call it was passed to; slots and block state belong to the game (hoppers, menus, drops and saves see them), the mod keeps the rest of its state itself.

### record `block-entity-position`

| Field | Type | |
| --- | --- | --- |
| `dimension` | `string` |  |
| `x` | `s32` |  |
| `y` | `s32` |  |
| `z` | `s32` |  |

### record `item-stack`

An item and how many. Read or write its non-default components with `item-data`.

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | `minecraft:coal`, `<mod>:<path>`, ... |
| `count` | `u32` |  |

### enum `cooking-kind`

| Case | |
| --- | --- |
| `smelting` |  |
| `blasting` |  |
| `smoking` |  |
| `campfire` |  |

### record `cooking-recipe`

| Field | Type | |
| --- | --- | --- |
| `output` | `item-stack` |  |
| `cooking-time` | `u32` | Ticks the recipe takes in its vanilla block. |
| `experience` | `f32` |  |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-slot` | `func(handle: u32, slot: u32) -> option<item-stack>` | The block entity's slot, none when empty or out of range. |
| `set-slot` | `func(handle: u32, slot: u32, stack: option<item-stack>) -> bool` | Empties the slot with none; a stack over the item's maximum is cut to it. False for a slot out of range, an unknown item or a handle not in this call. |
| `insert-slot` | `func(handle: u32, slot: u32, stack: item-stack) -> bool` | Atomically adds the whole default-component stack to an empty or matching slot. It will not merge into a slot with custom components, since this record does not carry components. Returns false without changing the slot if it is full, contains another item, has custom components, the item is unknown, or the handle/slot is invalid. Use `max-stack-size` to decide whether a larger result should be split. |
| `get-position` | `func(handle: u32) -> option<block-entity-position>` | The loaded block entity's dimension and world position. |
| `get-world-rule` | `func(name: string) -> option<string>` | Reads a game-side rule value attached to this world's mod runtime. |
| `set-world-rule` | `func(name: string, value: string) -> result<_, string>` | Sets a namespaced rule value for coordination between this mod's machines. |
| `get-property` | `func(handle: u32, name: string) -> option<string>` | A property of the block entity's block state (`lit` -> `true`). |
| `set-property` | `func(handle: u32, name: string, value: string) -> bool` | False when the block has no such property or value. |
| `max-stack-size` | `func(id: string) -> u32` | The most of `id` one slot holds; 0 for an unknown item. |
| `burn-time` | `func(id: string) -> u32` | Ticks `id` burns as a furnace's fuel (the game's fuel values), 0 if it is no fuel. |
| `remainder` | `func(id: string) -> option<string>` | What is left of `id` once used up as fuel or an ingredient (a bucket from a lava bucket). |
| `cooking` | `func(kind: cooking-kind, input: string) -> option<cooking-recipe>` | The data pack's cooking recipe of `kind` for `input`. |

## `item-transfer`

Access: capability `item-transfer`.

Capability `item-transfer` (API 0.4): sided access for mod machine slots. Rules are owned by the mod's block entity and reset when it is recreated.

### enum `face`

| Case | |
| --- | --- |
| `down` |  |
| `up` |  |
| `north` |  |
| `south` |  |
| `west` |  |
| `east` |  |

| Function | Signature | What it does |
| --- | --- | --- |
| `set-slot-access` | `func(handle: u32, slot: u32, face: face, insert: bool, extract: bool) -> bool` | Sets whether hoppers may insert into or extract from one slot through `face`. Unconfigured slot faces allow both. Returns false for an unknown block entity handle or out-of-range slot. |

## `item-data`

Access: capability `item-data`.

Capability `item-data` (API 0.4): opaque data components on machine slots.

| Function | Signature | What it does |
| --- | --- | --- |
| `get-slot-components` | `func(handle: u32, slot: u32) -> list<u8>` | Returns the stack's `minecraft:components` compound as binary network NBT (empty when the stack has only defaults, or the slot is empty). |
| `set-slot-components` | `func(handle: u32, slot: u32, components: list<u8>) -> bool` | Replaces the compound with binary network NBT; empty clears it. The value must be a compound root and at most 64 KiB. False for an invalid value, empty slot, or out-of-range slot. |

## `block-entities`

Access: exported by the mod: the game calls it.

What the game calls on a mod's block entities (API 0.4). Only for the blocks `types` names, which must be the mod's own.

### record `block-entity-type`

| Field | Type | |
| --- | --- | --- |
| `block` | `string` | The mod's block, `<mod>:<path>`. |
| `slots` | `u32` | Slots the game keeps for each of its block entities. |
| `ticks` | `bool` | Call `tick` every server tick while it is loaded. |

| Function | Signature | What it does |
| --- | --- | --- |
| `types` | `func() -> list<block-entity-type>` | Once, after `init`. |
| `create` | `func(handle: u32, block: string, saved: list<u8>)` | A block entity was placed or loaded: `saved` is what `save` returned for it, empty for a new one. |
| `tick` | `func(handles: list<u32>)` | One server tick of every loaded ticking block entity of this mod. |
| `save` | `func(handle: u32) -> list<u8>` | Its state for the save, beside the slots the game saves itself. |
| `remove` | `func(handle: u32)` | Broken or unloaded: forget it. |
| `data` | `func(handle: u32) -> list<s32>` | The values its menu shows (vanilla's `ContainerData`: progress bars, fuel left). |

## `config`

Access: capability `config`.

Capability `config` (API 0.3): typed settings from `horizon-mod.toml`'s `[config.<key>]` table and player overrides.

| Function | Signature | What it does |
| --- | --- | --- |
| `get-bool` | `func(key: string) -> option<bool>` | Reads a boolean setting, or none if unknown or of another type. |
| `get-int` | `func(key: string) -> option<s64>` | Reads an integer setting, or none if unknown or of another type. |
| `get-float` | `func(key: string) -> option<f64>` | Reads a float setting, or none if unknown or of another type. |
| `get-string` | `func(key: string) -> option<string>` | Reads a string setting, or none if unknown or of another type. |
| `set-bool` | `func(key: string, value: bool) -> result<_, string>` | Updates a boolean setting within its declared schema. |
| `set-int` | `func(key: string, value: s64) -> result<_, string>` | Updates an integer setting within its declared range. |
| `set-float` | `func(key: string, value: f64) -> result<_, string>` | Updates a float setting within its declared range. |
| `set-string` | `func(key: string, value: string) -> result<_, string>` | Updates a string setting within its declared choices. |

## `server`

Access: capability `server`.

Capability `server` (API 0.3): server-side world queries and mutations during block, item, command and server-event hooks.

### record `block-pos`

| Field | Type | |
| --- | --- | --- |
| `x` | `s32` | Block X coordinate. |
| `y` | `s32` | Block Y coordinate. |
| `z` | `s32` | Block Z coordinate. |

### record `item-drop`

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | Item identifier (`minecraft:diamond`, `<mod>:<item>`). |
| `count` | `u32` | Stack count (1 to 99). |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-block` | `func(dimension: string, pos: block-pos) -> option<string>` | Block state string at `pos` in `dimension`, or none if unloaded. |
| `set-block` | `func(dimension: string, pos: block-pos, state: string) -> result<_, string>` | Replaces the block at `pos` in `dimension` with `state`. |
| `schedule-tick` | `func(dimension: string, pos: block-pos, block: string, delay: u32)` | Schedules a block tick for `block` at `pos` after `delay` ticks. |
| `spawn-drop` | `func(dimension: string, pos: block-pos, drop: item-drop)` | Spawns an item stack drop at `pos` in `dimension`. |
| `spawn-mob` | `func(dimension: string, pos: block-pos, entity: string) -> result<_, string>` | Spawns a vanilla living entity at `pos` in `dimension`. |
| `play-sound` | `func(dimension: string, pos: block-pos, sound: string, volume: f32, pitch: f32)` | Plays sound `sound` at `pos` in `dimension`. |
| `get-rule` | `func(name: string) -> option<string>` | Reads a game rule value as text (`true`, `false`, or integer). |
| `set-rule` | `func(name: string, value: string) -> result<_, string>` | Sets a game rule value as text. |
| `get-block-face` | `func() -> option<string>` | Hit face of the block for the current block callback, if available. |

## `keys`

Access: capability `keys`.

Capability `keys` (API 0.3): mod keybindings shown in the Controls screen.

### record `key-binding`

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | Action identifier (`[a-z0-9_.-]{1,64}`). |
| `label` | `string` | Label shown in Controls. |
| `default-key` | `string` | Default key code (`KeyM`, `KeyR`, `Digit1`, ...). |

| Function | Signature | What it does |
| --- | --- | --- |
| `register` | `func(binding: key-binding) -> result<_, string>` | Registers a keybinding for the mod. |
| `is-down` | `func(id: string) -> bool` | True while the player holds the key down (outside menus/chat). |
| `was-pressed` | `func(id: string) -> bool` | True once per press since the last tick. |

## `target`

Access: capability `target`.

Capability `target` (API 0.3): what the local player's crosshair points at.

### record `block-hit`

| Field | Type | |
| --- | --- | --- |
| `x` | `s32` | Block X coordinate. |
| `y` | `s32` | Block Y coordinate. |
| `z` | `s32` | Block Z coordinate. |
| `state` | `string` | Block state string (`minecraft:chest[facing=north]`). |
| `face` | `string` | Hit face (`down`, `up`, `north`, `south`, `west`, `east`). |
| `owner` | `string` | Mod id or `minecraft`. |

### record `entity-hit`

| Field | Type | |
| --- | --- | --- |
| `kind` | `string` | Entity type (`minecraft:villager`, `<mod>:<entity>`). |
| `health` | `f32` | Current health. |
| `max-health` | `f32` | Maximum health. |
| `name` | `string` | Custom or translated display name. |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-block` | `func() -> option<block-hit>` | The block in the player's crosshair, if any. |
| `get-entity` | `func() -> option<entity-hit>` | The entity in the player's crosshair, if any. |

## `inventory`

Access: capability `inventory-read`.

Capability `inventory-read` (API 0.3): read-only view of the player's inventory and open container slots.

### record `stack-view`

| Field | Type | |
| --- | --- | --- |
| `slot` | `u32` | Slot index. |
| `id` | `string` | Item identifier (`minecraft:iron_pickaxe`). |
| `count` | `u32` | Stack count. |
| `damage` | `u32` | Current damage (0 when undamaged). |
| `max-damage` | `u32` | Maximum durability (0 when not damageable). |

| Function | Signature | What it does |
| --- | --- | --- |
| `get-main-hand` | `func() -> option<stack-view>` | The item in the player's main hand, if any. |
| `get-off-hand` | `func() -> option<stack-view>` | The item in the player's off hand, if any. |
| `get-player-inventory` | `func() -> list<stack-view>` | Non-empty slots in the player's inventory (0..36 hotbar+main, 36..40 armor, 40 offhand). |
| `get-open-container` | `func() -> list<stack-view>` | Non-empty slots in the currently open container menu, if any. |

## `capabilities`

Access: capability `capabilities`.

Capability `capabilities` (API 0.4): standard Forge Energy (FE), fluid tanks, item handlers, data attachments and Curios accessory slots.

### record `energy-view`

| Field | Type | |
| --- | --- | --- |
| `stored` | `u64` | Stored Forge Energy (FE). |
| `capacity` | `u64` | Capacity in FE. |
| `max-receive` | `u64` | Max FE received per tick. |
| `max-extract` | `u64` | Max FE extracted per tick. |

### record `fluid-view`

| Field | Type | |
| --- | --- | --- |
| `fluid` | `string` | Fluid id (`minecraft:water`) or chemical id (`mekanism:hydrogen`), or empty. |
| `amount` | `u64` | Amount in millibuckets (mB). |
| `capacity` | `u64` | Tank capacity in mB. |

### record `item-slot`

| Field | Type | |
| --- | --- | --- |
| `slot` | `u32` | Slot index. |
| `id` | `string` | Item id (`minecraft:iron_ingot`). |
| `count` | `u32` | Item count. |

| Function | Signature | What it does |
| --- | --- | --- |
| `set-energy-buffer` | `func(pos: string, view: energy-view)` | Declares or updates the energy buffer at `pos` (`dimension:x,y,z`). Only the mod that owns a registered block entity at this position may define it; other mods use receive/extract to transfer energy. |
| `get-energy` | `func(pos: string) -> option<energy-view>` | Queries the energy buffer at `pos` (`dimension:x,y,z`). |
| `receive-energy` | `func(pos: string, amount: u64, simulate: bool) -> u64` | Pushes up to `amount` FE into `pos`; returns FE accepted. |
| `extract-energy` | `func(pos: string, amount: u64, simulate: bool) -> u64` | Pulls up to `amount` FE from `pos`; returns FE extracted. |
| `set-fluid-tank` | `func(pos: string, tank: u32, view: fluid-view)` | Declares or updates fluid or chemical tank `tank` at `pos`. Only the mod that owns a registered block entity at this position may define it; other mods use fill/drain to transfer resources. |
| `get-fluid` | `func(pos: string, tank: u32) -> option<fluid-view>` | Queries fluid or chemical tank `tank` at `pos`. |
| `fill-fluid` | `func(pos: string, fluid: string, amount: u64, simulate: bool) -> u64` | Fills up to `amount` mB of `fluid` into `pos`; returns mB accepted. |
| `fill-fluid-tank` | `func(pos: string, tank: u32, fluid: string, amount: u64, simulate: bool) -> u64` | Fills tank `tank` at `pos`; for chemical tanks the id is a chemical id. |
| `drain-fluid` | `func(pos: string, fluid: string, amount: u64, simulate: bool) -> u64` | Drains up to `amount` mB of `fluid` from `pos`; returns mB drained. |
| `drain-fluid-tank` | `func(pos: string, tank: u32, fluid: string, amount: u64, simulate: bool) -> u64` | Drains tank `tank` at `pos`; for chemical tanks the id is a chemical id. |
| `set-item-slot` | `func(pos: string, slot: u32, id: string, count: u32)` | Sets item slot `slot` at `pos` (count 0 clears). |
| `get-item-slots` | `func(pos: string) -> list<item-slot>` | Lists non-empty item slots at `pos`. |
| `get-attachment` | `func(target: string, key: string) -> option<list<u8>>` | Reads a mod data attachment on `target` (`entity:<id>`, `chunk:<dim>:<x>,<z>`, `level:<dim>`) under `key`. |
| `set-attachment` | `func(target: string, key: string, value: list<u8>) -> result<_, string>` | Writes a mod data attachment on `target` under `key`. |
| `delete-attachment` | `func(target: string, key: string)` | Deletes a mod data attachment on `target` under `key`. |
| `get-accessory` | `func(entity: string, slot-kind: string, index: u32) -> option<item-slot>` | Reads a Curios-style accessory slot (`ring`, `belt`, `charm`, `back`, ...) on `entity`. |
| `set-accessory` | `func(entity: string, slot-kind: string, index: u32, id: string, count: u32) -> result<_, string>` | Writes a Curios-style accessory slot on `entity`. |

## `ui`

Access: capability `ui`.

Capability `ui` (API 0.4): declarative container menus and screens.

### enum `widget-kind`

| Case | |
| --- | --- |
| `slot` |  |
| `progress-arrow` |  |
| `flame` |  |
| `energy-bar` |  |
| `fluid-gauge` |  |
| `label` |  |
| `button` |  |

### record `widget`

| Field | Type | |
| --- | --- | --- |
| `id` | `u32` | Widget id within the screen. |
| `kind` | `widget-kind` | Visual/interactive role. |
| `x` | `s32` | X offset in screen pixels. |
| `y` | `s32` | Y offset in screen pixels. |
| `width` | `u32` | Width in pixels. |
| `height` | `u32` | Height in pixels. |
| `text` | `string` | Label or tooltip text. |
| `value` | `u64` | Current value (progress, energy, fluid amount). |
| `max-value` | `u64` | Maximum value (capacity or max progress). |

### record `screen-spec`

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | Screen type id (`<mod>:<menu>`). |
| `title` | `string` | Title shown at the top of the screen. |
| `width` | `u32` | Width in pixels. |
| `height` | `u32` | Height in pixels. |
| `widgets` | `list<widget>` | Widgets drawn by the game. |

### record `screen-background`

A textured region drawn behind the widgets in a screen.

| Field | Type | |
| --- | --- | --- |
| `texture` | `string` | Sprite identifier from the mod or vanilla atlas. |
| `x` | `s32` | X offset in screen pixels. |
| `y` | `s32` | Y offset in screen pixels. |
| `width` | `u32` | Width in pixels. |
| `height` | `u32` | Height in pixels. |

| Function | Signature | What it does |
| --- | --- | --- |
| `open-screen` | `func(spec: screen-spec) -> result<_, string>` | Opens or updates a declarative container screen for the player. |
| `set-backgrounds` | `func(screen-id: string, backgrounds: list<screen-background>)` | Sets the textured regions for the currently open screen. |
| `close-screen` | `func()` | Closes the mod's open screen, if any. |
| `set-slot` | `func(screen-id: string, slot: u32, id: string, count: u32)` | Displays `id` and `count` in the slot widget whose id is `slot` in the open screen. This is display content; use machine `set-slot` for the machine's actual inventory. |
| `set-gauge` | `func(screen-id: string, widget-id: u32, value: u64, max-value: u64, text: string)` | Updates widget `widget-id`'s value/max-value/text in the open screen. |

## `net`

Access: capability `net`.

Capability `net` (API 0.4): bounded mod packets between client and server on the mod's own channels.

| Function | Signature | What it does |
| --- | --- | --- |
| `send-to-server` | `func(channel: string, payload: list<u8>) -> result<_, string>` | Sends `payload` (at most 32 KiB) on `channel` from client to server. |
| `send-to-client` | `func(player: string, channel: string, payload: list<u8>) -> result<_, string>` | Sends `payload` (at most 32 KiB) from the server to `player`, a UUID as 32 lowercase hexadecimal digits (`*` for all connected players). |

## `recipes`

Access: capability `recipes`.

Capability `recipes` (API 0.4): read the loaded recipe graph and register custom machine recipe types (`<mod>:<type>`).

### record `ingredient-slot`

| Field | Type | |
| --- | --- | --- |
| `items` | `list<string>` | Accepted item ids or `#tag` strings. For resource inputs, ids use `@fluid/<tank>/<id>` or `@chemical/<tank>/<id>`. |
| `count` | `u32` | Required count. |

### record `recipe-entry`

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | Recipe id (`minecraft:iron_ingot_from_smelting_raw_iron`, `<mod>:<name>`). |
| `kind` | `string` | Recipe type (`minecraft:crafting_shaped`, `minecraft:smelting`, `<mod>:<machine>`). |
| `inputs` | `list<ingredient-slot>` | Input ingredients. |
| `outputs` | `list<string>` | Output item ids and counts (`id*count`), or resource outputs as `@fluid/<tank>/<id>*<amount>` / `@chemical/<tank>/<id>*<amount>`. |
| `ticks` | `u32` | Processing time in ticks (0 for instant crafting). |
| `energy` | `u64` | Energy cost in FE (0 when none). |

| Function | Signature | What it does |
| --- | --- | --- |
| `get` | `func(id: string) -> option<recipe-entry>` | Looks up a recipe by id. |
| `by-type` | `func(kind: string) -> list<recipe-entry>` | Lists recipes of a given type (`minecraft:smelting`, `<mod>:crusher`, ...). |
| `for-input` | `func(item-id: string) -> list<recipe-entry>` | Lists recipes that consume `item-id`, including ingredients whose item tags include it in the active content registry. |
| `for-output` | `func(item-id: string) -> list<recipe-entry>` | Lists recipes that produce `item-id`. |
| `register` | `func(entry: recipe-entry) -> result<_, string>` | Registers a custom machine recipe under the mod's namespace. Entries must use valid namespaced ids and bounded item/resource selectors; each mod may register at most 4 MiB of recipe data. |

## `world-read`

Access: capability `world-read`.

Capability `world-read` (API 0.3): read-only block, biome and light queries in loaded cells within four blocks of the local player. Coordinates outside the current snapshot return `none`.

| Function | Signature | What it does |
| --- | --- | --- |
| `get-block` | `func(x: s32, y: s32, z: s32) -> option<string>` | Block state string at `(x, y, z)` in the current world, if loaded. |
| `get-biome` | `func(x: s32, y: s32, z: s32) -> option<string>` | Biome identifier at `(x, y, z)`, if loaded. |
| `get-light` | `func(x: s32, y: s32, z: s32) -> option<u8>` | Packed block and sky light (`(sky << 4) \| block`) at `(x, y, z)`, if loaded. |

## `render`

Access: capability `render`.

Capability `render` (API 0.4): declarative 3D overlays (wireframe boxes, lines and translucent ghost blocks) drawn by the game's renderer.

### record `box-shape`

| Field | Type | |
| --- | --- | --- |
| `min-x` | `f64` | Minimum X in world coordinates. |
| `min-y` | `f64` | Minimum Y in world coordinates. |
| `min-z` | `f64` | Minimum Z in world coordinates. |
| `max-x` | `f64` | Maximum X in world coordinates. |
| `max-y` | `f64` | Maximum Y in world coordinates. |
| `max-z` | `f64` | Maximum Z in world coordinates. |
| `color` | `u32` | 0xAARRGGBB color. |

### record `ghost-block`

| Field | Type | |
| --- | --- | --- |
| `x` | `s32` | Block X coordinate. |
| `y` | `s32` | Block Y coordinate. |
| `z` | `s32` | Block Z coordinate. |
| `state` | `string` | Block state string to preview. |
| `alpha` | `f32` | Opacity (0.0 to 1.0). |

| Function | Signature | What it does |
| --- | --- | --- |
| `set-shapes` | `func(boxes: list<box-shape>)` | Replaces the mod's 3D wireframe boxes (up to 256). |
| `set-ghost-blocks` | `func(blocks: list<ghost-block>)` | Replaces the mod's translucent ghost-block preview (up to 512 blocks). |
| `clear` | `func()` | Clears all 3D shapes and ghost blocks for this mod. |

## `logic`

Access: exported by the mod: the game calls it.

What the game calls for API 0.3 and 0.4 block, item, server, UI and network hooks. Only the hooks `hooks` asks for are ever called.

### enum `block-action`

| Case | |
| --- | --- |
| `use-without-item` |  |
| `use-item-on` |  |
| `place` |  |
| `broken` |  |
| `attack` |  |
| `random-tick` |  |
| `scheduled-tick` |  |
| `neighbor-changed` |  |
| `entity-inside` |  |

### record `block-event`

| Field | Type | |
| --- | --- | --- |
| `action` | `block-action` | Which block callback fired. |
| `dimension` | `string` | Dimension (`minecraft:overworld`, ...). |
| `x` | `s32` | Block X coordinate. |
| `y` | `s32` | Block Y coordinate. |
| `z` | `s32` | Block Z coordinate. |
| `state` | `string` | Block state string (`<mod>:<block>[...]`). |
| `actor` | `string` | Player name or entity id, if triggered by one. |
| `held-item` | `string` | Held item id, if any. |

### enum `item-action`

| Case | |
| --- | --- |
| `use-in-air` |  |
| `use-on-block` |  |
| `attack-entity` |  |
| `finish-using` |  |

### record `item-event`

| Field | Type | |
| --- | --- | --- |
| `action` | `item-action` | Which item callback fired. |
| `item` | `string` | Item identifier (`<mod>:<item>`). |
| `player` | `string` | Player name. |
| `dimension` | `string` | Dimension (`minecraft:overworld`, ...). |
| `x` | `s32` | Target X coordinate (block or entity). |
| `y` | `s32` | Target Y coordinate. |
| `z` | `s32` | Target Z coordinate. |
| `target` | `string` | Target entity or block state, if any. |

### enum `server-event-kind`

| Case | |
| --- | --- |
| `player-join` |  |
| `player-leave` |  |
| `entity-spawn` |  |
| `entity-death` |  |
| `block-break` |  |
| `block-place` |  |

### record `server-event`

| Field | Type | |
| --- | --- | --- |
| `kind` | `server-event-kind` | Which server event fired. |
| `subject` | `string` | Subject (player name or entity id). |
| `dimension` | `string` | Dimension (`minecraft:overworld`, ...). |
| `x` | `s32` | X coordinate. |
| `y` | `s32` | Y coordinate. |
| `z` | `s32` | Z coordinate. |
| `detail` | `string` | Extra context (block state, damage source, ...). |

### enum `event-outcome`

| Case | |
| --- | --- |
| `pass` | Let vanilla or default behaviour continue. |
| `consume` | Handled by the mod; swing hand and stop default handling. |
| `cancel` | Cancel the action. |

### enum `vanilla-hook-kind`

| Case | |
| --- | --- |
| `loot-drop` | Modify block or entity loot drops before they spawn. |
| `entity-damage` | Modify incoming or outgoing entity damage. |
| `xp-gain` | Modify experience orb or level gains. |
| `crop-grow` | Modify plant age stages, sapling trees, chorus growth, growing-plant segments and vine or mushroom spread during vanilla random ticks. |
| `explosion-detonate` | Filter or modify blocks affected by an explosion. |
| `enchantment-level` | Modify effective enchantment levels on an item or entity. |

### record `vanilla-hook-event`

| Field | Type | |
| --- | --- | --- |
| `kind` | `vanilla-hook-kind` | Which named vanilla hook fired. |
| `subject` | `string` | Subject identifier (block state, entity id, or player name). |
| `dimension` | `string` | Dimension (`minecraft:overworld`, ...). |
| `x` | `s32` | X coordinate. |
| `y` | `s32` | Y coordinate. |
| `z` | `s32` | Z coordinate. |
| `value` | `f32` | Numeric value associated with the event (damage amount, XP, next age stage, target height or tree/spread attempt count). |
| `context` | `string` | Secondary context (attacker id, tool id, enchantment id). |

### record `logic-hooks`

| Field | Type | |
| --- | --- | --- |
| `blocks` | `list<string>` | Owned block ids (`<mod>:<block>`) that receive `on-block`. |
| `items` | `list<string>` | Owned item ids (`<mod>:<item>`) that receive `on-item`. |
| `server-events` | `list<server-event-kind>` | Server events (`on-server`) the mod subscribes to. |
| `channels` | `list<string>` | Packet channels (`on-packet`) the mod listens on. |

| Function | Signature | What it does |
| --- | --- | --- |
| `hooks` | `func() -> logic-hooks` | Once, after `init`: which blocks, items, server events and channels the mod handles. |
| `on-block` | `func(event: block-event) -> event-outcome` | A block callback on one of the mod's registered `blocks`. |
| `on-item` | `func(event: item-event) -> event-outcome` | An item callback on one of the mod's registered `items`. |
| `on-server` | `func(event: server-event) -> event-outcome` | A server lifecycle or world event the mod subscribed to. |
| `on-menu-click` | `func(screen-id: string, slot: u32, button: u8)` | A slot click in a screen opened by the mod via `ui.open-screen`. |
| `on-menu-button` | `func(screen-id: string, button: u8)` | A button click in a screen opened by the mod via `ui.open-screen`. |
| `on-packet` | `func(channel: string, payload: list<u8>, from-player: option<string>)` | A packet received on `channel` from `net.send-to-server` or `net.send-to-client`. |
| `on-vanilla-hook` | `func(event: vanilla-hook-event) -> event-outcome` | A named vanilla hook (`vanilla-hooks` capability, API 0.5). |

## `entities`

Access: capability `entities`.

Capability `entities` (API 0.5): spawn, query and steer custom and world entities.

### record `entity-state`

| Field | Type | |
| --- | --- | --- |
| `handle` | `u32` | Runtime handle of the entity. |
| `kind` | `string` | Entity type identifier (`<mod>:<entity>` or `minecraft:<entity>`). |
| `dimension` | `string` | Dimension (`minecraft:overworld`, ...). |
| `x` | `f64` | X position in blocks. |
| `y` | `f64` | Y position in blocks. |
| `z` | `f64` | Z position in blocks. |
| `vx` | `f64` | X velocity in blocks per tick. |
| `vy` | `f64` | Y velocity in blocks per tick. |
| `vz` | `f64` | Z velocity in blocks per tick. |
| `yaw` | `f32` | Yaw in degrees. |
| `pitch` | `f32` | Pitch in degrees. |
| `health` | `f32` | Current health points. |
| `max-health` | `f32` | Maximum health points. |

| Function | Signature | What it does |
| --- | --- | --- |
| `spawn` | `func(dimension: string, kind: string, x: f64, y: f64, z: f64) -> result<u32, string>` | Spawns an entity of `kind` at `(x, y, z)` in `dimension`; returns its runtime handle. |
| `despawn` | `func(handle: u32) -> bool` | Removes an entity owned by the mod. |
| `get-state` | `func(handle: u32) -> option<entity-state>` | Reads the current state of an entity by handle. |
| `set-position` | `func(handle: u32, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) -> bool` | Moves an entity to `(x, y, z)` with `(yaw, pitch)`. |
| `set-velocity` | `func(handle: u32, vx: f64, vy: f64, vz: f64) -> bool` | Sets the velocity `(vx, vy, vz)` of an entity. |
| `set-health` | `func(handle: u32, health: f32) -> bool` | Sets the current health of an entity. |
| `navigate-to` | `func(handle: u32, x: f64, y: f64, z: f64, speed: f32) -> bool` | Instructs a mob entity to pathfind toward `(x, y, z)` at `speed`. |
| `nearby` | `func(dimension: string, x: f64, y: f64, z: f64, radius: f64) -> list<entity-state>` | Lists entity handles within `radius` blocks of `(x, y, z)` in `dimension`. |

## `mod-entities`

Access: exported by the mod: the game calls it.

Exported by a mod with capability `entities` (API 0.5): custom entity types, reusable vanilla AI goals and batched entity ticks.

### enum `spawn-category`

| Case | |
| --- | --- |
| `monster` |  |
| `creature` |  |
| `ambient` |  |
| `water-creature` |  |
| `misc` |  |

### enum `ai-goal`

| Case | |
| --- | --- |
| `float-in-water` |  |
| `melee-attack` |  |
| `random-stroll` |  |
| `look-at-player` |  |
| `hurt-by-target` |  |
| `nearest-attackable-player` |  |
| `follow-owner` |  |
| `tempt` |  |
| `fly-wander` |  |

### record `entity-type`

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | Entity identifier in one of the mod's namespaces (`<mod>:<name>`). |
| `category` | `spawn-category` | Spawn category (`monster`, `creature`, `misc`, ...). |
| `width` | `f32` | Bounding-box width in blocks. |
| `height` | `f32` | Bounding-box height in blocks. |
| `max-health` | `f32` | Base maximum health points. |
| `move-speed` | `f32` | Base movement speed. |
| `attack-damage` | `f32` | Base attack damage. |
| `fire-immune` | `bool` | Whether the entity is immune to fire and lava. |
| `goals` | `list<ai-goal>` | Reusable vanilla AI goals run by the engine between mod ticks. |

| Function | Signature | What it does |
| --- | --- | --- |
| `types` | `func() -> list<entity-type>` | The mod's custom entity types, called once after `init`. |
| `create` | `func(handle: u32, kind: string, saved: list<u8>)` | An entity of one of `types` was spawned (`saved` empty) or loaded from disk. |
| `tick` | `func(handles: list<u32>)` | One server tick of all loaded custom entities of this mod. |
| `save` | `func(handle: u32) -> list<u8>` | Serializes the mod's private state for `handle` when saving the chunk/world. |
| `remove` | `func(handle: u32)` | Called when `handle` is despawned or unloaded. |
| `on-interact` | `func(handle: u32, player: string, held-item: string) -> bool` | A player interacted (right-clicked) with `handle` holding `held-item`. |
| `on-hurt` | `func(handle: u32, amount: f32, source: string) -> bool` | `handle` took `amount` damage from `source`; returns true to allow the damage. |

## `models`

Access: capability `models`.

Capability `models` (API 0.5): animated models (GeckoLib / Bedrock bones) and budgeted declarative draw lists for block entities, entities and items.

### record `bone-pose`

| Field | Type | |
| --- | --- | --- |
| `bone` | `string` | Bone name in the geometry file (`assets/<mod>/geo/*.json`). |
| `rot-x` | `f32` | Rotation around X, Y, Z in degrees. |
| `rot-y` | `f32` |  |
| `rot-z` | `f32` |  |
| `pos-x` | `f32` | Pivot translation offset in model units (1/16th block). |
| `pos-y` | `f32` |  |
| `pos-z` | `f32` |  |
| `scale-x` | `f32` | Scale factors along X, Y, Z (1.0 is identity). |
| `scale-y` | `f32` |  |
| `scale-z` | `f32` |  |

### enum `draw-part-kind`

| Case | |
| --- | --- |
| `item-display` | Renders an item stack at a local offset and rotation (pedestals, belts). |
| `fluid-cuboid` | Renders a fluid cuboid with a fill level and tint (tanks, pipes). |
| `model-part` | Renders a baked sub-model with rotation (shafts, cogwheels, contraptions). |
| `floating-text` | Renders a camera-facing text label above the block or entity (Waystones, monitors). |

### record `draw-part`

| Field | Type | |
| --- | --- | --- |
| `kind` | `draw-part-kind` | What kind of declarative attachment to draw. |
| `asset` | `string` | Asset id (item id, fluid id, model id) or text string for `floating-text`. |
| `x` | `f32` | Local translation offset from the target origin in blocks. |
| `y` | `f32` |  |
| `z` | `f32` |  |
| `rot-x` | `f32` | Rotation around X, Y, Z in degrees. |
| `rot-y` | `f32` |  |
| `rot-z` | `f32` |  |
| `scale-x` | `f32` | Scale or cuboid dimensions along X, Y, Z. |
| `scale-y` | `f32` |  |
| `scale-z` | `f32` |  |
| `color` | `u32` | RGBA tint colour (`0xFFFFFFFF` for un-tinted). |

| Function | Signature | What it does |
| --- | --- | --- |
| `set-animation` | `func(target: string, animation: string, looping: bool, speed: f32)` | Triggers a named animation from `assets/<mod>/animations/*.json` on `target` (`block:x,y,z` or `entity:handle`). |
| `set-bone-poses` | `func(target: string, poses: list<bone-pose>)` | Overrides individual bone poses on `target` (`block:x,y,z` or `entity:handle`), up to 64 bones. |
| `set-draw-list` | `func(target: string, parts: list<draw-part>)` | Replaces the declarative draw list on `target` (`block:x,y,z` or `entity:handle`), up to 32 parts. |

## `dimensions`

Access: capability `dimensions`.

Capability `dimensions` (API 0.5): custom dimensions, portals and cross-dimension travel.

### enum `generator-kind`

| Case | |
| --- | --- |
| `noise` | Standard 3D noise terrain using worldgen JSON settings. |
| `flat` | Flat layered terrain. |
| `void-world` | Empty void world (compact machines, skyblock). |
| `custom` | Driven by the mod's surface/column rules. |

### record `dimension-spec`

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | Dimension identifier (`<mod>:<name>`). |
| `ultrawarm` | `bool` | Whether water evaporates and lava flows faster (like the Nether). |
| `natural` | `bool` | Whether compasses spin randomly (`false`) or point to spawn (`true`). |
| `coordinate-scale` | `f64` | Coordinate ratio relative to the Overworld (8.0 for the Nether, 1.0 default). |
| `has-skylight` | `bool` | Whether the dimension has sky light. |
| `has-ceiling` | `bool` | Whether the dimension has a bedrock ceiling. |
| `min-y` | `s32` | Minimum build Y coordinate (multiple of 16, e.g. -64). |
| `height` | `u32` | Total build height in blocks (multiple of 16, e.g. 384). |
| `ambient-light` | `f32` | Ambient light level (0.0 to 1.0). |
| `fixed-time` | `option<u64>` | Fixed day-time ticks, if time does not advance. |
| `generator` | `generator-kind` | Which chunk generator drives the dimension. |
| `surface-block` | `string` | Default surface block state for `flat` or `custom` generators. |

### record `portal-spec`

| Field | Type | |
| --- | --- | --- |
| `id` | `string` | Portal identifier (`<mod>:<name>`). |
| `frame-block` | `string` | Block id forming the portal frame (`minecraft:reinforced_deepslate`, `<mod>:frame`, ...). |
| `activation-item` | `string` | Item id that activates the portal (`minecraft:flint_and_steel`, `<mod>:key`, ...). |
| `target-dimension` | `string` | Target dimension id (`<mod>:<dimension>`). |
| `tint-color` | `u32` | RGB tint colour of the portal interior (`0xRRGGBB`). |

| Function | Signature | What it does |
| --- | --- | --- |
| `register-dimension` | `func(spec: dimension-spec) -> result<_, string>` | Registers a custom dimension in one of the mod's namespaces. |
| `register-portal` | `func(spec: portal-spec) -> result<_, string>` | Registers a custom portal linking to `spec.target-dimension`. |
| `teleport` | `func(subject: string, dimension: string, x: f64, y: f64, z: f64) -> result<_, string>` | Teleports a player or entity to `(x, y, z)` in `dimension`. |
| `list-dimensions` | `func() -> list<string>` | Lists all registered dimension ids (vanilla and mod-added). |

## `vanilla-hooks`

Access: capability `vanilla-hooks`.

Capability `vanilla-hooks` (API 0.5): named hooks into vanilla behaviour (what Mixins do on NeoForge/Fabric, exposed as safe, budgeted callbacks).

| Function | Signature | What it does |
| --- | --- | --- |
| `subscribe` | `func(hook: string) -> result<_, string>` | Subscribes the mod to a named vanilla hook (`loot-drop`, `entity-damage`, `xp-gain`, `crop-grow`, `explosion-detonate`, `enchantment-level`). |
| `set-damage-multiplier` | `func(factor: f32)` | During an `entity-damage` hook: scales the final damage by `factor`. |
| `add-loot-drop` | `func(item: string, count: u32)` | During a `loot-drop` hook: appends `count` of `item` to the loot output. |
| `set-xp-multiplier` | `func(factor: f32)` | During an `xp-gain` hook: scales the experience value by `factor`. |
| `set-crop-growth-multiplier` | `func(factor: f32)` | During `crop-grow`: scales age stages, sapling tree attempts, vertical segments or spread attempts (default 1; values below 1 suppress growth; larger values round down and cap at 64; sapling feature placement is limited to four attempts per random tick; pass applies; consume/cancel suppresses the vanilla growth action). |
| `add-enchantment-bonus` | `func(bonus: s32)` | During an `enchantment-level` hook: adds `bonus` levels to the queried enchantment. |

# Mod entity models and dimension data

Custom entity types registered through API 0.5 may provide an articulated
model in `assets/<namespace>/horizon/entity_models/<entity-path>.json`, where
the namespace and path match the entity id. Its `texture` is a namespaced
sprite under `textures/entity/` (without `.png`), and `model` is a tree of
parts using the same baked geometry as `entity-models.json`:

```json
{"texture":"example:entity/automaton","model":{"children":{"body":{"pose":[0,0,0,0,0,0,1,1,1],"polygons":[[[0,0,0,0,0],[16,0,0,1,0],[16,16,0,1,1],[0,16,0,0,1]]]}}}}
```

The corresponding texture is `assets/example/textures/entity/automaton.png`.
Vertices contain x/y/z in model pixels followed by normalized u/v; each
polygon contains four vertices. Parts can have named children, a pose with
translation, rotation in radians and scale, and optional per-polygon normals.
Named parts can receive the existing model bone pose overrides. Model data is
limited to 4 MiB, 32 nested levels and 4096 parts plus polygons. Missing models
use the existing fallback.

The model declaration can instead reference a Geo asset:

```json
{"texture":"example:entity/automaton","geo":"example:entity/automaton"}
```

This loads `assets/example/geo/entity/automaton.geo.json`. The supported
subset of [Minecraft's documented geometry format](https://learn.microsoft.com/en-us/minecraft/creator/reference/content/visualreference/geometry.v1.14.0?view=minecraft-bedrock-stable)
includes one geometry, named parented bones, pivots, rotations, cubes, inflate,
mirror and box or per-face UVs. Bone poses can target named bones. Geo animation
files, multiple geometries, polygon/texture meshes and face material overrides
are not supported. The importer bounds geometry size and rejects unsupported
mesh types and invalid hierarchies.

For a registered `flat` dimension, an optional data-pack definition at
`data/<namespace>/dimension/<dimension-path>.json` supplies the biome and
bottom-up layers. For example:

```json
{"type":"example:moon","generator":{"type":"minecraft:flat","settings":{"biome":"example:moon","layers":[{"block":"minecraft:bedrock","height":1},{"block":"example:soil","height":48}]}}}
```

Dimensions with a namespaced `data/<namespace>/dimension/` definition, an
available referenced dimension type and a supported flat or noise generator
are registered from their data, including
their referenced dimension type, without requiring a WASM registration call.
The type's vertical range and skylight are synchronized to clients. The
`--dimension-only` integration fixture exercises a data-only flat dimension
whose minimum Y is zero, including a world reopen, in both single-player modes.
The WASM `dimensions::list-dimensions` interface currently lists vanilla and
WASM-registered dimensions, but does not yet include dimensions registered only
from data.

The biome and blocks must be registered, and the layers must fit within both
the dimension and the engine's supported chunk height. Flat features and lakes
are not supported. Without a definition, the existing surface-block flat
generator remains in use. Noise dimensions use the existing worldgen data
pipeline and fail with a diagnostic when their definition or required data
cannot be loaded; they no longer silently become flat terrain.
The `--deep-only` API integration fixture packages its own
`test_machine:crystal_grove` biome and verifies its ID in the generated flat
dimension before and after reopening the world and in a fixed-biome noise
dimension in both single-player modes.
This fixture does not prove that every biome feature or third-party biome codec
is supported.
Legacy biome imports also migrate `precipitation` and the old sky, fog and
water-fog colors into their current 26.3 attributes, preserving explicit
new-format values. The optional `HORIZON_TEST_COMPACTMACHINES_BIOME` test can
check a locally carried Compact Machines biome without including it in the tree.
The existing noise pipeline accepts fixed, checkerboard and multi-noise biome
sources, subject to the worldgen codecs and registries available in the loaded
resource bundle. This does not yet imply support for arbitrary mod-specific
generators or worldgen codecs.

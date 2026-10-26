//! Laser Bridges: two laser source blocks facing each other carry a beam that
//! hurts whatever walks through it.
//!
//! Right-clicking a source looks for its partner along the six axes and lays
//! the powered beam blocks between them; left-clicking clears the beam again.
//! Damaging entities needs entity damage hooks, which the API does not expose
//! to mods' blocks yet.
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    server::{self, BlockPos},
};

const MAX_LENGTH: i32 = 48;

const SOURCES: [&str; 4] = [
    "laser_source_block",
    "laser_source_fullblock",
    "laser_fence_source_block",
    "laser_fence_source_fullblock",
];

const DIRECTIONS: [(i32, i32, i32); 6] = [
    (1, 0, 0),
    (-1, 0, 0),
    (0, 1, 0),
    (0, -1, 0),
    (0, 0, 1),
    (0, 0, -1),
];

fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

fn powered_for(source: &str) -> &'static str {
    if source.starts_with("laser_fence") {
        "laserbridges:laser_fence_powered"
    } else {
        "laserbridges:laser_block_powered"
    }
}

struct LaserBridges {
    /// The beam the last click formed.
    beam: Vec<BlockPos>,
    dimension: String,
}

impl LaserBridges {
    fn clear(&mut self) {
        for pos in std::mem::take(&mut self.beam) {
            let state = server::get_block(&self.dimension, pos).unwrap_or_default();
            if path_of(&state).ends_with("_powered") {
                let _ = server::set_block(&self.dimension, pos, "minecraft:air");
            }
        }
    }

    /// The other source along an axis, with the number of blocks between them.
    fn partner(&self, event: &BlockEvent) -> Option<((i32, i32, i32), i32)> {
        for (dx, dy, dz) in DIRECTIONS {
            for step in 1..=MAX_LENGTH {
                let pos = BlockPos {
                    x: event.x + dx * step,
                    y: event.y + dy * step,
                    z: event.z + dz * step,
                };
                let Some(state) = server::get_block(&event.dimension, pos) else {
                    break;
                };
                let path = path_of(&state);
                if SOURCES.contains(&path) {
                    return Some(((dx, dy, dz), step));
                }
                if !state.starts_with("minecraft:air") && !path.ends_with("_powered") {
                    break;
                }
            }
        }
        None
    }
}

impl Mod for LaserBridges {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            LaserBridges {
                beam: Vec::new(),
                dimension: String::new(),
            },
            Registration::new().command("laserbridge"),
        ))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for source in SOURCES {
            hooks = hooks.block(&format!("laserbridges:{source}"));
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action == BlockAction::Attack {
            self.clear();
            chat::show("⚡ Laser bridge cleared.");
            return EventOutcome::Consume;
        }
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let source = path_of(&event.state).to_string();
        if !SOURCES.contains(&source.as_str()) {
            return EventOutcome::Pass;
        }

        let Some(((dx, dy, dz), length)) = self.partner(&event) else {
            chat::show(&format!(
                "⚡ Laser source: no matching source within {MAX_LENGTH} blocks."
            ));
            return EventOutcome::Consume;
        };

        self.clear();
        self.dimension = event.dimension.clone();
        let powered = powered_for(&source);
        for step in 1..length {
            let pos = BlockPos {
                x: event.x + dx * step,
                y: event.y + dy * step,
                z: event.z + dz * step,
            };
            let state = server::get_block(&event.dimension, pos).unwrap_or_default();
            if state.starts_with("minecraft:air")
                && server::set_block(&event.dimension, pos, powered).is_ok()
            {
                self.beam.push(pos);
            }
        }
        server::play_sound(
            &event.dimension,
            pos_of(&event),
            "minecraft:block.beacon.activate",
            0.6,
            1.4,
        );
        chat::show(&format!(
            "⚡ Laser bridge: {length} blocks to the other source ({} beam blocks).",
            self.beam.len()
        ));
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "laserbridge" {
            return Err(format!("Unknown command /{name}"));
        }
        Ok(Some(format!(
            "Laser Bridges: place two sources up to {MAX_LENGTH} blocks apart; right-click a source to form the beam, left-click to clear it."
        )))
    }
}

fn pos_of(event: &BlockEvent) -> BlockPos {
    BlockPos {
        x: event.x,
        y: event.y,
        z: event.z,
    }
}

horizon_mod_sdk::export_mod!(LaserBridges);

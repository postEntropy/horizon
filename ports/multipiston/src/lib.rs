//! Multi-Piston: pushes a whole line of blocks, and pulls it back.
//!
//! Right-click pushes up to twelve blocks one step along the piston's facing
//! (or the player's facing when the carried block state has none); left-click
//! pulls the same assembly one step back.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    chat, game,
    server::{self, BlockPos},
};

const LIMIT: i32 = 12;

/// Blocks a piston may not move, whichever state they are in.
const IMMOVABLE: [&str; 10] = [
    "minecraft:bedrock",
    "minecraft:obsidian",
    "minecraft:crying_obsidian",
    "minecraft:chest",
    "minecraft:trapped_chest",
    "minecraft:ender_chest",
    "minecraft:spawner",
    "minecraft:reinforced_deepslate",
    "minecraft:barrier",
    "minecraft:command_block",
];

#[derive(Clone, Copy)]
struct Dir {
    dx: i32,
    dy: i32,
    dz: i32,
}

fn step(x: i32, y: i32, z: i32, dir: Dir, n: i32) -> BlockPos {
    BlockPos {
        x: x + dir.dx * n,
        y: y + dir.dy * n,
        z: z + dir.dz * n,
    }
}

fn base_id(state: &str) -> &str {
    state.split('[').next().unwrap_or(state)
}

fn is_free(state: Option<String>) -> bool {
    let Some(state) = state else {
        return false;
    };
    if state.starts_with("minecraft:air")
        || state.starts_with("minecraft:cave_air")
        || state.starts_with("minecraft:water")
        || state.starts_with("minecraft:lava")
    {
        return true;
    }
    base_id(&state).ends_with("_grass")
        && state.starts_with("minecraft:short")
}

fn is_immovable(state: &str) -> bool {
    let id = base_id(state);
    IMMOVABLE.contains(&id) || id.ends_with("_bed") || id.contains("shulker_box")
}

/// The piston's facing: its own state when the template has one, else the
/// player's yaw (0° is south, 90° west, 180° north, 270° east).
fn facing(event: &BlockEvent) -> Dir {
    if let Some(rest) = event.state.split("facing=").nth(1) {
        let name = rest.split([',', ']']).next().unwrap_or("");
        let dir = match name {
            "north" => Some(Dir { dx: 0, dy: 0, dz: -1 }),
            "south" => Some(Dir { dx: 0, dy: 0, dz: 1 }),
            "east" => Some(Dir { dx: 1, dy: 0, dz: 0 }),
            "west" => Some(Dir { dx: -1, dy: 0, dz: 0 }),
            "up" => Some(Dir { dx: 0, dy: 1, dz: 0 }),
            "down" => Some(Dir { dx: 0, dy: -1, dz: 0 }),
            _ => None,
        };
        if let Some(dir) = dir {
            return dir;
        }
    }
    let Some(player) = game::get_player() else {
        return Dir { dx: 0, dy: 0, dz: 1 };
    };
    let yaw = ((player.yaw as f64) % 360.0 + 360.0) % 360.0;
    if (45.0..135.0).contains(&yaw) {
        Dir { dx: -1, dy: 0, dz: 0 }
    } else if (135.0..225.0).contains(&yaw) {
        Dir { dx: 0, dy: 0, dz: -1 }
    } else if (225.0..315.0).contains(&yaw) {
        Dir { dx: 1, dy: 0, dz: 0 }
    } else {
        Dir { dx: 0, dy: 0, dz: 1 }
    }
}

/// The contiguous run of blocks the piston can move, nearest first.
fn run(event: &BlockEvent, dir: Dir) -> Vec<(i32, String)> {
    let mut moved = Vec::new();
    for offset in 1..=LIMIT {
        let pos = step(event.x, event.y, event.z, dir, offset);
        let Some(state) = server::get_block(&event.dimension, pos) else {
            break;
        };
        if is_free(Some(state.clone())) {
            break;
        }
        moved.push((offset, state));
    }
    moved
}

fn push(event: &BlockEvent) -> EventOutcome {
    let dir = facing(event);
    let moved = run(event, dir);
    if moved.is_empty() {
        chat::show("🫸 Nothing to push.");
        return EventOutcome::Consume;
    }
    if moved.iter().any(|(_, state)| is_immovable(state)) {
        chat::show("🚫 The line contains a block a piston cannot move.");
        return EventOutcome::Consume;
    }
    if moved.len() as i32 >= LIMIT {
        chat::show("🚫 The line is too long for the multi-piston.");
        return EventOutcome::Consume;
    }
    let beyond = step(event.x, event.y, event.z, dir, moved.len() as i32 + 1);
    if !is_free(server::get_block(&event.dimension, beyond)) {
        chat::show("🚫 No room in front of the line.");
        return EventOutcome::Consume;
    }

    for (offset, state) in moved.iter().rev() {
        let target = step(event.x, event.y, event.z, dir, offset + 1);
        let _ = server::set_block(&event.dimension, target, state);
    }
    let front = step(event.x, event.y, event.z, dir, 1);
    let _ = server::set_block(&event.dimension, front, &event.state);
    let _ = server::set_block(
        &event.dimension,
        BlockPos {
            x: event.x,
            y: event.y,
            z: event.z,
        },
        "minecraft:air",
    );

    server::play_sound(
        &event.dimension,
        front,
        "minecraft:block.piston.extend",
        1.0,
        0.9,
    );
    chat::show(&format!("🫸 Pushed {} blocks.", moved.len()));
    EventOutcome::Consume
}

fn pull(event: &BlockEvent) -> EventOutcome {
    let dir = facing(event);
    let back = step(event.x, event.y, event.z, dir, -1);
    if !is_free(server::get_block(&event.dimension, back)) {
        chat::show("🫷 No room behind the multi-piston.");
        return EventOutcome::Consume;
    }

    let moved = run(event, dir);
    let _ = server::set_block(&event.dimension, back, &event.state);
    if moved.is_empty() {
        let _ = server::set_block(
            &event.dimension,
            BlockPos {
                x: event.x,
                y: event.y,
                z: event.z,
            },
            "minecraft:air",
        );
    } else {
        for (offset, state) in &moved {
            let target = step(event.x, event.y, event.z, dir, offset - 1);
            let _ = server::set_block(&event.dimension, target, state);
        }
        let tail = step(event.x, event.y, event.z, dir, moved.len() as i32);
        let _ = server::set_block(&event.dimension, tail, "minecraft:air");
    }

    server::play_sound(
        &event.dimension,
        back,
        "minecraft:block.piston.contract",
        1.0,
        0.9,
    );
    chat::show(&format!("🫷 Pulled {} blocks back.", moved.len()));
    EventOutcome::Consume
}

struct MultiPiston;

impl Mod for MultiPiston {
    fn init() -> Result<(Self, Registration), String> {
        Ok((MultiPiston, Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for item in items::ITEMS {
            hooks = hooks.item(item);
        }
        for block in items::BLOCKS {
            hooks = hooks.block(block);
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        match event.action {
            BlockAction::UseWithoutItem | BlockAction::UseItemOn => push(&event),
            BlockAction::Attack => pull(&event),
            _ => EventOutcome::Pass,
        }
    }
}

horizon_mod_sdk::export_mod!(MultiPiston);

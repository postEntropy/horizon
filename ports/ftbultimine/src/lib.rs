//! FTB Ultimine: vein mining of connected matching blocks on trigger.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    keys::{self, KeyBinding},
    server::{self, BlockPos, ItemDrop},
};
use std::collections::VecDeque;

struct FtbUltimine;

impl Mod for FtbUltimine {
    fn init() -> Result<(Self, Registration), String> {
        let _ = keys::register(&KeyBinding {
            id: "ultimine_active".into(),
            label: "FTB Ultimine Active".into(),
            default_key: "Grave".into(),
        });
        Ok((FtbUltimine, Registration::new()))
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
        if event.action == BlockAction::Broken {
            if keys::is_down("ultimine_active") {
                // BFS up to 32 adjacent matching blocks
                let mut queue = VecDeque::new();
                queue.push_back((event.x, event.y, event.z));
                let mut mined = 0;

                while let Some((cx, cy, cz)) = queue.pop_front() {
                    if mined >= 32 {
                        break;
                    }
                    for (dx, dy, dz) in [
                        (1, 0, 0), (-1, 0, 0),
                        (0, 1, 0), (0, -1, 0),
                        (0, 0, 1), (0, 0, -1),
                    ] {
                        let nx = cx + dx;
                        let ny = cy + dy;
                        let nz = cz + dz;
                        if nx == event.x && ny == event.y && nz == event.z {
                            continue;
                        }
                        let pos = BlockPos { x: nx, y: ny, z: nz };
                        if let Some(state) = server::get_block(&event.dimension, pos) {
                            if state == event.state {
                                let _ = server::set_block(&event.dimension, pos, "minecraft:air");
                                server::spawn_drop(&event.dimension, pos, &ItemDrop {
                                    id: event.state.split('[').next().unwrap_or(&event.state).to_string(),
                                    count: 1,
                                });
                                queue.push_back((nx, ny, nz));
                                mined += 1;
                                if mined >= 32 {
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(FtbUltimine);

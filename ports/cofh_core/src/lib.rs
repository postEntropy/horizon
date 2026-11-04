//! CoFH Core 9.2.3 gameplay behavior carried into Horizon.
use horizon_mod_sdk::server::{self, BlockPos};
use horizon_mod_sdk::{BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration};

const GLOSSY: &str = "cofh_core:glossed_magma";

pub struct CoFHCore;

/// Returns the next Glossed Magma state. Age 0–2 advances and returns false;
/// age 3 becomes lava and returns true, which triggers neighbor spread.
pub fn slightly_melt(state: &str) -> (Option<u8>, bool) {
    let age = state
        .split_once("[age=")
        .and_then(|(_, tail)| tail.split(']').next())
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(0)
        .min(3);
    if age == 3 {
        (None, true)
    } else {
        (Some(age + 1), false)
    }
}

impl Mod for CoFHCore {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Self, Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new().block(GLOSSY)
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if !event.state.starts_with(GLOSSY) {
            return EventOutcome::Pass;
        }

        let directions = [
            (0, -1, 0),
            (0, 1, 0),
            (0, 0, -1),
            (0, 0, 1),
            (-1, 0, 0),
            (1, 0, 0),
        ];
        let neighbors = directions.map(|(dx, dy, dz)| BlockPos {
            x: event.x + dx,
            y: event.y + dy,
            z: event.z + dz,
        });
        let adjacent = neighbors
            .iter()
            .filter(|pos| {
                server::get_block(&event.dimension, **pos)
                    .is_some_and(|block| block.starts_with(GLOSSY))
            })
            .count();

        match event.action {
            BlockAction::NeighborChanged => {
                if adjacent < 2 {
                    let _ = server::set_block(
                        &event.dimension,
                        BlockPos {
                            x: event.x,
                            y: event.y,
                            z: event.z,
                        },
                        "minecraft:lava",
                    );
                    EventOutcome::Consume
                } else {
                    EventOutcome::Pass
                }
            }
            BlockAction::RandomTick => {
                // The source calls nextInt(9) before the neighbor test, even
                // when the block is guaranteed to advance.
                let Some(roll) = server::random_int(9) else {
                    return EventOutcome::Pass;
                };
                if roll == 0 || adjacent < 4 {
                    let current = BlockPos {
                        x: event.x,
                        y: event.y,
                        z: event.z,
                    };
                    let (age, melted) = slightly_melt(&event.state);
                    let state = age
                        .map(|age| format!("{GLOSSY}[age={age}]"))
                        .unwrap_or_else(|| "minecraft:lava".to_owned());
                    let _ = server::set_block(&event.dimension, current, &state);

                    if !melted {
                        let Some(delay) = server::random_int(21) else {
                            return EventOutcome::Pass;
                        };
                        server::schedule_random_tick(
                            &event.dimension,
                            current,
                            GLOSSY,
                            (20 + delay) as u32,
                        );
                    } else {
                        for pos in neighbors {
                            let Some(block) = server::get_block(&event.dimension, pos) else {
                                continue;
                            };
                            if !block.starts_with(GLOSSY) {
                                continue;
                            }
                            let (age, melted) = slightly_melt(&block);
                            let state = age
                                .map(|age| format!("{GLOSSY}[age={age}]"))
                                .unwrap_or_else(|| "minecraft:lava".to_owned());
                            let _ = server::set_block(&event.dimension, pos, &state);
                            if !melted {
                                let Some(delay) = server::random_int(21) else {
                                    return EventOutcome::Pass;
                                };
                                server::schedule_random_tick(
                                    &event.dimension,
                                    pos,
                                    GLOSSY,
                                    (20 + delay) as u32,
                                );
                            }
                        }
                    }
                } else {
                    let Some(delay) = server::random_int(21) else {
                        return EventOutcome::Pass;
                    };
                    server::schedule_random_tick(
                        &event.dimension,
                        BlockPos {
                            x: event.x,
                            y: event.y,
                            z: event.z,
                        },
                        GLOSSY,
                        (20 + delay) as u32,
                    );
                }
                EventOutcome::Consume
            }
            _ => EventOutcome::Pass,
        }
    }
}

horizon_mod_sdk::export_mod!(CoFHCore);

#[cfg(test)]
mod tests {
    use super::slightly_melt;

    #[test]
    fn ages_and_melts_glossed_magma() {
        assert_eq!(
            slightly_melt("cofh_core:glossed_magma"),
            (Some(1), false)
        );
        assert_eq!(
            slightly_melt("cofh_core:glossed_magma[age=1]"),
            (Some(2), false)
        );
        assert_eq!(
            slightly_melt("cofh_core:glossed_magma[age=2]"),
            (Some(3), false)
        );
        assert_eq!(
            slightly_melt("cofh_core:glossed_magma[age=3]"),
            (None, true)
        );
    }
}

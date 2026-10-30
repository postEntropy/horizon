//! Small second mod used to verify that machine capabilities cross mod owners.
use horizon_mod_sdk::{
    BlockEntityType, Mod, Registration,
    capabilities::{self, EnergyView, FluidView},
    machine,
    server::{self, BlockPos},
};

struct Peer;

impl Mod for Peer {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Self, Registration::new().command("peer_api")))
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        if args.trim() != "set_peer_block" {
            return Ok(Some("usage: peer_api set_peer_block".into()));
        }
        server::set_block(
            "minecraft:overworld",
            BlockPos {
                x: -30,
                y: 70,
                z: -9,
            },
            "test_peer:cell",
        )?;
        Ok(Some("requested peer machine block".into()))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        vec![BlockEntityType {
            block: "test_peer:cell".into(),
            slots: 0,
            ticks: true,
        }]
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &handle in handles {
            if let Some(position) = machine::get_position(handle) {
                let key = format!(
                    "{}:{},{},{}",
                    position.dimension, position.x, position.y, position.z
                );
                let neighbor = format!(
                    "{}:{},{},{}",
                    position.dimension,
                    position.x - 1,
                    position.y,
                    position.z
                );
                // A neighboring mod may transfer through the public
                // capability API, but must not redefine this machine's
                // capacity, rate limits, or fluid identity.
                if capabilities::get_energy(&neighbor).is_some() {
                    capabilities::set_energy_buffer(
                        &neighbor,
                        EnergyView {
                            stored: 0,
                            capacity: u64::MAX,
                            max_receive: u64::MAX,
                            max_extract: u64::MAX,
                        },
                    );
                    capabilities::set_fluid_tank(
                        &neighbor,
                        0,
                        &FluidView {
                            fluid: "minecraft:lava".into(),
                            amount: 0,
                            capacity: u64::MAX,
                        },
                    );
                    capabilities::set_item_slot(&neighbor, 0, "minecraft:bedrock", 64);
                }
                if capabilities::get_energy(&key).is_none() {
                    capabilities::set_energy_buffer(
                        &key,
                        EnergyView {
                            stored: 0,
                            capacity: 1_000,
                            max_receive: 1,
                            max_extract: 100,
                        },
                    );
                }
                if capabilities::get_fluid(&key, 0).is_none() {
                    capabilities::set_fluid_tank(
                        &key,
                        0,
                        &FluidView {
                            fluid: String::new(),
                            amount: 0,
                            capacity: 16,
                        },
                    );
                }
            }
        }
    }
}

horizon_mod_sdk::export_mod!(Peer);

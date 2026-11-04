//! Macaw's Lights and Lamps: the two measured item-on-lamp outcomes only.
//!
//! The original public trace observed red dye and a stick on a white paper lamp
//! returning CONSUME without changing the block or the held stack. Other lamp
//! colors, light toggles, and item interactions remain unimplemented pending
//! player-facing traces.
use horizon_mod_sdk::{BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration};

const OBSERVED_BLOCKS: [&str; 1] = ["mcwlights:white_paper_lamp"];
const OBSERVED_ITEMS: [&str; 2] = ["minecraft:red_dye", "minecraft:stick"];

fn observed_outcome(event: &BlockEvent) -> EventOutcome {
    if event.action == BlockAction::UseItemOn
        && event.state.split('[').next() == Some(OBSERVED_BLOCKS[0])
        && OBSERVED_ITEMS.contains(&event.held_item.as_str())
    {
        EventOutcome::Consume
    } else {
        EventOutcome::Pass
    }
}

struct McwLights;

impl Mod for McwLights {
    fn init() -> Result<(Self, Registration), String> {
        Ok((McwLights, Registration::new()))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        LogicHooks::new().block(OBSERVED_BLOCKS[0])
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        observed_outcome(&event)
    }
}

horizon_mod_sdk::export_mod!(McwLights);

#[cfg(test)]
mod tests {
    use super::*;

    fn event(action: BlockAction, state: &str, held_item: &str) -> BlockEvent {
        BlockEvent {
            action,
            dimension: "minecraft:overworld".into(),
            x: 0,
            y: 0,
            z: 0,
            state: state.into(),
            actor: "Player".into(),
            held_item: held_item.into(),
        }
    }

    #[test]
    fn measured_white_lamp_interactions_consume_without_world_mutation() {
        for item in OBSERVED_ITEMS {
            assert_eq!(
                observed_outcome(&event(
                    BlockAction::UseItemOn,
                    "mcwlights:white_paper_lamp[lit=false,powered=false]",
                    item
                )),
                EventOutcome::Consume
            );
        }
    }

    #[test]
    fn unmeasured_actions_pass() {
        assert_eq!(
            observed_outcome(&event(
                BlockAction::UseWithoutItem,
                "mcwlights:garden_light[lit=false,powered=false]",
                "minecraft:air"
            )),
            EventOutcome::Pass
        );
        assert_eq!(
            observed_outcome(&event(
                BlockAction::UseItemOn,
                "mcwlights:white_paper_lamp",
                "minecraft:blue_dye"
            )),
            EventOutcome::Pass
        );
    }
}

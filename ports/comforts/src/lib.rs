//! Comforts: hammocks and sleeping bags for day sleeping and mobile rests.
mod items;
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration, chat,
    server::{self, BlockPos},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RestType {
    HammockDaySleep,
    SleepingBagRest,
}

pub fn get_rest_type(state: &str) -> Option<RestType> {
    if state.contains("hammock") {
        Some(RestType::HammockDaySleep)
    } else if state.contains("sleeping_bag") {
        Some(RestType::SleepingBagRest)
    } else {
        None
    }
}

pub struct Comforts;

impl Mod for Comforts {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Comforts, Registration::new()))
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
        if event.action == BlockAction::UseWithoutItem || event.action == BlockAction::UseItemOn {
            if let Some(rest) = get_rest_type(&event.state) {
                let pos = BlockPos {
                    x: event.x,
                    y: event.y,
                    z: event.z,
                };
                match rest {
                    RestType::HammockDaySleep => {
                        server::play_sound(
                            &event.dimension,
                            pos,
                            "minecraft:block.wool.place",
                            1.0,
                            0.8,
                        );
                        chat::show(
                            "😴 Resting in hammock: taking an afternoon nap until sunset...",
                        );
                        return EventOutcome::Consume;
                    }
                    RestType::SleepingBagRest => {
                        server::play_sound(
                            &event.dimension,
                            pos,
                            "minecraft:block.wool.step",
                            1.0,
                            0.9,
                        );
                        chat::show(
                            "⛺ Resting in sleeping bag: resting without changing your spawn point.",
                        );
                        return EventOutcome::Consume;
                    }
                }
            }
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Comforts);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rest_type_detection() {
        assert_eq!(
            get_rest_type("comforts:hammock_cyan"),
            Some(RestType::HammockDaySleep)
        );
        assert_eq!(
            get_rest_type("comforts:sleeping_bag_red"),
            Some(RestType::SleepingBagRest)
        );
        assert_eq!(get_rest_type("comforts:rope_and_nail"), None);
    }
}

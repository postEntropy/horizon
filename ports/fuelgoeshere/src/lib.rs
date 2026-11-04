//! Fuel Goes Here currently contributes only its carried `forced_fuels` tag.
//!
//! The original export has no `/fuel` command, and no original gameplay trace
//! demonstrates furnace burn durations. Keep the runtime inert until that
//! behavior is observed; the loaded tag is handled by Horizon's content/tag
//! registry.
use horizon_mod_sdk::{Mod, Registration};

pub struct FuelGoesHere;

impl Mod for FuelGoesHere {
    fn init() -> Result<(Self, Registration), String> {
        Ok((FuelGoesHere, Registration::new()))
    }
}

horizon_mod_sdk::export_mod!(FuelGoesHere);

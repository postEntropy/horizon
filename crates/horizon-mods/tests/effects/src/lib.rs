use horizon_mod_sdk::{Mod, Registration, status_effects};

struct Effects;

impl Mod for Effects {
    fn init() -> Result<(Self, Registration), String> {
        status_effects::register("test_effects:steady").map_err(|error| error.to_string())?;
        Ok((Self, Registration::new().on_tick()))
    }

    fn tick(&mut self, _tick: u64) {
        if status_effects::player_id() != "test-player-uuid" {
            return;
        }
        for effect in status_effects::active() {
            let _ = status_effects::set_food(
                &effect.id,
                effect.food.min(19) + 1,
                (effect.saturation + 0.25).min(20.0),
            );
        }
    }
}

horizon_mod_sdk::export_mod!(Effects);

//! Measurements: tape measure tool for distance and volume measurement.
mod items;
use horizon_mod_sdk::{
    EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat,
};

struct Measurements {
    start_point: Option<(String, i32, i32, i32)>,
}

impl Mod for Measurements {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Measurements { start_point: None }, Registration::new()))
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

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if event.action == ItemAction::UseOnBlock {
            let x = event.x;
            let y = event.y;
            let z = event.z;
            match self.start_point.take() {
                None => {
                    self.start_point = Some((event.dimension.clone(), x, y, z));
                    chat::show(&format!("📍 Point 1: [{x}, {y}, {z}]"));
                }
                Some((dim, x1, y1, z1)) => {
                    if dim == event.dimension {
                        let dx = (x - x1).abs() + 1;
                        let dy = (y - y1).abs() + 1;
                        let dz = (z - z1).abs() + 1;
                        let dist = (((x - x1).pow(2) + (y - y1).pow(2) + (z - z1).pow(2)) as f64).sqrt();
                        chat::show(&format!(
                            "📏 Distance: {dist:.1}m | Size: {dx}x{dy}x{dz} ({vol} blocks)",
                            vol = dx * dy * dz
                        ));
                    } else {
                        self.start_point = Some((event.dimension.clone(), x, y, z));
                        chat::show(&format!("📍 Reset Point 1: [{x}, {y}, {z}]"));
                    }
                }
            }
            return EventOutcome::Consume;
        }
        EventOutcome::Pass
    }
}

horizon_mod_sdk::export_mod!(Measurements);

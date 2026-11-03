//! Measurements: tape measure tool for distance, area and volume measurement.
//!
//! Provides point-to-point Euclidean and Manhattan distance calculation,
//! bounding box dimensions (dx x dy x dz), floor footprint area, and block volume.

mod items;
use horizon_mod_sdk::{EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MeasurementResult {
    pub dx: i32,
    pub dy: i32,
    pub dz: i32,
    pub volume: u64,
    pub area_floor: u64,
    pub euclidean: f64,
    pub manhattan: i32,
}

pub fn calculate_measurement(p1: (i32, i32, i32), p2: (i32, i32, i32)) -> MeasurementResult {
    let dx = (p2.0 - p1.0).abs() + 1;
    let dy = (p2.1 - p1.1).abs() + 1;
    let dz = (p2.2 - p1.2).abs() + 1;
    let volume = (dx as u64) * (dy as u64) * (dz as u64);
    let area_floor = (dx as u64) * (dz as u64);

    let raw_dx = (p2.0 - p1.0) as f64;
    let raw_dy = (p2.1 - p1.1) as f64;
    let raw_dz = (p2.2 - p1.2) as f64;
    let euclidean = (raw_dx.powi(2) + raw_dy.powi(2) + raw_dz.powi(2)).sqrt();
    let manhattan = (p2.0 - p1.0).abs() + (p2.1 - p1.1).abs() + (p2.2 - p1.2).abs();

    MeasurementResult {
        dx,
        dy,
        dz,
        volume,
        area_floor,
        euclidean,
        manhattan,
    }
}

pub struct Measurements {
    pub start_point: Option<(String, i32, i32, i32)>,
    pub last_result: Option<MeasurementResult>,
}

impl Measurements {
    pub fn new() -> Self {
        Self {
            start_point: None,
            last_result: None,
        }
    }

    pub fn reset(&mut self) {
        self.start_point = None;
    }
}

impl Mod for Measurements {
    fn init() -> Result<(Self, Registration), String> {
        Ok((
            Measurements::new(),
            Registration::new().command("measurements"),
        ))
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
        if event.action == ItemAction::UseInAir {
            self.reset();
            chat::show("📏 Tape Measure: selection cleared.");
            return EventOutcome::Consume;
        }

        if event.action == ItemAction::UseOnBlock {
            let x = event.x;
            let y = event.y;
            let z = event.z;

            match self.start_point.take() {
                None => {
                    self.start_point = Some((event.dimension.clone(), x, y, z));
                    chat::show(&format!("📍 Tape Point 1: [{x}, {y}, {z}]"));
                }
                Some((dim, x1, y1, z1)) => {
                    if dim == event.dimension {
                        let res = calculate_measurement((x1, y1, z1), (x, y, z));
                        self.last_result = Some(res);
                        chat::show(&format!(
                            "📏 Distance: {:.2}m (Manhattan: {}m) | Box: {}x{}x{} ({} blocks, floor: {}m²)",
                            res.euclidean,
                            res.manhattan,
                            res.dx,
                            res.dy,
                            res.dz,
                            res.volume,
                            res.area_floor
                        ));
                    } else {
                        self.start_point = Some((event.dimension.clone(), x, y, z));
                        chat::show(&format!(
                            "📍 Reset Point 1 in {}: [{x}, {y}, {z}]",
                            event.dimension
                        ));
                    }
                }
            }
            return EventOutcome::Consume;
        }

        EventOutcome::Pass
    }

    fn command(&mut self, name: &str, args: &str) -> Result<Option<String>, String> {
        if name != "measurements" {
            return Err(format!("Unknown command /{name}"));
        }
        match args.trim() {
            "clear" | "reset" => {
                self.reset();
                Ok(Some("Tape measure anchor cleared.".into()))
            }
            "last" => {
                if let Some(res) = self.last_result {
                    Ok(Some(format!(
                        "Last measurement: {:.2}m distance, {}x{}x{} box ({} blocks, area {}m²)",
                        res.euclidean, res.dx, res.dy, res.dz, res.volume, res.area_floor
                    )))
                } else {
                    Ok(Some("No measurements taken yet.".into()))
                }
            }
            _ => Ok(Some(
                "Measurements: right-click blocks to measure distance & volume, right-click air to clear."
                    .into(),
            )),
        }
    }
}

horizon_mod_sdk::export_mod!(Measurements);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_measurement_single_block() {
        let res = calculate_measurement((10, 64, 10), (10, 64, 10));
        assert_eq!(res.dx, 1);
        assert_eq!(res.dy, 1);
        assert_eq!(res.dz, 1);
        assert_eq!(res.volume, 1);
        assert_eq!(res.area_floor, 1);
        assert_eq!(res.manhattan, 0);
        assert!((res.euclidean - 0.0).abs() < 1e-5);
    }

    #[test]
    fn test_calculate_measurement_cuboid() {
        // From (0, 0, 0) to (2, 3, 4) inclusive -> dx=3, dy=4, dz=5
        let res = calculate_measurement((0, 0, 0), (2, 3, 4));
        assert_eq!(res.dx, 3);
        assert_eq!(res.dy, 4);
        assert_eq!(res.dz, 5);
        assert_eq!(res.volume, 60);
        assert_eq!(res.area_floor, 15);
        assert_eq!(res.manhattan, 9);
    }

    #[test]
    fn test_calculate_measurement_euclidean_distance() {
        // 3D 3-4-0 -> distance 5
        let res = calculate_measurement((0, 0, 0), (3, 4, 0));
        assert!((res.euclidean - 5.0).abs() < 1e-5);
    }

    #[test]
    fn test_measurements_reset() {
        let mut m = Measurements::new();
        m.start_point = Some(("minecraft:overworld".into(), 10, 20, 30));
        assert!(m.start_point.is_some());
        m.reset();
        assert!(m.start_point.is_none());
    }
}

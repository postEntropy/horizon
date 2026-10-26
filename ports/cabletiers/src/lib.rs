//! Cable Tiers: elite, ultra, mega and creative versions of AE2's import,
//! export, construction, destruction, interface, pattern-provider and
//! autocrafting parts, in the pack's sixteen colours.
//!
//! The parts move items through an AE2 network, which the sandbox cannot join;
//! the port keeps each block's tier card — throughput and energy — on a screen,
//! plus a `/cabletiers` table.
use horizon_mod_sdk::{
    BlockAction, BlockEvent, EventOutcome, LogicHooks, Mod, Registration,
    ui::{self, ScreenSpec, Widget, WidgetKind},
};

/// `(tier, items per operation, FE per tick)`; 0 means unlimited.
const TIERS: [(&str, u64, u64); 4] = [
    ("elite", 8, 1_000),
    ("ultra", 32, 8_000),
    ("mega", 128, 64_000),
    ("creative", 0, 0),
];

const DEVICES: [(&str, &str); 7] = [
    ("autocrafter", "Autocrafter"),
    ("constructor", "Constructor"),
    ("destructor", "Destructor"),
    ("disk_interface", "Disk Interface"),
    ("exporter", "Exporter"),
    ("importer", "Importer"),
    ("interface", "Interface"),
];

const COLORS: [&str; 16] = [
    "white",
    "orange",
    "magenta",
    "light_blue",
    "yellow",
    "lime",
    "pink",
    "gray",
    "light_gray",
    "cyan",
    "purple",
    "blue",
    "brown",
    "green",
    "red",
    "black",
];

fn path_of(state: &str) -> &str {
    let id = state.split('[').next().unwrap_or(state);
    id.split(':').nth(1).unwrap_or(id)
}

fn tier_of(path: &str) -> Option<(&'static str, u64, u64)> {
    TIERS
        .iter()
        .copied()
        .find(|(tier, _, _)| path.split('_').any(|part| part == *tier))
}

fn device_of(path: &str) -> Option<&'static str> {
    DEVICES
        .iter()
        .find(|(device, _)| path.contains(device))
        .map(|(_, name)| *name)
}

fn label(id: u32, y: i32, text: String) -> Widget {
    Widget {
        id,
        kind: WidgetKind::Label,
        x: 8,
        y,
        width: 160,
        height: 10,
        text,
        value: 0,
        max_value: 0,
    }
}

fn tier_screen(tier: &str, items: u64, energy: u64, device: &str) -> ScreenSpec {
    let throughput = if items == 0 {
        "Throughput: unlimited".to_string()
    } else {
        format!("Throughput: {items} items per operation")
    };
    let power = if energy == 0 {
        "Energy: none needed".to_string()
    } else {
        format!("Energy: {energy} FE/t")
    };
    ScreenSpec {
        id: "screen:cabletiers:tier".into(),
        title: format!("{device} · {}", capitalize(tier)),
        width: 176,
        height: 120,
        widgets: vec![
            label(0, 20, format!("Tier: {}", capitalize(tier))),
            label(1, 34, throughput),
            label(2, 48, power),
            label(
                3,
                70,
                "Right-click any tiered part to read its card.".into(),
            ),
            Widget {
                id: 4,
                kind: WidgetKind::EnergyBar,
                x: 8,
                y: 88,
                width: 160,
                height: 10,
                text: "FE".into(),
                value: 0,
                max_value: if energy == 0 { 1 } else { energy },
            },
        ],
    }
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

struct CableTiers;

impl Mod for CableTiers {
    fn init() -> Result<(Self, Registration), String> {
        Ok((CableTiers, Registration::new().command("cabletiers")))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for (tier, _, _) in TIERS {
            for (device, _) in DEVICES {
                hooks = hooks.block(&format!("cabletiers:{tier}_{device}"));
                for color in COLORS {
                    hooks = hooks.block(&format!("cabletiers:{color}_{tier}_{device}"));
                }
            }
        }
        hooks
    }

    fn on_block(&mut self, event: BlockEvent) -> EventOutcome {
        if event.action != BlockAction::UseWithoutItem && event.action != BlockAction::UseItemOn {
            return EventOutcome::Pass;
        }
        let path = path_of(&event.state);
        let Some((tier, items, energy)) = tier_of(path) else {
            return EventOutcome::Pass;
        };
        let device = device_of(path).unwrap_or("Tiered Part");
        let _ = ui::open_screen(&tier_screen(tier, items, energy, device));
        EventOutcome::Consume
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "cabletiers" {
            return Err(format!("Unknown command /{name}"));
        }
        let table: Vec<String> = TIERS
            .iter()
            .map(|(tier, items, energy)| {
                if *items == 0 {
                    format!("{}: unlimited, no energy", capitalize(tier))
                } else {
                    format!(
                        "{}: {items} items/operation, {energy} FE/t",
                        capitalize(tier)
                    )
                }
            })
            .collect();
        Ok(Some(format!(
            "Cable Tiers — {} ({} colours). {}",
            table.join(" · "),
            COLORS.len(),
            DEVICES
                .iter()
                .map(|(_, name)| *name)
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

horizon_mod_sdk::export_mod!(CableTiers);

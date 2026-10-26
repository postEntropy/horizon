//! Ender Drives: AE2 storage cells whose contents follow a frequency across
//! dimensions, in five channel colours and six capacities.
//!
//! Storage cells hold their items in AE2's own cell data, so the port carries
//! each cell's capacity and channel and reports them.
use horizon_mod_sdk::{EventOutcome, ItemAction, ItemEvent, LogicHooks, Mod, Registration, chat};

const CHANNELS: [&str; 5] = ["default", "blue", "green", "red", "yellow"];

/// `(cell kind, capacities)` in bytes, AE2's own units.
const CAPACITIES: [(&str, u32); 6] = [
    ("1k", 1_024),
    ("4k", 4_096),
    ("16k", 16_384),
    ("64k", 65_536),
    ("256k", 262_144),
    ("creative", 0),
];

struct EnderDrives;

fn channel_of(path: &str) -> &'static str {
    CHANNELS
        .iter()
        .copied()
        .find(|channel| *channel != "default" && path.ends_with(&format!("_{channel}")))
        .unwrap_or("default")
}

fn capacity_of(path: &str) -> Option<&'static str> {
    CAPACITIES
        .iter()
        .find(|(kind, _)| path.contains(&format!("_{kind}")))
        .map(|(kind, _)| *kind)
}

fn storage_of(path: &str) -> &'static str {
    if path.starts_with("ender_fluid_disk") || path.contains("fluid_disk") {
        "fluid"
    } else {
        "items"
    }
}

fn describe(item: &str) -> Option<String> {
    let path = item.split([':', '[']).nth(1).unwrap_or(item);
    if !path.starts_with("ender_disk") && !path.starts_with("ender_fluid_disk") {
        return None;
    }
    let capacity = capacity_of(path)?;
    let channel = channel_of(path);
    let kind = storage_of(path);
    Some(if capacity == "creative" {
        format!(
            "💾 Ender {kind} disk: {capacity}, channel {channel} — limited only by the network."
        )
    } else {
        let bytes = CAPACITIES
            .iter()
            .find(|(name, _)| *name == capacity)
            .map(|(_, bytes)| *bytes)
            .unwrap_or(0);
        format!("💾 Ender {kind} disk {capacity} ({bytes} bytes), channel {channel}.")
    })
}

impl Mod for EnderDrives {
    fn init() -> Result<(Self, Registration), String> {
        Ok((EnderDrives, Registration::new().command("enderdrives")))
    }

    fn logic_hooks(&mut self) -> LogicHooks {
        let mut hooks = LogicHooks::new();
        for (kind, _) in CAPACITIES {
            hooks = hooks.item(&format!("enderdrives:ender_disk_{kind}"));
            hooks = hooks.item(&format!("enderdrives:ender_fluid_disk_{kind}"));
            for channel in &CHANNELS[1..] {
                hooks = hooks.item(&format!("enderdrives:ender_disk_{kind}_{channel}"));
                hooks = hooks.item(&format!("enderdrives:ender_fluid_disk_{kind}_{channel}"));
            }
        }
        hooks
    }

    fn on_item(&mut self, event: ItemEvent) -> EventOutcome {
        if !matches!(event.action, ItemAction::UseInAir | ItemAction::UseOnBlock) {
            return EventOutcome::Pass;
        }
        match describe(&event.item) {
            Some(text) => {
                chat::show(&text);
                EventOutcome::Consume
            }
            None => EventOutcome::Pass,
        }
    }

    fn command(&mut self, name: &str, _args: &str) -> Result<Option<String>, String> {
        if name != "enderdrives" {
            return Err(format!("Unknown command /{name}"));
        }
        let capacities: Vec<String> = CAPACITIES
            .iter()
            .map(|(kind, bytes)| {
                if *bytes == 0 {
                    "creative (unlimited)".to_string()
                } else {
                    format!("{kind} ({bytes} bytes)")
                }
            })
            .collect();
        Ok(Some(format!(
            "Ender Drives: item and fluid disks — {} · channels: {}.",
            capacities.join(", "),
            CHANNELS.join(", ")
        )))
    }
}

horizon_mod_sdk::export_mod!(EnderDrives);

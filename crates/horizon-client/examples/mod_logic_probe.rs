//! The Horizon side of a logic-mod behavior probe.
//!
//! cargo run -p horizon-client --features mods --example mod_logic_probe -- MOD.hmod SCENARIOS.json OUT.json
//!
//! Scenarios seed a small server-block snapshot, send one API 0.3 block event,
//! and record the server mutations the component requests.
use horizon_client::Result;
use horizon_mods::{BlockAction, BlockEvent, Mods, Output, ServerAction, Status};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn action(name: &str) -> Result<BlockAction> {
    match name {
        "broken" => Ok(BlockAction::Broken),
        "random_tick" => Ok(BlockAction::RandomTick),
        "scheduled_tick" => Ok(BlockAction::ScheduledTick),
        "use_without_item" => Ok(BlockAction::UseWithoutItem),
        "use_item_on" => Ok(BlockAction::UseItemOn),
        _ => Err(format!("unsupported block action {name:?}").into()),
    }
}

fn run(mods: &mut Mods, scenario: &Value) -> Result<Value> {
    let dimension = scenario["dimension"]
        .as_str()
        .unwrap_or("minecraft:overworld");
    let (x, y, z) = (
        scenario["x"].as_i64().unwrap_or(0) as i32,
        scenario["y"].as_i64().unwrap_or(64) as i32,
        scenario["z"].as_i64().unwrap_or(0) as i32,
    );
    for cell in scenario["world"].as_array().into_iter().flatten() {
        mods.set_world_block(
            dimension,
            cell["x"].as_i64().unwrap_or(0) as i32,
            cell["y"].as_i64().unwrap_or(64) as i32,
            cell["z"].as_i64().unwrap_or(0) as i32,
            cell["state"].as_str().unwrap_or("minecraft:air"),
        );
    }
    mods.tick(scenario["tick"].as_u64().unwrap_or(0));
    let event = BlockEvent {
        action: action(scenario["action"].as_str().unwrap_or("random_tick"))?,
        dimension: dimension.to_owned(),
        x,
        y,
        z,
        state: scenario["state"]
            .as_str()
            .ok_or("scenario without state")?
            .to_owned(),
        actor: String::new(),
        held_item: String::new(),
    };
    let outcome = mods.block_event_with_context(
        event,
        scenario["face"].as_str().unwrap_or_default().to_owned(),
        scenario["sneaking"].as_bool().unwrap_or(false),
    );
    let screen = mods.open_screens().next().map(|(_, spec)| {
        json!({
            "id":spec.id,
            "title":spec.title,
            "width":spec.width,
            "height":spec.height,
            "widgets":spec.widgets.iter().map(|widget| json!({"id":widget.id,"kind":format!("{:?}",widget.kind)})).collect::<Vec<_>>(),
        })
    });
    let actions: Vec<Value> = mods
        .drain()
        .into_iter()
        .filter_map(|output| match output {
            Output::Server(ServerAction::SetBlock {
                dimension,
                x,
                y,
                z,
                state,
            }) => Some(json!({
                "kind": "set_block", "dimension": dimension,
                "x": x, "y": y, "z": z, "state": state
            })),
            Output::Server(ServerAction::ScheduleRandomTick {
                dimension,
                x,
                y,
                z,
                block,
                delay,
            }) => Some(json!({
                "kind": "schedule_random_tick", "dimension": dimension,
                "x": x, "y": y, "z": z, "block": block, "delay": delay
            })),
            Output::Chat { name, text } => Some(json!({"kind":"chat", "name":name, "text":text})),
            _ => None,
        })
        .collect();
    if let Some(expected) = scenario.get("expect_action")
        && !actions.contains(expected)
    {
        return Err(format!(
            "scenario {}: expected action {expected}, got {}",
            scenario["name"].as_str().unwrap_or("unnamed"),
            serde_json::to_string(&actions)?
        )
        .into());
    }
    if let Some(expected) = scenario.get("expect_outcome").and_then(Value::as_str)
        && format!("{outcome:?}") != expected
    {
        return Err(format!(
            "scenario {}: expected outcome {expected}, got {outcome:?}",
            scenario["name"].as_str().unwrap_or("unnamed")
        )
        .into());
    }
    if let Some(expected) = scenario.get("expect_chat").and_then(Value::as_str)
        && !actions.iter().any(|action| action["text"] == expected)
    {
        return Err(format!(
            "scenario {}: expected chat {expected:?}, got {}",
            scenario["name"].as_str().unwrap_or("unnamed"),
            serde_json::to_string(&actions)?
        )
        .into());
    }
    if let Some(expected) = scenario.get("expect_screen_id").and_then(Value::as_str)
        && screen.as_ref().and_then(|value| value["id"].as_str()) != Some(expected)
    {
        return Err(format!(
            "scenario {}: expected screen {expected:?}, got {}",
            scenario["name"].as_str().unwrap_or("unnamed"),
            screen
                .as_ref()
                .map_or_else(|| "none".into(), Value::to_string)
        )
        .into());
    }
    Ok(json!({
        "name": scenario["name"],
        "outcome": format!("{outcome:?}"),
        "actions": actions,
        "screen": screen
    }))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [module, scenarios, out] = args.as_slice() else {
        return Err("usage: mod_logic_probe MOD.hmod SCENARIOS.json OUT.json".into());
    };
    let mut mods = Mods::load(&[Path::new(module).to_owned()], |_| true, |_| false);
    if let Some(entry) = mods.entries().first()
        && let Status::Failed(why) = &entry.status
    {
        return Err(format!("{module}: {why}").into());
    }
    let scenarios: Value = serde_json::from_slice(&fs::read(scenarios)?)?;
    let mut results = Vec::new();
    for scenario in scenarios.as_array().into_iter().flatten() {
        results.push(run(&mut mods, scenario)?);
    }
    fs::write(out, serde_json::to_vec_pretty(&results)?)?;
    Ok(())
}

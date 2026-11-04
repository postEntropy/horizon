use horizon_mod_sdk::{chat, config, server, storage, Mod, Registration};

struct FtbTeamsMod;

impl FtbTeamsMod {
    fn current_party() -> Option<String> {
        storage::get("current_party")
            .and_then(|b| String::from_utf8(b).ok())
            .filter(|s| !s.is_empty())
    }

    fn all_teams() -> Vec<String> {
        storage::get("teams_list")
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_default()
            .split(',')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect()
    }

    fn save_teams(teams: &[String]) {
        let joined = teams.join(",");
        let _ = storage::set("teams_list", joined.as_bytes());
        let _ = server::set_rule("ftbteams:teams", &joined);
    }
}

impl Mod for FtbTeamsMod {
    fn init() -> Result<(Self, Registration), String> {
        let party = Self::current_party().unwrap_or_else(|| "none".into());
        let _ = server::set_rule("ftbteams:active_party", &party);
        Ok((Self, Registration::new().command("ftbteams")))
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] | ["info"] | ["party", "info"] | ["status"] => {
                let Some(party) = Self::current_party() else {
                    return Ok(Some("You haven't created or joined a party!".into()));
                };
                let all = Self::all_teams();
                Ok(Some(format!(
                    "FTB Teams: party='{party}' total_teams={} friendly_fire={}",
                    all.len(),
                    config::get_bool("friendly_fire").unwrap_or(false)
                )))
            }
            ["list"] => {
                let all = Self::all_teams();
                Ok(Some(format!(
                    "FTB Teams ({}): [{}]",
                    all.len(),
                    all.join(", ")
                )))
            }
            ["party", "create", name] | ["create", name] => {
                if Self::current_party().is_some() {
                    return Err("You are already in a party!".into());
                }
                if name.len() < 3 {
                    return Err("Team name is too short! (must be >=3 characters)".into());
                }
                let mut all = Self::all_teams();
                if all.iter().any(|t| t == name) {
                    return Err(format!("Team {name} already exists!"));
                }
                all.push((*name).to_owned());
                Self::save_teams(&all);
                storage::set("current_party", name.as_bytes())?;
                let _ = server::set_rule("ftbteams:active_party", name);
                Ok(Some(format!("Created party '{name}'")))
            }
            ["party", "leave"] | ["leave"] => {
                let Some(party) = Self::current_party() else {
                    return Err("You haven't created or joined a party!".into());
                };
                storage::delete("current_party");
                let _ = server::set_rule("ftbteams:active_party", "none");
                Ok(Some(format!("Left party '{party}'")))
            }
            ["msg", rest @ ..] if !rest.is_empty() => {
                let Some(party) = Self::current_party() else {
                    return Err("You haven't created or joined a party!".into());
                };
                let text = rest.join(" ");
                chat::show(&format!("[Team {party}] {text}"));
                Ok(Some(format!("Sent message to party '{party}'")))
            }
            _ => Err(
                "Usage: /ftbteams [info | list | party create <name> | party leave | msg <text>]"
                    .into(),
            ),
        }
    }
}

horizon_mod_sdk::export_mod!(FtbTeamsMod);

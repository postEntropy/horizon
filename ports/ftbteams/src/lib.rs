use horizon_mod_sdk::{chat, config, game, storage, Mod, Registration};

#[derive(Default)]
struct Team {
    name: String,
    owner: String,
    members: Vec<Member>,
    invites: Vec<String>,
}

#[derive(Clone)]
struct Member {
    id: String,
    name: String,
}

struct FtbTeamsMod;

impl FtbTeamsMod {
    fn player() -> Result<game::CommandPlayer, String> {
        game::get_command_player().ok_or_else(|| "This command requires a player".into())
    }

    fn player_key(id: &str) -> String {
        format!("player_{}", id.chars().filter(|c| c.is_ascii_hexdigit()).collect::<String>().to_ascii_lowercase())
    }

    fn current_party(id: &str) -> Option<String> {
        storage::get(&Self::player_key(id))
            .and_then(|b| String::from_utf8(b).ok())
            .filter(|s| !s.is_empty())
    }

    fn teams() -> Vec<String> {
        storage::get("teams_list")
            .and_then(|b| String::from_utf8(b).ok())
            .unwrap_or_default()
            .lines()
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect()
    }

    fn team_key(name: &str) -> String {
        let slug: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        format!("team_{}", slug.to_ascii_lowercase())
    }

    fn read_team(name: &str) -> Option<Team> {
        let text = String::from_utf8(storage::get(&Self::team_key(name))?).ok()?;
        let mut lines = text.lines();
        let owner = lines.next()?.to_owned();
        let members = lines.next().unwrap_or("").split(';').filter_map(|m| {
            let (id, name) = m.split_once(':')?;
            Some(Member { id: id.to_owned(), name: name.to_owned() })
        }).collect();
        let invites = lines.next().unwrap_or("").split(';').filter(|s| !s.is_empty()).map(str::to_owned).collect();
        Some(Team { name: name.to_owned(), owner, members, invites })
    }

    fn save_team(team: &Team) -> Result<(), String> {
        let members = team.members.iter().map(|m| format!("{}:{}", m.id, m.name)).collect::<Vec<_>>().join(";");
        let invites = team.invites.join(";");
        let text = format!("{}\n{}\n{}", team.owner, members, invites);
        storage::set(&Self::team_key(&team.name), text.as_bytes())
    }

    fn add_team(name: &str) -> Result<(), String> {
        let mut teams = Self::teams();
        teams.push(name.to_owned());
        storage::set("teams_list", teams.join("\n").as_bytes())
    }

    fn valid_name(name: &str) -> bool {
        (3..=32).contains(&name.len()) && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    }

}

impl Mod for FtbTeamsMod {
    fn init() -> Result<(Self, Registration), String> {
        Ok((Self, Registration::new().command("ftbteams")))
    }

    fn command(&mut self, _name: &str, args: &str) -> Result<Option<String>, String> {
        let player = Self::player()?;
        let parts: Vec<&str> = args.split_whitespace().collect();
        match parts.as_slice() {
            [] | ["info"] | ["party", "info"] | ["status"] => {
                let Some(name) = Self::current_party(&player.id) else {
                    return Ok(Some("You haven't created or joined a party!".into()));
                };
                let team = Self::read_team(&name).ok_or("Party data is missing")?;
                Ok(Some(format!("Party '{}' owner={} members={}", team.name, team.owner, team.members.len())))
            }
            ["list"] => Ok(Some(format!("FTB Teams ({}): [{}]", Self::teams().len(), Self::teams().join(", ")))),
            ["members"] | ["party", "members"] => {
                let name = Self::current_party(&player.id).ok_or("You haven't created or joined a party!")?;
                let team = Self::read_team(&name).ok_or("Party data is missing")?;
                Ok(Some(format!("Party '{}': {}", name, team.members.iter().map(|m| m.name.as_str()).collect::<Vec<_>>().join(", "))))
            }
            ["create", name] | ["party", "create", name] => {
                if Self::current_party(&player.id).is_some() { return Err("You are already in a party!".into()); }
                if !Self::valid_name(name) { return Err("Team name must be 3-32 letters, numbers, or underscores".into()); }
                if Self::teams().iter().any(|t| t.eq_ignore_ascii_case(name)) { return Err(format!("Team {name} already exists!")); }
                let team = Team { name: (*name).into(), owner: player.id.clone(), members: vec![Member { id: player.id.clone(), name: player.name.clone() }], invites: Vec::new() };
                Self::save_team(&team)?;
                Self::add_team(name)?;
                storage::set(&Self::player_key(&player.id), name.as_bytes())?;
                Ok(Some(format!("Created party '{name}'")))
            }
            ["invite", id] | ["party", "invite", id] => {
                let name = Self::current_party(&player.id).ok_or("You haven't created or joined a party!")?;
                let mut team = Self::read_team(&name).ok_or("Party data is missing")?;
                if team.owner != player.id { return Err("Only the party owner can invite players".into()); }
                if !id.contains('-') || id.len() != 36 { return Err("Invite target must be a player UUID".into()); }
                if team.invites.iter().any(|i| i == id) { return Err("That player already has an invite".into()); }
                team.invites.push((*id).to_owned());
                Self::save_team(&team)?;
                Ok(Some(format!("Invited player {id} to '{name}'")))
            }
            ["accept", name] | ["party", "accept", name] => {
                if Self::current_party(&player.id).is_some() { return Err("You are already in a party!".into()); }
                let mut team = Self::read_team(name).ok_or("No invitation found for that party")?;
                if !team.invites.iter().any(|id| id.eq_ignore_ascii_case(&player.id)) { return Err("No invitation found for that party".into()); }
                let max = config::get_int("max_party_size").unwrap_or(8).max(2) as usize;
                if team.members.len() >= max { return Err("Party is full".into()); }
                team.invites.retain(|id| !id.eq_ignore_ascii_case(&player.id));
                team.members.push(Member { id: player.id.clone(), name: player.name.clone() });
                Self::save_team(&team)?;
                storage::set(&Self::player_key(&player.id), name.as_bytes())?;
                Ok(Some(format!("Joined party '{name}'")))
            }
            ["leave"] | ["party", "leave"] => {
                let name = Self::current_party(&player.id).ok_or("You haven't created or joined a party!")?;
                let mut team = Self::read_team(&name).ok_or("Party data is missing")?;
                team.members.retain(|m| m.id != player.id);
                storage::delete(&Self::player_key(&player.id));
                if team.members.is_empty() {
                    storage::delete(&Self::team_key(&name));
                    storage::set("teams_list", Self::teams().into_iter().filter(|t| t != &name).collect::<Vec<_>>().join("\n").as_bytes())?;
                    Ok(Some(format!("Disbanded party '{name}'")))
                } else {
                    if team.owner == player.id { team.owner = team.members[0].id.clone(); }
                    Self::save_team(&team)?;
                    Ok(Some(format!("Left party '{name}'")))
                }
            }
            ["msg", rest @ ..] if !rest.is_empty() => {
                let party = Self::current_party(&player.id).ok_or("You haven't created or joined a party!")?;
                chat::show(&format!("[Team {party}] {}: {}", player.name, rest.join(" ")));
                Ok(Some("Party message shown locally".into()))
            }
            _ => Err("Usage: /ftbteams [info | list | create <name> | members | invite <uuid> | accept <name> | leave | msg <text>]".into()),
        }
    }
}

horizon_mod_sdk::export_mod!(FtbTeamsMod);

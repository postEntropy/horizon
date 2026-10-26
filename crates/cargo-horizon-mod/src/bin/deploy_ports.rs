//! Deploys compiled ports from `ports/target/wasm32-unknown-unknown/release/`
//! into a modpack work directory (`carry/mod.wasm` and `carry/horizon-mod.toml`).
use horizon_mods::manifest::{Capability, component_imports, componentize};
use std::path::{Path, PathBuf};

fn api_rank(api: &str) -> u8 {
    match api {
        "0.1" => 1,
        "0.2" => 2,
        "0.3" => 3,
        "0.4" => 4,
        "0.5" => 5,
        _ => 0,
    }
}

/// The manifest with its `api` line set to `api`.
fn set_api(text: &str, api: &str) -> String {
    if let Some(start) = text.find("api = \"") {
        let value_start = start + "api = \"".len();
        if let Some(end) = text[value_start..].find('"') {
            let mut out = text.to_string();
            out.replace_range(value_start..value_start + end, api);
            return out;
        }
    }
    format!("{text}\napi = \"{api}\"\n")
}

/// Capabilities a port needs that none of its component's imports reveal.
const EXTRA_CAPABILITIES: &[(&str, &str)] = &[
    ("advanced_ae", "block-entities"),
    ("apothic_enchanting", "block-entities"),
    ("appflux", "block-entities"),
    ("ars_technica", "block-entities"),
    ("bibliocraft", "block-entities"),
    ("bigreactors", "block-entities"),
    ("botanypots", "block-entities"),
    ("dimstorage", "block-entities"),
    ("disenchanting_table", "block-entities"),
    ("enderstorage", "block-entities"),
    ("expandedae", "block-entities"),
    ("extended_industrialization", "block-entities"),
    ("extendedae", "block-entities"),
    ("extradisks", "block-entities"),
    ("extrastorage", "block-entities"),
    ("farmersdelight", "block-entities"),
    ("farmingforblockheads", "block-entities"),
    ("fluxnetworks", "block-entities"),
    ("ftbquests", "block-entities"),
    ("generatorgalore", "block-entities"),
    ("hostilenetworks", "block-entities"),
    ("industrialforegoing", "block-entities"),
    ("industrialforegoingsouls", "block-entities"),
    ("industrialization_overdrive", "block-entities"),
    ("ironfurnaces", "block-entities"),
    ("mob_grinding_utils", "block-entities"),
    ("modularrouters", "block-entities"),
    ("mysticalagriculture", "block-entities"),
    ("powah", "block-entities"),
    ("productivemetalworks", "block-entities"),
    // Batch 10
    ("irons_jewelry", "block-entities"),
    ("laserio", "block-entities"),
    ("lootr", "block-entities"),
    ("mcwfurnitures", "block-entities"),
    ("megacells", "block-entities"),
    ("mekmm", "block-entities"),
    ("merequester", "block-entities"),
    // Batch 12
    ("pylons", "block-entities"),
    ("quarryplus", "block-entities"),
    ("rangedpumps", "block-entities"),
    ("refinedtypes", "block-entities"),
    ("rftoolsbase", "block-entities"),
    ("rftoolspower", "block-entities"),
    ("rftoolsstorage", "block-entities"),
    ("rftoolsutility", "block-entities"),
    // Batch 13
    ("sawmill", "block-entities"),
    ("sophisticatedstorage", "block-entities"),
    ("stepcrafter", "block-entities"),
    ("sushigocrafting", "block-entities"),
    ("trashcans", "block-entities"),
    ("treetap", "block-entities"),
    ("trophymanager", "block-entities"),
    // Batch 14
    ("waystones", "block-entities"),
    ("wirelesschargers", "block-entities"),
    ("xnet", "block-entities"),
    ("xycraft_machines", "block-entities"),
    ("ad_astra", "block-entities"),
    ("ae2", "block-entities"),
    ("aether", "block-entities"),
    ("allthemodium", "block-entities"),
    ("apotheosis", "block-entities"),
    // Batch 15
    ("ars_elemental", "block-entities"),
    ("ars_nouveau", "block-entities"),
    ("astralsorcery", "block-entities"),
    ("buildinggadgets2", "block-entities"),
    ("cataclysm", "block-entities"),
    // Batch 16
    ("compactmachines", "block-entities"),
    ("computercraft", "block-entities"),
    ("copycats", "block-entities"),
    ("create", "block-entities"),
    ("create_aquatic_ambitions", "block-entities"),
    ("create_dragons_plus", "block-entities"),
    ("create_enchantment_industry", "block-entities"),
    ("create_hypertube", "block-entities"),
    ("createaddition", "block-entities"),
    // Batch 17
    ("deeperdarker", "block-entities"),
    ("domum_ornamentum", "block-entities"),
    ("draconicevolution", "block-entities"),
    ("dysoncubeproject", "block-entities"),
    ("easy_villagers", "block-entities"),
    ("enderio", "block-entities"),
    ("eternal_starlight", "block-entities"),
    // Batch 18
    ("evilcraft", "block-entities"),
    ("forbidden_arcanus", "block-entities"),
    ("framedblocks", "block-entities"),
    ("gateways", "block-entities"),
    ("iceandfire", "block-entities"),
    ("immersiveengineering", "block-entities"),
    ("integratedcrafting", "block-entities"),
    ("integrateddynamics", "block-entities"),
    // Batch 19
    ("integratedscripting", "block-entities"),
    ("integratedterminals", "block-entities"),
    ("integratedtunnels", "block-entities"),
    ("irons_spellbooks", "block-entities"),
    ("justdirethings", "block-entities"),
    ("little_big_redstone", "block-entities"),
    ("mahoutsukai", "block-entities"),
    ("mekanism", "block-entities"),
    ("mekanismgenerators", "block-entities"),
    // Batch 20
    ("mffs", "block-entities"),
    ("minecolonies", "block-entities"),
    ("mininggadgets", "block-entities"),
    ("modern_industrialization", "block-entities"),
    ("naturesaura", "block-entities"),
    ("neovitae", "block-entities"),
    ("occultism", "block-entities"),
    ("oritech", "block-entities"),
    ("pneumaticcraft", "block-entities"),
    ("productivebees", "block-entities"),
    // Batch 21
    ("railcraft", "block-entities"),
    ("refinedstorage", "block-entities"),
    ("refurbished_furniture", "block-entities"),
    ("reliquary", "block-entities"),
    ("rftoolsbuilder", "block-entities"),
    ("rootsclassic", "block-entities"),
    ("securitycraft", "block-entities"),
    ("sfm", "block-entities"),
    // Batch 22
    ("silentgear", "block-entities"),
    ("sophisticatedbackpacks", "block-entities"),
    ("stevescarts", "block-entities"),
    ("supplementaries", "block-entities"),
    ("tempad", "block-entities"),
    ("the_bumblezone", "block-entities"),
    ("theurgy", "block-entities"),
    ("tombstone", "block-entities"),
    ("twilightforest", "block-entities"),
    ("undergarden", "block-entities"),
    // Batch 23
    ("chipped", "block-entities"),
    // Batch 24
    ("herbsandharvest", "block-entities"),
];

const PORTS: &[(&str, &str)] = &[
    // Batch 1
    ("jumpboat", "jumpboat_port.wasm"),
    ("accelerateddecay", "accelerateddecay_port.wasm"),
    ("bridgingmod", "bridgingmod_port.wasm"),
    ("crafting_on_a_stick", "crafting_on_a_stick_port.wasm"),
    ("elevatorid", "elevatorid_port.wasm"),
    (
        "repeatable_trial_vaults",
        "repeatable_trial_vaults_port.wasm",
    ),
    ("moredragoneggs", "moredragoneggs_port.wasm"),
    ("heyberryshutup", "heyberryshutup_port.wasm"),
    ("blueflame", "blueflame_port.wasm"),
    ("torchmaster", "torchmaster_port.wasm"),
    // Batch 2
    ("tiab", "tiab_port.wasm"),
    ("simplemagnets", "simplemagnets_port.wasm"),
    ("akashictome", "akashictome_port.wasm"),
    ("measurements", "measurements_port.wasm"),
    ("allthetweaks", "allthetweaks_port.wasm"),
    ("solcarrot", "solcarrot_port.wasm"),
    ("bhc", "bhc_port.wasm"),
    ("ftbultimine", "ftbultimine_port.wasm"),
    ("restrictions", "restrictions_port.wasm"),
    ("aeinfinitybooster", "aeinfinitybooster_port.wasm"),
    // Batch 3
    ("allthearcanistgear", "allthearcanistgear_port.wasm"),
    ("allthewizardgear", "allthewizardgear_port.wasm"),
    ("apothic_spawners", "apothic_spawners_port.wasm"),
    ("ars_additions", "ars_additions_port.wasm"),
    ("ars_controle", "ars_controle_port.wasm"),
    ("ars_elemancy", "ars_elemancy_port.wasm"),
    ("ars_ocultas", "ars_ocultas_port.wasm"),
    ("ars_polymorphia", "ars_polymorphia_port.wasm"),
    ("comforts", "comforts_port.wasm"),
    ("constructionstick", "constructionstick_port.wasm"),
    // Batch 4
    ("dyenamics", "dyenamics_port.wasm"),
    ("fuelgoeshere", "fuelgoeshere_port.wasm"),
    ("imfast", "imfast_port.wasm"),
    ("ironjetpacks", "ironjetpacks_port.wasm"),
    ("mcwlights", "mcwlights_port.wasm"),
    ("mcwwindows", "mcwwindows_port.wasm"),
    ("merrymaking", "merrymaking_port.wasm"),
    ("morered", "morered_port.wasm"),
    ("multipiston", "multipiston_port.wasm"),
    ("mysticalagradditions", "mysticalagradditions_port.wasm"),
    // Batch 5
    ("not_enough_glyphs", "not_enough_glyphs_port.wasm"),
    ("productivetrees", "productivetrees_port.wasm"),
    ("redstonepen", "redstonepen_port.wasm"),
    (
        "refinedstorage_quartz_arsenal",
        "refinedstorage_quartz_arsenal_port.wasm",
    ),
    ("silentgems", "silentgems_port.wasm"),
    ("starbunclemania", "starbunclemania_port.wasm"),
    ("utilitarian", "utilitarian_port.wasm"),
    ("wstweaks", "wstweaks_port.wasm"),
    ("xycraft_world", "xycraft_world_port.wasm"),
    ("actuallyadditions", "actuallyadditions_port.wasm"),
    // Batch 6
    ("advanced_ae", "advanced_ae_port.wasm"),
    ("ae2importexportcard", "ae2importexportcard_port.wasm"),
    ("ae2netanalyser", "ae2netanalyser_port.wasm"),
    ("ae2wtlib", "ae2wtlib_port.wasm"),
    ("apothic_enchanting", "apothic_enchanting_port.wasm"),
    ("appflux", "appflux_port.wasm"),
    ("ars_technica", "ars_technica_port.wasm"),
    ("bibliocraft", "bibliocraft_port.wasm"),
    ("bigreactors", "bigreactors_port.wasm"),
    ("botanypots", "botanypots_port.wasm"),
    // Batch 7
    ("cabletiers", "cabletiers_port.wasm"),
    ("charginggadgets", "charginggadgets_port.wasm"),
    ("chisel", "chisel_port.wasm"),
    ("cookingforblockheads", "cookingforblockheads_port.wasm"),
    ("cosmeticarmorreworked", "cosmeticarmorreworked_port.wasm"),
    ("dimstorage", "dimstorage_port.wasm"),
    ("disenchanting_table", "disenchanting_table_port.wasm"),
    ("enderdrives", "enderdrives_port.wasm"),
    ("enderstorage", "enderstorage_port.wasm"),
    ("energymeter", "energymeter_port.wasm"),
    // Batch 8
    ("entangled", "entangled_port.wasm"),
    ("expandedae", "expandedae_port.wasm"),
    ("explorerscompass", "explorerscompass_port.wasm"),
    (
        "extended_industrialization",
        "extended_industrialization_port.wasm",
    ),
    ("extendedae", "extendedae_port.wasm"),
    ("extradisks", "extradisks_port.wasm"),
    ("extrastorage", "extrastorage_port.wasm"),
    ("farmersdelight", "farmersdelight_port.wasm"),
    ("farmingforblockheads", "farmingforblockheads_port.wasm"),
    ("fluxnetworks", "fluxnetworks_port.wasm"),
    // Batch 9
    ("ftbquests", "ftbquests_port.wasm"),
    ("functionalstorage", "functionalstorage_port.wasm"),
    ("generatorgalore", "generatorgalore_port.wasm"),
    ("handcrafted", "handcrafted_port.wasm"),
    ("hostilenetworks", "hostilenetworks_port.wasm"),
    ("industrialforegoing", "industrialforegoing_port.wasm"),
    (
        "industrialforegoingsouls",
        "industrialforegoingsouls_port.wasm",
    ),
    (
        "industrialization_overdrive",
        "industrialization_overdrive_port.wasm",
    ),
    (
        "interdimensionalwirelesstransmitter",
        "interdimensionalwirelesstransmitter_port.wasm",
    ),
    ("ironfurnaces", "ironfurnaces_port.wasm"),
    // Batch 11
    ("mob_grinding_utils", "mob_grinding_utils_port.wasm"),
    ("moderndynamics", "moderndynamics_port.wasm"),
    ("modularbees", "modularbees_port.wasm"),
    ("modularrouters", "modularrouters_port.wasm"),
    ("mysticalagriculture", "mysticalagriculture_port.wasm"),
    ("naturescompass", "naturescompass_port.wasm"),
    ("pipez", "pipez_port.wasm"),
    ("pocketstorage", "pocketstorage_port.wasm"),
    ("powah", "powah_port.wasm"),
    ("productivemetalworks", "productivemetalworks_port.wasm"),
    // Batch 10
    ("irons_jewelry", "irons_jewelry_port.wasm"),
    ("itemcollectors", "itemcollectors_port.wasm"),
    ("laserbridges", "laserbridges_port.wasm"),
    ("laserio", "laserio_port.wasm"),
    ("lootr", "lootr_port.wasm"),
    ("mcwfurnitures", "mcwfurnitures_port.wasm"),
    ("megacells", "megacells_port.wasm"),
    ("mekmm", "mekmm_port.wasm"),
    ("merequester", "merequester_port.wasm"),
    ("mifa", "mifa_port.wasm"),
    // Batch 12
    ("pylons", "pylons_port.wasm"),
    ("quarryplus", "quarryplus_port.wasm"),
    ("rangedpumps", "rangedpumps_port.wasm"),
    ("rechiseled", "rechiseled_port.wasm"),
    ("refinedtypes", "refinedtypes_port.wasm"),
    ("rftoolsbase", "rftoolsbase_port.wasm"),
    ("rftoolspower", "rftoolspower_port.wasm"),
    ("rftoolsstorage", "rftoolsstorage_port.wasm"),
    ("rftoolsutility", "rftoolsutility_port.wasm"),
    ("rsinsertexportupgrade", "rsinsertexportupgrade_port.wasm"),
    // Batch 13
    ("sawmill", "sawmill_port.wasm"),
    ("sophisticatedstorage", "sophisticatedstorage_port.wasm"),
    ("stepcrafter", "stepcrafter_port.wasm"),
    ("sushigocrafting", "sushigocrafting_port.wasm"),
    ("toolbelt", "toolbelt_port.wasm"),
    ("transfer_labels", "transfer_labels_port.wasm"),
    ("trashcans", "trashcans_port.wasm"),
    ("treetap", "treetap_port.wasm"),
    ("trophymanager", "trophymanager_port.wasm"),
    ("universalgrid", "universalgrid_port.wasm"),
    // Batch 14
    ("waystones", "waystones_port.wasm"),
    ("wirelesschargers", "wirelesschargers_port.wasm"),
    ("xnet", "xnet_port.wasm"),
    ("xycraft_machines", "xycraft_machines_port.wasm"),
    ("ad_astra", "ad_astra_port.wasm"),
    ("ae2", "ae2_port.wasm"),
    ("aether", "aether_port.wasm"),
    ("allthemodium", "allthemodium_port.wasm"),
    ("amendments", "amendments_port.wasm"),
    ("apotheosis", "apotheosis_port.wasm"),
    // Batch 15
    ("aquaculture", "aquaculture_port.wasm"),
    ("ars_elemental", "ars_elemental_port.wasm"),
    ("ars_nouveau", "ars_nouveau_port.wasm"),
    ("artifacts", "artifacts_port.wasm"),
    ("astralsorcery", "astralsorcery_port.wasm"),
    ("auroral", "auroral_port.wasm"),
    ("bellsandwhistles", "bellsandwhistles_port.wasm"),
    ("biomeswevegone", "biomeswevegone_port.wasm"),
    ("buildinggadgets2", "buildinggadgets2_port.wasm"),
    ("cataclysm", "cataclysm_port.wasm"),
    // Batch 16
    ("compactmachines", "compactmachines_port.wasm"),
    ("computercraft", "computercraft_port.wasm"),
    ("copycats", "copycats_port.wasm"),
    ("create", "create_port.wasm"),
    (
        "create_aquatic_ambitions",
        "create_aquatic_ambitions_port.wasm",
    ),
    ("create_dragons_plus", "create_dragons_plus_port.wasm"),
    (
        "create_enchantment_industry",
        "create_enchantment_industry_port.wasm",
    ),
    ("create_hypertube", "create_hypertube_port.wasm"),
    ("createaddition", "createaddition_port.wasm"),
    ("creeperoverhaul", "creeperoverhaul_port.wasm"),
    // Batch 17
    ("deeperdarker", "deeperdarker_port.wasm"),
    ("domum_ornamentum", "domum_ornamentum_port.wasm"),
    ("draconicevolution", "draconicevolution_port.wasm"),
    ("dummmmmmy", "dummmmmmy_port.wasm"),
    ("dungeoncrawl", "dungeoncrawl_port.wasm"),
    ("dysoncubeproject", "dysoncubeproject_port.wasm"),
    ("easy_villagers", "easy_villagers_port.wasm"),
    ("enderio", "enderio_port.wasm"),
    ("endermanoverhaul", "endermanoverhaul_port.wasm"),
    ("eternal_starlight", "eternal_starlight_port.wasm"),
    // Batch 18
    ("evilcraft", "evilcraft_port.wasm"),
    ("fireproofboats", "fireproofboats_port.wasm"),
    ("forbidden_arcanus", "forbidden_arcanus_port.wasm"),
    ("framedblocks", "framedblocks_port.wasm"),
    ("gateways", "gateways_port.wasm"),
    ("iceandfire", "iceandfire_port.wasm"),
    ("illagerwarship", "illagerwarship_port.wasm"),
    ("immersiveengineering", "immersiveengineering_port.wasm"),
    ("integratedcrafting", "integratedcrafting_port.wasm"),
    ("integrateddynamics", "integrateddynamics_port.wasm"),
    // Batch 19
    ("integratedscripting", "integratedscripting_port.wasm"),
    ("integratedterminals", "integratedterminals_port.wasm"),
    ("integratedtunnels", "integratedtunnels_port.wasm"),
    ("irons_spellbooks", "irons_spellbooks_port.wasm"),
    ("justdirethings", "justdirethings_port.wasm"),
    ("little_big_redstone", "little_big_redstone_port.wasm"),
    ("livingthings", "livingthings_port.wasm"),
    ("mahoutsukai", "mahoutsukai_port.wasm"),
    ("mekanism", "mekanism_port.wasm"),
    ("mekanismgenerators", "mekanismgenerators_port.wasm"),
    // Batch 20
    ("mffs", "mffs_port.wasm"),
    ("minecolonies", "minecolonies_port.wasm"),
    ("mininggadgets", "mininggadgets_port.wasm"),
    (
        "modern_industrialization",
        "modern_industrialization_port.wasm",
    ),
    ("naturesaura", "naturesaura_port.wasm"),
    ("neovitae", "neovitae_port.wasm"),
    ("occultism", "occultism_port.wasm"),
    ("oritech", "oritech_port.wasm"),
    ("pneumaticcraft", "pneumaticcraft_port.wasm"),
    ("productivebees", "productivebees_port.wasm"),
    // Batch 21
    ("railcraft", "railcraft_port.wasm"),
    ("refinedstorage", "refinedstorage_port.wasm"),
    ("refurbished_furniture", "refurbished_furniture_port.wasm"),
    ("relics", "relics_port.wasm"),
    ("reliquary", "reliquary_port.wasm"),
    ("rftoolsbuilder", "rftoolsbuilder_port.wasm"),
    ("rootsclassic", "rootsclassic_port.wasm"),
    ("securitycraft", "securitycraft_port.wasm"),
    ("sfm", "sfm_port.wasm"),
    ("shrink", "shrink_port.wasm"),
    // Batch 22
    ("silentgear", "silentgear_port.wasm"),
    ("sophisticatedbackpacks", "sophisticatedbackpacks_port.wasm"),
    ("stevescarts", "stevescarts_port.wasm"),
    ("supplementaries", "supplementaries_port.wasm"),
    ("tempad", "tempad_port.wasm"),
    ("the_bumblezone", "the_bumblezone_port.wasm"),
    ("theurgy", "theurgy_port.wasm"),
    ("tombstone", "tombstone_port.wasm"),
    ("twilightforest", "twilightforest_port.wasm"),
    ("undergarden", "undergarden_port.wasm"),
    // Batch 23
    ("variantsandventures", "variantsandventures_port.wasm"),
    ("betterdeserttemples", "betterdeserttemples_port.wasm"),
    ("betterdungeons", "betterdungeons_port.wasm"),
    ("betterfortresses", "betterfortresses_port.wasm"),
    ("betterjungletemples", "betterjungletemples_port.wasm"),
    ("bettermineshafts", "bettermineshafts_port.wasm"),
    ("betterstrongholds", "betterstrongholds_port.wasm"),
    ("betterwitchhuts", "betterwitchhuts_port.wasm"),
    ("chipped", "chipped_port.wasm"),
    ("crystalix", "crystalix_port.wasm"),
    // Batch 24
    ("ctov", "ctov_port.wasm"),
    ("everythingcopper", "everythingcopper_port.wasm"),
    ("factory_blocks", "factory_blocks_port.wasm"),
    ("formationsnether", "formationsnether_port.wasm"),
    ("formationsoverworld", "formationsoverworld_port.wasm"),
    ("glassential", "glassential_port.wasm"),
    ("herbsandharvest", "herbsandharvest_port.wasm"),
    ("mcwbridges", "mcwbridges_port.wasm"),
    ("mcwdoors", "mcwdoors_port.wasm"),
    ("mcwfences", "mcwfences_port.wasm"),
    // Batch 25
    ("mcwholidays", "mcwholidays_port.wasm"),
    ("mcwpaths", "mcwpaths_port.wasm"),
    ("mcwroofs", "mcwroofs_port.wasm"),
    ("mcwstairs", "mcwstairs_port.wasm"),
    ("mns", "mns_port.wasm"),
    ("mostructures", "mostructures_port.wasm"),
    ("regions_unexplored", "regions_unexplored_port.wasm"),
    ("repurposed_structures", "repurposed_structures_port.wasm"),
    ("simplylight", "simplylight_port.wasm"),
    ("villagesandpillages", "villagesandpillages_port.wasm"),
];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let work_arg = args.next();
    let work = match work_arg {
        Some(w) => PathBuf::from(w),
        None => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".local/share/horizon/ports/all-the-mods-10-atm10")
        }
    };

    println!("Deploying ports to {}", work.display());
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap();
    let wasm_dir = repo_root.join("ports/target/wasm32-unknown-unknown/release");

    let mut deployed = 0;
    for (mod_id, wasm_file) in PORTS {
        let mut core_path = wasm_dir.join(wasm_file);
        if !core_path.is_file() {
            let alt = wasm_dir.join(format!("{mod_id}.wasm"));
            if alt.is_file() {
                core_path = alt;
            } else {
                eprintln!("Skipping {mod_id}: {} not found", core_path.display());
                continue;
            }
        }

        let core_bytes = std::fs::read(&core_path)?;
        let component = componentize(&core_bytes)?;
        let imports = component_imports(&component)?;

        let carry_dir = work.join("mods").join(mod_id).join("carry");
        std::fs::create_dir_all(&carry_dir)?;

        let wasm_out = carry_dir.join("mod.wasm");
        std::fs::write(&wasm_out, &component)?;

        let manifest_path = carry_dir.join("horizon-mod.toml");
        let mut manifest_text = if manifest_path.is_file() {
            std::fs::read_to_string(&manifest_path)?
        } else {
            format!(
                "id = \"{mod_id}\"\nname = \"{mod_id}\"\nversion = \"1.0.0\"\napi = \"0.3\"\nside = \"both\"\ncapabilities = []\n"
            )
        };

        // Update the API line to the newest one the component's imports need.
        // A component that imports `machine` has block entities, which the
        // manifest declares as the `block-entities` capability.
        let mut caps: Vec<String> = vec!["registries".to_string()];
        let mut api = "0.2".to_string();
        for import in &imports {
            match Capability::of_import(import) {
                Ok(Some(cap)) => {
                    let name = cap.to_string();
                    if !caps.contains(&name) {
                        caps.push(name);
                    }
                    if api_rank(cap.min_api()) > api_rank(&api) {
                        api = cap.min_api().to_string();
                    }
                }
                Ok(None) => {}
                Err(error) => eprintln!("  ! {mod_id}: {error}"),
            }
        }
        // Exports carry capabilities no import shows: every component exports
        // `block-entities` and `mod-entities`, so the port source is what says
        // whether it declares types. These mods keep item slots but never read
        // them through `machine`, so nothing else would reveal their block
        // entities.
        for (id, capability) in EXTRA_CAPABILITIES {
            if *id != *mod_id || caps.iter().any(|c| c == capability) {
                continue;
            }
            caps.push((*capability).to_string());
            let min_api = match *capability {
                "entities" | "models" | "dimensions" | "vanilla-hooks" => "0.5",
                _ => "0.4",
            };
            if api_rank(min_api) > api_rank(&api) {
                api = min_api.to_string();
            }
        }
        manifest_text = set_api(&manifest_text, &api);

        let caps_toml = format!(
            "capabilities = [{}]",
            caps.iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );

        if manifest_text.contains("capabilities = [") {
            if let Some(start) = manifest_text.find("capabilities = [") {
                if let Some(end) = manifest_text[start..].find(']') {
                    manifest_text.replace_range(start..start + end + 1, &caps_toml);
                }
            }
        } else {
            manifest_text.push_str(&format!("\n{caps_toml}\n"));
        }

        std::fs::write(&manifest_path, &manifest_text)?;
        println!(
            "  ✓ Deployed {mod_id} -> {} ({} bytes, capabilities: {caps:?})",
            wasm_out.display(),
            component.len()
        );
        deployed += 1;
    }

    println!("\nSuccessfully deployed {deployed} ports! Refreshing modpack report...");

    let status = std::process::Command::new("cargo")
        .args([
            "run",
            "-p",
            "horizon-port-pack",
            "--",
            "report",
            work.to_str().unwrap(),
        ])
        .status()?;

    if status.success() {
        println!("Coverage report updated successfully.");
    }

    Ok(())
}

//! Default result properties observed from the original mod via vanilla public
//! APIs; see farmersdelight-original-facts-validation.json for hashes/provenance.
//! Explicit container overrides from the shipped recipe data are included.
//! Unknown results are refused until their serving metadata is available.
pub fn result_facts(id: &str) -> Option<(u32, Option<&'static str>)> {
    match id {
        "farmersdelight:apple_cider" => Some((16, Some("minecraft:glass_bottle"))),
        "farmersdelight:baked_cod_stew" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:beef_stew" => Some((16, Some("minecraft:bowl"))),
        "minecraft:beetroot_soup" => Some((1, Some("minecraft:bowl"))),
        "farmersdelight:bone_broth" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:cabbage_rolls" => Some((64, None)),
        "farmersdelight:chicken_soup" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:cooked_rice" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:dog_food" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:dumplings" => Some((64, None)),
        "farmersdelight:fish_stew" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:fried_rice" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:glow_berry_custard" => Some((16, Some("minecraft:glass_bottle"))),
        "farmersdelight:hot_cocoa" => Some((16, Some("minecraft:glass_bottle"))),
        "farmersdelight:mushroom_rice" => Some((16, Some("minecraft:bowl"))),
        "minecraft:mushroom_stew" => Some((1, Some("minecraft:bowl"))),
        "farmersdelight:noodle_soup" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:onion_soup" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:pasta_with_meatballs" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:pasta_with_mutton_chop" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:pumpkin_soup" => Some((16, Some("minecraft:bowl"))),
        "minecraft:rabbit_stew" => Some((1, Some("minecraft:bowl"))),
        "farmersdelight:ratatouille" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:squid_ink_pasta" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:stuffed_pumpkin_block" => Some((1, Some("minecraft:pumpkin"))),
        "farmersdelight:tomato_sauce" => Some((64, Some("minecraft:bowl"))),
        "farmersdelight:vegetable_noodles" => Some((16, Some("minecraft:bowl"))),
        "farmersdelight:vegetable_soup" => Some((16, Some("minecraft:bowl"))),
        _ => None,
    }
}

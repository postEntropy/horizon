package dev.horizonlab.neoforge;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.google.gson.stream.JsonWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.MinecraftServer;
import net.neoforged.neoforge.common.util.FakePlayerFactory;
import net.minecraft.world.effect.MobEffectInstance;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.entity.animal.Wolf;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;

/** Black-box trace of registered food effects using public game APIs only. */
final class FoodTrace {
    static void run(MinecraftServer server, Path input, JsonWriter json) throws IOException {
        JsonArray scenarios = JsonParser.parseString(Files.readString(input)).getAsJsonArray();
        json.beginObject();
        json.name("schema").value(1);
        json.name("scenarios").beginArray();
        for (JsonElement element : scenarios) {
            JsonObject scenario = element.getAsJsonObject();
            String name = scenario.get("name").getAsString();
            String effect = scenario.get("effect").getAsString();
            int duration = scenario.get("duration").getAsInt();
            int ticks = scenario.get("ticks").getAsInt();
            int food = scenario.get("food").getAsInt();
            float saturation = scenario.get("saturation").getAsFloat();
            var level = server.overworld();
            Player player = FakePlayerFactory.getMinecraft(level);
            player.getFoodData().setFoodLevel(food);
            player.getFoodData().setSaturation(saturation);
            player.getFoodData().setExhaustion(0.0F);
            var holder = effect.isEmpty() ? null : BuiltInRegistries.MOB_EFFECT.getHolder(ResourceLocation.parse(effect)).orElseThrow();
            if (holder != null && !scenario.has("eat")) {
                player.addEffect(new MobEffectInstance(holder, duration, 0, false, false, false));
            }
            json.beginObject();
            json.name("name").value(name);
            json.name("effect_registered").value(holder != null);
            ItemStack remaining = ItemStack.EMPTY;
            if (scenario.has("eat")) {
                var item = BuiltInRegistries.ITEM.get(ResourceLocation.parse(scenario.get("eat").getAsString()));
                int count = scenario.has("count") ? scenario.get("count").getAsInt() : 1;
                remaining = new ItemStack(item, count);
                if (scenario.has("target_wolf")) {
                    Wolf wolf = net.minecraft.world.entity.EntityType.WOLF.create(level);
                    if (wolf == null) throw new IllegalStateException("could not create a test wolf");
                    wolf.moveTo(2.0, 64.0, 2.0, 0.0F, 0.0F);
                    if (scenario.get("target_wolf").getAsBoolean()) wolf.tame(player);
                    wolf.setHealth(scenario.has("wolf_health") ? scenario.get("wolf_health").getAsFloat() : 8.0F);
                    level.addFreshEntity(wolf);
                    var result = item.interactLivingEntity(remaining, player, wolf, InteractionHand.MAIN_HAND);
                    json.name("wolf_interaction").beginObject();
                    json.name("result").value(result.toString());
                    json.name("tamed").value(wolf.isTame());
                    json.name("owner_is_player").value(player.getUUID().equals(wolf.getOwnerUUID()));
                    json.name("health_before").value(scenario.has("wolf_health") ? scenario.get("wolf_health").getAsFloat() : 8.0F);
                    json.name("health_after").value(wolf.getHealth());
                    json.name("remaining_item").value(BuiltInRegistries.ITEM.getKey(remaining.getItem()).toString());
                    json.name("remaining_count").value(remaining.getCount());
                    json.endObject();
                } else {
                    remaining = item.finishUsingItem(remaining, level, player);
                }
            }
            if (scenario.has("eat")) {
                json.name("consumed").beginObject();
                json.name("item").value(scenario.get("eat").getAsString());
                json.name("input_count").value(scenario.has("count") ? scenario.get("count").getAsInt() : 1);
                json.name("remaining_item").value(BuiltInRegistries.ITEM.getKey(remaining.getItem()).toString());
                json.name("remaining_count").value(remaining.getCount());
                json.name("food_after").value(player.getFoodData().getFoodLevel());
                json.name("saturation_after").value(player.getFoodData().getSaturationLevel());
                json.name("active_effects").beginArray();
                for (MobEffectInstance active : player.getActiveEffects()) {
                    json.beginObject();
                    json.name("id").value(BuiltInRegistries.MOB_EFFECT.getKey(active.getEffect().value()).toString());
                    json.name("duration").value(active.getDuration());
                    json.name("amplifier").value(active.getAmplifier());
                    json.name("ambient").value(active.isAmbient());
                    json.name("visible").value(active.isVisible());
                    json.name("show_icon").value(active.showIcon());
                    json.endObject();
                }
                json.endArray();
                json.endObject();
            }
            json.name("trace").beginArray();
            snapshot(json, 0, player);
            for (int tick = 1; tick <= ticks; tick++) {
                for (MobEffectInstance active : player.getActiveEffects()) {
                    if (!active.tick(player, () -> {})) player.removeEffect(active.getEffect());
                }
                if (scenario.has("exhaustion_per_tick")) {
                    player.getFoodData().addExhaustion(scenario.get("exhaustion_per_tick").getAsFloat());
                }
                player.getFoodData().tick(player);
                if (tick <= 40 || tick % 20 == 0 || tick == duration - 1 || tick == duration || tick == duration + 1 || tick == ticks) {
                    snapshot(json, tick, player);
                }
            }
            json.endArray();
            json.endObject();
        }
        json.endArray();
        json.endObject();
    }

    private static void snapshot(JsonWriter json, int tick, Player player) throws IOException {
        json.beginObject();
        json.name("tick").value(tick);
        json.name("food").value(player.getFoodData().getFoodLevel());
        json.name("saturation").value(player.getFoodData().getSaturationLevel());
        json.name("exhaustion").value(player.getFoodData().getExhaustionLevel());
        json.name("active_effects").value(player.getActiveEffects().size());
        json.name("effect_duration").value(player.getActiveEffects().stream().findFirst().map(MobEffectInstance::getDuration).orElse(0));
        json.endObject();
    }
}

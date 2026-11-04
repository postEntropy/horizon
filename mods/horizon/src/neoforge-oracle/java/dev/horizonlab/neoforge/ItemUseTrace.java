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
import net.minecraft.world.InteractionHand;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.item.ItemStack;
import net.neoforged.neoforge.common.util.FakePlayerFactory;

/** Black-box item-use trace using only the public game API and a server FakePlayer. */
final class ItemUseTrace {
    static void run(MinecraftServer server, Path input, JsonWriter json) throws IOException {
        JsonArray scenarios = JsonParser.parseString(Files.readString(input)).getAsJsonArray();
        json.beginObject();
        json.name("schema").value(1);
        json.name("scenarios").beginArray();
        for (JsonElement element : scenarios) {
            JsonObject scenario = element.getAsJsonObject();
            var level = server.overworld();
            Player player = FakePlayerFactory.getMinecraft(level);
            var item = BuiltInRegistries.ITEM.get(ResourceLocation.parse(scenario.get("item").getAsString()));
            ItemStack stack = new ItemStack(item, scenario.has("count") ? scenario.get("count").getAsInt() : 1);
            player.setItemInHand(InteractionHand.MAIN_HAND, stack);
            json.beginObject();
            json.name("name").value(scenario.has("name") ? scenario.get("name").getAsString() : "item-use");
            json.name("item").value(BuiltInRegistries.ITEM.getKey(item).toString());
            json.name("before_count").value(stack.getCount());
            try {
                var result = item.use(level, player, InteractionHand.MAIN_HAND);
                json.name("result").value(result.getResult().toString());
                json.name("returned_item").value(BuiltInRegistries.ITEM.getKey(result.getObject().getItem()).toString());
                json.name("returned_count").value(result.getObject().getCount());
                json.name("hand_item").value(BuiltInRegistries.ITEM.getKey(player.getItemInHand(InteractionHand.MAIN_HAND).getItem()).toString());
                json.name("hand_count").value(player.getItemInHand(InteractionHand.MAIN_HAND).getCount());
                json.name("menu").value(player.containerMenu.getClass().getName());
                json.name("exception").nullValue();
            } catch (Throwable error) {
                json.name("exception").value(error.getClass().getName() + ": " + error.getMessage());
            }
            json.endObject();
        }
        json.endArray();
        json.endObject();
    }
}

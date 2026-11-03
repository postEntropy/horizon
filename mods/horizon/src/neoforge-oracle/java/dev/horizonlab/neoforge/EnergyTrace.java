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
import net.minecraft.world.item.ItemStack;
import net.neoforged.neoforge.capabilities.Capabilities;
import net.neoforged.neoforge.energy.IEnergyStorage;

/** Black-box FE capability probes using only NeoForge's public energy API. */
final class EnergyTrace {
    static void run(MinecraftServer server, Path input, JsonWriter json) throws IOException {
        JsonArray scenarios = JsonParser.parseString(Files.readString(input)).getAsJsonArray();
        json.beginObject();
        json.name("schema").value(1);
        json.name("scenarios").beginArray();
        for (JsonElement element : scenarios) {
            JsonObject scenario = element.getAsJsonObject();
            String id = scenario.get("item").getAsString();
            ItemStack stack = new ItemStack(
                    BuiltInRegistries.ITEM.get(ResourceLocation.parse(id)),
                    scenario.has("count") ? scenario.get("count").getAsInt() : 1);
            json.beginObject();
            json.name("item").value(id);
            json.name("stack_count").value(stack.getCount());
            if (stack.isEmpty()) {
                json.name("supported").value(false);
                json.name("reason").value("registry returned empty stack");
                json.endObject();
                continue;
            }
            IEnergyStorage energy = stack.getCapability(Capabilities.EnergyStorage.ITEM);
            json.name("supported").value(energy != null);
            if (energy != null) {
                json.name("before").beginObject();
                writeEnergy(json, energy);
                json.endObject();
                int request = scenario.has("request") ? scenario.get("request").getAsInt() : 100_000;
                json.name("simulate_receive").value(energy.receiveEnergy(request, true));
                json.name("simulate_extract").value(energy.extractEnergy(request, true));
                int received = energy.receiveEnergy(request, false);
                json.name("receive").value(received);
                json.name("after_receive").beginObject();
                writeEnergy(json, energy);
                json.endObject();
                int extracted = energy.extractEnergy(request, false);
                json.name("extract").value(extracted);
                json.name("after_extract").beginObject();
                writeEnergy(json, energy);
                json.endObject();
            }
            json.endObject();
        }
        json.endArray();
        json.endObject();
    }

    private static void writeEnergy(JsonWriter json, IEnergyStorage energy) throws IOException {
        json.name("stored").value(energy.getEnergyStored());
        json.name("capacity").value(energy.getMaxEnergyStored());
        json.name("can_receive").value(energy.canReceive());
        json.name("can_extract").value(energy.canExtract());
    }
}

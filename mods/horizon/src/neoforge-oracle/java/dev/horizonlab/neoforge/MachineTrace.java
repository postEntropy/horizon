package dev.horizonlab.neoforge;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.google.gson.stream.JsonWriter;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.Container;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.entity.ExperienceOrb;
import net.minecraft.world.InteractionHand;
import net.minecraft.world.ItemInteractionResult;
import net.minecraft.world.phys.BlockHitResult;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import net.neoforged.neoforge.capabilities.Capabilities;
import net.neoforged.neoforge.energy.IEnergyStorage;
import net.minecraft.world.SimpleContainer;
import net.minecraft.world.MenuProvider;
import net.minecraft.world.inventory.AbstractContainerMenu;
import net.neoforged.neoforge.common.util.FakePlayerFactory;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.entity.BlockEntityTicker;
import net.minecraft.world.level.block.entity.BlockEntityType;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.block.state.properties.Property;

/**
 * Dev-only machine oracle for Horizon's code-mod harness (tools/mod_machine_harness.py): places
 * each scenario's block, fills its container's slots, runs its block entity ticker N times and
 * writes the slots, the asked block-state properties and the asked integer fields of its saved
 * tag after every tick. The same scenarios run on the Horizon port through
 * crates/horizon-client/examples/mod_machine_probe.rs. Never shipped.
 */
final class MachineTrace {
    private static final BlockPos AT = new BlockPos(0, 100, 0);

    private MachineTrace() {}

    static void run(MinecraftServer server, Path scenarios, JsonWriter json) throws IOException {
        ServerLevel level = server.overworld();
        JsonArray list = JsonParser.parseString(Files.readString(scenarios)).getAsJsonArray();
        json.beginArray();
        for (JsonElement e : list) {
            scenario(server, level, e.getAsJsonObject(), json);
        }
        json.endArray();
    }

    private static ItemStack stack(JsonElement v) {
        if (v == null || v.isJsonNull()) {
            return ItemStack.EMPTY;
        }
        JsonArray a = v.getAsJsonArray();
        return new ItemStack(BuiltInRegistries.ITEM.get(ResourceLocation.parse(a.get(0).getAsString())),
                a.get(1).getAsInt());
    }

    @SuppressWarnings({"unchecked", "rawtypes"})
    private static BlockState with(BlockState state, String name, String value) {
        for (Property<?> p : state.getProperties()) {
            if (p.getName().equals(name)) {
                Property raw = p;
                java.util.Optional<Comparable> parsed = raw.getValue(value);
                return parsed.isPresent() ? (BlockState) state.setValue(raw, parsed.get()) : state;
            }
        }
        return state;
    }

    private static String property(BlockState state, String name) {
        for (Property<?> p : state.getProperties()) {
            if (p.getName().equals(name)) {
                return String.valueOf(state.getValue(p));
            }
        }
        return null;
    }

    @SuppressWarnings("unchecked")
    private static void scenario(MinecraftServer server, ServerLevel level, JsonObject s, JsonWriter json)
            throws IOException {
        Block block = BuiltInRegistries.BLOCK.get(ResourceLocation.parse(s.get("block").getAsString()));
        BlockState state = block.defaultBlockState();
        JsonObject properties = s.has("properties") ? s.getAsJsonObject("properties") : new JsonObject();
        for (Map.Entry<String, JsonElement> p : properties.entrySet()) {
            state = with(state, p.getKey(), p.getValue().getAsJsonArray().get(0).getAsString());
        }
        // Optional fixture blocks, observed only through vanilla world APIs.
        JsonArray neighbors = s.has("neighbors") ? s.getAsJsonArray("neighbors") : new JsonArray();
        for (JsonElement entry : neighbors) {
            JsonObject neighbor = entry.getAsJsonObject();
            BlockPos pos = AT.offset(neighbor.get("x").getAsInt(), neighbor.get("y").getAsInt(),
                    neighbor.get("z").getAsInt());
            if (pos.equals(AT)) throw new IllegalArgumentException("neighbor overlaps machine");
            BlockState neighborState = BuiltInRegistries.BLOCK.get(
                    ResourceLocation.parse(neighbor.get("block").getAsString())).defaultBlockState();
            if (neighbor.has("properties")) {
                for (Map.Entry<String, JsonElement> property : neighbor.getAsJsonObject("properties").entrySet()) {
                    neighborState = with(neighborState, property.getKey(), property.getValue().getAsString());
                }
            }
            level.setBlock(pos, neighborState, 3);
        }
        level.setBlock(AT, state, 3);
        BlockEntity be = level.getBlockEntity(AT);
        JsonArray entityInputs = s.has("entity_inputs") ? s.getAsJsonArray("entity_inputs") : new JsonArray();
        for (JsonElement input : entityInputs) {
            JsonObject entity = input.getAsJsonObject();
            double x = AT.getX() + entity.get("x").getAsDouble();
            double y = AT.getY() + entity.get("y").getAsDouble();
            double z = AT.getZ() + entity.get("z").getAsDouble();
            if (entity.has("item")) {
                ItemEntity item = new ItemEntity(level, x, y, z, stack(entity.get("item")));
                item.setDefaultPickUpDelay();
                level.addFreshEntity(item);
            } else if (entity.has("xp")) {
                ExperienceOrb.award(level, new Vec3(x, y, z), entity.get("xp").getAsInt());
            }
        }
        Container container = null;
        if (be instanceof Container vanilla) {
            container = vanilla;
        } else if (s.has("menu_slots")) {
            var player = FakePlayerFactory.getMinecraft(level);
            var provider = be instanceof MenuProvider direct ? direct : state.getMenuProvider(level, AT);
            if (provider == null) throw new IllegalStateException("machine has no public menu");
            AbstractContainerMenu menu = provider.createMenu(0, player.getInventory(), player);
            if (menu == null) throw new IllegalStateException("machine did not create public menu");
            int count = s.get("menu_slots").getAsInt();
            container = new SimpleContainer(count) {
                @Override public ItemStack getItem(int slot) { return menu.getSlot(slot).getItem(); }
                @Override public void setItem(int slot, ItemStack stack) { menu.getSlot(slot).set(stack); }
                @Override public void clearContent() {
                    for (int i = 0; i < getContainerSize(); i++) menu.getSlot(i).set(ItemStack.EMPTY);
                }
            };
        }
        if (s.has("slots")) {
            if (container == null) {
                throw new IllegalStateException(s.get("block") + " has no public inventory");
            }
            for (Map.Entry<String, JsonElement> slot : s.getAsJsonObject("slots").entrySet()) {
                container.setItem(Integer.parseInt(slot.getKey()), stack(slot.getValue()));
            }
        }
        int energyInjected = 0;
        int energyFirstReceive = 0;
        if (s.has("energy_input")) {
            IEnergyStorage energy = level.getCapability(Capabilities.EnergyStorage.BLOCK, AT, null);
            int remaining = s.get("energy_input").getAsInt();
            while (energy != null && remaining > 0) {
                int accepted = energy.receiveEnergy(remaining, false);
                if (accepted <= 0) break;
                if (energyInjected == 0) energyFirstReceive = accepted;
                energyInjected += accepted;
                remaining -= accepted;
            }
        }
        JsonArray uses = s.has("uses") ? s.getAsJsonArray("uses") : new JsonArray();
        JsonArray useResults = new JsonArray();
        if (!uses.isEmpty()) {
            var player = FakePlayerFactory.getMinecraft(level);
            for (JsonElement element : uses) {
                JsonObject use = element.getAsJsonObject();
                ItemStack held = stack(use.get("item"));
                player.setItemInHand(InteractionHand.MAIN_HAND, held);
                player.setShiftKeyDown(use.has("sneaking") && use.get("sneaking").getAsBoolean());
                BlockPos pos = AT.offset(use.has("x") ? use.get("x").getAsInt() : 0,
                        use.has("y") ? use.get("y").getAsInt() : 0,
                        use.has("z") ? use.get("z").getAsInt() : 0);
                var hit = new BlockHitResult(
                        new Vec3(pos.getX() + 0.5, pos.getY() + 1.0, pos.getZ() + 0.5),
                        Direction.UP, pos, false);
                ItemInteractionResult outcome = level.getBlockState(pos).useItemOn(
                        player.getItemInHand(InteractionHand.MAIN_HAND), level, player,
                        InteractionHand.MAIN_HAND, hit);
                JsonObject result = new JsonObject();
                result.addProperty("item", use.get("item").getAsJsonArray().get(0).getAsString());
                result.addProperty("result", outcome.toString());
                result.addProperty("held_item", BuiltInRegistries.ITEM
                        .getKey(player.getItemInHand(InteractionHand.MAIN_HAND).getItem()).toString());
                result.addProperty("held_count", player.getItemInHand(InteractionHand.MAIN_HAND).getCount());
                useResults.add(result);
            }
        }
        JsonArray fields = s.has("fields") ? s.getAsJsonArray("fields") : new JsonArray();
        JsonArray events = s.has("events") ? s.getAsJsonArray("events") : new JsonArray();
        json.beginObject();
        json.name("name").value(s.get("name").getAsString());
        json.name("trace").beginArray();
        int ticks = s.get("ticks").getAsInt();
        for (int tick = 0; tick < ticks; tick++) {
            for (JsonElement e : events) {
                JsonObject ev = e.getAsJsonObject();
                if (ev.get("tick").getAsInt() == tick) {
                    if (ev.has("block")) {
                        level.setBlock(AT.offset(ev.get("x").getAsInt(), ev.get("y").getAsInt(),
                                ev.get("z").getAsInt()), BuiltInRegistries.BLOCK.get(
                                ResourceLocation.parse(ev.get("block").getAsString())).defaultBlockState(), 3);
                    } else {
                        container.setItem(ev.get("slot").getAsInt(), stack(ev.get("stack")));
                    }
                }
            }
            BlockState now = level.getBlockState(AT);
            BlockEntityTicker<BlockEntity> ticker =
                    now.getTicker(level, (BlockEntityType<BlockEntity>) be.getType());
            if (ticker != null) {
                ticker.tick(level, AT, now, be);
            }
            now = level.getBlockState(AT);
            CompoundTag tag = be.saveWithoutMetadata(server.registryAccess());
            json.beginObject();
            json.name("saved_tag").value(tag.toString());
            json.name("use_results").beginArray();
            for (JsonElement resultElement : useResults) {
                JsonObject result = resultElement.getAsJsonObject();
                json.beginObject();
                json.name("item").value(result.get("item").getAsString());
                json.name("result").value(result.get("result").getAsString());
                json.name("held_item").value(result.get("held_item").getAsString());
                json.name("held_count").value(result.get("held_count").getAsInt());
                json.endObject();
            }
            json.endArray();
            json.name("nearby_items").beginObject();
            var nearbyItems = level.getEntitiesOfClass(ItemEntity.class, new AABB(AT).inflate(16));
            java.util.TreeMap<String, Integer> itemCounts = new java.util.TreeMap<>();
            for (ItemEntity item : nearbyItems) {
                itemCounts.merge(BuiltInRegistries.ITEM.getKey(item.getItem().getItem()).toString(),
                        item.getItem().getCount(), Integer::sum);
            }
            for (var item : itemCounts.entrySet()) json.name(item.getKey()).value(item.getValue());
            json.endObject();
            json.name("nearby_xp").value(level.getEntitiesOfClass(ExperienceOrb.class, new AABB(AT).inflate(16))
                    .stream().mapToInt(ExperienceOrb::getValue).sum());
            json.name("neighbor_states").beginObject();
            for (JsonElement entry : neighbors) {
                JsonObject neighbor = entry.getAsJsonObject();
                int dx = neighbor.get("x").getAsInt();
                int dy = neighbor.get("y").getAsInt();
                int dz = neighbor.get("z").getAsInt();
                BlockPos pos = AT.offset(dx, dy, dz);
                json.name(dx + "," + dy + "," + dz).value(level.getBlockState(pos).toString());
            }
            json.endObject();
            json.name("energy_injected").value(energyInjected);
            json.name("energy_first_receive").value(energyFirstReceive);
            IEnergyStorage energy = level.getCapability(Capabilities.EnergyStorage.BLOCK, AT, null);
            if (energy != null) {
                json.name("energy").beginObject();
                json.name("stored").value(energy.getEnergyStored());
                json.name("capacity").value(energy.getMaxEnergyStored());
                json.name("can_receive").value(energy.canReceive());
                json.name("can_extract").value(energy.canExtract());
                json.name("simulate_receive_100").value(energy.receiveEnergy(100, true));
                json.name("simulate_extract_100").value(energy.extractEnergy(100, true));
                json.endObject();
            }
            json.name("slots").beginObject();
            for (int i = 0; container != null && i < container.getContainerSize(); i++) {
                ItemStack item = container.getItem(i);
                if (!item.isEmpty()) {
                    json.name(Integer.toString(i)).beginArray()
                            .value(BuiltInRegistries.ITEM.getKey(item.getItem()).toString())
                            .value(item.getCount()).endArray();
                }
            }
            json.endObject();
            json.name("properties").beginObject();
            for (String name : properties.keySet()) {
                json.name(name).value(property(now, name));
            }
            json.endObject();
            json.name("fields").beginObject();
            for (JsonElement f : fields) {
                json.name(f.getAsString()).value(tag.getInt(f.getAsString()));
            }
            json.endObject();
            json.endObject();
        }
        json.endArray();
        json.endObject();
        if (container != null) container.clearContent();
        for (ItemEntity item : level.getEntitiesOfClass(ItemEntity.class, new AABB(AT).inflate(16))) item.discard();
        for (ExperienceOrb orb : level.getEntitiesOfClass(ExperienceOrb.class, new AABB(AT).inflate(16))) orb.discard();
        level.setBlock(AT, Blocks.AIR.defaultBlockState(), 3);
        for (JsonElement entry : neighbors) {
            JsonObject neighbor = entry.getAsJsonObject();
            level.setBlock(AT.offset(neighbor.get("x").getAsInt(), neighbor.get("y").getAsInt(),
                    neighbor.get("z").getAsInt()), Blocks.AIR.defaultBlockState(), 3);
        }
    }
}

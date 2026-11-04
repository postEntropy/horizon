package dev.horizonlab.neoforge;

import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import com.google.gson.Gson;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Map;
import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.nbt.CompoundTag;
import net.minecraft.resources.ResourceLocation;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.Container;
import net.minecraft.world.MenuProvider;
import net.minecraft.world.SimpleContainer;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.entity.player.Player;
import net.minecraft.world.inventory.AbstractContainerMenu;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.phys.AABB;
import net.neoforged.neoforge.capabilities.Capabilities;
import net.neoforged.neoforge.energy.IEnergyStorage;
import net.minecraft.world.level.block.entity.BlockEntity;
import net.minecraft.world.level.block.state.BlockState;
import net.neoforged.neoforge.common.util.FakePlayerFactory;
import net.neoforged.neoforge.common.NeoForge;
import net.neoforged.neoforge.event.tick.ServerTickEvent;

/** Runs machine scenarios across real server ticks for black-box timing behavior. */
final class LiveMachineTrace {
    private static final BlockPos AT = new BlockPos(0, 100, 0);
    private static final Gson GSON = new Gson();

    private final MinecraftServer server;
    private final ServerLevel level;
    private final Path out;
    private final JsonArray scenarios;
    private final JsonArray results = new JsonArray();
    private int scenarioIndex;
    private int elapsed;
    private JsonObject scenario;
    private BlockEntity blockEntity;
    private Container container;
    private java.util.List<JsonObject> trace;
    private JsonArray neighbors;
    private int energyInjected;

    private LiveMachineTrace(MinecraftServer server, Path scenariosPath, Path out) throws IOException {
        this.server = server;
        this.level = server.overworld();
        this.out = out;
        this.scenarios = JsonParser.parseString(Files.readString(scenariosPath)).getAsJsonArray();
    }

    static void start(MinecraftServer server, Path scenarios, Path out) throws IOException {
        LiveMachineTrace run = new LiveMachineTrace(server, scenarios, out);
        run.prepare();
        NeoForge.EVENT_BUS.addListener(run::afterTick);
    }

    private void prepare() {
        if (scenarioIndex >= scenarios.size()) {
            finish();
            return;
        }
        scenario = scenarios.get(scenarioIndex).getAsJsonObject();
        elapsed = 0;
        energyInjected = 0;
        trace = new java.util.ArrayList<>();
        level.setChunkForced(AT.getX() >> 4, AT.getZ() >> 4, true);
        level.getChunk(AT);
        neighbors = scenario.has("neighbors") ? scenario.getAsJsonArray("neighbors") : new JsonArray();
        for (JsonElement entry : neighbors) {
            JsonObject neighbor = entry.getAsJsonObject();
            BlockPos pos = AT.offset(neighbor.get("x").getAsInt(), neighbor.get("y").getAsInt(),
                    neighbor.get("z").getAsInt());
            BlockState neighborState = BuiltInRegistries.BLOCK.get(
                    ResourceLocation.parse(neighbor.get("block").getAsString())).defaultBlockState();
            if (neighbor.has("properties")) {
                for (Map.Entry<String, JsonElement> p : neighbor.getAsJsonObject("properties").entrySet()) {
                    neighborState = MachineTrace.with(neighborState, p.getKey(), p.getValue().getAsString());
                }
            }
            level.setBlock(pos, neighborState, 3);
            if (neighbor.has("slots") && level.getBlockEntity(pos) instanceof Container neighborContainer) {
                for (Map.Entry<String, JsonElement> slot : neighbor.getAsJsonObject("slots").entrySet()) {
                    neighborContainer.setItem(Integer.parseInt(slot.getKey()), MachineTrace.stack(slot.getValue()));
                }
            }
        }
        BlockState state = BuiltInRegistries.BLOCK.get(
                ResourceLocation.parse(scenario.get("block").getAsString())).defaultBlockState();
        if (scenario.has("properties")) {
            for (Map.Entry<String, JsonElement> p : scenario.getAsJsonObject("properties").entrySet()) {
                state = MachineTrace.with(state, p.getKey(), p.getValue().getAsJsonArray().get(0).getAsString());
            }
        }
        level.setBlock(AT, state, 3);
        blockEntity = level.getBlockEntity(AT);
        if (blockEntity instanceof Container direct
                && !(scenario.has("use_menu") && scenario.get("use_menu").getAsBoolean())) {
            container = direct;
        } else {
            Player player = FakePlayerFactory.getMinecraft(level);
            MenuProvider provider = blockEntity instanceof MenuProvider direct
                    ? direct : state.getMenuProvider(level, AT);
            if (provider == null) throw new IllegalStateException("machine has no public menu");
            AbstractContainerMenu menu = provider.createMenu(0, player.getInventory(), player);
            if (menu == null) throw new IllegalStateException("machine did not create public menu");
            int count = scenario.get("menu_slots").getAsInt();
            container = new SimpleContainer(count) {
                @Override public ItemStack getItem(int slot) { return menu.getSlot(slot).getItem(); }
                @Override public void setItem(int slot, ItemStack stack) { menu.getSlot(slot).set(stack); }
                @Override public void clearContent() {
                    for (int i = 0; i < getContainerSize(); i++) menu.getSlot(i).set(ItemStack.EMPTY);
                }
            };
        }
        for (Map.Entry<String, JsonElement> entry : scenario.getAsJsonObject("slots").entrySet()) {
            container.setItem(Integer.parseInt(entry.getKey()), MachineTrace.stack(entry.getValue()));
        }
        if (scenario.has("energy_input") && !scenario.has("energy_input_tick")) {
            injectEnergy(scenario.get("energy_input").getAsInt());
        }
        if (scenario.has("entity_inputs") && !scenario.has("entity_inputs_after_ticks")) spawnEntityInputs();
    }

    private void afterTick(ServerTickEvent.Post event) {
        if (scenarioIndex >= scenarios.size()) return;
        elapsed++;
        if (scenario.has("entity_inputs_after_ticks")
                && scenario.get("entity_inputs_after_ticks").getAsInt() == elapsed) spawnEntityInputs();
        if (scenario.has("energy_input_tick") && scenario.get("energy_input_tick").getAsInt() == elapsed) {
            injectEnergy(scenario.get("energy_input").getAsInt());
        }
        JsonObject snapshot = new JsonObject();
        snapshot.addProperty("container_size", container.getContainerSize());
        snapshot.addProperty("block_state", level.getBlockState(AT).toString());
        snapshot.addProperty("energy_injected", energyInjected);
        CompoundTag tag = blockEntity.saveWithoutMetadata(server.registryAccess());
        snapshot.addProperty("saved_tag", tag.toString());
        snapshot.addProperty("game_time", level.getGameTime());
        JsonObject slots = new JsonObject();
        for (int i = 0; i < container.getContainerSize(); i++) {
            ItemStack item = container.getItem(i);
            if (!item.isEmpty()) {
                JsonArray value = new JsonArray();
                value.add(BuiltInRegistries.ITEM.getKey(item.getItem()).toString());
                value.add(item.getCount());
                slots.add(Integer.toString(i), value);
            }
        }
        snapshot.add("slots", slots);
        JsonObject neighborStates = new JsonObject();
        for (JsonElement entry : neighbors) {
            JsonObject neighbor = entry.getAsJsonObject();
            int dx = neighbor.get("x").getAsInt();
            int dy = neighbor.get("y").getAsInt();
            int dz = neighbor.get("z").getAsInt();
            neighborStates.addProperty(dx + "," + dy + "," + dz,
                    level.getBlockState(AT.offset(dx, dy, dz)).toString());
        }
        snapshot.add("neighbor_states", neighborStates);
        JsonObject nearbyItems = new JsonObject();
        java.util.TreeMap<String, Integer> counts = new java.util.TreeMap<>();
        for (ItemEntity item : level.getEntitiesOfClass(ItemEntity.class, new AABB(AT).inflate(16))) {
            counts.merge(BuiltInRegistries.ITEM.getKey(item.getItem().getItem()).toString(),
                    item.getItem().getCount(), Integer::sum);
        }
        counts.forEach((id, count) -> nearbyItems.addProperty(id, count));
        snapshot.add("nearby_items", nearbyItems);
        JsonArray itemEntities = new JsonArray();
        for (ItemEntity item : level.getEntitiesOfClass(ItemEntity.class, new AABB(AT).inflate(16))) {
            JsonObject entity = new JsonObject();
            entity.addProperty("entity_id", item.getId());
            entity.addProperty("item", BuiltInRegistries.ITEM.getKey(item.getItem().getItem()).toString());
            entity.addProperty("count", item.getItem().getCount());
            entity.addProperty("x", item.getX());
            entity.addProperty("y", item.getY());
            entity.addProperty("z", item.getZ());
            itemEntities.add(entity);
        }
        snapshot.add("item_entities", itemEntities);
        IEnergyStorage energy = level.getCapability(Capabilities.EnergyStorage.BLOCK, AT, null);
        if (energy != null) {
            JsonObject storage = new JsonObject();
            storage.addProperty("stored", energy.getEnergyStored());
            storage.addProperty("capacity", energy.getMaxEnergyStored());
            storage.addProperty("can_receive", energy.canReceive());
            storage.addProperty("can_extract", energy.canExtract());
            snapshot.add("energy", storage);
        }
        trace.add(snapshot);
        if (elapsed >= scenario.get("ticks").getAsInt()) {
            JsonObject result = new JsonObject();
            result.addProperty("name", scenario.get("name").getAsString());
            JsonArray ticks = new JsonArray();
            trace.forEach(ticks::add);
            result.add("trace", ticks);
            results.add(result);
            container.clearContent();
            level.setBlock(AT, net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(), 3);
            for (ItemEntity item : level.getEntitiesOfClass(ItemEntity.class, new AABB(AT).inflate(16))) item.discard();
            for (JsonElement entry : neighbors) {
                JsonObject neighbor = entry.getAsJsonObject();
                level.setBlock(AT.offset(neighbor.get("x").getAsInt(), neighbor.get("y").getAsInt(),
                        neighbor.get("z").getAsInt()), net.minecraft.world.level.block.Blocks.AIR.defaultBlockState(), 3);
            }
            scenarioIndex++;
            prepare();
        }
    }

    private void finish() {
        try {
            Files.createDirectories(out.getParent());
            Files.writeString(out, GSON.toJson(results));
            System.out.println("horizon-oracle: wrote live machine trace " + out);
            server.halt(false);
        } catch (IOException e) {
            throw new RuntimeException(e);
        }
    }

    private void spawnEntityInputs() {
        for (JsonElement element : scenario.getAsJsonArray("entity_inputs")) {
            JsonObject input = element.getAsJsonObject();
            if (!input.has("item")) continue;
            ItemEntity item = new ItemEntity(level, AT.getX() + input.get("x").getAsDouble(),
                    AT.getY() + input.get("y").getAsDouble(), AT.getZ() + input.get("z").getAsDouble(),
                    MachineTrace.stack(input.get("item")));
            item.setNoPickUpDelay();
            if (input.has("stationary") && input.get("stationary").getAsBoolean()) {
                item.setNoGravity(true);
                item.setDeltaMovement(net.minecraft.world.phys.Vec3.ZERO);
            }
            level.addFreshEntity(item);
        }
    }

    private void injectEnergy(int amount) {
        IEnergyStorage energy = level.getCapability(Capabilities.EnergyStorage.BLOCK, AT, null);
        int remaining = amount;
        while (energy != null && remaining > 0) {
            int accepted = energy.receiveEnergy(remaining, false);
            if (accepted <= 0) break;
            energyInjected += accepted;
            remaining -= accepted;
        }
    }
}

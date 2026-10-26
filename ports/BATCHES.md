# Guia de Portabilidade em Lotes: All the Mods 10 (ATM10)

Este documento organiza todos os mods do ATM10 pendentes em **lotes de 10**. Diferentes agentes ou instâncias de trabalho podem pegar lotes específicos em paralelo, marcar o checkbox correspondente como `[x]`, e seguir o padrão de salvamento e compilação descrito abaixo.

---

## 🛠️ Padrão de Implementação (Onde Salvar & Como Rodar)

Cada agente que for portar um lote **DEVE** seguir estes passos exatos:

### 1. Criar o crate do mod em `ports/<mod_id>/`
Cada mod possui seu próprio crate dentro do sub-workspace `ports/`:

- **`ports/<mod_id>/Cargo.toml`**:
  ```toml
  [package]
  name = "<mod_id>-port"
  version = "0.1.0"
  edition = "2024"

  [lib]
  crate-type = ["cdylib"]

  [dependencies]
  horizon-mod-sdk = { path = "../../crates/horizon-mod-sdk" }
  ```

- **`ports/<mod_id>/src/lib.rs`**:
  Implementação Rust limpa utilizando as APIs de `horizon_mod_sdk`:
  ```rust
  use horizon_mod_sdk::{Mod, Registration, export_mod, ...};
  
  struct MyMod;
  impl Mod for MyMod {
      fn init() -> Result<(Self, Registration), String> {
          Ok((MyMod, Registration::new()))
      }
  }
  export_mod!(MyMod);
  ```

### 2. Registrar no workspace `ports/Cargo.toml`
Adicionar `"<mod_id>"` na lista `members = [...]` de `ports/Cargo.toml`.

### 3. Registrar no script de deploy
Em `crates/cargo-horizon-mod/src/bin/deploy_ports.rs`, adicionar a tupla na constante `PORTS`:
```rust
("<mod_id>", "<mod_id>_port.wasm"),
```

### 4. Compilar para WebAssembly
```bash
cargo build --manifest-path ports/Cargo.toml --target wasm32-unknown-unknown --release
```

### 5. Fazer Deploy e Atualizar o Relatório
```bash
cargo run -p cargo-horizon-mod --bin deploy_ports
```
O script:
1. Converte o `.wasm` compilado em WebAssembly Component (`wasm32-unknown-unknown` -> component).
2. Injeta em `~/.local/share/horizon/ports/all-the-mods-10-atm10/mods/<mod_id>/carry/mod.wasm`.
3. Atualiza `horizon-mod.toml` para `api = "0.3"` (ou `0.4`/`0.5`) e declara as capabilities necessárias.
4. Reexecuta `horizon-port-pack report` atualizando o `coverage.json` e `coverage.md`.

---

## 📋 Tabela de Lotes

### ✅ Lote 1 (Concluído)
- [x] **`jumpboat`** (Jumpy Boats) — *Tier: logic* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`accelerateddecay`** (Accelerated Decay) — *Tier: logic* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`bridgingmod`** (Bridging Mod) — *Tier: logic* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`crafting_on_a_stick`** (Crafting On A Stick) — *Tier: logic* (ported-wasm (0 blocks, 9 items, mod.wasm))
- [x] **`elevatorid`** (ElevatorMod) — *Tier: logic* (ported-wasm (16 blocks, 0 items, mod.wasm))
- [x] **`repeatable_trial_vaults`** (Repeatable Trial Vaults) — *Tier: logic* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`moredragoneggs`** (More Dragon Eggs) — *Tier: logic* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`heyberryshutup`** (Hey Berry! SHUT UP) — *Tier: logic* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`blueflame`** (Blue Flame Burning) — *Tier: logic* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`torchmaster`** (Torchmaster) — *Tier: logic* (ported-wasm (4 blocks, 1 items, mod.wasm))

### ✅ Lote 2 (Concluído)
- [x] **`tiab`** (Time In A Bottle) — *Tier: logic* (ported-wasm (0 blocks, 1 items, mod.wasm))
- [x] **`simplemagnets`** (Simple Magnets) — *Tier: logic* (ported-wasm (2 blocks, 2 items, mod.wasm))
- [x] **`akashictome`** (Akashic Tome) — *Tier: logic* (ported-wasm (0 blocks, 1 items, mod.wasm))
- [x] **`measurements`** (Measurements) — *Tier: logic* (ported-wasm (0 blocks, 1 items, mod.wasm))
- [x] **`allthetweaks`** (AllTheTweaks) — *Tier: logic* (ported-wasm (8 blocks, 13 items, mod.wasm))
- [x] **`solcarrot`** (Spice of Life: Carrot Edition) — *Tier: logic* (ported-wasm (0 blocks, 1 items, mod.wasm))
- [x] **`bhc`** (Baubley Heart Canisters) — *Tier: logic* (ported-wasm (0 blocks, 35 items, mod.wasm))
- [x] **`ftbultimine`** (FTB Ultimine) — *Tier: logic* (ported-wasm (0 blocks, 1 items, mod.wasm))
- [x] **`restrictions`** (Restrictions) — *Tier: logic* (ported-wasm (4 blocks, 1 items, mod.wasm))
- [x] **`aeinfinitybooster`** (AEInfinityBooster) — *Tier: logic* (ported-wasm (0 blocks, 2 items, mod.wasm))

### ✅ Lote 3 (Concluído)
- [x] **`allthearcanistgear`** (All The Arcanist Gear) — *Tier: logic* (ported-wasm (0 blocks, 20 items, mod.wasm))
- [x] **`allthewizardgear`** (All the Wizard Gear) — *Tier: logic* (ported-wasm (0 blocks, 15 items, mod.wasm))
- [x] **`apothic_spawners`** (Apothic Spawners) — *Tier: logic* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`ars_additions`** (Ars Additions) — *Tier: logic* (ported-wasm (23 blocks, 68 items, mod.wasm))
- [x] **`ars_controle`** (Ars Controle) — *Tier: logic* (ported-wasm (3 blocks, 11 items, mod.wasm))
- [x] **`ars_elemancy`** (Ars Elemancy) — *Tier: logic* (ported-wasm (0 blocks, 107 items, mod.wasm))
- [x] **`ars_ocultas`** (Ars Ocultas) — *Tier: logic* (ported-wasm (1 blocks, 0 items, mod.wasm))
- [x] **`ars_polymorphia`** (Ars Polymorphia) — *Tier: logic* (ported-wasm (0 blocks, 0 items, mod.wasm); waits on polymorph)
- [x] **`comforts`** (Comforts) — *Tier: logic* (ported-wasm (32 blocks, 0 items, mod.wasm))
- [x] **`constructionstick`** (Construction Sticks) — *Tier: logic* (ported-wasm (0 blocks, 20 items, mod.wasm))

### ✅ Lote 4 (Concluído)
- [x] **`dyenamics`** (Dyenamics) — *Tier: logic* (ported-wasm (252 blocks, 18 items, mod.wasm))
- [x] **`fuelgoeshere`** (Fuel Goes Here) — *Tier: logic* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`imfast`** (I'm Fast) — *Tier: logic* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`ironjetpacks`** (Iron Jetpacks) — *Tier: logic* (ported-wasm (0 blocks, 9 items, mod.wasm))
- [x] **`mcwlights`** (Macaw's Lights and Lamps) — *Tier: logic* (ported-wasm (73 blocks, 0 items, mod.wasm))
- [x] **`mcwwindows`** (Macaw's Windows) — *Tier: logic* (ported-wasm (55 blocks, 16 items, mod.wasm))
- [x] **`merrymaking`** (Mama's MerryMaking) — *Tier: logic* (ported-wasm (418 blocks, 101 items, mod.wasm))
- [x] **`morered`** (More Red) — *Tier: logic* (ported-wasm (6 blocks, 4 items, mod.wasm))
- [x] **`multipiston`** (Multi-Piston) — *Tier: logic* (ported-wasm (1 blocks, 0 items, mod.wasm))
- [x] **`mysticalagradditions`** (Mystical Agradditions) — *Tier: logic* (ported-wasm (25 blocks, 33 items, mod.wasm))

### ✅ Lote 5 (Concluído)
- [x] **`not_enough_glyphs`** (Not Enough Glyphs) — *Tier: logic* (ported-wasm (0 blocks, 43 items, mod.wasm))
- [x] **`productivetrees`** (Productive Trees) — *Tier: logic* (ported-wasm (3604 blocks, 127 items, mod.wasm))
- [x] **`redstonepen`** (Redstone Pen) — *Tier: logic* (ported-wasm (4 blocks, 40 items, mod.wasm))
- [x] **`refinedstorage_quartz_arsenal`** (Refined Storage - Quartz Arsenal) — *Tier: logic* (ported-wasm (0 blocks, 4 items, mod.wasm))
- [x] **`silentgems`** (Silent's Gems) — *Tier: logic* (ported-wasm (384 blocks, 40 items, mod.wasm))
- [x] **`starbunclemania`** (Starbunclemania) — *Tier: logic* (ported-wasm (8 blocks, 32 items, mod.wasm))
- [x] **`utilitarian`** (Utilitarian) — *Tier: logic* (ported-wasm (49 blocks, 10 items, mod.wasm))
- [x] **`wstweaks`** (Wither Skeleton Tweaks) — *Tier: logic* (ported-wasm (0 blocks, 2 items, mod.wasm))
- [x] **`xycraft_world`** (XyCraft World) — *Tier: logic* (ported-wasm (241 blocks, 12 items, mod.wasm))
- [x] **`actuallyadditions`** (Actually Additions) — *Tier: machines* (ported-wasm (125 blocks, 113 items, mod.wasm))

### ✅ Lote 6 (Concluído)
- [x] **`advanced_ae`** (Advanced AE) — *Tier: machines* (ported-wasm (6 blocks, 51 items, mod.wasm))
- [x] **`ae2importexportcard`** (AE2 Import Export Card) — *Tier: machines* (ported-wasm (0 blocks, 3 items, mod.wasm))
- [x] **`ae2netanalyser`** (AE2NetworkAnalyzer) — *Tier: machines* (ported-wasm (0 blocks, 2 items, mod.wasm))
- [x] **`ae2wtlib`** (AE2WTLib) — *Tier: machines* (ported-wasm (0 blocks, 5 items, mod.wasm))
- [x] **`apothic_enchanting`** (Apothic Enchanting) — *Tier: machines* (ported-wasm (33 blocks, 23 items, mod.wasm))
- [x] **`appflux`** (AppliedFlux) — *Tier: machines* (ported-wasm (2 blocks, 42 items, mod.wasm))
- [x] **`ars_technica`** (Ars Technica) — *Tier: machines* (ported-wasm (3 blocks, 34 items, mod.wasm))
- [x] **`bibliocraft`** (Bibliocraft Legacy) — *Tier: machines* (ported-wasm (472 blocks, 890 items, mod.wasm))
- [x] **`bigreactors`** (Extreme Reactors) — *Tier: machines* (ported-wasm (89 blocks, 28 items, mod.wasm))
- [x] **`botanypots`** (BotanyPots) — *Tier: machines* (ported-wasm (183 blocks, 0 items, mod.wasm))

### ✅ Lote 7 (Concluído)
- [x] **`cabletiers`** (Cable Tiers) — *Tier: machines* (ported-wasm (64 blocks, 16 items, mod.wasm))
- [x] **`charginggadgets`** (Charging Gadgets) — *Tier: machines* (ported-wasm (1 blocks, 0 items, mod.wasm))
- [x] **`chisel`** (Chisel Reborn) — *Tier: machines* (ported-wasm (1293 blocks, 1 items, mod.wasm))
- [x] **`cookingforblockheads`** (Cooking for Blockheads) — *Tier: machines* (ported-wasm (90 blocks, 8 items, mod.wasm))
- [x] **`cosmeticarmorreworked`** (CosmeticArmorReworked) — *Tier: machines* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`dimstorage`** (DimStorage) — *Tier: machines* (ported-wasm (2 blocks, 4 items, mod.wasm))
- [x] **`disenchanting_table`** (Dis-Enchanting Table) — *Tier: machines* (ported-wasm (1 blocks, 0 items, mod.wasm))
- [x] **`enderdrives`** (Ender Drives) — *Tier: machines* (ported-wasm (0 blocks, 72 items, mod.wasm))
- [x] **`enderstorage`** (EnderStorage) — *Tier: machines* (ported-wasm (2 blocks, 53 items, mod.wasm))
- [x] **`energymeter`** (Energy Meter) — *Tier: machines* (ported-wasm (1 blocks, 1 items, mod.wasm))

### ✅ Lote 8 (Concluído)
- [x] **`entangled`** (Entangled) — *Tier: machines* (ported-wasm (0 blocks, 1 items, mod.wasm))
- [x] **`expandedae`** (Expanded AE) — *Tier: machines* (ported-wasm (2 blocks, 7 items, mod.wasm))
- [x] **`explorerscompass`** (Explorer's Compass) — *Tier: machines* (ported-wasm (0 blocks, 32 items, mod.wasm))
- [x] **`extended_industrialization`** (Extended Industrialization) — *Tier: machines* (ported-wasm (60 blocks, 47 items, mod.wasm))
- [x] **`extendedae`** (ExtendedAE) — *Tier: machines* (ported-wasm (25 blocks, 45 items, mod.wasm))
- [x] **`extradisks`** (Extra Disks) — *Tier: machines* (ported-wasm (19 blocks, 39 items, mod.wasm))
- [x] **`extrastorage`** (ExtraStorage) — *Tier: machines* (ported-wasm (8 blocks, 18 items, mod.wasm))
- [x] **`farmersdelight`** (Farmer's Delight) — *Tier: machines* (ported-wasm (113 blocks, 96 items, mod.wasm))
- [x] **`farmingforblockheads`** (Farming for Blockheads) — *Tier: machines* (ported-wasm (3 blocks, 3 items, mod.wasm))
- [x] **`fluxnetworks`** (Flux Networks) — *Tier: machines* (ported-wasm (5 blocks, 4 items, mod.wasm))

### ✅ Lote 9 (Concluído)
- [x] **`ftbquests`** (FTB Quests) — *Tier: machines* (ported-wasm (10 blocks, 5 items, mod.wasm); waits on ftbteams)
- [x] **`functionalstorage`** (Functional Storage) — *Tier: machines* (ported-wasm (0 blocks, 17 items, mod.wasm))
- [x] **`generatorgalore`** (Generator Galore) — *Tier: machines* (ported-wasm (45 blocks, 15 items, mod.wasm))
- [x] **`handcrafted`** (Handcrafted) — *Tier: machines* (ported-wasm (71 blocks, 18 items, mod.wasm))
- [x] **`hostilenetworks`** (Hostile Neural Networks) — *Tier: machines* (ported-wasm (3 blocks, 11 items, mod.wasm))
- [x] **`industrialforegoing`** (Industrial Foregoing) — *Tier: machines* (ported-wasm (18 blocks, 149 items, mod.wasm))
- [x] **`industrialforegoingsouls`** (Industrial Foregoing Souls) — *Tier: machines* (ported-wasm (1 blocks, 0 items, mod.wasm))
- [x] **`industrialization_overdrive`** (Industrialization Overdrive) — *Tier: machines* (ported-wasm (2 blocks, 6 items, mod.wasm))
- [x] **`interdimensionalwirelesstransmitter`** (Interdimensional Wireless Transmitter) — *Tier: machines* (ported-wasm (0 blocks, 0 items, mod.wasm))
- [x] **`ironfurnaces`** (Iron Furnaces) — *Tier: machines* (ported-wasm (1 blocks, 29 items, mod.wasm))

### ✅ Lote 10 (Concluído)
- [x] **`irons_jewelry`** (Iron's Gems 'n Jewelry) — *Tier: machines* (ported-wasm (1 blocks, 11 items, mod.wasm))
- [x] **`itemcollectors`** (Item Collectors) — *Tier: machines* (ported-wasm (2 blocks, 0 items, mod.wasm))
- [x] **`laserbridges`** (Laser Bridges) — *Tier: machines* (ported-wasm (6 blocks, 0 items, mod.wasm))
- [x] **`laserio`** (LaserIO) — *Tier: machines* (ported-wasm (3 blocks, 30 items, mod.wasm))
- [x] **`lootr`** (Lootr) — *Tier: machines* (ported-wasm (7 blocks, 1 items, mod.wasm))
- [x] **`mcwfurnitures`** (Macaw's Furniture) — *Tier: machines* (ported-wasm (312 blocks, 2 items, mod.wasm))
- [x] **`megacells`** (MEGA Cells) — *Tier: machines* (ported-wasm (5 blocks, 95 items, mod.wasm))
- [x] **`mekmm`** (Mekanism: MoreMachine) — *Tier: machines* (ported-wasm (20 blocks, 21 items, mod.wasm))
- [x] **`merequester`** (ME Requester) — *Tier: machines* (ported-wasm (0 blocks, 2 items, mod.wasm))
- [x] **`mifa`** (More Industrial Foregoing Addons) — *Tier: machines* (ported-wasm (0 blocks, 19 items, mod.wasm))

### ✅ Lote 11 (Concluído)
- [x] **`mob_grinding_utils`** (Mob Grinding Utils) — *Tier: machines* (ported-wasm (18 blocks, 26 items, mod.wasm))
- [x] **`moderndynamics`** (Modern Dynamics) — *Tier: machines* (ported-wasm (7 blocks, 6 items, mod.wasm))
- [x] **`modularbees`** (ModularBees) — *Tier: machines* (ported-wasm (1 blocks, 2 items, mod.wasm))
- [x] **`modularrouters`** (Modular Routers) — *Tier: machines* (ported-wasm (1 blocks, 54 items, mod.wasm))
- [x] **`mysticalagriculture`** (Mystical Agriculture) — *Tier: machines* (ported-wasm (201 blocks, 591 items, mod.wasm))
- [x] **`naturescompass`** (Nature's Compass) — *Tier: machines* (ported-wasm (0 blocks, 32 items, mod.wasm))
- [x] **`pipez`** (Pipez) — *Tier: machines* (ported-wasm (0 blocks, 7 items, mod.wasm))
- [x] **`pocketstorage`** (Pocket Storage) — *Tier: machines* (ported-wasm (0 blocks, 4 items, mod.wasm))
- [x] **`powah`** (Powah) — *Tier: machines* (ported-wasm (92 blocks, 43 items, mod.wasm))
- [x] **`productivemetalworks`** (Productive Metalworks) — *Tier: machines* (ported-wasm (128 blocks, 55 items, mod.wasm))

### ✅ Lote 12 (Concluído)
- [x] **`pylons`** (Pylons) — *Tier: machines* (ported-wasm (5 blocks, 5 items, mod.wasm))
- [x] **`quarryplus`** (QuarryPlus) — *Tier: machines* (ported-wasm (13 blocks, 18 items, mod.wasm))
- [x] **`rangedpumps`** (Ranged Pumps) — *Tier: machines* (ported-wasm (1 blocks, 0 items, mod.wasm))
- [x] **`rechiseled`** (Rechiseled) — *Tier: machines* (ported-wasm (3627 blocks, 1 items, mod.wasm))
- [x] **`refinedtypes`** (Refined Types) — *Tier: machines* (ported-wasm (30 blocks, 57 items, mod.wasm))
- [x] **`rftoolsbase`** (RFToolsBase) — *Tier: machines* (ported-wasm (7 blocks, 10 items, mod.wasm))
- [x] **`rftoolspower`** (RFToolsPower) — *Tier: machines* (ported-wasm (8 blocks, 7 items, mod.wasm))
- [x] **`rftoolsstorage`** (RFToolsStorage) — *Tier: machines* (ported-wasm (2 blocks, 8 items, mod.wasm))
- [x] **`rftoolsutility`** (RFToolsUtility) — *Tier: machines* (ported-wasm (14 blocks, 71 items, mod.wasm))
- [x] **`rsinsertexportupgrade`** (Refined Wireless Upgrades) — *Tier: machines* (ported-wasm (0 blocks, 5 items, mod.wasm))

### ✅ Lote 13 (Concluído)
- [x] **`sawmill`** (Universal Sawmill) — *Tier: machines* (ported-wasm (13 worldgen files, mod.wasm))
- [x] **`sophisticatedstorage`** (Sophisticated Storage) — *Tier: machines* (ported-wasm (29 blocks, 88 items, mod.wasm))
- [x] **`stepcrafter`** (Step Crafter) — *Tier: machines* (ported-wasm (0 blocks, 1 items, mod.wasm))
- [x] **`sushigocrafting`** (Sushi Go Crafting) — *Tier: machines* (ported-wasm (10 blocks, 63 items, mod.wasm))
- [x] **`toolbelt`** (Tool Belt) — *Tier: machines* (ported-wasm (0 blocks, 3 items, mod.wasm))
- [x] **`transfer_labels`** (Transfer Labels) — *Tier: machines* (ported-wasm (0 blocks, 5 items, mod.wasm))
- [x] **`trashcans`** (Trash Cans) — *Tier: machines* (ported-wasm (4 blocks, 0 items, mod.wasm))
- [x] **`treetap`** (Tree Tap) — *Tier: machines* (ported-wasm (4 blocks, 0 items, mod.wasm))
- [x] **`trophymanager`** (Jonn's Trophies) — *Tier: machines* (ported-wasm (1 blocks, 0 items, mod.wasm))
- [x] **`universalgrid`** (Universal Grid) — *Tier: machines* (ported-wasm (0 blocks, 4 items, mod.wasm))

### ✅ Lote 14 (Concluído)
- [x] **`waystones`** (Waystones) — *Tier: machines* (ported-wasm (43 blocks, 18 items, mod.wasm))
- [x] **`wirelesschargers`** (Wireless Chargers) — *Tier: machines* (ported-wasm (4 blocks, 0 items, mod.wasm))
- [x] **`xnet`** (XNet) — *Tier: machines* (ported-wasm (9 blocks, 16 items, mod.wasm))
- [x] **`xycraft_machines`** (XyCraft Machines) — *Tier: machines* (ported-wasm (19 blocks, 69 items, mod.wasm))
- [x] **`ad_astra`** (Ad Astra) — *Tier: deep* (ported-wasm (309 blocks, 85 items, mod.wasm))
- [x] **`ae2`** (Applied Energistics 2) — *Tier: deep* (ported-wasm (78 blocks, 276 items, mod.wasm))
- [x] **`aether`** (The Aether) — *Tier: deep* (ported-wasm (120 blocks, 490 items, mod.wasm))
- [x] **`allthemodium`** (Allthemodium) — *Tier: deep* (ported-wasm (99 blocks, 107 items, mod.wasm))
- [x] **`amendments`** (Amendments) — *Tier: deep* (ported-wasm (28 blocks, 2 items, mod.wasm))
- [x] **`apotheosis`** (Apotheosis) — *Tier: deep* (ported-wasm (8 blocks, 71 items, mod.wasm))

### ✅ Lote 15 (Concluído)
- [x] **`aquaculture`** (Aquaculture 2) — *Tier: deep* (ported-wasm (3 blocks, 125 items, mod.wasm))
- [x] **`ars_elemental`** (Ars Elemental) — *Tier: deep* (ported-wasm (27 blocks, 147 items, mod.wasm))
- [x] **`ars_nouveau`** (Ars Nouveau) — *Tier: deep* (ported-wasm (185 blocks, 264 items, mod.wasm))
- [x] **`artifacts`** (Artifacts) — *Tier: deep* (ported-wasm (0 blocks, 54 items, mod.wasm))
- [x] **`astralsorcery`** (Astral Sorcery) — *Tier: deep* (ported-wasm (66 blocks, 72 items, mod.wasm))
- [x] **`auroral`** (Auroral) — *Tier: deep* (ported-wasm (5 blocks, 58 items, mod.wasm))
- [x] **`bellsandwhistles`** (Create: Bells & Whistles) — *Tier: deep* (ported-wasm (36 blocks, 0 items, mod.wasm))
- [x] **`biomeswevegone`** (Oh The Biomes We've Gone) — *Tier: deep* (ported-wasm (995 blocks, 81 items, mod.wasm))
- [x] **`buildinggadgets2`** (Building Gadgets 2) — *Tier: deep* (ported-wasm (6 blocks, 26 items, mod.wasm))
- [x] **`cataclysm`** (L_Ender's Cataclysm 1.21.1) — *Tier: deep* (ported-wasm (136 blocks, 170 items, mod.wasm))

### ✅ Lote 16 (Concluído)
- [x] **`compactmachines`** (Compact Machines) — *Tier: deep* (ported-wasm (4 blocks, 3 items, mod.wasm))
- [x] **`computercraft`** (CC: Tweaked) — *Tier: deep* (ported-wasm (5 blocks, 15 items, mod.wasm))
- [x] **`copycats`** (Create: Copycats+) — *Tier: deep* (ported-wasm (44 blocks, 2 items, mod.wasm))
- [x] **`create`** (Create) — *Tier: deep* (ported-wasm (542 blocks, 185 items, mod.wasm))
- [x] **`create_aquatic_ambitions`** (Create Aquatic Ambitions) — *Tier: deep* (ported-wasm (3 blocks, 8 items, mod.wasm))
- [x] **`create_dragons_plus`** (Create: Dragons Plus) — *Tier: deep* (ported-wasm (55 blocks, 56 items, mod.wasm))
- [x] **`create_enchantment_industry`** (Create: Enchantment Industry) — *Tier: deep* (ported-wasm (15 blocks, 18 items, mod.wasm))
- [x] **`create_hypertube`** (Create Hypertube) — *Tier: deep* (ported-wasm (3 blocks, 4 items, mod.wasm))
- [x] **`createaddition`** (Create Crafts & Additions) — *Tier: deep* (ported-wasm (15 blocks, 29 items, mod.wasm))
- [x] **`creeperoverhaul`** (Creeper Overhaul) — *Tier: deep* (ported-wasm (2 blocks, 17 items, mod.wasm))

### ✅ Lote 17 (Concluído)
- [x] **`deeperdarker`** (Deeper and Darker) — *Tier: deep* (ported-wasm (143 blocks, 211 items, mod.wasm))
- [x] **`domum_ornamentum`** (Domum Ornamentum) — *Tier: deep* (ported-wasm (91 blocks, 18 items, mod.wasm))
- [x] **`draconicevolution`** (Draconic Evolution) — *Tier: deep* (ported-wasm (45 blocks, 133 items, mod.wasm))
- [x] **`dummmmmmy`** (MmmMmmMmmMmm) — *Tier: deep* (ported-wasm (0 blocks, 1 items, mod.wasm))
- [x] **`dungeoncrawl`** (Dungeon Crawl) — *Tier: deep* (ported-wasm (2 worldgen files, mod.wasm))
- [x] **`dysoncubeproject`** (Dyson Cube Project) — *Tier: deep* (ported-wasm (3 blocks, 4 items, mod.wasm))
- [x] **`easy_villagers`** (Easy Villagers) — *Tier: deep* (ported-wasm (8 blocks, 1 items, mod.wasm))
- [x] **`enderio`** (Ender IO) — *Tier: deep* (ported-wasm (820 blocks, 131 items, mod.wasm))
- [x] **`endermanoverhaul`** (Enderman Overhaul) — *Tier: deep* (ported-wasm (0 blocks, 35 items, mod.wasm))
- [x] **`eternal_starlight`** (Eternal Starlight) — *Tier: deep* (ported-wasm (710 blocks, 489 items, mod.wasm))

### ✅ Lote 18 (Concluído)
- [x] **`evilcraft`** (EvilCraft) — *Tier: deep* (ported-wasm (43 blocks, 101 items, mod.wasm))
- [x] **`fireproofboats`** (Fireproof Boats) — *Tier: deep* (ported-wasm (0 blocks, 4 items, mod.wasm))
- [x] **`forbidden_arcanus`** (Forbidden Arcanus) — *Tier: deep* (ported-wasm (130 blocks, 131 items, mod.wasm))
- [x] **`framedblocks`** (FramedBlocks) — *Tier: deep* (ported-wasm (223 blocks, 12 items, mod.wasm))
- [x] **`gateways`** (Gateways To Eternity) — *Tier: deep* (ported-wasm (0 blocks, 3 items, mod.wasm))
- [x] **`iceandfire`** (Ice And Fire Community Edition) — *Tier: deep* (ported-wasm (106 blocks, 418 items, mod.wasm); waits on uranus)
- [x] **`illagerwarship`** (Illager-Warship) — *Tier: deep* (ported-wasm (5 worldgen files, mod.wasm))
- [x] **`immersiveengineering`** (Immersive Engineering) — *Tier: deep* (ported-wasm (352 blocks, 323 items, mod.wasm))
- [x] **`integratedcrafting`** (IntegratedCrafting) — *Tier: deep* (ported-wasm (2 blocks, 0 items, mod.wasm))
- [x] **`integrateddynamics`** (IntegratedDynamics) — *Tier: deep* (ported-wasm (53 blocks, 22 items, mod.wasm))

### ✅ Lote 19 (Concluído)
- [x] **`integratedscripting`** (IntegratedScripting) — *Tier: deep* (ported-wasm (3 blocks, 1 items, mod.wasm))
- [x] **`integratedterminals`** (IntegratedTerminals) — *Tier: deep* (ported-wasm (4 blocks, 1 items, mod.wasm))
- [x] **`integratedtunnels`** (IntegratedTunnels) — *Tier: deep* (ported-wasm (3 blocks, 1 items, mod.wasm))
- [x] **`irons_spellbooks`** (Iron's Spells 'n Spellbooks) — *Tier: deep* (ported-wasm (21 blocks, 300 items, mod.wasm))
- [x] **`justdirethings`** (Just Dire Things) — *Tier: deep* (ported-wasm (58 blocks, 343 items, mod.wasm))
- [x] **`little_big_redstone`** (Little Big Redstone) — *Tier: deep* (ported-wasm (16 blocks, 70 items, mod.wasm))
- [x] **`livingthings`** (Living Things) — *Tier: deep* (ported-wasm (0 blocks, 37 items, mod.wasm))
- [x] **`mahoutsukai`** (Mahou Tsukai) — *Tier: deep* (ported-wasm (61 blocks, 113 items, mod.wasm))
- [x] **`mekanism`** (Mekanism) — *Tier: deep* (ported-wasm (125 blocks, 219 items, mod.wasm))
- [x] **`mekanismgenerators`** (Mekanism: Generators) — *Tier: deep* (ported-wasm (24 blocks, 9 items, mod.wasm))

### ✅ Lote 20 (Concluído - GPT)
- [x] **`mffs`** (Modular Force Field System) — *Tier: deep* (ported-wasm (1 blocks, 42 items, mod.wasm))
- [x] **`minecolonies`** (MineColonies) — *Tier: deep* (ported-wasm (87 blocks, 147 items, mod.wasm))
- [x] **`mininggadgets`** (Mining Gadgets) — *Tier: deep* (ported-wasm (3 blocks, 27 items, mod.wasm))
- [x] **`modern_industrialization`** (Modern Industrialization) — *Tier: deep* (ported-wasm (306 blocks, 615 items, mod.wasm))
- [x] **`naturesaura`** (NaturesAura) — *Tier: deep* (ported-wasm (77 blocks, 77 items, mod.wasm))
- [x] **`neovitae`** (Neo Vitae) — *Tier: deep* (ported-wasm (210 blocks, 397 items, mod.wasm))
- [x] **`occultism`** (Occultism) — *Tier: deep* (ported-wasm (129 blocks, 457 items, mod.wasm))
- [x] **`oritech`** (Oritech) — *Tier: deep* (ported-wasm (161 blocks, 129 items, mod.wasm))
- [x] **`pneumaticcraft`** (PneumaticCraft: Repressurized) — *Tier: deep* (ported-wasm (112 blocks, 168 items, mod.wasm))
- [x] **`productivebees`** (Productive Bees) — *Tier: deep* (ported-wasm (51 blocks, 96 items, mod.wasm))

### ✅ Lote 21 (Concluído)
- [x] **`railcraft`** (Railcraft Reborn) — *Tier: deep* (ported-wasm (218 blocks, 165 items, mod.wasm))
- [x] **`refinedstorage`** (Refined Storage) — *Tier: deep* (ported-wasm (46 blocks, 75 items, mod.wasm))
- [x] **`refurbished_furniture`** (MrCrayfish's Furniture Mod: Refurbished) — *Tier: deep* (ported-wasm (360 blocks, 28 items, mod.wasm))
- [x] **`relics`** (Relics) — *Tier: deep* (ported-wasm (1 blocks, 94 items, mod.wasm))
- [x] **`reliquary`** (Reliquary Reincarnations) — *Tier: deep* (ported-wasm (21 blocks, 100 items, mod.wasm))
- [x] **`rftoolsbuilder`** (RFToolsBuilder) — *Tier: deep* (ported-wasm (20 blocks, 16 items, mod.wasm))
- [x] **`rootsclassic`** (Roots Classic) — *Tier: deep* (ported-wasm (18 blocks, 48 items, mod.wasm))
- [x] **`securitycraft`** (SecurityCraft) — *Tier: deep* (ported-wasm (620 blocks, 83 items, mod.wasm))
- [x] **`sfm`** (Super Factory Manager (SFM)) — *Tier: deep* (ported-wasm (11 blocks, 7 items, mod.wasm))
- [x] **`shrink`** (Shrink) — *Tier: deep* (ported-wasm (0 blocks, 4 items, mod.wasm))

### ✅ Lote 22 (Concluído)
- [x] **`silentgear`** (Silent Gear) — *Tier: deep* (ported-wasm (63 blocks, 257 items, mod.wasm))
- [x] **`sophisticatedbackpacks`** (Sophisticated Backpacks) — *Tier: deep* (ported-wasm (6 blocks, 73 items, mod.wasm))
- [x] **`stevescarts`** (StevesCarts2) — *Tier: deep* (ported-wasm (11 blocks, 171 items, mod.wasm))
- [x] **`supplementaries`** (Supplementaries) — *Tier: deep* (ported-wasm (154 blocks, 87 items, mod.wasm))
- [x] **`tempad`** (Tempad) — *Tier: deep* (ported-wasm (4 blocks, 61 items, mod.wasm))
- [x] **`the_bumblezone`** (The Bumblezone) — *Tier: deep* (ported-wasm (52 blocks, 178 items, mod.wasm))
- [x] **`theurgy`** (Theurgy) — *Tier: deep* (ported-wasm (15 blocks, 440 items, mod.wasm))
- [x] **`tombstone`** (Corail Tombstone) — *Tier: deep* (ported-wasm (21 blocks, 114 items, mod.wasm))
- [x] **`twilightforest`** (The Twilight Forest) — *Tier: deep* (ported-wasm (411 blocks, 586 items, mod.wasm))
- [x] **`undergarden`** (The Undergarden) — *Tier: deep* (ported-wasm (230 blocks, 151 items, mod.wasm))

### ✅ Lote 23 (Concluído)
- [x] **`variantsandventures`** (Variants&Ventures) — *Tier: deep* (ported-wasm (0 blocks, 4 items, mod.wasm))
- [x] **`betterdeserttemples`** (YUNG's Better Desert Temples) — *Tier: data* (ported-wasm (229 worldgen files, mod.wasm))
- [x] **`betterdungeons`** (YUNG's Better Dungeons) — *Tier: data* (ported-wasm (282 worldgen files, mod.wasm))
- [x] **`betterfortresses`** (YUNG's Better Nether Fortresses) — *Tier: data* (ported-wasm (187 worldgen files, mod.wasm))
- [x] **`betterjungletemples`** (YUNG's Better Jungle Temples) — *Tier: data* (ported-wasm (148 worldgen files, mod.wasm))
- [x] **`bettermineshafts`** (YUNG's Better Mineshafts) — *Tier: data* (ported-wasm (14 worldgen files, mod.wasm))
- [x] **`betterstrongholds`** (YUNG's Better Strongholds) — *Tier: data* (ported-wasm (120 worldgen files, mod.wasm))
- [x] **`betterwitchhuts`** (YUNG's Better Witch Huts) — *Tier: data* (ported-wasm (14 worldgen files, mod.wasm))
- [x] **`chipped`** (Chipped) — *Tier: data* (ported-wasm (6936 blocks, 12 items, mod.wasm))
- [x] **`crystalix`** (Crystalix) — *Tier: data* (ported-wasm (0 blocks, 1 items, mod.wasm))

### ✅ Lote 24 (Concluído)
- [x] **`ctov`** (ChoiceTheorem's Overhauled Village) — *Tier: data* (ported-wasm (2628 worldgen files, mod.wasm))
- [x] **`everythingcopper`** (Everything is Copper) — *Tier: data* (ported-wasm (89 blocks, 247 items, mod.wasm))
- [x] **`factory_blocks`** (Factory Blocks) — *Tier: data* (ported-wasm (49 blocks, 1 items, mod.wasm))
- [x] **`formationsnether`** (Formations Nether) — *Tier: data* (ported-wasm (81 worldgen files, mod.wasm))
- [x] **`formationsoverworld`** (Formations Overworld) — *Tier: data* (ported-wasm (146 worldgen files, mod.wasm))
- [x] **`glassential`** (Glassential-renewed) — *Tier: data* (ported-wasm (110 blocks, 1 items, mod.wasm))
- [x] **`herbsandharvest`** (Mama's Herbs and Harvest) — *Tier: data* (ported-wasm (106 blocks, 344 items, mod.wasm))
- [x] **`mcwbridges`** (Macaw's Bridges) — *Tier: data* (ported-wasm (42 blocks, 1 items, mod.wasm))
- [x] **`mcwdoors`** (Macaw's Doors) — *Tier: data* (ported-wasm (240 blocks, 14 items, mod.wasm))
- [x] **`mcwfences`** (Macaw's Fences and Walls) — *Tier: data* (ported-wasm (150 blocks, 0 items, mod.wasm))

### ✅ Lote 25 (Concluído)
- [x] **`mcwholidays`** (Macaw's Holidays) — *Tier: data* (ported-wasm (308 blocks, 1 items, mod.wasm))
- [x] **`mcwpaths`** (Macaw's Paths and Pavings) — *Tier: data* (ported-wasm (258 blocks, 0 items, mod.wasm))
- [x] **`mcwroofs`** (Macaw's Roofs) — *Tier: data* (ported-wasm (493 blocks, 2 items, mod.wasm))
- [x] **`mcwstairs`** (Macaw's Stairs and Balconies) — *Tier: data* (ported-wasm (168 blocks, 0 items, mod.wasm))
- [x] **`mns`** (MoogsNetherStructures) — *Tier: data* (ported-wasm (750 worldgen files, mod.wasm))
- [x] **`mostructures`** (Mo' Structures) — *Tier: data* (ported-wasm (227 worldgen files, mod.wasm))
- [x] **`regions_unexplored`** (Regions Unexplored) — *Tier: data* (ported-wasm (805 blocks, 41 items, mod.wasm))
- [x] **`repurposed_structures`** (Repurposed Structures) — *Tier: data* (ported-wasm (5025 worldgen files, mod.wasm))
- [x] **`simplylight`** (Simply Light) — *Tier: data* (ported-wasm (112 blocks, 0 items, mod.wasm))
- [x] **`villagesandpillages`** (Villages&Pillages) — *Tier: data* (ported-wasm (39 worldgen files, mod.wasm))

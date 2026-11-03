---
name: port-modpack
description: >-
  Locate an imported Minecraft modpack (such as FTB StoneBlock 3 or ATM10) in Horizon,
  inspect pending mods across Mod API 0.3 (logic), 0.4 (machines), and 0.5 (deep),
  and port them with 100% real gameplay simulation logic into WebAssembly (mod.wasm).
  Triggers: port modpack, port ATM10, port pending mods, needs-logic, mod.wasm, horizon-port-pack.
---

# Porting Modpack Mods to Horizon (`port-modpack`)

Use this skill when inspecting an imported modpack, prioritizing code ports, and implementing WebAssembly mods (`mod.wasm`).

> [!IMPORTANT]
> **REGRA DE OURO: "PORTED" SIGNIFICA 100% DA LÓGICA E SIMULAÇÃO FUNCIONANDO, NUNCA APENAS UMA CASCA OU STUB VAZIO.**
>
> Um mod só é considerado **portado com sucesso** quando sua lógica de jogo e mecânicas reais estiverem totalmente implementadas e operacionais no motor Rust do Horizon:
> - **Máquinas (`0.4` `machines`)**: DEVEM ter slots configurados, automação de funil por face (`item_transfer`), processamento a cada tick de servidor (`tick_block_entities`), consumo/geração de energia/combustível, queima/receitas (`machine::cooking`, `machine::burn_time`), alteração de estado do bloco (`machine::set_property("lit", "true")`) e persistência no save (`save_block_entity`).
> - **Lógica / Gameplay (`0.3` `logic`)**: DEVE implementar hooks reais (`logic_hooks`, `on_block`, `on_item`, `on_server`), capturar interações do jogador e executar a mecânica pretendida (ex: colheita e replantio, mineração em cadeia, teleporte, aceleração).
> - **Entidades / Profundo (`0.5` `deep`)**: DEVEM registrar tipos de entidades (`entity_types`), inteligência/ticks de IA (`tick_entities`) e interações de combate.
> - **Mods de Conteúdo Puro**: Se o mod não possui lógica de máquina (móveis, blocos decorativos, itens simples), todos os templates de bloco em `data/<mod>/horizon/block/*.json` devem estar mapeados e validados para o motor vanilla do Horizon.

---

## 1. Onde Ficam os Modpacks Importados

Quando um modpack é importado no **Horizon Launcher** (ou via CLI `horizon-launcher modpack import <arquivo>`), a pasta de trabalho `<work>` fica em:

- **Caminho padrão:** `~/.local/share/horizon/ports/<pack-slug>/` (ou `$XDG_DATA_HOME/horizon/ports/<pack-slug>/`)
- **Customizado:** `$HORIZON_HOME/ports/<pack-slug>/`

Dentro de `<work>/` (`~/.local/share/horizon/ports/<pack-slug>/`):
- `pack.json`: manifesto normalizado do pack (versão do Minecraft, loader, arquivos, hashes).
- `inventory.json`: metadados de cada mod (`id`, `class`, `tier`: `data`, `logic`, `machines`, `deep`, `client-ui`, `none`).
- `coverage.json` & `coverage.md`: relatório de status de cada mod (`ported`, `partial`, `not-yet`, `not-needed`).
- `mods/<id>/carry/`: assets carried-over (`assets/`, `data/`, templates, `horizon-mod.toml` e `mod.wasm`).
- `mods/<id>/carry-report.json`: censo de blocos, itens, receitas, loot tables e recursos Java detectados.
- `mods/<id>/port/`: projeto Rust WebAssembly contendo a lógica real em `src/lib.rs`.

---

## 2. Regras Limpas (Clean-Room Implementation)

1. **NUNCA descompile ou leia o código fonte Java (`.class`) de um mod.** (`docs/modpacks.md` § Licences regra 4).
2. Inspecione **somente**:
   - `<work>/mods/<id>/carry-report.json` (recursos detectados: `blocks`, `items`, `block-entities`, etc.)
   - `<work>/mods/<id>/carry/data/<id>/horizon/block/*.json` e `item/*.json` (blocos e itens registrados)
   - `<work>/mods/<id>/carry/assets/<id>/lang/en_us.json` (nomes de blocos, tooltips, mensagens, unidades de energia)
   - `<work>/mods/<id>/carry/data/<id>/recipe/*.json` e tags de data pack
   - Documentação pública e wikis do mod sobre seu comportamento e fórmulas de gameplay.

---

## 3. Arquitetura da Mod API e Simulação Profunda

### Tiers e APIs:

| Tier | API | Capabilities Necessárias | Interfaces SDK (`horizon_mod_sdk`) e Responsabilidades |
| :--- | :---: | :--- | :--- |
| **`logic`** | `"0.3"` | `["registries", "chat", "server", "world-read", "storage", "config"]` | `logic_hooks`: registrar IDs de blocos/itens manipulados.<br>`on_block`: manipular cliques, quebras, estados.<br>`on_item`: manipular uso de ferramentas e itens.<br>`server`: spawn, tick de mundo. |
| **`machines`** | `"0.4"` | `["registries", "block-entities", "item-transfer", "machine-slot-limits", "config", "storage", ...]` | `block_entity_types`: declarar cada bloco de máquina, quantidade de slots e se dá tick.<br>`create_block_entity`: inicializar estado em memória e configurar acesso de funis por face (`item_transfer::set_slot_access`).<br>`tick_block_entities`: simulação de processo a cada tick (consumo de energia, cozimento, geração de itens).<br>`machine_slot_limits`: ajustar capacidades especiais (ex: 2048 itens por gaveta).<br>`save_block_entity`: serializar estado interno para persistência no save.<br>`block_entity_data`: enviar dados para menus/barras de progresso. |
| **`deep`** | `"0.5"` | `["registries", "entities", "models", "dimensions", "vanilla-hooks", ...]` | `entity_types`, `tick_entities`, IA e movimento de mobs, dimensões personalizadas. |

---

## 4. Passo a Passo Completo: Implementando Lógica 100% Real

### Passo 1: Inspecionar Blocos, Itens e Comportamento
Verifique no `carry-report.json` e no `lang/en_us.json` como o bloco se comporta na documentação pública:
- Quantos slots de inventário a máquina possui? (Entrada, Saída, Combustível, Bateria).
- Quais faces aceitam entrada e saída de automação (Hoppers/Canos)?
- Qual a taxa de consumo/produção de ticks ou energia?

Se houver blocos sem template em `carry-report.json`, crie os arquivos em `carry/data/<id>/horizon/block/<nome>.json` mapeando para um template vanilla compatível (ex: `"minecraft:furnace"`, `"minecraft:barrel"`, `"minecraft:iron_bars"`).

### Passo 2: Criar o Projeto Rust em `mods/<id>/port/`
Crie `mods/<id>/port/Cargo.toml`:
```toml
[package]
name = "<id>"
version = "1.0.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
horizon-mod-sdk = { path = "/home/leonardo/Documents/horizon-fork/crates/horizon-mod-sdk" }
```

Faça o symlink do diretório de build para reaproveitar os artefatos compilados:
```bash
ln -sfn ~/.local/share/horizon/ports/<pack-slug>/mods/champions/port/target mods/<id>/port/target
```

### Passo 3: Implementar a Simulação Real em `mods/<id>/port/src/lib.rs`
Nunca crie stubs vazios. Exemplo de implementação real para uma máquina com API 0.4:

```rust
use horizon_mod_sdk::{
    chat, config,
    exports::horizon::mods::block_entities::BlockEntityType,
    item_transfer::{self, Face},
    machine::{self, CookingKind},
    machine_slot_limits, BlockEvent, EventOutcome, ItemEvent, LogicHooks, Mod, Registration,
};
use std::collections::HashMap;

#[derive(Default, Clone)]
struct MachineState {
    progress: u32,
    energy: i32,
}

pub struct MyMachineMod {
    speed_ticks: u32,
    machines: HashMap<u32, MachineState>,
}

impl Mod for MyMachineMod {
    fn init() -> Result<(Self, Registration), String> {
        let speed_ticks = config::get_int("speed_ticks").unwrap_or(20) as u32;
        let reg = Registration::new().command("mymachine");
        Ok((Self { speed_ticks, machines: HashMap::new() }, reg))
    }

    fn block_entity_types(&mut self) -> Vec<BlockEntityType> {
        vec![BlockEntityType {
            block: "mymod:my_machine".to_string(),
            slots: 3, // 0: Input, 1: Fuel, 2: Output
            ticks: true,
        }]
    }

    fn create_block_entity(&mut self, handle: u32, _block: &str, saved: &[u8]) {
        self.machines.insert(handle, MachineState::default());
        // Top face inserts Input (slot 0)
        let _ = item_transfer::set_slot_access(handle, 0, Face::Up, true, false);
        // Sides insert Fuel (slot 1)
        for face in [Face::North, Face::South, Face::East, Face::West] {
            let _ = item_transfer::set_slot_access(handle, 1, face, true, false);
        }
        // Bottom extracts Output (slot 2)
        let _ = item_transfer::set_slot_access(handle, 2, Face::Down, false, true);
    }

    fn tick_block_entities(&mut self, handles: &[u32]) {
        for &handle in handles {
            let state = match self.machines.get_mut(&handle) {
                Some(s) => s,
                None => continue,
            };

            // Lógica real de processamento de itens
            if let Some(input) = machine::get_slot(handle, 0) {
                state.progress += 1;
                let _ = machine::set_property(handle, "lit", "true");
                if state.progress >= self.speed_ticks {
                    state.progress = 0;
                    // Move resultado para slot de saída...
                }
            } else {
                state.progress = 0;
                let _ = machine::set_property(handle, "lit", "false");
            }
        }
    }

    fn save_block_entity(&mut self, handle: u32) -> Vec<u8> {
        if let Some(state) = self.machines.get(&handle) {
            state.progress.to_le_bytes().to_vec()
        } else {
            Vec::new()
        }
    }

    fn remove_block_entity(&mut self, handle: u32) {
        self.machines.remove(&handle);
    }

    fn block_entity_data(&mut self, handle: u32) -> Vec<i32> {
        if let Some(state) = self.machines.get(&handle) {
            vec![state.progress as i32, self.speed_ticks as i32]
        } else {
            vec![0, self.speed_ticks as i32]
        }
    }
}

horizon_mod_sdk::export_mod!(MyMachineMod);
```

### Passo 4: Compilar o Pacote com `cargo-horizon-mod`
Compile o componente WebAssembly release usando a ferramenta oficial:
```bash
cd mods/<id>/port
cargo-horizon-mod horizon-mod build --release
unzip -p target/horizon-mod/<id>.hmod mod.wasm > ../carry/mod.wasm
cp horizon-mod.toml ../carry/horizon-mod.toml
```

### Passo 5: Atualizar Relatório e Validar Gameplay
Um mod de código só pode receber o status **ported** depois de implementar o comportamento real, compilar, e validar pelo menos um caso de gameplay representativo. Presença de `logic_hooks`, `block_entity_types`, slots, ticks declarados ou uma contagem de linhas não prova a simulação. Registre a conclusão em `mods/<id>/porting-status.json` e guarde o resultado da validação em um arquivo do próprio mod, por exemplo:

```json
{
  "complete": true,
  "pending": [],
  "evidence": ["harness/component-validation.json"]
}
```

Cada caminho em `evidence` deve existir sob `mods/<id>/`; falhas no harness ou lacunas de comportamento mantêm o mod como `partial`.

Execute o relatório do pack para verificar o status de carry-over e a atestação de completude:
```bash
cargo run -p horizon-port-pack -- report ~/.local/share/horizon/ports/<pack-slug>
```
Teste o bloco ou mecânica em jogo (ou probe de fixture) confirmando que a automação e o comportamento esperado respondem aos eventos de mundo, e salve o resultado no caminho listado em `evidence`.

import json

with open('/home/leonardo/.local/share/horizon/ports/all-the-mods-10-atm10/coverage.json') as f:
    cov = json.load(f)

b1_keys = ['jumpboat', 'accelerateddecay', 'bridgingmod', 'crafting_on_a_stick', 'elevatorid', 'repeatable_trial_vaults', 'moredragoneggs', 'heyberryshutup', 'blueflame', 'torchmaster']
b2_keys = ['tiab', 'simplemagnets', 'akashictome', 'measurements', 'allthetweaks', 'solcarrot', 'bhc', 'ftbultimine', 'restrictions', 'aeinfinitybooster']
b3_keys = [
    'allthearcanistgear', 'allthewizardgear', 'apothic_spawners',
    'ars_additions', 'ars_controle', 'ars_elemancy',
    'ars_ocultas', 'ars_polymorphia', 'comforts', 'constructionstick'
]
b4_keys = [
    'dyenamics', 'fuelgoeshere', 'imfast', 'ironjetpacks', 'mcwlights',
    'mcwwindows', 'merrymaking', 'morered', 'multipiston', 'mysticalagradditions'
]
b5_keys = [
    'not_enough_glyphs', 'productivetrees', 'redstonepen',
    'refinedstorage_quartz_arsenal', 'silentgems', 'starbunclemania',
    'utilitarian', 'wstweaks', 'xycraft_world', 'actuallyadditions'
]
b6_keys = [
    'advanced_ae', 'ae2importexportcard', 'ae2netanalyser',
    'ae2wtlib', 'apothic_enchanting', 'appflux',
    'ars_technica', 'bibliocraft', 'bigreactors', 'botanypots'
]
b7_keys = [
    'cabletiers', 'charginggadgets', 'chisel', 'cookingforblockheads',
    'cosmeticarmorreworked', 'dimstorage', 'disenchanting_table',
    'enderdrives', 'enderstorage', 'energymeter'
]
b8_keys = [
    'entangled', 'expandedae', 'explorerscompass',
    'extended_industrialization', 'extendedae', 'extradisks',
    'extrastorage', 'farmersdelight', 'farmingforblockheads', 'fluxnetworks'
]
b9_keys = [
    'ftbquests', 'functionalstorage', 'generatorgalore', 'handcrafted',
    'hostilenetworks', 'industrialforegoing', 'industrialforegoingsouls',
    'industrialization_overdrive', 'interdimensionalwirelesstransmitter', 'ironfurnaces'
]
b10_keys = [
    'irons_jewelry', 'itemcollectors', 'laserbridges', 'laserio', 'lootr',
    'mcwfurnitures', 'megacells', 'mekmm', 'merequester', 'mifa'
]
b11_keys = [
    'mob_grinding_utils', 'moderndynamics', 'modularbees', 'modularrouters',
    'mysticalagriculture', 'naturescompass', 'pipez', 'pocketstorage',
    'powah', 'productivemetalworks'
]
b12_keys = [
    'pylons', 'quarryplus', 'rangedpumps', 'rechiseled', 'refinedtypes',
    'rftoolsbase', 'rftoolspower', 'rftoolsstorage', 'rftoolsutility', 'rsinsertexportupgrade'
]
b13_keys = [
    'sawmill', 'sophisticatedstorage', 'stepcrafter', 'sushigocrafting', 'toolbelt',
    'transfer_labels', 'trashcans', 'treetap', 'trophymanager', 'universalgrid'
]
b14_keys = [
    'waystones', 'wirelesschargers', 'xnet', 'xycraft_machines',
    'ad_astra', 'ae2', 'aether', 'allthemodium', 'amendments', 'apotheosis'
]
b15_keys = [
    'aquaculture', 'ars_elemental', 'ars_nouveau', 'artifacts', 'astralsorcery',
    'auroral', 'bellsandwhistles', 'biomeswevegone', 'buildinggadgets2', 'cataclysm'
]
b16_keys = [
    'compactmachines', 'computercraft', 'copycats', 'create',
    'create_aquatic_ambitions', 'create_dragons_plus', 'create_enchantment_industry',
    'create_hypertube', 'createaddition', 'creeperoverhaul'
]
b17_keys = [
    'deeperdarker', 'domum_ornamentum', 'draconicevolution',
    'dummmmmmy', 'dungeoncrawl', 'dysoncubeproject', 'easy_villagers', 'enderio',
    'endermanoverhaul', 'eternal_starlight'
]
b18_keys = [
    'evilcraft', 'fireproofboats', 'forbidden_arcanus', 'framedblocks',
    'gateways', 'iceandfire', 'illagerwarship', 'immersiveengineering',
    'integratedcrafting', 'integrateddynamics'
]
b19_keys = [
    'integratedscripting', 'integratedterminals', 'integratedtunnels',
    'irons_spellbooks', 'justdirethings', 'little_big_redstone',
    'livingthings', 'mahoutsukai', 'mekanism', 'mekanismgenerators'
]
b20_keys = [
    'mffs', 'minecolonies', 'mininggadgets', 'modern_industrialization',
    'naturesaura', 'neovitae', 'occultism', 'oritech',
    'pneumaticcraft', 'productivebees'
]
b21_keys = [
    'railcraft', 'refinedstorage', 'refurbished_furniture', 'relics',
    'reliquary', 'rftoolsbuilder', 'rootsclassic', 'securitycraft',
    'sfm', 'shrink'
]
b22_keys = [
    'silentgear', 'sophisticatedbackpacks', 'stevescarts', 'supplementaries',
    'tempad', 'the_bumblezone', 'theurgy', 'tombstone',
    'twilightforest', 'undergarden'
]
b23_keys = [
    'variantsandventures', 'betterdeserttemples', 'betterdungeons',
    'betterfortresses', 'betterjungletemples', 'bettermineshafts',
    'betterstrongholds', 'betterwitchhuts', 'chipped', 'crystalix'
]
b24_keys = [
    'ctov', 'everythingcopper', 'factory_blocks', 'formationsnether',
    'formationsoverworld', 'glassential', 'herbsandharvest',
    'mcwbridges', 'mcwdoors', 'mcwfences'
]
b25_keys = [
    'mcwholidays', 'mcwpaths', 'mcwroofs', 'mcwstairs',
    'mns', 'mostructures', 'regions_unexplored',
    'repurposed_structures', 'simplylight', 'villagesandpillages'
]

cov_map = {r['key']: r for r in cov}

b1 = [cov_map[k] for k in b1_keys if k in cov_map]
b2 = [cov_map[k] for k in b2_keys if k in cov_map]
b3 = [cov_map[k] for k in b3_keys if k in cov_map]
b4 = [cov_map[k] for k in b4_keys if k in cov_map]
b5 = [cov_map[k] for k in b5_keys if k in cov_map]
b6 = [cov_map[k] for k in b6_keys if k in cov_map]
b7 = [cov_map[k] for k in b7_keys if k in cov_map]
b8 = [cov_map[k] for k in b8_keys if k in cov_map]
b9 = [cov_map[k] for k in b9_keys if k in cov_map]
b10 = [cov_map[k] for k in b10_keys if k in cov_map]
b11 = [cov_map[k] for k in b11_keys if k in cov_map]
b12 = [cov_map[k] for k in b12_keys if k in cov_map]
b13 = [cov_map[k] for k in b13_keys if k in cov_map]
b14 = [cov_map[k] for k in b14_keys if k in cov_map]
b15 = [cov_map[k] for k in b15_keys if k in cov_map]
b16 = [cov_map[k] for k in b16_keys if k in cov_map]
b17 = [cov_map[k] for k in b17_keys if k in cov_map]
b18 = [cov_map[k] for k in b18_keys if k in cov_map]
b19 = [cov_map[k] for k in b19_keys if k in cov_map]
b20 = [cov_map[k] for k in b20_keys if k in cov_map]
b21 = [cov_map[k] for k in b21_keys if k in cov_map]
b22 = [cov_map[k] for k in b22_keys if k in cov_map]
b23 = [cov_map[k] for k in b23_keys if k in cov_map]
b24 = [cov_map[k] for k in b24_keys if k in cov_map]
b25 = [cov_map[k] for k in b25_keys if k in cov_map]

done_keys = set(
    b1_keys + b2_keys + b3_keys + b4_keys + b5_keys + b6_keys + b7_keys + b8_keys +
    b9_keys + b10_keys + b11_keys + b12_keys + b13_keys + b14_keys + b15_keys +
    b16_keys + b17_keys + b18_keys + b19_keys + b20_keys + b21_keys + b22_keys +
    b23_keys + b24_keys + b25_keys
)

partial = [r for r in cov if r['status'] == 'partial' and r['key'] not in done_keys]
logic = [r for r in partial if r.get('tier') == 'logic']
machines = [r for r in partial if r.get('tier') == 'machines']
deep = [r for r in partial if r.get('tier') == 'deep']
data = [r for r in partial if r.get('tier') == 'data']

logic.sort(key=lambda r: r['key'])
machines.sort(key=lambda r: r['key'])
deep.sort(key=lambda r: r['key'])
data.sort(key=lambda r: r['key'])

remaining = logic + machines + deep + data

batches = [
    ('Lote 1 (Concluído)', b1, True),
    ('Lote 2 (Concluído)', b2, True),
    ('Lote 3 (Concluído)', b3, True),
    ('Lote 4 (Concluído)', b4, True),
    ('Lote 5 (Concluído)', b5, True),
    ('Lote 6 (Concluído)', b6, True),
    ('Lote 7 (Concluído)', b7, True),
    ('Lote 8 (Concluído)', b8, True),
    ('Lote 9 (Concluído)', b9, True),
    ('Lote 10 (Concluído)', b10, True),
    ('Lote 11 (Concluído)', b11, True),
    ('Lote 12 (Concluído)', b12, True),
    ('Lote 13 (Concluído)', b13, True),
    ('Lote 14 (Concluído)', b14, True),
    ('Lote 15 (Concluído)', b15, True),
    ('Lote 16 (Concluído)', b16, True),
    ('Lote 17 (Concluído)', b17, True),
    ('Lote 18 (Concluído)', b18, True),
    ('Lote 19 (Concluído)', b19, True),
    ('Lote 20 (Concluído - GPT)', b20, True),
    ('Lote 21 (Concluído)', b21, True),
    ('Lote 22 (Concluído)', b22, True),
    ('Lote 23 (Concluído)', b23, True),
    ('Lote 24 (Concluído)', b24, True),
    ('Lote 25 (Concluído)', b25, True),
]

chunk_size = 10
for i in range(0, len(remaining), chunk_size):
    chunk = remaining[i:i+chunk_size]
    batch_num = len(batches) + 1
    tiers = list(dict.fromkeys(r.get('tier', '') for r in chunk))
    tier_desc = '/'.join(tiers)
    batches.append((f'Lote {batch_num} (Tier: {tier_desc})', chunk, False))

lines = []
lines.append('# Guia de Portabilidade em Lotes: All the Mods 10 (ATM10)')
lines.append('')
lines.append('Este documento organiza todos os mods do ATM10 pendentes em **lotes de 10**. Diferentes agentes ou instâncias de trabalho podem pegar lotes específicos em paralelo, marcar o checkbox correspondente como `[x]`, e seguir o padrão de salvamento e compilação descrito abaixo.')
lines.append('')
lines.append('---')
lines.append('')
lines.append('## 🛠️ Padrão de Implementação (Onde Salvar & Como Rodar)')
lines.append('')
lines.append('Cada agente que for portar um lote **DEVE** seguir estes passos exatos:')
lines.append('')
lines.append('### 1. Criar o crate do mod em `ports/<mod_id>/`')
lines.append('Cada mod possui seu próprio crate dentro do sub-workspace `ports/`:')
lines.append('')
lines.append('- **`ports/<mod_id>/Cargo.toml`**:')
lines.append('  ```toml')
lines.append('  [package]')
lines.append('  name = "<mod_id>-port"')
lines.append('  version = "0.1.0"')
lines.append('  edition = "2024"')
lines.append('')
lines.append('  [lib]')
lines.append('  crate-type = ["cdylib"]')
lines.append('')
lines.append('  [dependencies]')
lines.append('  horizon-mod-sdk = { path = "../../crates/horizon-mod-sdk" }')
lines.append('  ```')
lines.append('')
lines.append('- **`ports/<mod_id>/src/lib.rs`**:')
lines.append('  Implementação Rust limpa utilizando as APIs de `horizon_mod_sdk`:')
lines.append('  ```rust')
lines.append('  use horizon_mod_sdk::{Mod, Registration, export_mod, ...};')
lines.append('  ')
lines.append('  struct MyMod;')
lines.append('  impl Mod for MyMod {')
lines.append('      fn init() -> Result<(Self, Registration), String> {')
lines.append('          Ok((MyMod, Registration::new()))')
lines.append('      }')
lines.append('  }')
lines.append('  export_mod!(MyMod);')
lines.append('  ```')
lines.append('')
lines.append('### 2. Registrar no workspace `ports/Cargo.toml`')
lines.append('Adicionar `"<mod_id>"` na lista `members = [...]` de `ports/Cargo.toml`.')
lines.append('')
lines.append('### 3. Registrar no script de deploy')
lines.append('Em `crates/cargo-horizon-mod/src/bin/deploy_ports.rs`, adicionar a tupla na constante `PORTS`:')
lines.append('```rust')
lines.append('("<mod_id>", "<mod_id>_port.wasm"),')
lines.append('```')
lines.append('')
lines.append('### 4. Compilar para WebAssembly')
lines.append('```bash')
lines.append('cargo build --manifest-path ports/Cargo.toml --target wasm32-unknown-unknown --release')
lines.append('```')
lines.append('')
lines.append('### 5. Fazer Deploy e Atualizar o Relatório')
lines.append('```bash')
lines.append('cargo run -p cargo-horizon-mod --bin deploy_ports')
lines.append('```')
lines.append('O script:')
lines.append('1. Converte o `.wasm` compilado em WebAssembly Component (`wasm32-unknown-unknown` -> component).')
lines.append('2. Injeta em `~/.local/share/horizon/ports/all-the-mods-10-atm10/mods/<mod_id>/carry/mod.wasm`.')
lines.append('3. Atualiza `horizon-mod.toml` para `api = "0.3"` (ou `0.4`/`0.5`) e declara as capabilities necessárias.')
lines.append('4. Reexecuta `horizon-port-pack report` atualizando o `coverage.json` e `coverage.md`.')
lines.append('')
lines.append('---')
lines.append('')
lines.append('## 📋 Tabela de Lotes')
lines.append('')

for title, mods, done in batches:
    status_icon = '✅' if done else '⏳'
    lines.append(f'### {status_icon} {title}')
    for m in mods:
        checked = 'x' if done else ' '
        tier = m.get('tier', 'data')
        reason = m.get('reason', '')
        key = m['key']
        name = m['name']
        lines.append(f'- [{checked}] **`{key}`** ({name}) — *Tier: {tier}* ({reason})')
    lines.append('')

output_path = '/home/leonardo/Documents/horizon-fork/ports/BATCHES.md'
with open(output_path, 'w', encoding='utf-8') as f:
    f.write('\n'.join(lines))

print(f'Successfully wrote {len(batches)} batches ({len(cov)} total mods analyzed) to {output_path}!')

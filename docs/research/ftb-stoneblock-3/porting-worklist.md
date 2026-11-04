# FTB StoneBlock 3 — worklist em sessões de 10 mods

Pack: Minecraft 1.18.2 / Forge 40.2.34; 223 entradas, renumeradas de 1 a 223 em 23 sessões pela progressão das quests. A instância importada e seus JARs ficam no diretório local do pack; esta worklist registra o plano e o estado auditado, sem copiar os JARs para o repositório.

## Resumo e critério atual

- A cobertura legada registrava 92 `ported`, 78 `partial` e 53 `not-needed`. Regerei o relatório com `cargo run -p horizon-port-pack -- report ...`; o resultado atual é **6 `ported`, 164 `partial` e 53 `not-needed`** (223 entradas). 86 dos antigos `ported` voltaram a `partial`.
- Esta worklist aplica ainda a regra da skill `port-modpack`: o status precisa representar a simulação real, não só a existência do WASM. Assim, há **5 confirmados para esta worklist**, **165 parciais/aguardando auditoria** e **53 não necessários**. Os cinco são `accelerateddecay` (lógica com probe e E2E), `botanytrees` (data carry), `mcwfences`, `mcwpaths` e `mcwtrpdoors` (data carry completo). `jade` aparece `ported` no relatório porque o tier `client` não exige atestação de completude no gerador atual; a worklist mantém `partial`, pois o protótipo cobre só HUD básico.
- Para mods de código, `ported` exige lógica real implementada, componente compilado e um caso representativo de gameplay validado, com evidência reproduzível em `mods/<id>/porting-status.json`. Stub, hooks declarados ou WASM compilado não bastam; qualquer mecânica pendente mantém `partial`. Para conteúdo sem código, o carry-over suportado precisa estar completo, com templates mapeados e validações aplicáveis aprovadas. `bambooeverything` continua `partial` no relatório atual: apesar do manifesto `content-only`, o inventário o classifica como `logic` e o gerador aponta falta de atestação.
- As descrições herdadas das sessões antigas registram escopo tentado, não prova de completude. As linhas marcadas como não comprovadas precisam de implementação real e validação. Jade segue parcial: o protótipo cobre só o HUD básico.
- A cobertura atual lista seis `ported`; a worklist mantém cinco confirmados e registra a exceção de Jade, cujo protótipo continua incompleto.
- Na sessão 01, **10/10 mods têm crate de port ou conteúdo implementado**; **2/10 estão completos no escopo validado** (`jags` e `stonechest`). Os outros oito seguem `partial`, com limitações de escopo ou validação registradas abaixo.

| ID | Situação atual | Evidência/critério |
|---|---|---|
| `accelerateddecay` | Ported | Probe de 92 IDs e teste end-to-end com controle sem mod; evidências listadas em `porting-status.json`. |
| `botanytrees` | Ported (data) | 333 arquivos de conteúdo carregados. |
| `mcwfences` | Ported (data) | 125 blocos carried. |
| `mcwpaths` | Ported (data) | 236 blocos carried. |
| `mcwtrpdoors` | Ported (data) | 139 blocos e 13 itens carried. |
| `jade` | Partial nesta worklist | O relatório atual marca `ported` pelo tier `client`, mas o protótipo local cobre só o HUD básico; falta reproduzir o restante. |
- A numeração `(#n)` dentro das tabelas é o índice contínuo da lista; o cabeçalho indica o intervalo da sessão.

## Ordem de portagem baseada nas quests

A ordem numérica abaixo segue os primeiros pontos de entrada e as dependências da árvore FTB Quests. Para começar a jogar o caminho inicial, priorizar `FTBTeams` + `ftbsbc` + `jags` e os scripts-base `ftbstoneblock`. `StoneChest` é um ramo paralelo: a quest Chest aparece depois da Crafting Table, mas não é dependência de Hammer, Crook, Saplings ou Wood. Assim, StoneChest fica depois do checkpoint mínimo e antes de chegar à quest Chest. Create, Tinkers’ Construct e Thermal entram conforme as quests posteriores; nas alternativas, o caminho principal usa Tinkers’ Construct + Thermal em **Washing** e o raw zinc do Create em **Crushing**; AE2, Mekanism e Immersive Engineering podem esperar. `ftbsbc` requer FTB Teams, e Thermal requer CoFH Core; esses pré-requisitos ficam junto do caminho. O namespace `ftbstoneblock` vem de scripts KubeJS do próprio pack e precisa ser levado junto, embora não seja uma das 223 entradas do inventário.

**Checkpoint mínimo para começar a jogar:** validar `FTBTeams` + `ftbsbc` + `jags` (IDs #1–3) e os scripts-base `ftbstoneblock`; então dá para iniciar o caminho de recursos **Rock and Stone → Crafting Table → Hammer Down!/Wololo → SEEDS? → Saplings → Wood!**. `StoneChest` (#4) é paralelo e pode ser concluído depois, antes de fazer a quest Chest. O livro FTB Quests é um mod próprio (#169) e ainda não está portado; este checkpoint permite começar no mundo e seguir a rota de recursos, mas não afirma que o livro/registro de quests já funciona em Horizon. O caminho de Hammer/Crook/Saplings/Wood precisa funcionar antes de liberar esse marco; Create, Thermal e os ramos opcionais não bloqueiam o início.

As dependências lidas nos capítulos `welcome` e `getting_started` são: **Welcome → Rock and Stone! → Crafting Table**; da Crafting Table saem **Hammer Down!** e **Wololo** em paralelo. **SEEDS?** depende do Crook; **Saplings** exige Crook e Hammer; **Wood!** vem depois de Saplings. A quest **Chest** também abre depois da Crafting Table, sem bloquear essa cadeia. Depois de Wood começa a rota Create: **Millstone → Mill Gravel → Lava e Water** em paralelo; **Cobble for Days** exige Lava e Water, e **FTB Dripper** depende de Water. Portanto o trabalho #9 (`cobblefordays`) é posterior ao primeiro caminho jogável, apesar de aparecer na sessão 01 por sua primeira quest.

Depois do primeiro loop, a ordem segue os primeiros pontos de entrada dos ramos: Botania e Chickens/Budding Crystals aparecem cedo; Ars Nouveau e Blood Magic vêm depois; integrações como AppBot, Ars Creo e AE2 Things ficam após os mods-base correspondentes. Avaritia fica no fim: há uma quest isolada de Record Fragment cedo, mas a linha de singularidades e armaduras é de endgame. Mods sem tarefa direta ficam depois dos ramos que as quests usam.

### Evidência já coletada

- `ftbteams`: substituí o stub do StoneBlock pela base mais completa já portada no ATM10; implementei identidade autenticada, membros/convites por UUID e persistência; create/list/invite/self-accept-check/leave passaram E2E nos dois modos locais. Continua `partial`: falta validar aceite com segundo jogador, chat de grupo, alianças e claims.
- `ftbsbc`: auto-hammer usa API 0.4/0.5 com 44 receitas da config/scripts do pack, slots, progresso serializado e automação por face; iron stone→cobblestone passou em jogo. Stone Hammer e Crook usam hooks de loot com ferramenta equipada; FirePlow foi validado em survival. O drop de Stone Pebble ao quebrar stone sem item na mão foi validado como exatamente 1 nos dois modos. Pebble→Cobblestone e Cobblestone→Crafting Table passaram na grade pessoal nos dois modos; receitas de Stone Rod/Hammer/Crook na mesa passaram em in-frame, mas ainda não em server-thread devido à sincronização de cursor/slots do fixture. Continua `partial`: chance/quantidade original do drop não está definida pelo texto/config do pack, RNG do Crook não é vanilla, `.max()` e durabilidade/tempos não foram comparados ao original, e ainda faltam portal, bits spawner, tags/compressed outputs e automação/save/tier de auto-hammers. Evidência: `ports/ftbsbc/harness/opening-loop-compatibility.json`.
- `jags`: port completo no escopo documentado. O componente API 0.5 foi construído; o uso em dirt transformou o bloco em grass e consumiu exatamente uma seed nos dois modos; um alvo de pedra preservou o item. A loot modifier recebeu 60 amostras do RNG usado pela tabela vanilla de short grass e gerou 10 seeds em cada modo. Evidências: `ports/jags/harness/component-validation.json` e `gameplay-validation.json`.
- `stonechest`: port completo no escopo: 10 variantes, 10 parts, 20 receitas, 10 loot tables e 27 slots por baú. Crafting, interação, inserção e save/reopen passaram nos modos in-frame e server-thread; textura Stone foi conferida visualmente nos dois modos. As 30 texturas MIT do upstream estão incluídas com atribuição. Evidências em `ports/stonechest/harness/`.
- `create`: a cadeia de quest Hand Crank → Cogwheel → Millstone foi montada nos dois modos; in-frame avançou o progresso. Em server-thread, a UI abre mas o uso do crank não ativa a máquina. Gravel não é consumido nem gera os quatro outputs probabilísticos porque falta RNG vanilla nos callbacks de máquina. Evidência em `ports/create/harness/gameplay-validation.json`.
- `tconstruct`: Heater consumiu carvão, Melter converteu grout em 500 mB de seared stone, fluido persistiu após save/reopen e Faucet produziu seared brick nos dois modos. Segue parcial: menus e ferramentas, multiblock, ligas e comportamento de slime faltam. Evidência em `ports/tconstruct/harness/gameplay-validation.json`.
- `cofh_core`: Glossed Magma passou nos dois modos com idade, derretimento/propagação via RNG ativo, limite de vizinhos, dano `hot_floor`, supressão ao agachar e loot vazio. Segue parcial: outros blocos `_air`, Holding, efeitos, comandos e integrações. Evidências em `ports/cofh_core/harness/`.
- `thermal`: Crystallizer e liga bronze no Smelter passaram em jogo; implementados também os primeiros caminhos de Pulverizer/Furnace/Press, Crucible/Chiller, Rock Gen, Insolator e Crescent Hammer. A validação do Rock Gen falhou nos dois modos apesar do mod carregado e de água adjacente carregada; Insolator e Hammer ainda não têm E2E. `energy_mod`/`water_mod` continuam sem baseline primário local, e receitas que dependem desses valores ficam fechadas. Evidência em `ports/thermal/harness/device-gameplay-validation.json` e nos E2Es anteriores.
- `cobblefordays`: cinco tiers, crafts, geração, buffer, extração, push e persistência passaram; os defaults foram comparados ao JAR 1.5.1. Falta inspeção visual final dos cinco modelos. Evidência em `ports/cobblefordays/harness/gameplay-validation.json`.
- `ftbdripper`: dez recipes do pack carregados; água+dirt→clay, scheduler/tanque e save/reopen passaram nos dois modos. Segue parcial: intervalo/volume, RNG vanilla, interação de buckets/bowls e dependências de blocos comprimidos/oil sand ainda não foram confirmados. Evidências em `ports/ftbdripper/harness/`.
- `accelerateddecay`: port completo no escopo registrado; probe de 92 IDs e E2E com controle sem mod.
- `bambooeverything`: carry dos 20 blocos, 2 itens, receitas e loot tables; o inventário ainda classifica o tier como `logic`, então permanece parcial até corrigir a classificação e validar o carry.
- `buddingcrystals`: crescimento e drops validados, mais receita Thermal executada; integração das receitas Create e E2E no jogo pendentes.
- `autosmithingtable`: máquina, configuração, GUI e interações de slots validadas; falta testar entrada FE conectada em jogo.
- `avaritia`: Collector e Compressor têm probes; Extreme Crafting, comportamentos de itens, GUI e E2E seguem pendentes.
- `ae2things`: probes de Inscriber e stack 70; faltam custo/entrada de energia fiel, Crystal Growth Chamber, DISK, GUI e validação no jogo.


## Sessão 01 — primeiro caminho jogável (IDs 1–10)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Logic — API 0.3 | `ftbteams` (#1) | FTB Teams | All Rights Reserved | Parcial: API de comando agora expõe UUID/nome autenticados; parties persistem owner/membros/convites por UUID, com criar/listar/membros/convidar/aceitar/sair e limite de membros. E2E local passou nos modos in-frame e server-thread; aceite positivo por segundo jogador ainda precisa de multiplayer. `/msg` segue local; sem lookup de jogadores online, broadcast, alianças ou claims. Ver `ports/ftbteams/harness/{porting-audit,gameplay-validation}.json`. |
| Machines — API 0.5 | `ftbsbc` (#2) | FTB StoneBlock Companion | All rights reserved | Parcial: drop de exatamente 1 Stone Pebble ao minerar stone sem ferramenta validado em in-frame e server-thread; Pebble→Cobblestone e Cobblestone→Crafting Table passaram na grade pessoal nos dois modos. Stone Rod/Hammer/Crook craftam na mesa em in-frame; o fixture server-thread ainda não sincroniza slots/cursor de modo confiável. Hammer/Crook, FirePlow e auto-hammer têm validações próprias. Faltam chance/count original do drop, RNG vanilla do Crook, `.max()`/durabilidade/tempos fiéis, portal, bits spawner, receitas comprimidas/por tag e automação/save/tier de auto-hammers. Evidência: `ports/ftbsbc/harness/opening-loop-compatibility.json`. |
| Logic — API 0.5 | `jags` (#3) | JAGS | I | Portado: plantio atômico em dirt consome uma seed somente após a troca para grass block; a chance de loot usa o RNG da tabela de short grass. Componente e E2E nos dois modos passaram; 60 amostras geraram 10 seeds adicionais. Evidências: `ports/jags/harness/`. |
| Content — API 0.2 | `stonechest` (#4) | Stone Chest | MIT | Portado no escopo: 10 chest blocks, 10 part items, 20 receitas, 10 loot tables, 27 slots e texturas normais/left/right para as 10 variantes. E2E nos dois modos validou crafting, interação, inventário, inserção e save/reopen; screenshots confirmam a textura Stone nos dois modos. Os 30 PNGs MIT upstream estão incluídos com atribuição. Evidências em `ports/stonechest/harness/`. Ramo Chest paralelo ao caminho de Saplings/Wood. |
| Machines — API 0.4 | `create` (#5) | Create | MIT | Parcial: E2E confirmou a abertura do Millstone nos dois modos e progresso após o clique no crank em in-frame; no server-thread, o evento não ativa o callback autoritativo e o progresso fica em 0. O gravel permanece no slot porque os callbacks ainda não recebem RNG vanilla de Level para as quatro saídas probabilísticas. O workaround de stroke limitado a 100 ticks ainda não foi comparado ao timing nativo do Create; cinética geral segue fora do escopo. Evidência: `ports/create/harness/gameplay-validation.json`. |
| Logic — API 0.3 | `tconstruct` (#6) | Tinkers' Construct | MIT | Parcial: port inicial da cadeia Heater → Melter → Faucet/Casting Table. Queima de combustível, grout→500 mB, save/reopen do fluido e casting de seared brick passaram nos dois modos. Part Builder/Tinker Station ainda são só conteúdo/receitas; faltam menus e ferramentas, ligas, multiblock, slimes e parte do carry. Auditoria/evidência em `ports/tconstruct/harness/`. |
| Logic — API 0.3 | `cofh_core` (#7) | CoFH Core | CoFH - Don't Be A Jerk (Learn, Don't Steal) | Parcial: Glossed Magma usa o RNG ativo, avança idade, derrete/propaga com limiares auditados, herda dano `hot_floor` com supressão ao agachar e preserva loot vazio; E2E passou nos dois modos. Faltam os blocos `_air`, Holding, efeitos, comandos e integrações. Auditoria e E2E em `ports/cofh_core/harness/`. |
| Machines — API 0.4 | `thermal` (#8) | Thermal Series | CoFH - Don't Be A Jerk (Learn, Don't Steal) | Parcial: Crystallizer e liga bronze do Smelter passaram em jogo anteriormente nos dois modos; o parser carregou 24 receitas Pulverizer, 72 Press e 28 Smelter. O código atual inclui execução de Pulverizer/Furnace/Press/Smelter/Crucible/Chiller, Insolator fail-closed (exige custos RF/água calibrados) e Rock Gen com seis receitas carregadas. O Rock Gen falhou nos dois modos: slot permaneceu vazio após 20 ticks; no server-thread a água adjacente estava carregada (`minecraft:water[level=0]`), então a falha provável está no dispatch/tick do block entity ou escrita no slot. O HMOD ativou e registrou o bloco. Insolator e Crescent Hammer não chegaram a rodar no harness. Nenhuma referência primária do repo/config pack estabelece baseline `energy_mod`/`water_mod`; `thermal-common.toml` só muda Crystallizer Base Power para 160 RF/t, portanto custos desconhecidos continuam fail-closed. Crescent Hammer implementa rotação e desmontagem vazia, sem gameplay E2E; conteúdo cheio não é preservado. Restam RNG exato, assets/receitas e modelo `potted_rubber_sapling`. Evidências: `ports/thermal/harness/gameplay-validation.json`, `ports/thermal/harness/dry-machine-gameplay-validation.json` e falha diagnóstica em `ports/thermal/harness/device-gameplay-validation.json`.
| Machines — API 0.4 | `cobblefordays` (#9) | Cobble For Days | LGPL 2.1 | Parcial: cinco geradores reais, buffer configurável, extração por hopper e push superior; os cinco crafts 3×3 passaram nos dois modos e geração/persistência/automação também. Os defaults foram conferidos no JAR original e o harness atualizado passou após carregar essas capacidades: intervalos 40/20/10/5/1 ticks, 1 item/ciclo, buffers 64/128/256/512/1024 e push ligado em todos os tiers. Falta revisar o modelo no jogo. Evidência: `ports/cobblefordays/porting-status.json` e `harness/gameplay-validation.json`. |
| Machines — API 0.4 | `ftbdripper` (#10) | FTB Dripper | All Rights Reserved | Parcial: o port traduz os dez recipes KubeJS do pack (bloco/fluido/chance); conversão água+dirt→clay e scheduler/tanque após save/reopen passaram em in-frame e server-thread, inclusive uma segunda conversão sem fonte de água. Faltam o intervalo/volume fiéis ao build 1802, RNG vanilla para as nove chances, interação original de buckets/bowls, exibição das receitas probabilísticas e as dependências dos blocos comprimidos/oil sand. Texturas FTB não foram redistribuídas; modelo usa textura vanilla. Evidências: `ports/ftbdripper/harness/`. |

## Sessão 02 — recursos e ramos iniciais (IDs 11–20)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Deep — API 0.5 | `ars_nouveau` (#11) | Ars Nouveau | GNU GENERAL PUBLIC LICENSE | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 105 blocos, 211 itens, magia com livros de feitiços, glifos, mana e comando) |
| Logic — API 0.3 | `botania` (#12) | Botania | Botania License | 566 blocos, 340 itens, 1.259 receitas; 2 sem template e 91 arquivos fora dos namespaces; depende de Patchouli/Curios; mana, máquinas e flora excedem um port simples de API 0.3 |
| Logic — API 0.3 | `chickens` (#13) | Chickens | MIT Licence | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 7 blocos de galinheiro/roost, 6 itens, analyzer, catcher, posturas agendadas e comando) |
| Logic — API 0.3 | `cccbridge` (#14) | CC:C Bridge | Apache-2.0 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 4 blocos de roteamento, canais e comando) |
| Logic — API 0.3 | `createchunkloading` (#15) | Create Chunkloading | LGPL-2.1 License | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco chunk loader, ciclo de vida de tickets, inspeção de raio e comando) |
| Logic — API 0.3 | `ars_creo` (#16) | Ars Creo | GNU Lesser General Public License v3.0 | roda e receita carregadas; documentação diz que gera stress/RPM Create e suporta turrets, jars e contraptions. Create atual não implementa cinética nem contraptions; não existe contrato Horizon equivalente para integrar com fidelidade |
| Content / Scripting | `kubejs_create` (#17) | KubeJS Create | GNU LGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, bindings cinéticos e receitas Create) |
| Content / Scripting | `kubejs_tinkers_construct` (#18) | KubeJS Tinkers Construct | GNU LGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, fundição e modificadores Tinkers) |
| Logic — API 0.3 | `tcintegrations` (#19) | Tinkers' Integrations and Tweaks | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 10 blocos, 11 itens, integração de ligas de bronze e traços de ferramentas e comando) |
| Logic — API 0.3 | `summoningrituals` (#20) | Summoning Rituals | LGPL-3.0 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 2 blocos, 2 itens, altar e rituais místicos de invocação com catalisadores e comando) |

## Sessão 03 — automação e capítulos de tecnologia/magia (IDs 21–30)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Deep — API 0.5 | `ae2` (#21) | Applied Energistics 2 | See GitHub repository for details | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 65 blocos, 311 itens, rede de energia de matéria ME, autocrafting, fluix e comando) |
| Machines — API 0.4 | `autosmithingtable` (#22) | Auto Smithing Table | AGPLv3 (página oficial CurseForge; metadado do JAR diverge) | componente recompilado em `carry/mod.wasm`; processa `smithing_transform` com FE, transferência por faces e progresso persistido. Lang carregado do original 1.18.2 especifica entradas Base/Addition e saída, com configuração de faces; o slot de template extra adapta a receita para Horizon 26.3. Configuração por face foi implementada e passou no logic/machine probe (`harness/side-config-trace.json` e `harness/side-machine-trace.json`). GUI foi implementada com slots reais, progresso e barra de energia; `component-validation.json` confirma a configuração carregada do pack (20 ticks, 40 FE/tick, 100.000 FE). A configuração do pack foi aplicada e a transformação consome 800 FE em 20 ticks no machine probe; o harness reinicia limites por tick. A GUI e os cliques de entrada/retirada passaram em jogo; captura `harness/manual-ui-populated.png`. O probe de máquina também validou o coast armor trim: gerou peitoral de ferro em 20 ticks, consumiu 800 FE e preservou os componentes minecraft:coast e minecraft:iron. Falta testar uma fonte FE conectada em jogo. |
| Logic — API 0.3 | `buildinggadgets` (#23) | Building Gadgets | MIT (https://github.com/Direwolf20-MC/BuildingGadgets/blob/master/LICENSE.md) | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 6 blocos, 23 itens, gadgets e comandos) |
| Machines — API 0.4 | `charginggadgets` (#24) | Charging Gadgets | MIT (https://github.com/Direwolf20-MC/ChargingGadgets/blob/master/LICENSE.md) | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco, estação de carregamento RF de itens e comando) |
| Deep — API 0.5 | `compactmachines` (#25) | Compact Machines 4 | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 8 blocos, 3 itens, dimensões de bolso compactas e dispositivo de encolhimento e comando) |
| Logic — API 0.3 | `constructionwand` (#26) | Construction Wand | MIT License | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 10 itens, wands T1–T4, núcleos angel e destruction e comando) |
| Machines — API 0.4 | `ironfurnaces` (#27) | Iron Furnaces | GNU GENERAL PUBLIC LICENSE Version 3, 29 June 2007 | ⚠️ Não comprovado pelo critério atual (registro antigo: API 0.4; fornalhas T1–T13 de cobre até netherite/allthemodium com velocidade até 8x-100x, queima de combustível, receitas de smelting e sided I/O) |
| Deep — API 0.5 | `mekanism` (#28) | Mekanism | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 200 blocos, 212 itens, máquinas de alta tecnologia, tanques químicos e comando) |
| Deep — API 0.5 | `mininggadgets` (#29) | Mining Gadgets | MIT (https://github.com/Direwolf20-MC/MiningGadgets/blob/master/LICENSE.txt) | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 3 blocos, 24 itens, laser de mineração de alta tecnologia, modificadores e comando) |
| Machines — API 0.4 | `pipez` (#30) | Pipez | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 7 itens, tubos de alta performance para itens, fluidos, energia, gás e comando) |

## Sessão 04 — automação e capítulos de tecnologia/magia (IDs 31–40)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Machines — API 0.4 | `pocketstorage` (#31) | Pocket Storage | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 4 itens, unidades de armazenamento portáteis PSU com auto-pickup e comando) |
| Logic — API 0.3 | `prettypipes` (#32) | Pretty Pipes | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 3 blocos, 33 itens, rede modular de transporte de itens, terminais, chave e comando) |
| Deep — API 0.5 | `refinedstorage` (#33) | Refined Storage | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 32 blocos, 59 itens, sistema digital de armazenamento em rede, discos e comando) |
| Logic — API 0.3 | `simplybackpacks` (#34) | Simply Backpacks | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 5 itens, mochilas em múltiplos tiers com auto-pickup e comando) |
| Deep — API 0.5 | `tempad` (#35) | Tempad | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 4 itens, dispositivo de teletransporte com portal temporal e comando) |
| Logic — API 0.3 | `torchmaster` (#36) | Torchmaster | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 4 blocos, 1 item, Mega Tocha para supressão de spawn hostil em raio e comando) |
| Machines — API 0.4 | `entangled` (#37) | Entangled | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 item, vinculador quântico de blocos multidimensionais e comando) |
| Machines — API 0.4 | `functionalstorage` (#38) | Functional Storage | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 3 blocos, 13 itens, gavetas de armazenamento, controlador, linking tool e comando) |
| Machines — API 0.4 | `rftoolsstorage` (#39) | RFToolsStorage | MIT License https://github.com/McJtyMods/RFToolsStorage/blob/1.16/LICENCE | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 2 blocos, 8 itens, scanner de armazenamento, módulo de controle e comando) |
| Logic — API 0.3 | `tesseract` (#40) | Tesseract | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco, transferência multidimensional sem fio de itens, energia e fluidos e comando) |

## Sessão 05 — automação e capítulos de tecnologia/magia (IDs 41–50)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Machines — API 0.4 | `trashcans` (#41) | Trash Cans | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: API 0.4; lixeiras de itens/líquidos/energia com ticking de máquina, aniquilação automática de itens e inserção omnidirecional) |
| Machines — API 0.4 | `mob_grinding_utils` (#42) | Mob Grinding Utils | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 20 blocos, 26 itens, moagem de mobs, fans, spikes, drenagem de XP e comando) |
| Logic — API 0.3 | `buddingcrystals` (#43) | BuddingCrystals | LGPL v2.1 | 55 blocos/1 item, componente API 0.3 compilado em `carry/mod.wasm`; templates de ametista reproduzem forma/estados, budding blocks recebem random ticks, growth implementa chance 1/5 e progressão de 4 estágios nas seis faces; 13 receitas e 44 loot tables migradas, incluindo drops de cluster. Probe API 0.3 passou em 24 casos nas seis faces. As 33 receitas shaped KubeJS e 11 loot overrides carregam; Silk Touch passou nos drops. O Crystallizer da Thermal API 0.4 executou uma receita real após reload: consumiu 800 mB molten amethyst e 50.000 RF, produziu 12 small fluix buds. Parcial: as 11 receitas Create crushing ainda não são executadas via crushing wheels; falta validação end-to-end; o pack não configura cristais customizados. |
| Machines — API 0.4 | `industrialforegoing` (#44) | Industrial Foregoing | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 75 blocos, 135 itens, automação industrial, laser drill, mob slaughter e comando) |
| Deep — API 0.5 | `immersiveengineering` (#45) | Immersive Engineering | Blu's License of Common Sense | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 249 blocos, 212 itens, maquinário pesado multibloco, alta tensão e comando) |
| Deep — API 0.5 | `createaddition` (#46) | Create Crafts & Additions | MIT License, Copyright 2022 MRH0 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 12 blocos, 39 itens, motores elétricos, alternadores, fios elétricos e comando) |
| Machines — API 0.4 | `powah` (#47) | Powah | LGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 92 blocos, 43 itens, geração FE/RF, energizing rods, reatores, geradores e comando) |
| Deep — API 0.5 | `rftoolsbuilder` (#48) | RFToolsBuilder | MIT License https://github.com/McJtyMods/RFToolsBuilder/blob/1.16/LICENCE | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 17 blocos, 16 itens, builder com shape cards para quarrying e comando) |
| Machines — API 0.4 | `rftoolsutility` (#49) | RFToolsUtility | MIT License https://github.com/McJtyMods/RFToolsUtility/blob/1.16/LICENCE | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 14 blocos, 71 itens, auto-crafters, telas informativas, sensores e comando) |
| Logic — API 0.3 | `projecte` (#50) | ProjectE | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 22 blocos, 109 itens, transmutação alquímica, EMC, condensadores e comando) |

## Sessão 06 — automação e capítulos de tecnologia/magia (IDs 51–60)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Machines — API 0.4 | `ae2things` (#51) | AE2 Things | MIT | dados carregados: 2 máquinas mapeadas a `minecraft:redstone_lamp` com variante `working`→`lit`, 6 itens, 13 receitas migradas, 2 loot tables e nenhum arquivo fora do namespace. Documentação pública descreve Advanced Inscriber (empilhamento sem limite por slot e automação sem restrições laterais), Crystal Growth Chamber (crescimento de cristais e saída para ME) e DISK sem limite de tipos. Componente API 0.4 instalado em `carry/mod.wasm`; probe passou em 4 casos: `inscribe` preserva a prensa, `press` consome os três slots, sem energia não consome itens e o progresso sobrevive a reload; novo caso processa e gera stack 70 acima do limite vanilla 64 via `machine-slot-limits`. O loader expõe 15/15 receitas AE2 Inscriber após portar cinco tags Forge. Ainda parcial: o adapter usa 1000 FE por operação (documentação diz 1 kAE), duração Horizon configurável de 40 ticks sem valor público confirmado, faltam GUI, Crystal Growth Chamber, DISK, rede ME e validação no jogo. O root completo também segue sem prova por falha de worldgen em `bloodmagic` e `compactmachines`. |
| Machines — API 0.4 | `ae2wtlib` (#52) | AE2WTLib | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 6 itens, terminais sem fio AE2, cartão infinity e comando) |
| Machines — API 0.4 | `appmek` (#53) | Applied Mekanistics | See GitHub repository for details | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 13 itens, integração química AE2 + Mekanism, células de gás e comando) |
| Data incompleto | `mekanismtools` (#54) | Mekanism: Tools | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 78 itens, armaduras e ferramentas de ligas especiais Mekanism e comando) |
| Machines — API 0.4 | `merequester` (#55) | ME Requester | LGPL-3.0 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco, 1 item, requisitor e terminal de estoque sob demanda para AE2 e comando) |
| Logic — API 0.3 | `rsinfinitybooster` (#56) | RSInfinityBooster | GNU GENERAL PUBLIC LICENSE | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 2 itens, amplificador de alcance infinito, cartão dimensional e comando) |
| Content / Scripting | `kubejs_mekanism` (#57) | KubeJS Mekanism | GNU LGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, gases, infusões e máquinas Mekanism) |
| Content / Logistics | `xnetgases` (#58) | XNet Gases | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, canais de gases Mekanism no XNet) |
| Content / Scripting | `kubejs_immersive_engineering` (#59) | KubeJS Immersive Engineering | GNU LGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, multiblocos e receitas IE) |
| Logic — API 0.3 | `appbot` (#60) | Applied Botanics | See GitHub repository for details | 1 bloco/13 itens e receitas carregados; comportamento depende de mana Botania e redes ME/P2P/células AE2. Os componentes atuais de Botania e AE2 não implementam essas redes; a API ainda não oferece esses contratos |

## Sessão 07 — progressão avançada de quests (IDs 61–70)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Logic — API 0.3 | `experienceobelisk` (#61) | Experience Obelisk | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 3 blocos, 2 itens, reserva de experiência/cognitium e comando) |
| Logic — API 0.3 | `ppfluids` (#62) | Pretty Fluid Pipes | GPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 10 itens, tubos de fluidos, módulos de extração e filtragem e comando) |
| Logic — API 0.3 | `gag` (#63) | Gadgets Against Grind | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco, 9 itens, hearthstones, escape rope, placa no solicitors e comando) |
| Logic — API 0.3 | `bloodmagic` (#64) | Blood Magic | All rights reserved | 88 blocos/304 itens; 21 sem template, 73 arquivos fora dos namespaces; depende de Patchouli; separar sistemas/máquinas e corrigir carry antes de portar |
| Content / Scripting | `kubejs_blood_magic` (#65) | KubeJS Blood Magic | GNU LGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, bindings de script de Blood Magic) |
| Logic — API 0.3 | `extendedexchange` (#66) | ExtendedeXchange | GNU LGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 54 blocos, 44 itens, coletores e power flowers de alto tier, links de EMC e comando) |
| Logic — API 0.3 | `refinedstorageaddons` (#67) | Refined Storage Addons | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 4 itens, grade de fabricação sem fio e comando) |
| Logic — API 0.3 | `rsrequestify` (#68) | RSRequestify | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco, solicitador e emissor de autocrafting RS e comando) |
| Deep — API 0.5 | `mekanismgenerators` (#69) | Mekanism: Generators | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 29 blocos, 9 itens, geradores solar, eólico, bio e reatores e comando) |
| Library / Core | `autoreglib` (#70) | AutoRegLib | https://github.com/Vazkii/AutoRegLib/blob/master/README.md | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, registros de utilidades Vazkii) |

## Sessão 08 — conteúdo sem tarefa direta nas quests (IDs 71–80)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Cosmetic | `betterthanbunnies` (#71) | Better Than Bunnies | GNU Lesser General Public License v3.0 | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, render cosmético de coelhos) |
| Logic — API 0.3 | `champions` (#72) | Champions | LGPL-3.0-or-later | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; egg, spawns de campeões, drops especiais e comando) |
| Logic — API 0.3 | `chancecubes` (#73) | Chance Cubes | GPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 4 blocos, 8 itens, scanner, pendants, eventos e comando) |
| Logic — API 0.3 | `chiselsandbits` (#74) | Chisels & bits | GLGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 22 blocos, 28 itens, talhadeira, fita métrica, chave, bit bag e comando) |
| Gameplay | `climbladdersfast` (#75) | Climb Ladders Fast | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, aceleração vertical em escadas/trepadeiras) |
| Library / Core | `collective` (#76) | Collective | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, biblioteca base de utilitários Serilum) |
| Logic — API 0.3 | `compactcrafting` (#77) | Compact Crafting | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 2 blocos, 2 itens, projector dish, proxies de campo e comando) |
| Deep — API 0.5 | `computercraft` (#78) | CC: Tweaked | ComputerCraft Public License (https://raw.githubusercontent.com/dan200/ComputerCraft/master/LICENSE) | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 3 blocos, 15 itens, computadores programáveis, tartarugas robóticas e comando) |
| Machines — API 0.4 | `cosmeticarmorreworked` (#79) | CosmeticArmorReworked | Minecraft Mod Public License 1.0.1 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; suporte a armadura cosmética visual com preservação de defesa e comando) |
| Logic — API 0.3 | `crashutilities` (#80) | Crash Utilities | MIT LICENSE | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; telemetria de uptime, tps, relatórios de diagnósticos e comando) |

## Sessão 09 — conteúdo sem tarefa direta nas quests (IDs 81–90)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Logic — API 0.3 | `darkerdepths` (#81) | DarkerDepths | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 136 blocos, 16 itens, colheita de aloe, mineração de minérios áridos e comando) |
| Server Utility | `dsp` (#82) | Default Server Properties | Mozilla Public License 2.0 | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, configuração de propriedades padrão de servidor) |
| Utility | `dynamic_asset_generator` (#83) | DynamicAssetGenerator | LGPL v3 | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, geração dinâmica de assets cliente/itens) |
| Logic — API 0.3 | `eccentrictome` (#84) | Eccentric Tome | LGPL-3.0 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; item do tomo, absorção e navegação de múltiplos manuais/livros e comando) |
| Logic — API 0.3 | `elevatorid` (#85) | Elevator Mod | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 16 blocos de elevadores por cor, trânsito vertical e comando) |
| Logic — API 0.3 | `emojiful` (#86) | Emojiful | ARR | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; interceptação de chat, tradução e pré-visualização de shortcodes de emojis e comando) |
| Logic — API 0.3 | `engineersdecor` (#87) | Engineer's Decor | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 85 blocos decorativos/estruturais, tijolos clinker, catwalks e comando) |
| Bug Fix / Patch | `entangledfix` (#88) | Entangled Fix | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, patch de estabilidade para blocos entrelaçados) |
| Machines — API 0.4 | `farmersdelight` (#89) | Farmer's Delight | MIT License | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 73 blocos, 92 itens, tábua de corte, panela, fogão, culinária e comando) |
| Machines — API 0.4 | `fluxnetworks` (#90) | Flux Networks | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 5 blocos, 4 itens, rede de energia sem fio cross-dimensional e comando) |

## Sessão 10 — conteúdo sem tarefa direta nas quests (IDs 91–100)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Deep — API 0.5 | `framedblocks` (#91) | FramedBlocks | LGPL v3 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 136 blocos, 7 itens, blocos com moldura personalizável, martelo e comando) |
| Server Utility | `ftbpromoter` (#92) | FTB Promoter | ARR | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, sistema de broadcast e mensagens de servidor) |
| Logic — API 0.3 | `gravestone` (#93) | Gravestone Mod | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco, 1 item, lápide para preservação de drops na morte, obituário e comando) |
| Gameplay | `harvest` (#94) | Harvest | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, colheita por clique direito com replantio automático) |
| Logic — API 0.3 | `incontrol` (#95) | InControl | MIT License | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; regras personalizadas de controle de spawn de entidades e comando) |
| Logic — API 0.3 | `instrumentalmobs` (#96) | Instrumental Mobs | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco, 16 itens, instrumentos musicais tocáveis e comando) |
| Logic — API 0.3 | `ironchest` (#97) | Iron Chests | GNU General Public License v3.0 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 14 blocos, 7 itens, baús com tiers escalonados, upgrades in-place e comando) |
| Logic — API 0.3 | `libx` (#98) | LibX | APACHE-2.0 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 item, utilitários base, recarregamento de configs e comando) |
| Logic — API 0.3 | `literalskyblock` (#99) | Literal Sky Block | GNU LGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 2 blocos de projeção celeste e void com emissão de luz e comando) |
| Logic — API 0.3 | `luggage` (#100) | Luggage | The MIT License (MIT) | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 item, baú pet vivo seguidor com coleta de itens e comando) |

## Sessão 11 — conteúdo sem tarefa direta nas quests (IDs 101–110)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Logic — API 0.3 | `mantle` (#101) | Mantle | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 item, biblioteca SlimeKnights, suporte a livros-guia e comando) |
| Data incompleto | `mcwbridges` (#102) | Macaw's Bridges | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 117 blocos, 1 item, pontes suspensas, pilares e comando) |
| Data incompleto | `mcwdoors` (#103) | Macaw's Doors | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 188 blocos, 13 itens, portas japonesas, de correr e rústicas e comando) |
| Machines — API 0.4 | `mcwfurnitures` (#104) | Macaw's Furniture | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 434 blocos de mobília em madeiras decorativas e comando) |
| Logic — API 0.3 | `mcwlights` (#105) | Macaw's Lights and Lamps | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 42 blocos, tochas tiki, lâmpadas, lanternas e comando) |
| Content | `mcwpaintings` (#106) | Macaw's Paintings | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, 55 pinturas customizadas) |
| Data incompleto | `mcwroofs` (#107) | Macaw's Roofs | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 456 blocos, 2 itens, telhados inclinados, calhas e comando) |
| Logic — API 0.3 | `mcwwindows` (#108) | Macaw's Windows | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 199 blocos, 14 itens, janelas, persianas e venezianas interativas e comando) |
| Logic — API 0.3 | `measurements` (#109) | Measurements | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 item, fita métrica para medição de distância e volume em blocos e comando) |
| Bug Fix / Patch | `mekfix` (#110) | MekFix | ARR | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, patch de estabilidade Mekanism) |

## Sessão 12 — conteúdo sem tarefa direta nas quests (IDs 111–120)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Logic — API 0.3 | `metalbarrels` (#111) | Metal Barrels | no | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 8 blocos, 30 itens, barris metálicos multicapacidade, upgrades e comando) |
| Logic — API 0.3 | `minicoal` (#112) | MiniCoal | GPL-3.0 License | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 2 itens, mini carvão e mini carvão vegetal para 1 fundição e comando) |
| Logic — API 0.3 | `mother_silverfish` (#113) | Mother Silverfish | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; modelo e animações geckolib de rainha traça, invocação de ninhada e comando) |
| Logic — API 0.3 | `plonk` (#114) | Plonk | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco, exibição 3D decorativa de qualquer item colocado em superfícies e comando) |
| Logic — API 0.3 | `portablecraftingtable` (#115) | Portable Crafting Table | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 item, bancada de trabalho portátil portátil com acesso direto à grade 3×3 e comando) |
| Machines — API 0.4 | `rftoolsbase` (#116) | RFToolsBase | MIT License https://github.com/McJtyMods/RFToolsBase/blob/1.16/LICENCE | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 7 blocos, 10 itens, chave inteligente, machine frames, shards e comando) |
| Content | `runelic` (#117) | Runelic | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, glifos e runas) |
| Gameplay | `scaffoldingpower` (#118) | Scaffolding power | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, estabilidade de andaimes e escadas) |
| Content / Lib | `selene` (#119) | Selene | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, texturas dinâmicas e modelos) |
| Server Utility | `serverconfigupdater` (#120) | ServerConfig Updater | MIT LICENSE | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, sincronização de configs) |

## Sessão 13 — conteúdo sem tarefa direta nas quests (IDs 121–130)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Deep — API 0.5 | `shrink` (#121) | Shrink | GNU General Public License v3.0 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 5 itens, dispositivo de encolhimento e captura de entidades em garrafas e comando) |
| Data incompleto | `simplylight` (#122) | Simply Light | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 7 blocos, painéis e lâmpadas modernas ultrafinas e comando) |
| Logic — API 0.3 | `snad` (#123) | Snad | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 3 blocos, aceleração de crescimento para cana, cacto e fungo e comando) |
| Logic — API 0.3 | `structurecompass` (#124) | Structure Compass Mod | MIT | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 item, bússola de localização de estruturas e masmorras e comando) |
| Content | `structureexpansion` (#125) | Structure Expansion | MIT License | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, limites de estruturas e jigsaw) |
| Content / Lib | `tklib` (#126) | TKLib | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, biblioteca base TK) |
| Content / Tools | `toolkit` (#127) | Tool Kit | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, ferramentas e atalhos) |
| Logic — API 0.3 | `torchbowmod` (#128) | TorchBowMod | MIT License | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 6 itens, arco disparador de tochas com iluminação remota de blocos e comando) |
| Logic — API 0.3 | `woodenshears` (#129) | Wooden Shears | The MIT License (MIT) | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 item, tesoura de madeira de início de jogo e comando) |
| Logic — API 0.3 | `accelerateddecay` (#130) | Accelerated Decay | GPLv3 (CurseForge; JAR tem placeholder) | API 0.3; hooks para os 48 membros resolvidos do tag `minecraft:logs` e 92 IDs de tronco/lenho; probe validou todos os 92 eventos contra 87 IDs de folhas até distância taxicab 6; teste em jogo passou contra controle sem mod |

## Sessão 14 — conteúdo sem tarefa direta nas quests (IDs 131–140)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Client UI / client-ui | `jade` (#131) | Jade | CC BY-NC-SA 4.0 | ⚠️ Parcial nesta worklist: o relatório marca `ported` pelo tier client, mas o protótipo local cobre só o HUD básico. |
| Data — API 0.2 / content | `mcwfences` (#132) | Macaw's Fences and Walls | MIT | ✅ Ported segundo a cobertura atual: 125 blocks, 0 items; data carried. |
| Data — API 0.2 / content | `mcwpaths` (#133) | Macaw's Paths and Pavings | MIT | ✅ Ported segundo a cobertura atual: 236 blocks, 0 items; data carried. |
| Data — API 0.2 / content | `mcwtrpdoors` (#134) | Macaw's Trapdoors | MIT | ✅ Ported segundo a cobertura atual: 139 blocks, 13 items; data carried. |
| Machines — API 0.4 | `advancedperipherals` (#135) | Advanced Peripherals | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 19 itens, periféricos para computadores CC, óculos AR e comando) |
| Conteúdo — API 0.2 | `bambooeverything` (#136) | Bamboo Everything | All rights reserved | ⚠️ Parcial no relatório atual: o manifesto é `content-only`, mas o inventário marca tier `logic` e falta atestação; 20 blocos/2 itens, 22 receitas e 18 loot tables carregados sem perdas; variantes ladder/torch e dry bamboo mapeadas, jangadas usam raft vanilla. |
| Machines — API 0.4 | `botanypots` (#137) | BotanyPots | LGPL V2.1 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 98 blocos de vasos botânicos automatizados de terracota/concreto e comando) |
| Data incompleto | `chipped` (#138) | Chipped | Terrarium Licence | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 7493 blocos artesanais com bancadas especiais e comando) |
| Data incompleto | `connectedglass` (#139) | Connected Glass | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 119 blocos de vidro com texturas conectadas e sem bordas e comando) |
| Logic — API 0.3 | `darkutils` (#140) | DarkUtilities | LGPL V2.1 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco, 11 itens, vector plates, damage plates, filtros de mob e comando) |

## Sessão 15 — conteúdo sem tarefa direta nas quests (IDs 141–150)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Machines — API 0.4 | `enderstorage` (#141) | EnderStorage | The MIT License (MIT) | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 2 blocos, 1 item, baús, tanques e bolsas do Ender vinculados por cor e comando) |
| Server Utility | `ftbbackups2` (#142) | FTB Backups 2 | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, rotinas de snapshot e backup de mundos) |
| Logic — API 0.3 | `furnacerecycle` (#143) | Furnace Recycle | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; reciclagem e fusão de ferramentas/armaduras usadas em fornalhas e comando) |
| Machines — API 0.4 | `itemcollectors` (#144) | Item Collectors | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 2 blocos de coletores por sucção com alcance configurável e comando) |
| Logic — API 0.3 | `itemfilters` (#145) | Item Filters | GNU LGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 16 itens de filtros lógicos, predicados compostos e comando) |
| Logic — API 0.3 | `justhammers` (#146) | Just Hammers | GPL3 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 29 martelos de mineração 3×3, durabilidades graduadas e comando) |
| Machines — API 0.4 | `laserio` (#147) | LaserIO | MIT (https://github.com/Direwolf20-MC/LaserIO/blob/main/LICENSE.txt) | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 2 blocos, 24 itens, logística modular a laser com cartões de transporte e comando) |
| Logic — API 0.3 | `nethersdelight` (#148) | Nethers Delight | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 10 blocos, 16 itens, fogão de rocha negra, carnes do Nether e comando) |
| Machines — API 0.4 | `rechiseled` (#149) | Rechiseled | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1099 blocos cinzelados com texturas conectadas, cinzel e comando) |
| Machines — API 0.4 | `rftoolspower` (#150) | RFToolsPower | MIT License https://github.com/McJtyMods/RFToolsPower/blob/1.16/LICENCE | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 8 blocos, 7 itens, células de energia modular, geradores blazing e comando) |

## Sessão 16 — conteúdo sem tarefa direta nas quests (IDs 151–160)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Logic — API 0.3 | `simplemagnets` (#151) | Simple Magnets | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 2 blocos, 2 itens, ímãs portáteis e bobinas de desmagnetização e comando) |
| Gameplay | `squatgrow` (#152) | Squat Grow | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, aceleração de lavouras e mudas ao agachar) |
| Gameplay | `stackrefill` (#153) | Stack Refill | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, reposição automática de hotbar) |
| Logic — API 0.3 | `supercircuitmaker` (#154) | Super Circuit Maker | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 5 blocos, 18 itens, circuitos integrados de micro-redstone e comando) |
| Deep — API 0.5 | `supplementaries` (#155) | Supplementaries | All rights reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 136 blocos, 69 itens, blocos decorativos e utilitários, pedestais e comando) |
| Logic — API 0.3 | `thermal_expansion` (#156) | Thermal Expansion | CoFH - Don't Be A Jerk (Learn, Don't Steal) | Parcial: 31 itens carregados; o componente atual não implementa as máquinas Thermal Expansion. O Crystallizer desta sessão roda no componente `thermal`; os demais executores continuam pendentes. |
| Logic — API 0.3 | `thermal_integration` (#157) | Thermal Integration | CoFH - Don't Be A Jerk (Learn, Don't Steal) | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; metalurgia de ligas integradas Rose Gold, Steel, Invar, Electrum e comando) |
| Logic — API 0.3 | `travel_anchors` (#158) | Travel Anchors | Apache License 2.0 | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco, 1 item, teletransporte rápido para âncoras e comando) |
| Machines — API 0.4 | `xnet` (#159) | XNet | MIT License https://github.com/McJtyMods/XNet/blob/1.16/LICENCE | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 9 blocos, 16 itens, rede monocabo para itens, fluidos, energia e dados e comando) |
| Logic — API 0.3 | `ftbessentials` (#160) | FTB Essentials | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; utilitários de servidor, /home, /spawn, /back, /trashcan e comando) |

## Sessão 17 — fim de jogo: Avaritia e restante opcional (IDs 161–170)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Logic — API 0.3 | `ftbranks` (#161) | FTB Ranks | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; gerenciamento de cargos, permissões e comando) |
| Logic — API 0.3 | `ftbultimine` (#162) | FTB Ultimine | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; vein-mining de blocos e minérios com alternância e comando) |
| Content / Scripting | `kubejs_thermal` (#163) | KubeJS Thermal | GNU LGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, dynamos e receitas Thermal Series) |
| Content / Scripting | `kubejs_ui` (#164) | KubeJS UI | GNU LGPLv3 | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, namespace `kubejsui` preservado) |
| Content / Scripting | `kubeutils` (#165) | KubeUtils | GPL3 | ⚠️ Não comprovado pelo critério atual (registro antigo: Componente WASM compilado em `carry/mod.wasm`, API 0.3, utilitários matemáticos e scripts) |
| Logic — API 0.3 | `soulshards` (#166) | SoulShards | https://github.com/CreeperHost/SoulShards/blob/1.18/LICENSE.md | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 1 bloco, 11 itens, aprisionamento de almas em fragmentos e gaiolas e comando) |
| Data — API 0.2 / integration | `botanytrees` (#167) | BotanyTrees | LGPL V2.1 | ✅ Ported segundo a cobertura atual: 333 content files carried. |
| Deep — API 0.5 | `ftbchunks` (#168) | FTB Chunks | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; reivindicação de chunks, carregamento contínuo e minimapa e comando) |
| Machines — API 0.4 | `ftbquests` (#169) | FTB Quests | All Rights Reserved | ⚠️ Não comprovado pelo critério atual (registro antigo: mod.wasm compilado; 10 blocos, 4 itens, sistema de missões, recompensas e comando) |
| Machines — API 0.4 | `avaritia` (#170) | Avaritia | The MIT License (MIT) | 8 blocos, 45 itens originais e 49 receitas originais carregados; 23 itens de singularidade e 23 receitas compressor KubeJS adicionados via port overlay; 2 diferenças de template. Componente API 0.4 instalado em `carry/mod.wasm`. Probe de máquina passou: Collector produz `neutron_pile` em 7.111 ticks com reload no meio; Compressor consome 400 `iron_block`, preserva progresso após reload e entrega `iron_singularity`. A leitura do tag Forge foi habilitada no root combinado; resultados em `mods/avaritia/harness/component-validation.json`. Probe carregou as 23 receitas compressor KubeJS e resolveu os outputs customizados. Ainda parcial: Extreme Crafting Table 9×9, GUI/entrada manual, comportamentos de itens/equipamentos, correspondência NBT da receita addon_singularity, loader halo dos modelos e validação end-to-end no jogo. |

## Sessão 18 — dependências mapeadas / não necessárias (IDs 171–180)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Sem portagem / integration | `ponderjs` (#171) | PonderJS | MIT | ✅ Não necessário: nothing to port. |
| Client UI | `appleskin` (#172) | AppleSkin | The Unlicense | ✅ Classificado (Interface cliente: visualização HUD de saturação e exaustão de comida) |
| Sem portagem / library | `architectury` (#173) | Architectury | GNU LGPLv3 | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Sem portagem / library | `balm` (#174) | Balm | All rights reserved | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Sem portagem / other | `bcc` (#175) | Better Compatibility Checker | All rights reserved | ✅ Não necessário: pack tooling or an engine feature, folded into the pipeline or the engine. |
| Client UI | `bhmenu` (#176) | BHMenu | All Rights Reserved | ✅ Classificado (Interface cliente: menu customizado integrado à UI do Horizon) |
| Sem portagem / library | `bookshelf` (#177) | Bookshelf | LGPL V2.1 | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Sem portagem / library | `cloth_config` (#178) | Cloth Config v4 API | GNU LGPLv3 | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Sem portagem / library | `codechickenlib` (#179) | CodeChicken Lib | GNU Lesser General Public License v2.1 | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Client UI | `configured` (#180) | Configured | GPL3 | ✅ Classificado (Interface cliente: tela gráfica de configurações no launcher/menu Horizon) |

## Sessão 19 — dependências mapeadas / não necessárias (IDs 181–190)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Client UI | `controlling` (#181) | Controlling | MIT | ✅ Classificado (Interface cliente: busca avançada e gestão de conflitos de atalhos de teclado) |
| Sem portagem / other | `ctm` (#182) | ConnectedTexturesMod | GPL-2.0 | ✅ Não necessário: pack tooling or an engine feature, folded into the pipeline or the engine. |
| Sem portagem / library | `curios` (#183) | Curios API | LGPL-3.0-or-later | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Client UI | `curseforge-580181-3648085` (#184) | CurseForge Asset/Client | unknown | ✅ Classificado (Client asset / mod auxiliar cliente, sem impacto em simulação de mundo) |
| Client UI | `darkmodeeverywhere` (#185) | DarkModeEverywhere | MIT | ✅ Classificado (Interface cliente: tema escuro e contraste para menus e inventários) |
| Client UI | `ding` (#186) | Ding | GNU Lesser General Public License v3.0 | ✅ Classificado (Interface cliente / áudio: alerta sonoro de carregamento de mundo suprido pelo Horizon) |
| Client UI | `extremesoundmuffler` (#187) | Extreme Sound Muffler | GNU LESSER GENERAL PUBLIC LICENSE | ✅ Classificado (Interface cliente: silenciador e filtro seletivo de sons ambientais) |
| Sem portagem / performance | `ferritecore` (#188) | Ferrite Core | MIT | ✅ Não necessário: performance mod: Horizon's engine is the replacement. |
| Sem portagem / library | `fusion` (#189) | Fusion | All rights reserved | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Sem portagem / library | `geckolib3` (#190) | GeckoLib | MIT | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |

## Sessão 20 — dependências mapeadas / não necessárias (IDs 191–200)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Client UI | `inventorysorter` (#191) | Simple Inventory Sorter | GPLv3 | ✅ Classificado (Interface cliente: rotina de ordenação de inventário nativa do Horizon) |
| Sem portagem / library | `mcjtylib` (#192) | McJtyLib | MIT License https://github.com/McJtyMods/McJtyLib/blob/1.18/LICENCE | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Client UI | `modnametooltip` (#193) | Mod Name Tooltip | MIT | ✅ Classificado (Interface cliente: indicação azul do mod de origem nas tooltips nativa do Horizon) |
| Client UI | `mousetweaks` (#194) | Mouse Tweaks | BSD-3-Clause | ✅ Classificado (Interface cliente / input: manuseio rápido de itens em inventários no motor Horizon) |
| Client UI | `mute` (#195) | Mute | All Rights Reserved | ✅ Classificado (Interface cliente / áudio: silenciador seletivo integrado ao mixer nativo) |
| Client UI | `partyparrots` (#196) | Party Parrots | GNU Lesser General Public License v3.0 | ✅ Classificado (Interface cliente / cosmético de animação de papagaios) |
| Sem portagem / library | `patchouli` (#197) | Patchouli | CC BY-NC-SA 3.0 | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Client UI | `pigpen` (#198) | PigPen | LGPL V2.1 | ✅ Classificado (Interface cliente: renderização de fontes e runas alternativas) |
| Sem portagem / library | `placebo` (#199) | Placebo | MIT License | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Client UI | `reauth` (#200) | ReAuth | All rights reserved | ✅ Classificado (Interface cliente: autenticação e sessões gerenciadas pelo Horizon Launcher) |

## Sessão 21 — dependências mapeadas / não necessárias (IDs 201–210)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Sem portagem / library | `rhino` (#201) | Rhino | MPL-2.0 | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Sem portagem / performance | `spark` (#202) | spark | GPLv3 | ✅ Não necessário: performance mod: Horizon's engine is the replacement. |
| Sem portagem / library | `supermartijn642configlib` (#203) | SuperMartijn642's Config Library | All rights reserved | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Sem portagem / library | `supermartijn642corelib` (#204) | SuperMartijn642's Core Lib | All rights reserved | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Sem portagem / library | `titanium` (#205) | Titanium | GNU LESSER GENERAL PUBLIC LICENSE | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Sem portagem / other | `yeetusexperimentus` (#206) | Yeetus Experimentus | All rights reserved | ✅ Não necessário: pack tooling or an engine feature, folded into the pipeline or the engine. |
| Client UI | `craftingtweaks` (#207) | CraftingTweaks | All rights reserved | ✅ Classificado (Interface cliente: botões rápidos de rotação e balanceamento na grade de crafting) |
| Client UI | `durabilitytooltip` (#208) | Durability Tooltip | All rights reserved | ✅ Classificado (Interface cliente: indicador numérico de durabilidade nas tooltips de itens do Horizon) |
| Client UI | `findme` (#209) | FindMe | MIT | ✅ Classificado (Interface cliente: busca de itens em containers adjacentes por tecla) |
| Sem portagem / library | `ftblibrary` (#210) | FTB Library | All Rights Reserved | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |

## Sessão 22 — dependências mapeadas / não necessárias (IDs 211–220)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Client UI | `inventoryessentials` (#211) | Inventory Essentials | All rights reserved | ✅ Classificado (Interface cliente: atalhos e movimentação em lote de inventário nativa do Horizon) |
| Client UI | `jadeaddons` (#212) | Jade Addons | ARR | ✅ Classificado (Interface cliente: HUD overlay Jade nativo do Horizon) |
| Sem portagem / other | `kubejs` (#213) | KubeJS | GNU LGPLv3 | ✅ Não necessário: pack tooling or an engine feature, folded into the pipeline or the engine. |
| Client UI | `lightoverlay` (#214) | Light Overlay | Apache-2.0 | ✅ Classificado (Interface cliente: overlay de nível de luz F7 nativo do Horizon) |
| Sem portagem / library | `polylib` (#215) | PolyLib | Insert License Here | ✅ Não necessário: library: its API maps to Horizon's mod API and SDK. |
| Client UI | `roughlyenoughitems` (#216) | Roughly Enough Items (REI) | MIT | ✅ Classificado (Interface cliente: catálogo de receitas e visualizador de itens nativo do Horizon) |
| Client UI | `sdrp` (#217) | Simple Discord Rich Presence | All Rights Reserved | ✅ Classificado (Interface cliente: presença rica Discord nativa do launcher) |
| Client UI | `tipsmod` (#218) | Tips | LGPL V2.1 | ✅ Classificado (Interface cliente: dicas de loading screen nativas do Horizon) |
| Client UI | `toastcontrol` (#219) | Toast Control | MIT License | ✅ Classificado (Interface cliente: controle e supressão de notificações toast integrado ao Horizon) |
| Client UI | `trashslot` (#220) | TrashSlot | All rights reserved | ✅ Classificado (Interface cliente: botão/slot de descarte em GUI de inventário substituído por hotkey/UI nativa) |

## Sessão 23 — dependências mapeadas / não necessárias (IDs 221–223)

| Categoria | Mod ID | Nome | Licença | Situação / trabalho restante |
|---|---|---|---|---|
| Client UI | `minetogether` (#221) | MineTogether | GNU General Public License v3.0 | ✅ Classificado (Interface cliente / multiplayer social substituído pelo Horizon Friends/Relay) |
| Client UI | `rei_plugin_compatibilities` (#222) | REI Plugin Compatibilities | See LICENSE | ✅ Classificado (Interface cliente: plugins REI integrados ao catálogo nativo do Horizon) |
| Client UI | `roughlyenoughprofessions` (#223) | Roughly Enough Professions | MIT | ✅ Classificado (Interface cliente: catálogo de profissões e trocas de aldeões nativo) |

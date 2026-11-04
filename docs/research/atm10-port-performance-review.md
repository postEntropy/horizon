# Revisão dos ports ATM10 — 2026-11-01

A marcação de conclusão ainda não é confiável. Há simulação útil em parte dos
ports, mas também telas estáticas, processamento desconectado do inventário real
e erros de conservação de itens. Não há evidência para aprovar o desempenho do
pack completo.

Escopo: leitura dos caminhos de produção dos **57 mods marcados como Portado na
worklist atual**, mais triagem estrutural dos **250 checkboxes de BATCHES.md**.
A união contém **256 mods**. A triagem dos demais não equivale a uma revisão
funcional completa. Nenhum fonte Java de mod foi consultado.

Worklist local: `~/.local/share/horizon/ports/all-the-mods-10-atm10/porting-worklist.md`.
O inventário reproduzível fica em
`.local/atm10-port-audit/2026-11-01/inventory.json`, com hashes dos fontes Rust.
Os resultados descrevem a árvore de trabalho, que já continha alterações locais.

## Achados prioritários

1. **P1 — Custo quadrático nos acessos a máquinas.**
   [State::view](../../crates/horizon-mods/src/runtime.rs) faz
   `self.views.iter_mut().find(...)` em cada `get_slot`/`set_slot`.
   [DimStorage](../../ports/dimstorage/src/lib.rs) e
   [EnderStorage](../../ports/enderstorage/src/lib.rs) leem 27 slots por handle,
   inclusive vazios, em todo tick. Com N baús do mesmo mod, isso implica
   aproximadamente `27 × N × (N+1)/2` comparações de handles por tick, além das
   travessias WASM/host. Usar índice de handle para view; depois sincronizar
   inventários por alterações, com um estado canônico por canal.
2. **P1 — Sincronização dos baús repõe itens extraídos.** Ambos os loops tratam
   `(slot da máquina vazio, canal com item)` como cópia do canal para a máquina.
   Uma extração pode voltar no tick seguinte. Não há snapshot anterior para
   distinguir extração de carregamento inicial. EnderStorage também usa
   `"0:0:0"` para todos os handles no tick, ignorando a frequência configurada.
   Faltam callbacks de persistência/remoção nesses dois mods, deixando mapas
   acumularem handles e canais durante a sessão.
3. **P1 — Máquinas declaradas sem processamento.** Entre os 57, MEGA Cells,
   Mob Grinding Utils, Productive Metalworks, Pylons e Ranged Pumps declaram
   entidades com `ticks: true`, mas não implementam o tick. Entangled implementa
   um tick vazio. Modern Dynamics anuncia taxas, mas não transfere recursos.
   Custos baixos nesses casos não demonstram eficiência de uma implementação real.
4. **P1 — Simulação isolada do jogo.** Actually Additions, Advanced Peripherals,
   Ars Technica, BotanyPots, Cable Tiers e Tree Tap têm estruturas de simulação,
   mas os caminhos examinados não conectam as operações anunciadas aos slots,
   vizinhos e recursos reais. Ars Technica, BotanyPots e Cable Tiers percorrem
   seus mapas completos, ignorando os handles recebidos; também usam a primeira
   máquina do mapa para construir telas, sem identificar o bloco clicado.
5. **P1 — Conservação de recursos.** Charging Gadgets desconta FE quando há
   qualquer item no slot 0, sem escrever carga no item. Modular Routers insere
   cobblestone no buffer para puller/vacuum sem extrair de uma origem, e remove
   itens no breaker/dropper sem executar a ação de mundo correspondente.
   Farmer's Delight assume `is_heated = true` e não verifica compatibilidade e
   espaço no resultado antes de consumir ingredientes. Not Enough Glyphs apenas
   anuncia absorção e retorna `Pass`, sem cancelar/modificar o dano.
6. **P2 — Mais trabalho fora do benchmark.**
   [O loop do jogo](../../crates/horizon-client/src/game/mods.rs) cria views,
   copia slots/componentes e chama `save_block_entity` por entidade a cada tick,
   mesmo quando nada mudou. O runtime ainda semeia/reconcilia capabilities,
   recria coleções e filtra todas as views para cada mod. O custo do jogo real
   será maior que a medição isolada abaixo; medir antes de desenhar uma API de
   dirty tracking que preserve saves e transferências entre mods.

## Resultado por mod da worklist atual

“Baixo” descreve somente a estrutura de custo observada no código, não uma
aprovação de performance medida. “Parcial” aponta lacunas concretas; os demais
também precisam de validação integrada antes de receber “100%”.

| Mod | Custo / situação observada |
|---|---|
| akashictome | Tick retorna antes de ler inventário quando a tecla não foi pressionada. Biblioteca limitada; parcial: storage por mundo em vez de identidade/NBT de cada tomo. |
| allthetweaks | Interações originais de item ainda parciais: uso no ar de ATM Star, Greg Star e Withers Compass foi observado como PASS e validado no Horizon; uso em bloco/entidade, bússola e troféu seguem sem trace. |
| ars_elemancy | Copia inventário e cria vetor auxiliar todo tick; HUD só muda com loadout. Focos/mana explicitamente incompletos no próprio fonte. |
| blueflame | Trace com flint-and-steel em soul sand, soul soil e netherrack combinou com vanilla sem o mod; o evento mod-specific segue sem distinção nesses casos. Port ainda parcial. |
| bwncr | Por evento; consulta configs/storage e grava contadores/regras. Avaliar cache e flush agrupado em tempestades de eventos. |
| comforts | O oracle com FakePlayer aceitou hammock/bag (`CONSUME`) mas não dormiu nem avançou tempo; falta trace com jogador conectado para medir sono, spawn e tempo. Horizon segue parcial. |
| constructionstick | Trabalho limitado a 128 posições por interação; validar consumo, orientação, modos e inventário antes de medir construção real. |
| crafting_on_a_stick | FakePlayer observou SUCCESS e pilha intacta para os nove itens; menu real não é exposto pelo oracle. Horizon mostra telas sem inventário/receita autoritativos, então segue parcial até fixture de jogador e menus funcionais. |
| crashutilities | HUD a cada 10 ticks; varredura 5×5 apenas no comando. Sem crescimento por máquinas. |
| dyenamics | Uma recoloração por interação, com tentativa alternativa de estado; validar consumo de corante e propriedades. |
| elevatorid | Teclas filtram o tick; até 96 leituras de blocos por busca de 32 posições. Consultar espaço acima só após achar elevador; atualmente aceita qualquer cor. |
| DimStorage | A trace pública confirmou menu de 90 slots e persistência local de um diamante por três ticks. Compartilhamento exige uma interação de jogador configurando dois baús; o harness atual só coloca um baú por cenário. Sem conclusão de paridade. |
| Dis-Enchanting Table | O bloco salva inventário vazio e progresso zero, mas não expõe menu no harness público. Falta dirigir o uso do jogador com item encantado e observar saída/experiência. Sem conclusão de paridade. |
| fuelgoeshere | Consulta de combustível por comando; não demonstra o tooltip/automação anunciados na worklist. |
| heyberryshutup | Predicados por evento; validar cancelamento de lentidão e dano no jogo. |
| imfast | Teleporte e leitura de config por tick com tecla pressionada; não remove o limite da física. Validar colisão, autoridade e custo de rede. |
| incontrol | O(R) por spawn, atualmente uma regra fixa; reload restaura defaults, não carrega regras externas. |
| jumpboat | Consulta entidades só ao pressionar tecla; lista nearby e escolhe primeiro barco próximo, sem verificar montaria. |
| mcwlights | Port limitado aos quatro eventos medidos: jardim PASS sem alterar `lit`; white lamp com red dye/stick retorna Consume sem mudar bloco ou pilha. Validado em ambos os caminhos locais; as demais interações permanecem parciais. |
| mcwwindows | Observado: oak shutter abre/fecha com mão vazia, hammer ou key; o port valida esse toggle em ambos os modos. Remodelagem, lock e outros blocos seguem pendentes. |
| measurements | Aritmética por dois cliques; sem trabalho periódico. Validar overflow em seleções grandes. |
| merrymaking | Modelo atual tenta drop em `use` de present; probes públicos do oracle não capturaram o gatilho real. Presentes/stocking pendentes até trace de jogador. |
| moredragoneggs | Uma ação por morte; coordenada do portal fixa, validar portal real e entrega do evento. |
| morered | Colocação de poste por uso; mesa/redwire não implementam a rede lógica anunciada. |
| not_enough_glyphs | Baixo por dano; contingência apenas mostra chat, sem absorver o golpe. |
| novillagerdm | Config e duas gravações storage por evento compatível; não filtra explicitamente o kind de morte no callback. |
| bibliocraft | Storage sem tick; acesso por face configurado no create. Crafter não processa receitas. |
| charginggadgets | Até duas leituras de slot por estação por tick; consome energia sem carregar o item. |
| cosmeticarmorreworked | Tick filtra tecla; renderização cosmética explicitamente ausente no fonte. Slots mantêm IDs, validar movimentação real de itens. |
| energymeter | Amostragem periódica O(M), até seis vizinhos por medidor; mapa não tem remoção de bloco. Delta de buffer não representa fluxo em regime estável. |
| explorerscompass | Baixo porque usa coordenadas fixas de estruturas; não faz busca no mundo. |
| farmersdelight | Sete leituras por panela ociosa por tick + busca de receitas; calor constante e saída sem proteção, ver P1. |
| ironfurnaces | Duas leituras por fornalha ociosa por tick; processamento e save implementados. Escala afetada pelo lookup linear do host. |
| lootr | HashSets crescem por local/jogador, sem persistência; tela não gera inventário privado de loot. |
| mcwfurnitures | Storage sem tick; seis chamadas de acesso por slot na criação. Validar inventários reais e load/save no jogo. |
| megacells | Slots e telas; pattern provider declara tick sem processamento; sem rede/células reais. |
| mob_grinding_utils | Declara tick sem processamento; XP mostrado fixo; sem absorção/combate/vento. |
| moderndynamics | Sem tick/transferência; taxas são constantes e UI, não rede simulada. |
| modularrouters | Nove módulos por ciclo; cria/apaga itens sem a transferência ou ação correspondente. |
| naturescompass | Coordenadas fixas por bioma; nenhum custo de busca porque a busca não existe. |
| pocketstorage | Serialização por alteração de configuração; auto-pickup é flag, sem hook que recolha itens. |
| productivemetalworks | Declara tick sem processamento; casting/fundição não executam. |
| pylons | Declara tick sem processamento; colheita/infusão não executam. |
| rangedpumps | Declara tick sem processamento; telas usam FE/fluido fixos, sem bombeamento. |
| storagedelight | Storage de 27 slots sem tick; falta configuração explícita de faces, validar automação anunciada. |
| toolbelt | Tela de até nove slots sem armazenamento/troca de ferramentas; expansão só explica receita em chat. |
| treetap | Timer O(N) nos handles; seiva cresce em memória sem verificar árvore nem encher recipientes reais. |
| trophymanager | Save/load de mob e tier em mapa; tela/render e captura real ainda precisam de integração. |
| actuallyadditions | O(N) nos handles; estados internos não executam processamento sobre inventários reais. |
| advancedperipherals | O(N) nos handles; contadores internos não integram comandos ComputerCraft/sensoriamento real. |
| ars_technica | Varre mapa inteiro; estados desconectados de source/rotação/transmutação reais. |
| botanypots | Varre mapa inteiro; create não lê solo/semente dos slots; salva só timers, não cultura/solo/output interno. |
| cabletiers | Varre mapa inteiro; throughput interno sem transferência para inventários/capabilities de vizinhos. |
| chisel | Port agora passa sem mutação nos quatro right-clicks observados (stone, array/stone, bedrock e mão vazia). As ações principais, menu, materiais, padrões e modos continuam parciais. |
| cookingforblockheads | Harness de máquina não encontrou Container nem menu público do forno; tick/receita/save ainda sem trace black-box. |
| dimstorage | 27 leituras/handle/tick, lookup host O(N), reposição de extrações e mapas sem limpeza/persistência. |
| disenchanting_table | Três leituras e clone de estado por handle/tick; falta save/remove; validar componentes de encantamentos, não apenas IDs. |
| enderstorage | 27 leituras/handle/tick, canal fixo no tick e reposição de extrações; falta save/remove. |
| entangled | Tick vazio; vínculo é metadado/tela, sem proxy operacional de recursos. |

## Validação e reprodução

Medição release em AMD Ryzen 5 4600G, 100 ticks por cenário após 10 de
aquecimento, um mod por processo. Tempos em **ms por tick**:

| Mod / bloco | p50, 1 bloco | p50, 100 blocos | p50, 1.000 blocos | p95, 1.000 blocos |
|---|---:|---:|---:|---:|
| DimStorage / dimensional_chest | 0,0034 | 0,266 | 9,717 | 10,422 |
| EnderStorage / ender_chest | 0,0025 | 0,256 | 9,627 | 10,422 |
| Iron Furnaces / iron_furnace | 0,0014 | 0,107 | 1,661 | 1,957 |
| Charging Gadgets / charging_station | 0,0014 | 0,098 | 1,286 | 1,842 |
| Mob Grinding Utils / absorption_hopper | 0,0013 | 0,090 | 1,088 | 1,473 |

Multiplicar baús por dez elevou o p50 aproximadamente 37 vezes nos dois mods
dimensionais. Mil baús vazios de um deles já ocupam cerca de 19% dos 50 ms
disponíveis em um servidor a 20 TPS, antes do loop do jogo. O controle Mob Grinding
Utils mostra overhead mesmo com callback vazio; não executa absorção de itens.
Esses números são diagnósticos de uma execução local, sem isolamento de CPU;
houve compilação de Clippy em parte da coleta. A busca linear demonstrada no
fonte sustenta o problema de escala independentemente dessa variabilidade.

Nos seis cenários de baú (1/100/1.000 de cada mod), o campo
`slot_after_extraction` voltou a `["minecraft:diamond", 1]` depois de uma
extração explícita, com status `Running`. Todos os 15 cenários terminaram com o
componente rodando. Resultados JSON e logs em
[`.local/atm10-port-audit/2026-11-01/`](../../.local/atm10-port-audit/2026-11-01/).
Os cinco cores WASM foram reconstruídos a partir do fonte atual; o probe não
usa a cópia possivelmente antiga em `carry/mod.wasm`.

`cargo test --manifest-path ports/Cargo.toml --workspace --lib` passou:
**185 testes, 256 crates, zero falhas**. Muitos crates não têm testes; os testes
de helpers/telas não exercitam o componente WASM nem provam gameplay.
O novo probe passou build release, rustfmt e Clippy com `-D warnings`; o script
de inventário passou execução e compilação Python.

Das 57 marcações atuais, 30 não têm `porting-status.json`; as outras 27 apontam
evidências existentes, 15 explicitamente identificadas como `cargo test`.
As outras 12 têm formatos sem identificação de origem, que precisam ser
checados contra execução reproduzível. O resultado de Ranged Pumps, por exemplo,
afirma verificar buffers que não existem como simulação no fonte examinado.

Na união das listas, **114 mods declaram ticks sem override**. Esse número é um
alerta estrutural, não um classificador automático de funcionalidade.

```sh
python3 tools/audit_atm10_ports.py \
  --work ~/.local/share/horizon/ports/all-the-mods-10-atm10 \
  --out /tmp/atm10-port-inventory.json
cargo build --release -p horizon-mods --features runtime,pack --example port_tick_bench
# Reconstruir o core WASM do mod com o SDK atual antes de medir.
target/release/examples/port_tick_bench CARRY_DIR CORE_WASM BLOCK COUNT TICKS OUTPUT_JSON
```

O probe cria pacote e storage em diretório temporário, carrega o componente real,
mede o runtime após dez ticks de aquecimento e registra p50/p95/máximo. Também
registra o slot após inserir e extrair um diamante para reproduzir o P1 dos baús.
O cenário usa inventários vazios e não inclui o loop do jogo, renderização,
`ItemFacts`, saves por tick ou processamento ativo. Não mede FPS, RSS, pack
completo, jogo integrado ou paridade com NeoForge.

Prioridade de trabalho: corrigir conservação de recursos e estados canônicos;
indexar views por handle; ligar simulação aos inventários reais; adicionar cenários
WASM de automação/save/load; medir máquinas ociosas, ativas e redes em escala;
então atualizar conclusão e evidências de cada mod.

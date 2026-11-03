# ATM10 — acompanhamento central de performance

Este é o registro central das medições do [plano de correção](atm10-port-fix-plan.md).
Atualização iniciada em **2026-11-01**. Registrar ganhos, regressões e limites da
medição. Conclusão funcional e aprovação de performance exigem evidências
próprias; nenhuma tabela abaixo certifica um mod como completamente portado.

## Como registrar uma medição

Cada rodada informa: revisão/fontes e hashes WASM, cenário, número de entidades,
estado ativo/ocioso, modo de execução, aquecimento/amostras, CPU/afinidade,
prioridade, perfil do build, concorrência e artefatos brutos. Acrescentar p50,
p95, máximo, memória e tempo por etapa quando disponíveis. Campo não medido fica
como **pendente**, sem estimativa apresentada como resultado.

**Redução = 100 × (antes − depois) / antes.** Valor negativo significa aumento de
custo. **Razão = antes / depois**; abaixo de 1× também significa regressão.
Tempos nas tabelas são **ms por tick**.

## Rodada 1 — índice de handles no runtime (tarefa 1)

Comparação pareada no mesmo desktop, sem builds/testes concorrentes: Ryzen 5
4600G (6 núcleos / 12 CPUs lógicas), rustc 1.99.1, release, inventários vazios,
sem ItemFacts, 10 ticks de aquecimento + 200 amostras. Inclui host + WASM; não
inclui o loop do jogo, renderização nem persistência. Esta rodada ocorreu antes
do limite de CPU solicitado: sem afinidade fixa ou frequência travada.

| Mod | Baús | p50 antes | p50 depois | p95 antes | p95 depois | Redução p50 | Razão p50 | JSONs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| dimstorage | 1 | 0.002425 | 0.003016 | 0.002485 | 0.003066 | -24.4% | 0.80× | [antes](atm10-view-index-bench/before-dimstorage-1.json) / [depois](atm10-view-index-bench/after-dimstorage-1.json) |
| dimstorage | 100 | 0.266977 | 0.261366 | 0.435005 | 0.275583 | +2.1% | 1.02× | [antes](atm10-view-index-bench/before-dimstorage-100.json) / [depois](atm10-view-index-bench/after-dimstorage-100.json) |
| dimstorage | 1.000 | 9.579418 | 2.795121 | 11.664581 | 3.240035 | +70.8% | 3.43× | [antes](atm10-view-index-bench/before-dimstorage-1000.json) / [depois](atm10-view-index-bench/after-dimstorage-1000.json) |
| enderstorage | 1 | 0.002464 | 0.003035 | 0.002594 | 0.003968 | -23.2% | 0.81× | [antes](atm10-view-index-bench/before-enderstorage-1.json) / [depois](atm10-view-index-bench/after-enderstorage-1.json) |
| enderstorage | 100 | 0.252478 | 0.268990 | 0.357168 | 0.361826 | -6.5% | 0.94× | [antes](atm10-view-index-bench/before-enderstorage-100.json) / [depois](atm10-view-index-bench/after-enderstorage-100.json) |
| enderstorage | 1.000 | 9.648448 | 2.814477 | 10.800242 | 3.144794 | +70.8% | 3.43× | [antes](atm10-view-index-bench/before-enderstorage-1000.json) / [depois](atm10-view-index-bench/after-enderstorage-1000.json) |

Com 1.000 baús, a mediana caiu aproximadamente **71% (3,43×)** nos dois mods.
De 100 para 1.000 baús, p50 passou a crescer 10,69× em DimStorage e 10,46× em
EnderStorage; anteriormente crescia 35,88× e 38,22×. Para apenas um baú houve
regressão de aproximadamente 0,6 µs. Uma execução por combinação, sujeita a
ruído do desktop; não há intervalo de confiança calculado.

Detalhes: [método e validação](atm10-view-index-bench/README.md),
[ambiente e hashes](atm10-view-index-bench/environment.json).
Memória e tempo do jogo real: **não medidos nesta rodada**.

## Rodada 2 — inventários compartilhados canônicos (tarefa 2)

Comparação pareada concluída: mesmo host e SDK atuais, cores do guest anteriores
(commit `ac0a35ede47c92ec440dd13554cb3ad0061995d1`) versus cores com o inventário
compartilhado, CPU 0–5, nice 10, release, 10 warmup + 200 amostras, sem builds,
testes ou jogos concorrentes durante a coleta. [Ambiente e hashes](atm10-shared-chest-validation/benchmark/environment.json).
Não combinar esta rodada com a rodada 1 como se fossem condições idênticas.

| Mod | Baús | p50 antes | p50 depois | p95 antes | p95 depois | Redução p50 | JSONs |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| dimstorage | 1 | 0.002846 | 0.001854 | 0.002915 | 0.001884 | +34.9% | [antes](atm10-shared-chest-validation/benchmark/before-dimstorage-1.json) / [depois](atm10-shared-chest-validation/benchmark/after-dimstorage-1.json) |
| dimstorage | 100 | 0.245555 | 0.145275 | 0.282896 | 0.186754 | +40.8% | [antes](atm10-shared-chest-validation/benchmark/before-dimstorage-100.json) / [depois](atm10-shared-chest-validation/benchmark/after-dimstorage-100.json) |
| dimstorage | 1000 | 2.828317 | 1.740977 | 3.734242 | 2.525242 | +38.4% | [antes](atm10-shared-chest-validation/benchmark/before-dimstorage-1000.json) / [depois](atm10-shared-chest-validation/benchmark/after-dimstorage-1000.json) |
| enderstorage | 1 | 0.003938 | 0.001433 | 0.004318 | 0.001874 | +63.6% | [antes](atm10-shared-chest-validation/benchmark/before-enderstorage-1.json) / [depois](atm10-shared-chest-validation/benchmark/after-enderstorage-1.json) |
| enderstorage | 100 | 0.255754 | 0.123464 | 0.286743 | 0.148722 | +51.7% | [antes](atm10-shared-chest-validation/benchmark/before-enderstorage-100.json) / [depois](atm10-shared-chest-validation/benchmark/after-enderstorage-100.json) |
| enderstorage | 1000 | 2.839538 | 1.443815 | 3.343642 | 1.981443 | +49.2% | [antes](atm10-shared-chest-validation/benchmark/before-enderstorage-1000.json) / [depois](atm10-shared-chest-validation/benchmark/after-enderstorage-1000.json) |

O probe inclui preparação/reconciliação de views e callback WASM. Os novos baús
não precisam de tick do guest: o jogo acessa o inventário compartilhado diretamente.
O probe ainda fornece views explicitamente e, por haver tipos de tanque ticking
no mesmo mod, executa o callback padrão vazio. Portanto estes números não medem
o custo real do loop do jogo nem o custo de carregar/configurar os baús.

Pico RSS em **MiB**, do processo inteiro (inclui componentização/JIT/criação,
não somente os inventários em regime). Brutos `*-process.json` na mesma pasta:

| Mod | Baús | Pico antes | Pico depois | Variação RSS |
| --- | ---: | ---: | ---: | ---: |
| dimstorage | 1 | 17.97 | 21.75 | +21.0% |
| dimstorage | 100 | 18.07 | 21.71 | +20.2% |
| dimstorage | 1000 | 18.07 | 33.94 | +87.8% |
| enderstorage | 1 | 18.45 | 22.17 | +20.2% |
| enderstorage | 100 | 18.38 | 22.15 | +20.5% |
| enderstorage | 1000 | 18.51 | 22.22 | +20.1% |

A memória aumentou, especialmente no DimStorage com 1.000 baús. Um custo
identificado no fonte é registrar 27 × 6 = 162 regras de automação por endpoint,
inclusive as permissões padrão: até 162.000 entradas no host para esse cenário.
A tarefa 3 deve medir a retirada dessas entradas redundantes, preservando as
travas e a configuração das faces. O aumento não será omitido em favor da
redução de tempo.

| Métrica | DimStorage | EnderStorage |
| --- | --- | --- |
| p50/p95, 1 / 100 / 1.000 baús ociosos | Medido acima | Medido acima |
| cenário ativo, tempo por tick | Pendente | Pendente |
| memória / alocações | Pico RSS acima; alocações pendentes | Pico RSS acima; alocações pendentes |
| WASM: extração, frequência, proprietário e reabertura | [Passou](atm10-shared-chest-validation/dimstorage-component.json) | [Passou](atm10-shared-chest-validation/enderstorage-component.json) |
| funis e persistência in-frame | [Passou](atm10-shared-chest-validation/dimstorage-game.json) | [Passou](atm10-shared-chest-validation/enderstorage-game.json) |
| funis e persistência no servidor integrado | [Passou](atm10-shared-chest-validation/dimstorage-server-game.json) | [Passou](atm10-shared-chest-validation/enderstorage-game.json) |

Os testes de jogo dos dois mods, nos dois modos single-player, conservaram **18 diamantes** entre
inventário canônico, dois funis e suas saídas, sem reposição após extração.
Um canal com **11 lingotes de ouro** sobreviveu à remoção dos dois endpoints,
reabertura do mundo, recolocação dos baús e nova reabertura. São invariantes de
correção; não representam uma medição de velocidade.

A suite ampla do cliente encontrou uma queda durante inicialização paralela de
Vulkan; os três testes de `gfx::end_sky` passaram em série. A execução completa
em série terminou sem SIGSEGV: **1.092 passaram, 2 falharam, 26 ignorados**,
em **622,29 s**. As falhas restantes são tradução HLSL de `terrain.wgsl`
(`MissingDefaultCase`) e uma asserção de resolução em `model_library`;
esses arquivos não foram alterados pela implementação dos inventários.
[Resultado da suíte em série](atm10-shared-chest-validation/client-suite-serial.json). [Evidência da queda e limites da análise](atm10-shared-chest-validation/client-suite-crash.json).

## Rodada 3 — trabalho ocioso e permissões padrão (tarefa 3)

Concluída em 2026-11-02 para o escopo desta tarefa. `ActiveMachines` mantém
índices de seleção invalidados em inserção/remoção, visita apenas handles
recebidos, ignora ausentes e duplicados e reutiliza posições liberadas. Os ticks
das máquinas selecionadas mantêm o mesmo resultado; nenhuma entidade fora do
conjunto ativo avança. Não foi alterada a frequência de tick.

Medição **nativa dos loops Rust do guest**, sem WASM nem inventários do jogo:
200 amostras de 64 ticks cada, após 10 lotes de aquecimento, release,
CPU 0–5, nice 10 e sem builds/testes concorrentes na coleta. O agrupamento reduz
o peso/resolução do relógio nos cenários de apenas uma entidade. Os estados
finais das entidades selecionadas foram comparados exatamente antes/depois.
[Todos os cenários, hashes e dados brutos](atm10-idle-work-validation/native-batched/summary.json).
Valores abaixo são **µs por tick**, ao contrário das tabelas das rodadas anteriores.

| Mod | Carregadas | Handles recebidos | Cenário | p50 antes | p50 depois | Redução p50 |
| --- | ---: | ---: | --- | ---: | ---: | ---: |
| ars_technica | 1000 | 1 | idle | 1.5867 | 0.0284 | +98.2% |
| ars_technica | 1000 | 1 | active | 1.7941 | 0.0295 | +98.4% |
| ars_technica | 1000 | 1000 | idle | 1.5442 | 1.0204 | +33.9% |
| ars_technica | 1000 | 1000 | active | 1.8017 | 1.1797 | +34.5% |
| botanypots | 1000 | 1 | idle | 3.4343 | 0.0295 | +99.1% |
| botanypots | 1000 | 1 | active | 3.9734 | 0.0295 | +99.3% |
| botanypots | 1000 | 1000 | idle | 3.3895 | 2.5165 | +25.8% |
| botanypots | 1000 | 1000 | active | 3.9963 | 2.7566 | +31.0% |
| cabletiers | 1000 | 1 | idle | 1.1175 | 0.0229 | +97.9% |
| cabletiers | 1000 | 1 | active | 2.2884 | 0.0295 | +98.7% |
| cabletiers | 1000 | 1000 | idle | 1.1328 | 0.8119 | +28.3% |
| cabletiers | 1000 | 1000 | active | 2.3146 | 1.4383 | +37.9% |

Nos cenários operando com 1/1.000 handles recebido, o código anterior avançava
999 máquinas indevidamente; o novo avançou zero máquinas fora do conjunto.
O cenário com todas as máquinas ativas também ficou mais rápido. Os números
não certificam as mecânicas dos ports: BotanyPots e Cable Tiers ainda têm estados
internos desconectados de transferências reais, tratados na tarefa 4. Em uma
máquina, o custo medido é de dezenas de nanossegundos e há pequenas regressões,
sem relevância demonstrada para o jogo. As tentativas anteriores estão em
`native/`, `native-cached-indices/` e `native-final/`; a última apresentou
quantização forte em medições de um tick, motivando a coleta em lotes acima.

DimStorage não registra as 162 permissões padrão por baú desbloqueado.
O runtime remove regras explícitas ao restaurar `(true, true)`. A fixture
WASM confirmou **todos os 27 slots e seis faces**: padrão liberado, trava,
save/load travado, destrava e isolamento de outro endpoint.
[Resultado](atm10-shared-chest-validation/dimstorage-component.json).

Memória: três pares antes/depois por cenário, alternando a ordem dos processos;
mediana do pico **VmHWM de cada processo**, amostrado via `/proc/PID/status`
a cada 1 ms. Inclui componentização/JIT/criação; pode perder o último intervalo
menor que 1 ms. Não é memória do jogo real. Brutos e hashes:
[host-memory-sampled/summary.json](atm10-idle-work-validation/host-memory-sampled/summary.json).

| Mod | Baús | Pico antes (MiB) | Pico depois (MiB) | Variação |
| --- | ---: | ---: | ---: | ---: |
| dimstorage | 1 | 22.32 | 21.99 | -1.5% |
| dimstorage | 100 | 22.04 | 21.88 | -0.7% |
| dimstorage | 1000 | 34.25 | 22.11 | -35.5% |
| enderstorage | 1 | 22.34 | 21.88 | -2.1% |
| enderstorage | 100 | 22.01 | 21.95 | -0.3% |
| enderstorage | 1000 | 21.95 | 22.19 | +1.1% |

Em 1.000 baús DimStorage, a mediana caiu **12,14 MiB (35,5%)**.
EnderStorage serve como controle e variou pouco. A primeira tentativa em
`host-memory/` usou `wait4.ru_maxrss`, contaminado pelo pico herdado do processo
Python antes do exec; seus valores não sustentam a conclusão. A coleta de
VmHWM acima substitui essa tentativa. O tempo dos callbacks dos baús é apenas
diagnóstico: não houve mudança no tick deles nesta tarefa, e as variações
pequenas/ruído não são tratadas como ganho causal.

Validação desta rodada: 19 testes nativos dos cinco ports; 5 testes do SDK;
49 testes de horizon-mods passaram (1 ignorado), além de 2 doctests do SDK;
WASM release e Clippy dos cinco ports com warnings negados passaram.
A fixture DimStorage executou o WASM real e passou. A simulação completa dos
mods e a medição do loop do jogo permanecem nas tarefas 4–5.

## Tarefa 4 — checkpoint Charging Gadgets (2026-11-02)

O carregador agora desconta somente os FE que o item realmente recebeu.
A [extensão item-data](atm10-item-energy-design.md) usa uma declaração do dono
do item e saldo no componente do próprio ItemStack; não presume suporte pelo ID.

| Evidência | Resultado | Limite |
| --- | --- | --- |
| [Componente WASM real](atm10-resource-validation/charginggadgets-component.json) | Conserva 3.000 FE entre estação/item; consumidor separado extrai 300 FE; combustível de teste produz 120 FE; save/reabertura preserva dados | Provedor/consumidor de bateria de teste; não certifica ferramentas de outros ports |
| [Jogo real, in-frame e servidor integrado](atm10-resource-validation/charginggadgets-game.json) | Sem combustível não carrega; carvão real carrega até 2.500 FE; saturação, funil e reabertura preservam uma bateria com 2.500 FE | Taxas/capacidade são as do port e da bateria de teste; não comprovam paridade de configurações do mod original |
| Runtime | Suíte completa: 50 testes passaram e 1 ignorado; caso adicional no limite de 64 KiB passou | Desempenho desse processamento real ainda não medido |

[Ambiente, hashes e empacotamento](atm10-resource-validation/charginggadgets-environment.json).
Os testes também cobrem item cheio, ausente, comum, stack de duas baterias,
componentes inválidos, limites de recebimento/extração, simulação sem escrita,
isolamento de namespaces e preservação do nome do item. O save no meio da
queima mantém a geração restante sem consumir combustível novamente.

**Números de tempo/memória do carregamento ativo: pendentes.** Os valores acima
são conservação de recursos e capacidade, não ganhos de performance. A tarefa 4
continua em execução: falta integrar declarações/consumo nos ports reais de
ferramentas e tratar Modular Routers, Farmer's Delight e os demais casos da revisão.
Modular Routers agora tem transferências adjacentes reais, descritas no checkpoint
abaixo. Os módulos remotos/de mundo ainda são pendências concretas da
[extensão de transações](atm10-routing-transactions-design.md). Nenhum mod foi
marcado como completamente portado.

### Charging Gadgets com um item real do pack

A fixture [charginggadgets-mininggadget-game.json](atm10-resource-validation/charginggadgets-mininggadget-game.json)
passou em in-frame e servidor integrado. O Mining Gadget original tem capacidade
de 1.000.000 FE segundo a observação por capability pública registrada em
`mininggadgets-energy-original.json`. Com carvão, a estação transferiu 3.160 FE
em 80 ticks e 3.960 FE em 100 ticks. Após remover a estação, mover o mesmo item
para um baú, salvar e reabrir, os 3.960 FE foram preservados nos dois modos.
Sem combustível, a carga ficou em zero.

Esse caso valida a ligação da estação a um Mining Gadget real e a persistência
do componente. Não valida os upgrades de bateria, taxas/configuração original
da estação ou todos os três modelos de gadget. O processamento ativo ainda não
tem medição de tempo/memória.

## L6 — checkpoint Modern Dynamics: pendente por contrato de rede

A revisão da implementação local confirma que as taxas exibidas são constantes
de interface e que os pipes não transferem inventários. O contrato atual de
`machine::request-item-move` só descreve uma face adjacente a inventário. Ele não
expõe descoberta de topologia, caminho em vários blocos, filtros por segmento,
ou resolução de ciclos/rotas concorrentes de uma rede de tubos. Sem esses dados,
não há transferência real de pipe que possa ser validada para Modern Dynamics.
O caso fica pendente como não representável pelo contrato atual; não foi
adicionada uma simulação aproximada nem declarada paridade do mod. A fila L6
continua no próximo mod independente.

## L6 — checkpoint MEGA Cells: estado isolado observado; rede pendente

O exportador original, executado só com MEGA Cells 4.11.0, AE2 19.2.18 e
GuideME 21.1.19, registrou **15 blocos, 110 itens e 102 receitas**. A trace de
cinco ticks sem rede formada observou: a `mega_energy_cell` mantém
`internalCurrentPower: 0.0` e fullness 0; o `mega_pattern_provider` mantém
`sendList: []`, `returnInv: []` e priority 0; `mega_interface` mantém priority
0. Esses dados não demonstram inserção/extrusão por rede. A evidência bruta está
em [facts](atm10-resource-validation/megacells-original-facts.json) e
[trace](atm10-resource-validation/megacells-original-machine.json), com os
cenários em [megacells-scenarios.json](atm10-resource-validation/megacells-scenarios.json).

O caso fica pendente: o Horizon não implementa a rede AE2, seus canais, padrões,
armazenamento virtual ou formação de multiblocos. O buffer local de FE não
representa a energia AE da rede e não autoriza simular uma célula MEGA isolada
como armazenamento funcional. Não houve mudança no port MEGA Cells nem alegação
de equivalência funcional. A observação isolada apenas confirma o estado sem
rede; falta uma fixture de rede original para caracterizar operações conectadas.

## L6 — checkpoint Mob Grinding Utils: coleta observada; API de entidades pendente

Na execução original de Mob Grinding Utils 1.1.10, um Absorption Hopper recebeu
um ItemEntity com 5 diamantes e um orb com 7 XP, ambos a três blocos, sem
alteração de inventário externo. No primeiro tick observado, os cinco diamantes
estavam no slot 1 do hopper, os dois entities já não existiam na área e o tanque
persistia 140 mB de `mob_grinding_utils:fluid_xp`; esse estado permaneceu pelos
20 ticks. O export registrou 21 blocos, 46 itens e 47 receitas. Brutos:
[fatos](atm10-resource-validation/mobgrinding-original-facts.json),
[trace](atm10-resource-validation/mobgrinding-original-machine.json) e
[cenário](atm10-resource-validation/mobgrinding-scenarios.json).

Ainda não há implementação: o host pode consultar proximidade de entidades,
mas `entity-state` não inclui item/XP e `despawn` só remove entidades próprias
do mod. A operação observada exige retirar entidades do mundo e creditar item
e XP fluido atomicamente. Esta transação não é representável hoje; nenhuma
entidade foi removida nem recurso fabricado no port. O caso fica pendente e a
fila L6 continua ao próximo mod independente.

## L6 — checkpoint Productive Metalworks: operação pendente de entrada observável

O export original do Productive Metalworks 1.15.1 registrou 148 blocos,
158 itens e 311 receitas. A trace de 20 ticks do `casting_table` vazio manteve
`fluid:{}`, `coolingTime:0`, `maxAmount:1000`, com cast/inventário vazios e sem
mudança. A evidência está em [facts](atm10-resource-validation/productivemetalworks-original-facts.json),
[trace](atm10-resource-validation/productivemetalworks-original-machine.json) e
[cenário](atm10-resource-validation/productivemetalworks-scenarios.json).

Não foi possível alimentar a mesa pela fixture de slots: o block entity original
não expõe `Container` ou menu público. O export de receitas também não contém
as quantidades de fluido dos recipes de casting. A API de fluido permite buffers
por posição, porém falta evidência black-box do caminho original que preenche a
mesa e dos valores que consome. O port não foi alterado; casting permanece
pendente até observar essa operação, sem inferir uma receita pela lista de IDs.

## L6 — checkpoint Pylons: colheita observada; loot do mundo pendente

No original Pylons 5.4.2, a trace do `harvester_pylon` com três trigos maduros
próximos registrou **2 wheat e 6 wheat seeds** no mundo depois de 20 ticks,
com energia 0 e inventário interno vazio. O export original registrou 5 blocos,
10 itens e 15 receitas. Brutos: [facts](atm10-resource-validation/pylons-original-facts.json),
[trace](atm10-resource-validation/pylons-original-machine.json) e
[cenário](atm10-resource-validation/pylons-scenarios.json).

A captura por tick não salva a identidade/estado dos blocos vizinhos, então não
uso essa saída para inferir a quantidade exata de plantas removidas. Embora a
API permita ler/trocar bloco e criar drops, não oferece uma operação atômica de
colheita que aplique a loot table com ferramenta/encantamentos e deposite o
resultado no inventário do pylon. Harvester permanece pendente; a fixture não
valida módulos de infusão, filtros ou os demais pylons.

## L6 — checkpoint Ranged Pumps: bomba observada; mutação do mundo fora do tick

No original Ranged Pumps 1.3.0, o buffer FE recebeu **32.000/32.000 FE**; com
duas fontes de água em coluna abaixo da bomba, o tick converteu as duas em stone,
acumulou **2.000 mB de água** e consumiu **200 FE**. A tag passou de
`CurrentPos:99` para `98` e depois `97`; a trace mostra a primeira unidade no
tick 0 e a segunda até o tick 9. O export tinha 1 bloco, 1 item e 1 receita.
Brutos: [facts](atm10-resource-validation/rangedpumps-original-facts.json),
[trace](atm10-resource-validation/rangedpumps-original-machine.json) e
[cenário](atm10-resource-validation/rangedpumps-scenarios.json).

O comportamento está delimitado, mas não foi implementado: o callback de
block entity recebe slots/propriedades locais e pode usar buffers; `world-read`
é só leitura perto do jogador e `server::set-block` só está disponível em hooks,
não no tick. Falta uma operação autoritativa no tick que leia o bloco de origem,
verifique espaço/energia e publique a troca fonte→stone junto com a água e o
débito de 100 FE. O port não simula água ou energia. O oracle agora grava
capability FE e estados dos blocos vizinhos por tick para esse tipo de caso.

## L6 — checkpoint Actually Additions: crusher integrado; demais máquinas pendentes

O export original de Actually Additions 1.3.26 registrou 125 blocos, 231 itens
e 509 receitas. A trace pública do `crusher` mostrou buffer de 60.000 FE,
recebimento de até 100 FE por chamada e sem extração. Um minério de ferro com
10.000 FE gerou 2 raw iron no tick 99 (100º tick) e consumiu 4.000 FE. Com
100 FE, avançou dois passos, reteve o input com 20 FE e parou. Com output cheio
e incompatível, não iniciou e não gastou energia. Evidências originais:
[facts](atm10-resource-validation/actuallyadditions-original-facts.json),
[trace](atm10-resource-validation/actuallyadditions-original-machine.json) e
[cenários](atm10-resource-validation/actuallyadditions-scenarios.json).

O port liga o crusher aos slots reais e ao buffer FE do bloco, valida output
com `machine::process-items` atômico e salva o progresso. A fixture WASM
[Actually Additions](atm10-resource-validation/actuallyadditions-horizon-machine.json)
passou em sucesso, energia insuficiente, output bloqueado e reload no tick 50;
o resultado correspondeu à trace original nos três campos observados (slots,
energia e progresso). No jogo real, [in-frame e servidor integrado](atm10-resource-validation/actuallyadditions-crusher-game.json)
preservaram o input sem energia por 100 ticks e após salvar/reabrir.

O jogo não teve uma fonte FE no cenário, então o resultado completo permanece
validado no componente WASM, não em uma sessão de jogo com energia conectada.
Esta evidência cobre somente o crusher e a receita de iron ore; Atomic
Reconstructor, Coal Generator, Canola Press e outras receitas de crusher ainda
estão pendentes. O mod permanece parcial. Foram aprovados 6 testes nativos do
port, Clippy com warnings negados e build do guest WASM.

## L6 — checkpoint Advanced Peripherals: conexão CC pendente

O export original de Advanced Peripherals 0.8.2a com CC:Tweaked 1.120.2
registrou 16 blocos, 33 itens e 26 receitas. As traces de 20 ticks sem
computador conectado para `environment_detector`, `player_detector` e
`chat_box` mantiveram `Items:[]` e slots vazios. Evidências:
[facts](atm10-resource-validation/advancedperipherals-original-facts.json),
[trace](atm10-resource-validation/advancedperipherals-original-machine.json) e
[cenários](atm10-resource-validation/advancedperipherals-scenarios.json).

O estado sem conexão não caracteriza as chamadas dos periféricos. O contrato
Horizon não expõe registro de peripheral CC:Tweaked, attach/detach de modem,
sessões/computadores Lua, retorno de métodos ou eventos/rednet. Portanto não há
como ligar esses dispositivos a uma máquina real nem comparar uma chamada
conectada; o port fica pendente, sem resposta simulada ou status de conclusão.

## Tarefa 4 — checkpoint Modular Routers (2026-11-02)

Puller Mk1 e Sender Mk1 agora pedem transferências reais ao host. As operações
rodam em ordem após devolver todas as views; cada uma revalida handle,
dimensão, inventários carregados, faces, compatibilidade, espaço e aliases.
O resultado confirma a quantidade movida; enfileirar não significa sucesso.
O componente `minecraft:custom_data` do próprio módulo guarda sua direção.
Pedidos são transitórios e não são gravados como operações pendentes no save.

| Evidência | Resultado | Limite |
| --- | --- | --- |
| [Jogo real nos dois modos](atm10-resource-validation/modularrouters-game.json) | Último item disputado sem duplicação; 18 itens conservados na extração parcial; origem vazia, buffer cheio/incompatível e saída cheia preservados | Somente Puller/Sender Mk1 adjacentes |
| Mesmo teste | Nome, encantamento e campo de energia preservados; buffer bloqueado sobreviveu ao save/reabertura e foi enviado ao liberar a saída | Configuração de direção via componente; interface completa de configuração pendente |
| Mesmo teste com outro mod WASM | Face estrangeira respeitada, snapshot de capabilities atualizado após mover, aliases recusados e destino removido antes da aplicação sem débito na origem | Não certifica todos os caminhos de rede/capabilities |
| Sandbox WASM | Pedido não altera slots antecipadamente, confirmação consumida uma vez, propriedade do handle, remoção e trap revogam a autorização; teste adicional confirma que IDs reutilizados no hot-reload não alcançam outro handle | O sandbox verifica o protocolo; a movimentação real é verificada no jogo acima |

[Ambiente, hashes e escopo](atm10-resource-validation/modularrouters-environment.json).
A suíte completa de runtime terminou com **52 testes passando e 1 ignorado**;
os 4 testes de inventários compartilhados e os 2 testes nativos do router
também passaram. Rustfmt, Clippy dos alvos alterados e árvore pública passaram.
A criação artificial de cobblestone e a remoção sem ação de mundo foram
retiradas. Breaker, dropper, vacuum e módulos remotos ainda não estão
implementados; não são apresentados como processamento real. Filtros,
augments, redstone, configuração completa e custos de energia configuráveis
permanecem pendentes. Nenhum mod recebeu status `ported` por esses testes.

**Tempo por tick e memória: pendentes.** Os testes acima provam conservação,
componentes e persistência, não uma redução de tempo. Não comparar callbacks
antigos que fabricavam/apagavam itens com produção correta como se fosse o
mesmo trabalho. As medições do loop real permanecem na tarefa 5.

## Tarefa 4 — Farmer’s Delight em execução

Implementada a consulta `machine::block-in-tag` usando as tags ativas do jogo,
e atualização das duas células abaixo de cada panela na dimensão correta.
O port consulta fonte/condutor e só aceita uma fonte com propriedade `lit`
quando ela está ligada. Build WASM release, check do cliente e teste sandbox
com tags presentes/ausentes passaram; os três testes de referência da API passaram.
**Validação da panela no jogo, segurança da saída e performance ainda pendentes.**

O host agora oferece `machine::process-items`: valida todos os débitos e
resultados antes de publicar a alteração, sem truncar resultados ou mesclar
componentes customizados. Simulação não escreve slots/dirty flags. O teste
sandbox com WASM real passou para segunda saída cheia, resultado incompatível,
entrada insuficiente, entrada duplicada recusada e sucesso preservando os
componentes da quantidade restante. A transação foi integrada à panela. O resultado do componente abaixo verifica
essa integração, sem certificar receitas/layout do mod original.


A inspeção dos dados carregados encontrou 28 receitas de panela: 66 ingredientes
por tag, 23 por item, cinco uniões e quatro diferenças. Quatro receitas declaram
recipiente explícito e quatro declaram tempo. A tabela fixa do port contém só
cinco receitas, várias divergentes desses dados. Isso permanece uma pendência,
junto ao consumo de ingredientes extras e ao armazenamento de refeições antes
de servir em recipientes. Não foi declarado que o mod está completo.

Referência de comportamento: [tags do autor](https://github.com/vectorwing/FarmersDelight/wiki/Tags)
e [panela](https://github.com/vectorwing/FarmersDelight/wiki/Cooking-Pot).
Nenhum fonte Java do mod foi consultado. Tempo/memória deste trecho: não medidos.

O [probe da panela com WASM real](atm10-resource-validation/farmersdelight-component.json)
passou: saída cheia/incompatível, ingrediente faltante/extra, fonte fria/apagada,
fonte quente, condutor, débitos exatos e save/load no meio do progresso. Os três
testes nativos do port e Clippy do probe passaram. O matcher não ignora mais
itens extras, e nenhuma entrada é debitada antes de todos os resultados caberem.
O escopo usa facts de teste e a tabela/layout atuais do port; receitas reais,
reservatório de refeições, serviço em recipientes e jogo nos dois modos continuam
pendentes. Não há medição nova de tempo/memória nem conclusão da tarefa 4.

O carregador do cliente passou a reconhecer `farmersdelight:cooking` e publicar
as receitas carregadas no grafo de máquinas, com tempos e quantidades dos dados.
Ingredientes por item/tag, uniões e diferenças são expandidos; predicados
não suportados são recusados em vez de ignorados. Um teste nativo confirmou
união, subtração de um item, tag vazia, predicado desconhecido e limite de
profundidade. A consulta desse grafo pelo port e os metadados de recipientes
**ainda não foram integrados**; não há validação das 28 receitas em jogo nem
medição nova de performance desta etapa.

O oráculo NeoForge executou o Farmer’s Delight original em servidor isolado,
com afinidade 0–5 e nice 10. A exportação pela API pública vanilla confirmou
recipientes e limites reais: Beef Stew empilha 16 e usa tigela; Apple Cider e
Hot Cocoa usam garrafa; Cabbage Rolls não tem recipiente; Tomato Sauce usa tigela.
[Validação e hashes](atm10-resource-validation/farmersdelight-original-facts-validation.json).
O dump completo fica em `.local/`, sem recursos preparados publicados. Esses
fatos ainda precisam ser aplicados ao port; não são evidência de correção dele.

### Continuação — receitas da panela e fatos do mod original

O port passou a consultar `farmersdelight:cooking` no grafo carregado, em vez da
antiga tabela de cinco receitas. Usa duração e quantidade de saída dos dados.
O matcher faz correspondência entre slots e ingredientes com alternativas
sobrepostas, sem depender da ordem dos slots. Um cache por instância do guest
é invalidado por `recipes::revision`, atualizado tanto por publicação do host
quanto por registro/substituição do guest; não se copia o grafo para cada panela
em cada tick. Isso é uma mudança de implementação, **sem benchmark de ganho ainda**.

Aplicados os recipientes padrão e limites dos 28 resultados observados no oráculo
mais os recipientes explícitos das receitas distribuídas. Resultados desconhecidos
são recusados. A API ainda não transporta o recipiente específico de uma receita
customizada; o reservatório/serviço correto e os templates de comida ainda estão
pendentes. O layout temporário do port não foi certificado como equivalente ao
original.

Validações desta continuação:

- Quatro testes nativos do port passaram, incluindo alternativas sobrepostas e
  receita de 100 ticks; Clippy do port e do runtime/probe passou.
- [WASM real](atm10-resource-validation/farmersdelight-component.json): conservação,
  calor e condutor, limites observados de empilhamento, duração/quantidade do grafo,
  resultado sem recipiente, garrafa para cider, save/load e invalidação do cache
  nas duas formas de publicação do host passaram.
- Sandbox: **33 passaram, 1 ignorado**; inclui registro/substituição de receitas
  pelo guest incrementando a revisão. Referência da API: **3 passaram**.
  A suíte completa de `horizon-mods --features runtime,pack` passou:
  **54 testes passaram, 1 ignorado**, incluindo os 18 testes da biblioteca.
- [Carregador real do cliente](atm10-resource-validation/farmersdelight-recipe-graph.json):
  14/28 receitas com o carry instalado isolado; 28/28 em cópia temporária com tags
  comuns observadas pelo oráculo. Conferidos ids, saídas, quantidades, durações e
  grupos de ingredientes resolvidos. O carry instalado tem tags comuns ausentes
  ou incompletas; corrigir/regenerar essa cobertura permanece pendente. Não se
  alterou o pack instalado, e o resultado isolado não certifica o pack inteiro.

O [oráculo da panela original](atm10-resource-validation/farmersdelight-original-serving.json)
passou em **seis cenários**, usando apenas APIs públicas de mundo e menu:
cozinhar sem recipiente; servir com tigelas; saída servida bloqueada; reservatório
cheio; fogueira apagada; condutor sobre magma. Confirmou slots 6=reservatório,
7=recipiente e 8=saída. Saída servida bloqueada permite cozinhar enquanto houver
espaço no reservatório; reservatório cheio preserva os ingredientes. O fixture
próprio ganhou configuração de blocos vizinhos e adaptação de slots do menu para
máquinas sem `Container` vanilla. O comportamento anterior do driver para
máquinas com `Container` foi preservado. Os cerca de 14 segundos de execução do
oráculo incluem inicialização: **não são benchmark de tick da panela**.

Não há nova redução medida de tempo por tick ou memória. A tarefa 4 continua em
andamento; a 5 continua pendente.

### Continuação — reservatório e proteção de menus

Implementado o layout observado: ingredientes 0–5, reservatório 6 (64 refeições),
recipiente 7 e saída servida 8. Cozimento e serviço usam transações separadas;
saída bloqueada preserva recipientes e permite armazenar refeições enquanto
houver espaço. O serviço funciona sem calor e respeita o limite do item servido.
O oráculo original passou em nove cenários, incluindo serviço parcial e em lote.

A política `machine::set-player-slot-access` protege o reservatório no menu,
independentemente das regras por face. O servidor aplica a política aos pacotes.
O teste real encontrou um desvio no caminho antigo de cliques; ele passou a usar
o motor verificado. Capacidades especiais também são aplicadas antes de copiar
slots ao menu, evitando truncar 64 refeições ao limite normal do item.

Validação: suíte completa do runtime **55 passaram, 1 ignorado**; testes nativos
de menus **9 passaram**; teste isolado do mapeamento de linhas incompletas passou
para 1/2/9/15/27/54 slots. Componente WASM passou no novo layout. [Jogo in-frame](atm10-resource-validation/farmersdelight-game-in-frame.json)
passou novamente no cenário completo, com resultado separado preservado. O [servidor integrado](atm10-resource-validation/farmersdelight-game-server.json)
passou no cenário completo após aguardar a atualização assíncrona do inventário,
sem avançar ticks adicionais. Esses cenários cobrem Cooked Rice e os casos
listados no relatório; não certificam o mod inteiro. Não há medição nova de
performance nesta anotação.

Detalhes e pendências: [design da panela](atm10-cooking-pot-design.md).
Tarefa 4 continua em andamento; tarefa 5 pendente.

### Continuação — complemento de tags na importação

O caminho de carry com facts do oráculo ignorava qualquer tag comum cujo arquivo
já existisse. Passou a complementar os membros observados sem apagar referências,
entradas opcionais ou sua ordem; tags com `replace: true` são preservadas. Tags
novas vazias continuam sendo emitidas. Não houve alteração no pack instalado.
O teste novo passou. A suíte da biblioteca do importador terminou com **69 passes
e 4 falhas**: `upgrades_ingredients`, `content_only_works_and_code_makes_it_partly`,
`recipes_for_left_out_blocks_are_left_out_too` e
`copies_the_players_jars_and_lists_what_needs_a_key`. As causas e a regeneração
real do carry ainda precisam ser verificadas; não se declara resolvida a cobertura
28/28 do pack instalado. Sem medição nova de performance.

A [comparação controlada](atm10-resource-validation/port-pack-tag-baseline.json)
repetiu os mesmos 69 passes e quatro erros retirando somente o complemento de
tags, com restauração integral do arquivo ao terminar. Portanto, essas quatro
falhas não foram introduzidas pelo complemento. Isso não prova que já existiam
em HEAD: as demais mudanças locais foram mantidas. Diagnóstico individual e
regeneração real do carry continuam pendentes.

A falha `upgrades_ingredients` revelou que o conversor legado aceitava uniões
com tags que exigem expansão pelo registro, além de descartar campos de
predicados desconhecidos. Agora recusa essas conversões sem modificar a receita;
o teste existente e o caso novo de predicado passaram. A suíte do importador
ficou em **70 passes e 3 falhas** (os dois testes de `check` e o de `fetch`).
Log local: `.local/atm10-resource-validation/port-pack-after-ingredient-fix.log`.
Não se declara a tarefa 4 concluída nem ganho de performance por essa correção.

Diagnóstico adicional: `check` calcula receitas suportadas usando só a contagem
de receitas modificadas pelo conversor. Isso confunde uma receita já compatível
com uma receita não suportada. O outro teste ainda espera excluir um bloco com
propriedade customizada, mas o carry atual declara essas propriedades. Essas
expectativas precisam ser reconciliadas com o comportamento real, sem mascarar
cobertura incompleta. O teste de fetch espera reutilizar um arquivo CurseForge sem
hash nem `Record.jar`; o caminho atual exige essa evidência para reutilizá-lo.

Na mesma inspeção, corrigido descarte silencioso de `data` legado inválido nos
ingredientes: valores negativos, texto, booleano e null agora são recusados sem
alterar a receita. O caso `data: 0` continua aceito. O teste de ingredientes cobre
as recusas; diagnóstico e correção dos contadores de cobertura continuam pendentes.

O conversor agora distingue internamente receita não normalizável de receita
normalizada sem alteração. O relatório tem uma contagem separada de receitas já
compatíveis para os tipos vanilla explicitamente reconhecidos; receitas alteradas
continuam em `recipes_upgraded`. Um teste de idempotência confirma que normalizar
novamente preserva o conteúdo e não conta como conversão. O teste de conteúdo
simples voltou a passar. Suíte: **71 passes e 2 falhas**, no cenário antigo de
bloco excluído e na reutilização de arquivo CurseForge. Log:
`.local/atm10-resource-validation/port-pack-recipe-count.log`.
Ainda falta reforçar a validação estrutural antes de atribuir cobertura às receitas
já compatíveis e reconciliar o cenário de propriedades customizadas.

Adicionada validação estrutural antes de contar receitas já compatíveis:
resultado presente, quantidade válida, entradas exigidas pelo tipo e, para shaped,
linhas retangulares de até 3×3 com símbolos exatamente definidos na chave.
Receitas incompletas e ingredientes não representáveis não recebem esse crédito.
O teste novo cobre receita válida, lista vazia, quantidade zero, campos ausentes
e símbolo sem definição. Passou; suíte: **72 passes e 2 falhas**. Log local:
`.local/atm10-resource-validation/port-pack-structure.log`. Isso é cobertura
estrutural, não prova de resolução dos ids/tags nem de gameplay de cada receita.

A tentativa de separar propriedade válida de propriedade inválida revelou uma
lacuna: nesse caminho de `check`, ambos os fixtures geraram declaração de stone
sem `state_properties`, inclusive o fixture válido `wobble=1`. Logo, não basta
atualizar a expectativa do teste para aceitar o bloco. Os testes exploratórios
foram retirados e o cenário original preservado; é necessário localizar onde a
propriedade é descartada antes da declaração. Evidência local:
`.local/atm10-resource-validation/port-pack-custom-states.log` (72 passes e três
falhas enquanto os fixtures exploratórios estavam presentes). Não se considera
resolvida a falha de cobertura.

Localizado o descarte: `reduce_blockstate` removia eixos customizados quando os
modelos selecionados coincidiam. Isso preservava a imagem, mas podia perder estado
de gameplay. Removida essa redução automática; domínios válidos sem template usam
a declaração com propriedades customizadas, enquanto domínios inválidos continuam
fora. O teste de exclusão agora usa domínio inválido de fato. Suíte do importador:
**73 passes e 1 falha**, somente a reutilização de arquivo CurseForge sem evidência
no fixture. Log: `.local/atm10-resource-validation/port-pack-preserve-states.log`.
Ainda são necessárias validação positiva do carry regenerado e verificação das
receitas/tags no cliente; não há ganho de performance medido nesta alteração.

A suíte do importador passou: **74 testes, zero falhas**. O fixture de fetch
agora distingue arquivo CurseForge sem hash/associação (não reutiliza só pelo
nome) do arquivo associado por `Record.jar` no inventário (reutiliza). O teste
verifica também que a recusa preserva o arquivo local. Código de produção de fetch
não foi flexibilizado. Log: `.local/atm10-resource-validation/port-pack-suite-green.log`.
Ainda falta regenerar o carry real em staging e conferir as 28 receitas no cliente.

[Regeneração real em staging](atm10-resource-validation/farmersdelight-regenerated-carry.json):
o pipeline preservou `facing/support` da panela e emitiu 36 tags comuns. Aplicando
somente essas tags geradas a uma cópia do carry anterior, o carregador real passou
de 14 para **28 receitas**. Assim, a correção das tags foi validada sem escrever
manualmente os membros na fixture. A regeneração completa com oráculo carrega só
**4 receitas**, pois recusa **76 alimentos sem template** e quatro itens de bloco.
Essa diferença expõe a próxima lacuna de recursos; não se substituiu o pack
instalado nem se declarou completa a regeneração. Sem benchmark novo.

O [novo censo de alimentos](atm10-resource-validation/farmersdelight-food-observations.json)
executou novamente o original em NeoForge isolado e exportou duração de consumo
e quantidade de efeitos pela API pública vanilla, além dos fatos anteriores.
O dump preparado permanece em `.local/`. Esses dados especificam a futura
extensão de itens; ainda não implementam nem validam consumo no port. Sem
benchmark novo de gameplay.

O [oráculo ampliado de efeitos alimentares](atm10-resource-validation/farmersdelight-food-effects-observations.json)
exportou ids, duração, amplificador, probabilidade e flags visuais dos efeitos do
componente FOOD. A contagem declarada foi conferida com o tamanho de cada lista.
O driver reutilizável `tools/export_farmer_food_facts.py` exige afinidade até seis
CPUs, nice 10, saída local e cria uma instância exclusiva com homes temporários.
Esses fatos ainda não cobrem comportamento adicional das subclasses de item nem
implementam efeitos customizados no Horizon. A extensão de alimentos continua
pendente; não se certifica o mod nem performance por esse censo.

Implementado o transporte opcional de `crafting_remainder` do oráculo para a
declaração de item, o overlay de componentes e os fatos reais de itens do cliente.
O parser usa o componente explícito antes da tabela de remainders vanilla.
O teste `mod_item_crafting_remainder_reaches_real_item_facts` constrói o overlay
sobre recursos preparados e confirmou a tigela no item modificado; passou. O
check da biblioteca do cliente também passou. Isso é remainder de crafting:
`use_remainder`, alimento/consumo/efeitos, validação dos ids de recipiente e
cenários de consumo no jogo ainda precisam de implementação/validação.
Não se declara resolvida a importação dos 76 alimentos.

A declaração `crafting_remainder` agora exige sintaxe de resource id com
namespace e caminho não vazios. Referências a outros mods continuam permitidas;
ids sem namespace, com caracteres inválidos ou múltiplos `:` são recusados na
leitura do pacote. O teste cobre sintaxe e recusa real de um `.hmod` inválido.
A resolução da existência do item no registro e o formato de `use_remainder`
ainda são pendências distintas.

Adicionado `use_remainder` opcional à declaração de item, com validação sintática
do id e transporte para `minecraft:use_remainder` no overlay. O teste com recursos
reais confirmou simultaneamente crafting→tigela e consumo→garrafa no mesmo item,
sem confundir os dois campos. Os oito testes de conteúdo do pacote passaram.
Isso confirma os dados lidos, ainda não o ato de consumir no jogo. Exportação do
oráculo para essa declaração, alimentos, efeitos e resolução de ids no registro
continuam pendentes; a tarefa 4 permanece aberta.

Implementada declaração básica `food` com nutrição, saturação total,
`can_always_eat` e duração de consumo explícita. O overlay publica os componentes
FOOD e CONSUMABLE; o teste sobre recursos reais confirmou os valores nos fatos
lidos pelo jogo, junto aos dois tipos de recipiente. Valores negativos/não
finitos e duração nula são recusados; campos de efeitos ainda desconhecidos são
recusados em vez de ignorados. O teste da declaração inválida também passou.
O oráculo ainda não passa a emitir alimentos por essa declaração: efeitos e
animações/comportamentos adicionais precisam ser representados antes de importar
os casos completos. Consumo real no jogo e benchmarks continuam pendentes.

A declaração alimentar passou a aceitar uma lista ordenada de efeitos com id,
duração, amplificador e probabilidade. O overlay escreve uma operação ApplyEffects
por entrada, preservando a ordem e as chances individuais. A validação limita a
lista a 64 entradas, exige ids sintaticamente válidos, amplificador 0–255,
duração >= -1 e chance finita em 0–1. O teste do overlay confirmou a presença do
efeito lido pelo cliente; a validação básica da declaração também passou.
Ainda falta conferir todos os parâmetros em teste, suportar flags visuais,
resolver ids de efeitos no registro e implementar Nourishment. O importador ainda
não libera alimentos com comportamentos incompletos. Não há evidência nova de
consumo real nem de performance nesta etapa.

Reforçada a prova do transporte de efeitos: o teste do overlay agora compara a
lista completa de dois efeitos com ids distintos, ordem, duração (inclusive -1),
amplificadores e probabilidades diferentes. Passou. O teste de validação também
confirmou recusa de chance negativa, acima de 1, NaN/infinito e lista acima de 64.
Inspeção do caminho de aplicação confirmou a lacuna de flags: `ConsumeEffectSpec`
ainda não transporta ambient/visible/show_icon e `EffectInstance` não tem
show_icon. Não se certifica preservação dessas flags nem consumo equivalente ao
original antes de implementar e testar esse caminho.

O transporte de efeitos de alimentos agora inclui `ambient` e `visible`:
a declaração, o overlay e o parser preservam ambas as flags, e o caminho de
aplicação as copia para `EffectInstance`. O teste de overlay passou com dois
efeitos de valores diferentes; o teste da declaração e o `cargo check` do binário
com mods também passaram. O controle independente `show_icon` ainda não existe
no estado de efeitos. Falta validar a aplicação em jogo, importar os fatos de
alimentos e implementar Nourishment. Nenhum ganho de tempo/memória foi medido.

A importação com fatos do oráculo agora gera declarações para alimentos cuja
classe é exatamente vanilla `Item` e cuja lista de efeitos exportada está vazia.
Nutrição, saturação, duração e possibilidade de comer sem fome vêm dos fatos;
subclasses, efeitos e dados incompletos continuam recusados. Recipiente após uso
só é declarado quando o oráculo confirma quantidade 1. Uma nova regeneração em
staging declarou **48 itens** (antes 14) e o cliente carregou **5 receitas de
panela** (antes 4). Permanecem **46 itens excluídos**; não se substituiu o pack
instalado. [Resultados e limitações](atm10-resource-validation/farmersdelight-ordinary-food-carry.json).
A suíte do importador passou com **75 testes** e o Clippy passou. Não há benchmark
novo de consumo nem certificação do mod.

### L1 — efeitos alimentares vanilla observados

A extensão do importador aceita efeitos vanilla conhecidos quando os fatos
incluem todos os parâmetros e `show_icon=true`; parâmetros ausentes, ids
desconhecidos/customizados e `show_icon=false` continuam recusados. Os testes
unitários conferem o payload exportado de `FoodEffect`; a lista reconhecida foi
comparada com `crates/horizon-client/src/brewing.rs`.

Validação em 2026-11-02: **75 testes passaram**, Clippy (`--all-targets`, warnings
negados), rustfmt e `git diff --check` passaram. A regeneração somente em staging
`.local/atm10-resource-validation/farmer-food-regenerated` leu 1.192 blocos e
185 itens observados; emitiu 51 declarações explícitas de item e deixou 43 sem
declaração. O relatório lista as exclusões: 39 alimentos com classe própria e
4 itens de bloco sem bloco associado. `wheat_dough`, `raw_pasta` e `chicken_cuts`
estão nas declarações, cada um com classe vanilla `Item`, efeito Hunger de 600
ticks, amplificador 0, chance 0,3, `ambient=false`, `visible=true`,
`show_icon=true`; nutrição/saturação 2/1,2 para dough e pasta, 1/0,6 para chicken
cuts; duração de consumo 1,6 s, 1,6 s e 0,8 s. O cliente carregou **5 receitas**
de panela, igual ao checkpoint anterior de 48 itens; não se exigiu aumento
artificial de receitas. Logs: `.local/atm10-resource-validation/port-pack-food-l1.log`
e `.local/atm10-resource-validation/farmer-food-l1-probe.log`.

Limites: validação da declaração e carregamento, não consumo em jogo. O motor
ainda não representa `show_icon` independentemente nem implementa
`farmersdelight:nourishment`; subclasses e efeitos customizados permanecem
recusados. L1 encerrado, sem iniciar outros itens da tarefa 4 ou a tarefa 5.

## Próximas rodadas

### L2 — consumo real dos três alimentos vanilla

A fixture `tools/test_farmer_food_consumption.py` usa o staging L1, homes
temporários e controle real do jogo. O binário fast anterior ao staging não
conhecia os itens; foi reconstruído apenas `horizon-client --features mods`, sob
CPUs 0–5/nice 10. A primeira execução também revelou um erro de observação no
teste: o efeito Hunger 255 usado para reduzir a fome continuava ativo e era
confundido com o efeito do alimento, além de drenar saturação. A fixture agora
limpa esse efeito antes das tentativas e exige Hunger de amplificador 0; em uma
tentativa sem proc, reaplica/limpa o efeito de preparação antes da próxima.

Validação real passou nos modos **in-frame e servidor integrado**. Para os três
itens, segurar menos que a duração observada não consumiu stack; segurar até o
limite consumiu uma unidade. Valores confirmados no primeiro consumo de cada
item: wheat_dough 0→2 de fome e 0→1,2 saturação; raw_pasta 2→4 e +1,2;
chicken_cuts 4→5 e +0,6. O efeito Hunger (amplificador 0) apareceu após 2/4
tentativas de wheat_dough em in-frame/servidor, respectivamente, e na primeira
tentativa de raw_pasta e chicken_cuts em ambos os modos. JSON completo:
`.local/atm10-resource-validation/farmer-food-consumption-l2.json`.

`py_compile` da fixture e `git diff --check` passaram. Não houve alteração de
lógica de produção. Escopo limitado aos três alimentos importados: isso não
valida subclasses, efeitos customizados, Nourishment nem show_icon. L2 concluído;
seguir L3.

### L3 — auditoria inicial de receitas e recipientes (lacuna depois resolvida)

O facts dump caixa-preta disponível contém 28 receitas `farmersdelight:cooking`.
Os 28 resultados mapeiam para um único `crafting_remainder` por item de saída
(sem conflito entre receitas no conjunto observado): bowl para pratos/sopas,
glass bottle para bebidas, nenhum para pratos empilháveis; `cooked_rice` é bowl,
`dumplings` não usa recipiente, e resultados vanilla de sopa usam bowl. O port
configura serviço por item em `ports/farmersdelight/src/cooking_items.rs` e usa
transação atômica `process_items` ao cozinhar e servir.

O mesmo dump identifica ingredientes com `crafting_remainder` nas receitas da
panela: `milk_bottle` (glass bottle) em hot cocoa, glow berry custard, pumpkin
soup e onion soup; `tomato_sauce` (bowl) em pasta with meatballs, pasta with
mutton chop e fish stew — sete relações ingrediente/receita. A transação de
cozinha atual consome cada entrada sem publicar esses remainders. O trace original
(`.local/atm10-resource-validation/farmer-serving-trace.json`) cobre nove cenários
de Cooked Rice/Dumplings e não mostra o resultado desses ingredientes. Os fatos
de crafting não provam se a panela original devolve os recipientes nem em que
slot/quantidade; aplicar por analogia a regra de crafting seria inferir o
comportamento. Também não há teste de saída bloqueada/falta de recipiente para
esses casos no jogo real.

Naquele checkpoint, a evidência caixa-preta dos ingredientes com recipiente
faltava; por isso a execução foi pausada sem alteração de gameplay. O oracle
adicional abaixo resolveu a lacuna.

### L3 — recipientes e receitas da panela (concluído)

O bloqueio de evidência foi resolvido por execução do original em NeoForge
isolado, usando só os fatos observados, o menu público e APIs vanilla do mundo;
nenhuma fonte/classe Java do mod foi lida. A matriz das **28 receitas** executou
82 casos: 28 com recipiente disponível, 28 com slot de saída incompatível e 26
sem recipiente. Para quatro resultados sem crafting remainder observado, foram
testados os candidatos explícitos do port (bowl para beetroot/mushroom/rabbit
stew e pumpkin para stuffed pumpkin). Todos passaram: com recipiente ou resultado
containerless, a porção foi ao slot 8;
saída bloqueada preservou resultado no slot 6, recipiente no slot 7 e o stack
estranho no slot 8; sem recipiente, os 26 resultados que o exigem ficaram no
reservatório. Dumplings produz 2 porções; os demais resultados observados,
1. Evidência sumarizada e ligada aos dumps brutos:
`.local/atm10-resource-validation/farmersdelight-cooking-l3-validation.json`.

Sete relações ingrediente/receita com `milk_bottle` ou `tomato_sauce` foram
executadas também em sucesso e saída bloqueada (14 casos). A panela original
consumiu os ingredientes e não devolveu crafting remainders; portanto a
transação existente, que consome entradas e publica o resultado atomicamente,
representa esses casos sem estender a API. A matriz confirma a tabela de
recipientes de `ports/farmersdelight/src/cooking_items.rs`; não há conflito de
recipiente para o mesmo resultado entre as receitas observadas.

Validação Horizon já existente e sem alteração de código: o teste de componente
WASM cobre transação atômica, ingredientes ausentes/extras, saída cheia, falta
de recipiente e save/load (`farmersdelight-component.json`); o teste de jogo
Cooked Rice passou nos modos in-frame e servidor integrado, incluindo saída
bloqueada, capacidade cheia e reabertura (`farmersdelight-game.json`). O client
loader havia carregado 28/28 receitas com tags observadas no fixture
`farmer-regenerated-tags-only`; carry instalado isolado ainda carrega 14/28.
Não afirmar teste de gameplay in-game para cada uma das 28 receitas ou conclusão
do mod. Checks da nova ferramenta `py_compile` e `git diff --check` passaram.

L3 concluído; seguir L4. Permanecem templates completos para consumo, subclasses,
Nourishment e efeitos customizados.

### L4 — auditoria de alimentos restantes (em execução)

Os **39 itens de comida excluídos** foram agrupados a partir do dump de efeitos
observados, sem ler fonte/classe Java do mod: 35 `ConsumableItem`, 2
`DrinkableItem`, 1 `DogFoodItem` e 1 `PopsicleItem`. Há 14 assinaturas de consumo
distintas. **26** itens têm `farmersdelight:nourishment`, chance 1, amplificador
0 e flags `ambient=false`, `visible=false`, `show_icon=false`, distribuídos entre
600/1.200/3.600/6.000 ticks. Dez usam efeitos vanilla (cinco Speed, duas
Regeneration, uma Absorption, uma Glowing e uma Nausea); três não declaram
efeitos (`dog_food`, `melon_popsicle`, `tomato_sauce`). Além desses 39, quatro
itens não alimentares também foram excluídos por falta de bloco associado; estão
fora do agrupamento L4. Detalhamento por id e assinatura:
`.local/atm10-resource-validation/farmersdelight-l4-audit.json`.

O primeiro grupo já encontra um limite de arquitetura: a aplicação de consumo
resolve efeitos via `horizon_client::brewing::effect` e ignora ids não
registrados; `farmersdelight:nourishment` não é um efeito vanilla e a Mod API/WIT
não oferece registro nem callback de ciclo de vida para efeitos de jogador. Não
é seguro emitir essas refeições como se o efeito tivesse sido aplicado. Contrato
mínimo para revisão: registro de efeito namespaced com lifecycle/tick autoritativo
no servidor e transporte de id, duração, amplificador, chance e flags visuais
independentes. A semântica de gameplay do Nourishment ainda precisa ser observada
no original antes de implementar esse contrato.

O bloqueio de API foi resolvido no checkpoint de efeitos abaixo. Os grupos L4
devem agora ser observados por consumo real antes de implementar, um grupo de
assinatura por vez. Os três itens sem efeito seguem sem comportamento inferido.

#### Checkpoint de arquitetura e observação black-box (2026-11-02)

O usuário aprovou o contrato mínimo de efeitos modded: registro por ID
namespaced e ciclo de vida autoritativo no servidor. Antes de implementar
Nourishment, o harness público do servidor original foi ampliado para aplicar
`farmersdelight:nourishment` via registry e avançar `MobEffectInstance.tick` +
`FoodData.tick`, comparando com controle sem efeito. Não foi lida nenhuma classe
do mod. A observação inicial de aplicação direta mostrou preservação de fome e
saturação enquanto o efeito está ativo. A extensão agora consome efetivamente
`farmersdelight:cooked_rice` (2→1), partindo de fome 4/saturação 0: resultado
fome 10/saturação 4,8 e Nourishment por 600 ticks com amplificador 0 e as três
flags visuais falsas. Com exaustão induzida de 0,2/tick, fome/saturação ficam
estáveis até o tick 599; no tick 600 o efeito expirou; no tick 620 a saturação
caiu para 3,8. Isso cobre consumo, último tick, expiração e efeito pós-expiração.
O controle sem efeito continua registrado no trace anterior.

Raw `.local/atm10-resource-validation/nourishment-blackbox-raw.json`
(SHA-256 `94162d0864c0c35d6c744af6f0e9d369dae789c2474c2b5ee506d873d1ff1895`);
consumo raw `.local/atm10-resource-validation/nourishment-consumption-raw.json`
(SHA-256 `fa6ea8ec4593751f852026b3abc7e4938096010769db7e8a4badb2410cd16b91`)
e relatório `nourishment-consumption.json`. A fixture usa
`FakePlayer.finishUsingItem` e APIs públicas `MobEffectInstance`/`FoodData` por
tick; não mede o timing de entrada real. Hash do jar original:
`139ad7696462c89c03eea463f805abffa552526c5dadaadae221dd9624cb197c`.

Contrato mínimo implementado: capability/API 0.6 `status-effects`, registro
namespaced restrito ao namespace do mod, lista de efeitos ativos e atualização
de fome/saturação pelo callback do servidor. O ciclo de duração/expiração e
persistência reutiliza o estado autoritativo de efeitos do jogador; sync ao
cliente LAN tem pacote namespaced próprio, e clientes exibem flags independentes.
mods API 0.6 com `status-effects` recebem o hook `tick` por jogador, depois do
tick autoritativo de comida. Isso mantém a interface WIT antiga de eventos
compatível com componentes existentes. Os dados de contexto incluem UUID do
jogador, e o port guarda o baseline por UUID. A primeira fixture integrated
server-thread revelou que o hook global executava fora de `as_guest`; a mudança
foi corrigida e agora o teste do port passa nos dois modos. O pacote de teste
confirmou owner-scope e callback autoritativo. `status-effects-game.json`
continua cobrindo o contrato genérico, incluindo flags visuais, aplicação,
save/reopen, expiração e sync. `cargo build --profile fast -p horizon-client
--features mods`, biblioteca do runtime (21 testes), sandbox, importador e testes
focados do cliente passaram. O teste da capability/API 0.5 foi corrigido para
esperar erro no parse do manifesto.

#### L4 — grupo ConsumableItem com Nourishment (2026-11-02)

O oracle original consumiu uma unidade de cada um dos **25** alimentos
`ConsumableItem` com Nourishment, a partir de stack unitário, por
`Item.finishUsingItem`. Todos devolveram um bowl; fome, saturação, duração,
amplificador e flags coincidiram com os dados observados. Em cada caso, os dados
de comida permaneceram estáveis até o último tick ativo; após +40 ticks da
expiração, fome ou saturação já tinham mudado. Raw
`.local/atm10-resource-validation/nourishment-group-raw.json` (SHA-256
`b43f923cfa299b82c82faefcee19cc31da46b0954a8437979a36e876bd38a074`); relatório
`nourishment-group.json`. Usa APIs públicas e `FakePlayer`, não timing de input.

O importador agora libera somente os 25 IDs com classe, assinatura e parâmetros
observados; emite `minecraft:bowl` como use remainder a partir do consumo
completo observado. Regeneração em staging carregou 76 itens no total, incluindo
25 Nourishment; 18 continuam excluídos. Testes recusam `bone_broth` (Drinkable)
e item desconhecido da mesma classe. O componente Farmer's Delight atualizado
foi validado com `cooked_rice` em jogo in-frame e servidor integrado: consumo
2→1, bowl retornado, efeito namespaced visível no cliente, comida preservada
durante exaustão, save/reopen, expiração e retomada do dreno. Evidência
`.local/atm10-resource-validation/farmersdelight-nourishment-game.json`.

Escopo: os 25 passaram pelo oracle; o jogo testa `cooked_rice` como representante.
Não cobre o timing original de entrada nem todos os 25 individualmente no jogo.
O grupo com efeitos vanilla foi validado em jogo no checkpoint abaixo. Os quatro
alimentos restantes consumíveis foram observados, mas ainda precisam de
importação e validação no port. `DogFoodItem` requer decisão de arquitetura para
interação item-entidade. Não iniciar L5 até concluir L4.

#### L4 — efeitos vanilla e observação das subclasses restantes (2026-11-02)

O oracle original consumiu os nove `ConsumableItem` com efeitos vanilla por
`Item.finishUsingItem`: Speed nos cinco slices, Regeneration nos dois salads,
Glowing no Glow Berry Custard e Nausea no Nether Salad (chance observada 0,3).
Foram registrados duração, flags, nutrição, saturação e recipiente; os efeitos
aleatórios foram amostrados até proc. Importação limita-se aos IDs e parâmetros
observados. O port passou os nove alimentos em jogo, tanto in-frame quanto no
servidor integrado, incluindo efeito, nutrição e recipiente; evidência
`farmersdelight-vanilla-effect-game.json`.

Hashes e relatórios: `vanilla-effect-food-group.json` (raw SHA-256
`105fdf59b999c1ff4a8f5354c717391bff3280b8c06621c83d40c4234e5f6f83`, jar
`139ad7696462c89c03eea463f805abffa552526c5dadaadae221dd9624cb197c`);
`remaining-food-group.json` (raw SHA-256
`0070f42ed3b9a01ac96e4e7c5c77e2f435444c724ad197781f9cbfe1a7402140`). Este
último observou Apple Cider, Bone Broth, Tomato Sauce e Melon Popsicle via
consumo público: respectivamente Absorption + garrafa, Nourishment + tigela,
sem efeito + tigela, sem efeito + nenhum recipiente. Dog Food também é comida
para o jogador: acrescenta 4 de fome e 1,6 de saturação, retornando bowl. A
observação do grupo usa `finishUsingItem` sem wolf como alvo; o oracle separado
abaixo usa o resultado da interação item-entidade e confirma que essa rota não
consome stack nem altera fome.

Para esclarecer Dog Food, o oracle chamou `Item.interactLivingEntity` no original
com wolves criados por API pública. Em wolf domesticado do jogador, o resultado
é `SUCCESS`, mas saúde e stack ficam iguais; wolf não domesticado retorna
`PASS`. Relatório `dog-food-original.json`, raw SHA-256
`46b39f48b80ea925e4e56aa9f17b5acca4a7b530bcabd87d09c1dd37ccb68978`, mesmo jar
acima. O usuário aprovou o evento item-entidade autoritativo e os dados mínimos
de identidade; o checkpoint abaixo registra sua implementação.

#### L4 — interação item-entidade e alimentos restantes (2026-11-02)

O evento `interact-entity` roda no servidor e entrega o id namespaced do alvo;
wolves também informam `tamed` e `owner_is_player`, calculados pela comparação
autoritativa dos UUIDs. A resposta do callback diferencia `PASS` e sucesso pelo
resultado existente da API. Um `PASS` explícito termina a rota daquele item
contra a entidade antes da alimentação genérica, sem cura nem consumo.

Dog Food retorna sucesso somente para wolf domesticado pelo jogador e `PASS`
para wolf não domesticado ou domesticado por outro jogador. A fixture real do
jogo verificou wolf domesticado do jogador e wolf não domesticado em in-frame e
servidor integrado: saúde 8→8 e stack 1→1. O teste unitário do port confere os
resultados exatos do callback. Como comportamento separado, consumo normal por
jogador também foi mantido: +4 de fome, +1,6 de saturação e um bowl nos dois
caminhos. Nenhuma operação de cura ou consumo foi adicionada ao evento.

O importador agora libera somente os IDs da observação black-box para as quatro
comidas restantes e usa os valores realmente ganhos em `finishUsingItem`, que
divergem dos dados FoodProperties exportados para essas subclasses: Apple Cider
4/0 com Absorption 1200 e garrafa; Bone Broth 12/11,2 com Nourishment 1200 e
bowl; Melon Popsicle 7/1,2 sem recipiente; Tomato Sauce 8/3,2 com bowl. O grupo
passou no jogo in-frame e servidor integrado, verificando fome, saturação,
efeitos no cliente, stack e recipientes. Dog Food teve teste separado para sua
interação com wolf e consumo pelo jogador nos mesmos dois caminhos.

| Evidência | Resultado |
| --- | --- |
| [Interação e consumo de Dog Food](atm10-resource-validation/dog-food-port-game.json) | Quatro interações (dois resultados × dois caminhos) preservaram vida e stack; consumo pelo jogador também passou em ambos |
| [Quatro alimentos restantes](atm10-resource-validation/farmer-remaining-food-game.json) | Apple Cider, Bone Broth, Melon Popsicle e Tomato Sauce passaram nos dois caminhos |
| [Staging L4 regenerado](atm10-resource-validation/farmer-l4-final-carry/mods/farmersdelight/carry-report.json) | 187 definições de item adicionadas; quatro exclusões restantes são itens de bloco sem bloco associado |
| [Oracle original](atm10-resource-validation/remaining-food-group.json) | Raw SHA-256 `0070f42ed3b9a01ac96e4e7c5c77e2f435444c724ad197781f9cbfe1a7402140`; jar conforme hash registrado acima |

L4 concluído para os 39 alimentos auditados: 25 com Nourishment, 9 com efeitos
vanilla, quatro consumíveis restantes e Dog Food. O jogo não executou
individualmente os 34 primeiros alimentos nesta fixture; os escopos dos grupos
estão registrados acima. Nenhum status de Farmer's Delight ou de alimento foi
marcado como mod completo. Seguir L5.

| Tarefa | Cenários / números a registrar | Estado |
| --- | --- | --- |
| 3 — trabalho ocioso | Ativos versus carregados; operações por tick; remoção/descarregamento; produção por tick; antes/depois em 1/100/1.000 entidades | Concluída no escopo da tarefa; resultados na rodada 3 |
| 4 — simulação e conservação | Por mod: sucesso, saída bloqueada, recursos insuficientes e save/load; tempo com processamento real, não callback vazio | Em execução; checkpoints Charging Gadgets e Modular Routers acima |
| 5 — loop do jogo | Construção/cópia de views, serialização por entidade, reconciliação de capabilities, tempo total por tick e memória; ativo/ocioso nos dois modos single-player | Parcial: cache de propriedades medido; RSS server-thread +8,58 MiB sem causa confirmada; callback de 1.000 máquinas ainda >50 ms/tick |

## Condições para as próximas execuções

A pedido do usuário, builds e testes ficam limitados a **6 CPUs lógicas (0–5)**,
com prioridade **nice 10**. Usar `CARGO_BUILD_JOBS=6`, `RAYON_NUM_THREADS=6` e
`taskset -c 0-5 nice -n 10 ...`; testes gráficos podem precisar de execução em
série. Manter essa condição tanto no antes quanto no depois de cada comparação.
Não alterar a configuração global do desktop. Jogos usam exclusivamente
HORIZON_HOME, HORIZON_LAUNCHER_HOME, XDG_DATA_HOME e XDG_CONFIG_HOME temporários,
com self-update e áudio desativados conforme AGENTS.md.

## L6 — checkpoint Ars Technica: operação sem entrada ainda não demonstrada

O export de Ars Technica 2.7.6 com Create 6.0.10 e Ars Nouveau 5.13.3
registrou 3 blocos, 34 itens e 49 receitas. No traço de 20 ticks sem
conexões, `source_motor` manteve `Fueled:0`, `Speed:0.0` e
`GeneratedStressUnitsRatio:100`; `precise_relay` ficou sem fonte e
`transmutation_turret` manteve `spell_caster:{}`. Evidências:
[facts](atm10-resource-validation/arstechnica-original-facts.json),
[trace](atm10-resource-validation/arstechnica-original-machine.json) e
[cenários](atm10-resource-validation/arstechnica-scenarios.json).

Esse ensaio de repouso não especifica acionamento do motor, ligação cinética,
entrada de mana ou execução de transmutação. O Horizon não fornece contexto de
stress/kinetic do Create nem execução de spell/mana do Ars Nouveau ao callback
do port. O estado interno atual do port não prova comportamento operacional;
Ars Technica segue pendente, sem status de conclusão.

## L6 — checkpoint BotanyPots: cultivo iniciado; colheita ainda não observada

O export de BotanyPots 21.1.44 registrou 183 blocos, 183 itens e 375 receitas
(incluindo 73 receitas de cultura e 51 de solo). No original, um
`terracotta_botany_pot` recebeu `minecraft:dirt` no slot 0 e
`minecraft:wheat_seeds` no slot 1. Em 1.200 ticks `growth_time` avançou de
0 para 1.199, `comparator_level` chegou a 14, e ambos os itens permaneceram nos
slots; não apareceu saída nem queda de item. Evidências:
[facts](atm10-resource-validation/botanypots-original-facts.json),
[trace](atm10-resource-validation/botanypots-original-machine.json) e
[cenários](atm10-resource-validation/botanypots-scenarios.json).

Isso comprova apenas o estágio de crescimento parcial. O limiar de produção,
loot, decremento/replantio e interação de exportação permanecem sem observação;
as receitas de cultura do export não carregam ingredientes nem resultado
utilizável para derivar esses dados. Não ampliar a implementação interna atual
com valores presumidos. BotanyPots permanece parcial.

## L6 — checkpoint Cable Tiers: oracle impedido por dependência ausente

O manifesto público do JAR `cabletiers-neoforge-1.21.1-0.6.15.jar` exige
`refinedstorage` na versão `2.0.0-beta.17`. O carregador NeoForge recusou o
export isolado com dependência obrigatória ausente; a instância ATM10 contém
Cable Tiers e Applied Energistics 2, mas não contém o JAR de Refined Storage.
O log do oracle está em
[.local/neoforge-oracle/runs/export/server.log](../../.local/neoforge-oracle/runs/export/server.log).
Sem iniciar o mod original não há entradas/saídas observadas para validar as
transferências prometidas pelo port. O checkpoint não certifica nem implementa
throughput; Cable Tiers permanece pendente até haver o artefato original e a
rede correspondente disponível.

## L6 — checkpoint Tree Tap: reprodução ainda inconclusiva

O export original de Tree Tap 0.4.6 com Integrated Dynamics, Cyclops Core e
Common Capabilities registrou 4 blocos, 1 item e 2 receitas incondicionais
carregadas. A receita `integrateddynamics:menril_logs` publicada exige 2.400
unidades de processamento e seis logs; sua saída é o bucket de menril resin.
No cenário forçado com `treetap:tap` em `attached=true`, seis menril logs ao
redor e 2.500 ticks, a tag do bloco permaneceu vazia. Aplicar o item tap a uma
bétula retornou `PASS_TO_DEFAULT_BLOCK_INTERACTION`; isso não testa a receita
condicional de menril. [facts](atm10-resource-validation/treetap-original-facts.json),
[trace](atm10-resource-validation/treetap-original-machine.json) e
[cenários](atm10-resource-validation/treetap-scenarios.json).

O harness de BE não reproduz a colocação do bloco pelo item nem o estado/link
produzido por essa colocação. Portanto a trace forçada não é suficiente para
inferir o ciclo de produção, recipientes, sazonalidade ou loot. O port atual
mantém seiva/timers internos sem consultar árvore ou transferir para recipiente;
Tree Tap fica pendente, sem simulação declarada como válida.

## L7 — checkpoint do loop do jogo: cache de domínios; regressão RSS pendente

Adicionei medição opt-in em `game/mods.rs` para tempo total, criação de
block-entities, montagem/cópia das views, reconciliação de capabilities, callback
WASM e serialização privada por entidade. Sem `HORIZON_MOD_TICK_PROFILE`, a
medição não grava nem imprime amostras. O benchmark usa o crusher do Actually
Additions, sem item (ocioso) ou com iron ore e zero FE (entrada elegível sem
recurso). Não é um caso de produção ativa com energia.

Antes/depois: mesmo binário `fast` com `mods`, cenário, recursos, hmod e perfil;
40 callbacks por estado/tamanho; CPU 0–5, nice 10, seis threads, sem builds ou
outros jogos concorrentes. A única mudança entre as medições é cachear os
valores válidos de propriedades estáticas por ID de bloco/propriedade, evitando
varrer todo o registro para cada crusher. Tempos abaixo são p50 de tick, ms:

| Modo | Máquinas ociosas | p50 antes | p50 depois | View copy antes → depois |
| --- | ---: | ---: | ---: | ---: |
| in-frame | 1 | 0,733 | 0,085 | 0,637 → 0,006 |
| in-frame | 100 | 54,342 | 3,044 | 51,263 → 0,263 |
| in-frame | 1.000 | 633,669 | 118,069 | 512,960 → 2,977 |
| servidor integrado | 1 | 0,574 | 0,087 | 0,504 → 0,006 |
| servidor integrado | 100 | 40,790 | 3,105 | 38,063 → 0,271 |
| servidor integrado | 1.000 | 491,651 | 120,045 | 382,172 → 2,980 |

As métricas p50/p95/máximo por etapa, todos os casos com input sem FE, uso de
memória, logs brutos e hashes estão em
[comparação reproduzível](atm10-resource-validation/mod-tick-profile-comparison.json),
[antes in-frame](atm10-resource-validation/mod-tick-profile-before-in-frame.raw.log),
[antes servidor integrado](atm10-resource-validation/mod-tick-profile-before-server-thread.raw.log),
[depois in-frame](atm10-resource-validation/mod-tick-profile-after-memory-in-frame-0.log)
e [depois servidor integrado](atm10-resource-validation/mod-tick-profile-after-memory-server-thread-0.log).
O callback do crusher permanece dominante com 1.000 máquinas: p50 de cerca de
98–103 ms, contra aproximadamente 3 ms de montagem das views, 2 ms de
capabilities e 4,5 ms de serialização. Logo, mesmo depois da melhora, o tick
excede os 50 ms necessários para 20 TPS nesse caso.

RSS corrente no processo com 1.000 máquinas ociosas: in-frame caiu de 992,94
para 984,54 MiB; servidor integrado subiu de 1.180,04 para 1.188,62 MiB
(+8,58 MiB; VmHWM subiu de 1.317,85 para 1.333,82 MiB). Essa diferença de
memória do servidor integrado não é explicada pelo cache, que só guarda os
valores estáticos das propriedades do crusher; não tenho repetição suficiente
para separar variância de uma regressão real. Fica registrada como regressão
potencial sem causa confirmada; a tarefa 5 e o fechamento L8 ficam pendentes
até investigar esse resultado.

A fixture de save/reopen do crusher foi repetida e passou nos dois modos após a
mudança. `cargo build --profile fast -p horizon-client --features mods`, Clippy
com `-D warnings`, rustfmt e `git diff --check` passaram. Não foi introduzido
dirty tracking nem alterada a serialização/estado privado do guest. Um caminho
com produção alimentada e comparação temporal antes/depois para esse caminho
permanece pendente.

## L8 — auditoria final suspensa pela medição de memória da tarefa 5

O registro confirma L1–L3 concluídos no tracker; L4 concluído para os alimentos
auditados sem declarar Farmer’s Delight completo; L5 validado apenas para os
casos documentados de Mining Gadget e Puller/Sender Mk1; L6 tem checkpoints
parciais para os casos prioritários, além de outros ports concretos ainda sem
triagem. A tabela geral continua com tarefa 4 em execução. A tarefa 5 encontrou
o resultado RSS acima sem causa confirmada, então esta auditoria não aprova o
plano nem atualiza status de mods para completo. Retomar investigando memória
servidor integrada; depois fechar as lacunas de processamento ativo e seguir os
casos restantes de L6 antes de qualquer fechamento final.

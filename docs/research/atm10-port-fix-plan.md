# Plano de correção dos ports ATM10

Base: [revisão de 2026-11-01](atm10-port-performance-review.md).
Medições e números consolidados: [acompanhamento de performance](atm10-performance-tracker.md).
Execute uma tarefa por vez. O primeiro passo é pequeno e adequado a um modelo
mais barato; as mudanças de sincronização e save exigem revisão cuidadosa.
A tarefa 1 foi implementada e validada em 2026-11-01; veja os
[resultados e JSONs do benchmark](atm10-view-index-bench/README.md).
A tarefa 2 foi concluída para os baús compartilhados: a [extensão de inventário compartilhado](atm10-shared-inventory-design.md)
substitui cópias por referências ao inventário canônico no host. As fixtures
WASM e os testes com funis/save dos dois baús passaram nos caminhos in-frame e
servidor integrado. O benchmark pareado e os JSONs estão no acompanhamento central; a memória
extra de regras padrão no DimStorage será tratada na tarefa 3. A suíte geral
do cliente completou 1.092 testes, com duas falhas em arquivos sem alterações
desta implementação, registradas para investigação na validação final.
A tarefa 3 foi concluída no seu escopo: seleção de handles ativos, invalidação
em remoção e retirada das regras padrão redundantes, com testes e benchmarks
ativos/ociosos registrados na rodada 3 do acompanhamento central. A tarefa 4
está em execução: Charging Gadgets passou na fixture WASM e no jogo real
nos dois modos usando bateria de teste. Modular Routers passou nos dois modos
para Puller/Sender Mk1 adjacentes, com componentes, conservação e persistência;
os módulos de mundo/remotos e demais recursos ainda estão pendentes. Farmer's
Delight está em execução: calor e transações passaram no componente; o port já
consulta o grafo de receitas e invalida o cache, mas reservatório/serviço, tags
comuns do carry e validação no jogo continuam pendentes. Os demais casos
continuam pendentes. A 5 continua pendente. Nenhum mod foi marcado
como concluído.

Checkpoint 2026-11-02: a tarefa 4 tem novos casos registrados no
[acompanhamento central](atm10-performance-tracker.md), mas permanece em
execução; limites de API/referência não equivalem a mods completos. A tarefa 5
mediu e cacheou os domínios estáticos de propriedades das views. O callback
WASM ainda excede 50 ms/tick com 1.000 crushers, e a medição server-thread
mostrou +8,58 MiB de RSS corrente sem causa confirmada. A auditoria final foi
suspensa para investigar essa diferença de memória.

## Instrução para colar no modelo

> Leia AGENTS.md, CONTRIBUTING.md e
> docs/research/atm10-port-fix-plan.md. Execute somente a tarefa 1 deste plano.
> Preserve as alterações locais existentes. Não implemente as demais tarefas,
> não altere ports, WIT nem a API pública. Entregue o diff, testes e comparação
> de benchmark antes/depois. Não marque mods como concluídos.

## 1. Remover a busca linear de views no runtime — começar aqui

Arquivos: `crates/horizon-mods/src/runtime.rs` e os testes de runtime em
`crates/horizon-mods/tests/sandbox.rs` (ou testes internos apropriados).

Problema: `State::view(handle)` percorre todas as entidades emprestadas ao mod
em cada leitura/escrita de slot. Baús fazem 27 leituras por entidade por tick,
tornando esse caminho quadrático.

Implementação:

- Adicionar a `State` um índice `HashMap<u32, usize>` de handle para posição.
- Construí-lo uma vez quando as views são emprestadas ao callback. Reutilizar
  a capacidade do mapa quando possível. Inicializá-lo no construtor de State.
- Fazer `State::view` consultar o índice e acessar o vetor com `get_mut`.
- Auditar também os acessos diretos `views.iter().find/any` por handle, usando
  o mesmo índice quando o significado for o mesmo. Não mudar buscas espaciais.
- Invalidar o índice quando as views retornam ao jogo. Conferir a saída por
  trap/fuel exhausted, que move as views para `Mods::stranded`, e o reload.
- Preservar propriedade dos handles, ordem das views, mutações anteriores a
  um trap e reconciliação de capabilities entre mods. Não manter referências
  ou ponteiros para dentro do vetor.

Aceite:

- Testes com handles em ordem diferente da criação e subconjuntos diferentes
  em ticks consecutivos; handle ausente não pode alcançar uma view antiga.
- Testar leitura/escrita na máquina correta e isolamento entre dois mods.
  Exercitar também retorno de views após um callback que falha.
- `cargo test -p horizon-mods --features runtime,pack` passa.
- Rustfmt e Clippy dos alvos alterados passam, com warnings negados.
- Medir DimStorage e EnderStorage em 1, 100 e 1.000 baús vazios antes/depois.
  Guardar JSON e comparar p50/p95 sob as mesmas condições, sem builds
  concorrentes. O lookup deixa de percorrer N views por acesso; se o tempo
  continuar crescendo muito acima do linear, localizar o custo restante antes
  de declarar resolvida a performance global.

Benchmark disponível: `crates/horizon-mods/examples/port_tick_bench.rs`.
Build: `cargo build --release -p horizon-mods --features runtime,pack --example port_tick_bench`.
Uso: `target/release/examples/port_tick_bench CARRY_DIR CORE_WASM BLOCK COUNT TICKS OUTPUT_JSON`.
Reconstruir o core WASM com o SDK atual; não fazer deploy. Para estes mods:
`cargo build --manifest-path ports/Cargo.toml --target wasm32-unknown-unknown --release -p dimstorage-port -p enderstorage-port`.
Blocos: `dimstorage:dimensional_chest` e `enderstorage:ender_chest`.
Os cores ficam em `ports/target/wasm32-unknown-unknown/release/<id>_port.wasm`;
os manifests em `~/.local/share/horizon/ports/all-the-mods-10-atm10/mods/<id>/carry/`.
O probe empacota cópias temporárias, sem escrever no pack instalado.

O diamante ainda reaparecerá após a tarefa 1: corrigir isso pertence à tarefa 2.

## 2. Corrigir baús compartilhados — uma mudança por mod

Arquivos: `ports/dimstorage/src/lib.rs`, depois `ports/enderstorage/src/lib.rs`.
Antes de implementar, descrever como o estado canônico do canal recebe alterações
externas, resolve operações em dois endpoints e persiste. Um snapshot anterior
pode distinguir alterações externas de uma cópia antiga; não basta inverter
qual lado sobrescreve o outro. Verificar se a API permite representar as
operações sem ambiguidade; se faltar suporte, documentar o limite e propor uma
mudança pequena na API antes de alegar correção completa.

Respeitar frequência, dimensão e proprietário; remover o canal fixo de
EnderStorage. Implementar save/load e limpeza de handles, mantendo canais
com itens mesmo sem endpoints carregados. Não apagar itens ao remover o último
baú. Evitar persistir cópias divergentes do mesmo canal em múltiplos blocos.

Aceite no componente WASM: inserir/retirar parcial e totalmente, dois baús no
mesmo canal, canais diferentes, operações em ambos no mesmo tick, inverter
ordem dos handles, descarregar/recarregar e reabrir save. Quantidade total
conservada; nenhuma reposição após extração nem perda ao reiniciar.
Repetir o benchmark. Em seguida validar funis e persistência nos caminhos
in-frame e servidor integrado, sempre em diretórios temporários conforme AGENTS.md.

## 3. Reduzir trabalho ocioso — depois da correção funcional

Uma tarefa por vez: sincronizar apenas canais/slots modificados onde a API
permitir; iterar os handles recebidos em Ars Technica, BotanyPots e Cable Tiers;
invalidar mapas em remoção de máquinas. Manter a simulação limitada às entidades
ativas. Não alterar frequência de tick para esconder custo nem desativar máquinas.
Aceite: cenário ativo e inativo, descarregamento e remoção, mesma produção por
tick e menor trabalho ocioso demonstrado por benchmark.

## 4. Corrigir recursos e completar simulação — um mod por tarefa

Priorizar Charging Gadgets (carga efetiva no item), Modular Routers (origem e
destino reais, sem criar/apagar itens), Farmer's Delight (calor real e espaço na
saída antes de consumir). Depois seguir os demais casos da tabela da revisão.
Cada tarefa precisa de fixture WASM de sucesso, saída bloqueada, recursos
insuficientes e save/load. Consultar a skill port-modpack antes da implementação.
Se a API não permitir o comportamento, registrar pendência concreta.

## 5. Otimizar o loop do jogo e fechar validação

Medir separadamente criação/cópia de views, serialização por entidade e
reconciliação de capabilities em `crates/horizon-client/src/game/mods.rs`.
Só então introduzir dirty tracking, incluindo mudanças do estado privado do
guest: `view.changed` sozinho não cobre timers/energia mantidos pelo mod.
Preservar transferências entre mods, crash/reload e saves.

Aceite final: cenários ociosos e ativos em 1/100/1.000 máquinas, ambos os caminhos
single-player e teste de persistência; registrar tempo por tick e memória.
Atualizar evidências/status de cada mod apenas com gameplay validado. Os
checkpoints antigos e testes de UI não certificam conclusão nem performance.

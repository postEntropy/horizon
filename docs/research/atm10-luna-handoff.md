# Passagem ATM10 — execução por pacote

## Acordo de trabalho

Sol prepara e revisa checkpoints. Luna pode executar os pacotes **em sequência na
mesma conversa**, um de cada vez. Ao fim de cada pacote, registrar o resultado,
verificar os critérios de aceite e seguir para o próximo; parar se surgir uma
decisão de arquitetura, uma regressão sem causa ou falta de evidência. Não criar
uma meta persistente separada: este roteiro já define o escopo. A execução só
começa quando o usuário mandar o Luna continuar por este arquivo.

O objetivo completo continua sendo executar `atm10-port-fix-plan.md`. Tarefas
1–3 concluídas no escopo documentado; 4 em andamento; 5 pendente. Os números e
limitações estão em `atm10-performance-tracker.md`. Não declarar o plano ou um
mod completo só porque este pacote passou.

## Restrições persistentes

- Preservar todas as alterações locais e commits do usuário; worktree contém
  trabalho amplo anterior. Sem reset, limpeza, commit, PR ou instalação.
- Builds/testes: `taskset -c 0-5 nice -n 10`, `CARGO_BUILD_JOBS=6`,
  `RAYON_NUM_THREADS=6`; não rodar builds concorrentes.
- Não modificar o pack instalado ou dados reais de jogadores. Staging em `.local/`.
- Jogos/oráculos usam homes temporários, áudio e self-update desativados, conforme
  AGENTS.md. Java: `-XX:ActiveProcessorCount=6`.
- Clean room: não ler fontes/classes Java dos mods. Dados, recursos e execução
  original como caixa-preta são permitidos. Não publicar dumps preparados.
- Sem subagentes. Não inventar benchmark ou porcentagem de conclusão.

## Pacote L1 — validar a importação de alimentos com efeitos vanilla

**Resultado esperado:** verificar e, se necessário, corrigir somente a mudança
pendente em `crates/horizon-port-pack/src/oracle.rs`, produzir evidência da
regeneração em staging e encerrar. O último comando foi interrompido; não assumir
que seus testes terminaram. Verificar processos/sessões antes de reiniciar algo.

### Estado relevante

- A declaração de alimentos, seu overlay e o parser do cliente já transportam
  nutrição, saturação total, duração, efeitos ordenados, probabilidades e flags
  `ambient`/`visible`. Existe distinção entre crafting_remainder e use_remainder.
- O motor ainda não representa show_icon de forma independente e não implementa
  `farmersdelight:nourishment`. Classes próprias também podem alterar consumo.
- Último checkpoint validado do importador: 75 testes e Clippy passaram; staging
  com alimentos vanilla sem efeitos declarou 48 itens e carregou 5 receitas de
  panela. Isso não valida o consumo no jogo nem o mod completo.
- A alteração mais recente, ainda sem resultado confirmado, permite efeitos
  vanilla reconhecidos com show_icon=true, valida parâmetros e recusa os demais.
  Conferir a lista com `crates/horizon-client/src/brewing.rs` e conferir a saída
  com FoodEffect em `crates/horizon-mods/src/content.rs`. Não ampliar o suporte a
  subclasses ou efeitos customizados neste pacote.
- Dados originais observados: wheat_dough, raw_pasta e chicken_cuts são vanilla
  Item com Hunger, duração 600, amplificador 0, chance 0.3 e flags
  ambient=false, visible=true, show_icon=true.

### Execução

1. Ler AGENTS.md, CONTRIBUTING.md e `.agents/skills/port-modpack/SKILL.md`.
   Inspecionar o diff atual de oracle.rs e o teste
   `ordinary_food_uses_observed_components_and_refuses_incomplete_data`.
2. Confirmar que todos os parâmetros e a ordem são preservados; dados ausentes,
   efeitos desconhecidos/customizados e show_icon=false devem continuar recusados.
   Fazer apenas correções necessárias nesse arquivo e seus testes.
3. Rodar, com as restrições acima:
   - `cargo test -p horizon-port-pack --lib`
   - `cargo clippy -p horizon-port-pack --all-targets -- -D warnings`
   - `cargo fmt -p horizon-port-pack -- --check`
   - `git diff --check`
4. Regenerar somente o staging existente, que pertence a esta tarefa:
   `cargo run -p horizon-port-pack -- carry .local/atm10-resource-validation/farmer-food-regenerated --resources .local/horizon-client/26.3 farmersdelight`
   Seu inventory.json aponta para o jar original; oracle/facts.json é o dump
   observado. **Não usar o diretório do pack instalado como work.**
5. Executar o carregador real:
   `target/fast/examples/mod_machine_recipe_probe .local/horizon-client/26.3 .local/atm10-resource-validation/farmer-food-regenerated .local/atm10-resource-validation/farmer-food-regenerated-recipes.json farmersdelight`
   Se o executável estiver desatualizado, reconstruir apenas esse example com
   `cargo build --profile fast -p horizon-client --no-default-features --example mod_machine_recipe_probe`.
6. Conferir as três declarações geradas: efeito Hunger com todos os valores acima.
   Registrar contagens reais e motivos de exclusão, sem exigir aumento artificial
   da quantidade de receitas. Não sobrescrever evidências antigas com resultados
   diferentes sem atualizar sua descrição; preferir um novo JSON de checkpoint.
7. Atualizar o acompanhamento central com resultado, checks e limitações.

### Parada obrigatória do pacote

Encerrar após os checks e o relatório. Não iniciar consumo no jogo, Nourishment,
show_icon, outros mods ou tarefa 5. Se uma falha exigir mudança fora do escopo,
registrar diagnóstico concreto e parar para revisão do Sol. Não gastar ciclos
repetindo o mesmo teste sem uma mudança ou hipótese nova.

Entregar: arquivos alterados, checks que passaram/falharam, contagens observadas,
pendências e a confirmação de que parou ao final do L1.

## Estado e ordem de execução

L1 **concluído** em 2026-11-02: 75 testes, Clippy, rustfmt e `git diff --check`
passaram; staging com 51 itens declarados, 43 excluídos (39 com classe própria,
4 itens de bloco sem bloco associado) e 5 receitas carregadas. Detalhes e limites
no tracker. Não repetir L1. As tarefas 1–3 do plano principal já foram concluídas
no escopo documentado. Seguir L2 → L3 → L4 → L5 → L6 → L7 → L8; registrar cada
checkpoint no tracker antes de avançar. Testar só o que o pacote alterou, salvo
gate obrigatório. Se o mesmo teste passou e nada relevante mudou, não repeti-lo.

## L2 — consumo real dos três alimentos vanilla importados

Usar o staging de L1, sem regenerar o oráculo. Montar uma fixture pequena do jogo
para `wheat_dough`, `raw_pasta` e `chicken_cuts`: inventário antes/depois,
nutrição, duração e aplicação probabilística de Hunger (não exigir que uma única
tentativa de 30% produza o efeito). Cobrir in-frame e servidor integrado com homes
temporários; se a fixture existente não puder observar algum campo, registrar a
limitação, sem alegar equivalência. Corrigir apenas defeitos demonstrados na
cadeia de componentes/consumo desses itens. Rodar testes focados, fmt, Clippy dos
alvos alterados e `git diff --check`; guardar evidência no tracker.

## L3 — recipientes e receitas da panela Farmer's Delight

Comparar os recipientes e resultados por receita com os fatos do original via
oráculo caixa-preta. Implementar somente os casos cuja informação e API permitem
transação atômica: ingredientes, reservatório, recipiente e saída devem conservar
quantidades e componentes. Testar sucesso, saída bloqueada, falta de ingredientes
ou recipiente e save/load no componente WASM e nos dois caminhos do jogo. Recusar
receitas com recipientes não representáveis e listar seus ids. Não marcar o mod
como completo enquanto subclasses, Nourishment e efeitos customizados faltarem.

## L4 — alimentos e efeitos restantes Farmer's Delight

Auditar as 39 exclusões por classe própria, `show_icon=false`, Nourishment e
demais efeitos customizados usando dados e execução original, sem ler código Java
do mod. Agrupar por comportamento observado. Implementar um grupo por vez, com
fixture de consumo e persistência; não inferir comportamento de um item pela
classe ou pelo nome. Se a API do Horizon não expressar o efeito, registrar o
contrato mínimo necessário e parar para revisão antes de ampliá-la. Atualizar
contagem de itens/receitas realmente carregados, sem transformar exclusão em
declaração incompleta.

## L5 — concluir os casos já iniciados de Charging Gadgets e Modular Routers

Charging Gadgets: validar itens energizáveis reais e capacidade/FE na estação,
inclusive saída bloqueada, energia insuficiente e reabertura; bateria de teste
não certifica ferramentas do pack. Modular Routers: após Puller/Sender Mk1
adjacentes, tratar módulos de mundo/remotos por comportamento e API disponíveis.
Nunca criar, apagar ou duplicar itens como substituto de uma operação ausente.
Para cada módulo, testar conservação, bloqueio, falta de recurso, trap/reload e
save/load nos dois modos; deixar módulo não representável explicitamente pendente.

## L6 — demais ports com simulação ausente ou desconectada (tarefa 4)

Usar a tabela de `atm10-port-performance-review.md` como fila. Prioridade:
Modern Dynamics; MEGA Cells; Mob Grinding Utils; Productive Metalworks; Pylons;
Ranged Pumps; Actually Additions; Advanced Peripherals; Ars Technica;
BotanyPots; Cable Tiers; Tree Tap; depois os demais casos concretos da tabela.
Escolher **um mod por checkpoint**. Antes de mudar código, descrever entradas,
saídas, recursos e capacidade observados no original. Ligar a lógica ao estado
real do jogo e validar sucesso, saída bloqueada, recursos insuficientes,
remoção/descarregamento e save/load em WASM e jogo quando aplicável. Se faltar API
ou referência, registrar limite preciso e passar ao próximo caso independente;
não declarar `ported` nem medir callback vazio como simulação real.

## L7 — medir e otimizar o loop do jogo (tarefa 5)

Instrumentar separadamente em `crates/horizon-client/src/game/mods.rs` criação e
cópia de views, serialização por entidade, reconciliação de capabilities e tempo
total por tick. Medir 1/100/1.000 máquinas ociosas e ativas nos caminhos in-frame
e servidor integrado, CPU 0–5/nice 10, mesmo build/cenário antes e depois.
Registrar p50, p95, máximo, memória e artefatos brutos. Só após identificar o
custo dominante implementar otimização/dirty tracking, incluindo estado privado
do guest; validar transferências entre mods, trap/reload e save/load. Reportar
regressões mesmo quando o caso de 1.000 melhora.

## L8 — auditoria final do plano

Reexecutar somente gates afetados e cenários finais necessários. Comparar a
worklist e a tabela da revisão com evidência reproduzível por mod; distinguir
`parcial`, `não representável` e `validado`. Atualizar plano, tracker e status
apenas para gameplay comprovado. Registrar pendências de API e regressões com
reprodução. O plano de cinco tarefas só pode ser chamado concluído quando os
critérios das tarefas 4 e 5 estiverem demonstrados; caso contrário entregar a
lista exata do que falta, sem porcentagem inventada.

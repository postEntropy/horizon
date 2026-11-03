# Modular Routers — transferências atômicas e pendências

O port anterior criava cobblestone em puller/vacuum sem remover de uma origem e
consumia o buffer em breaker/dropper sem executar a ação correspondente.
Essa simulação foi retirada. Puller Mk1 e Sender Mk1 agora transferem entre
inventários reais; módulos remotos/de mundo continuam incompletos e não
fabricam/apagam itens para simular uma ação.

A [wiki pública do autor sobre módulos](https://github.com/desht/ModularRouters/wiki/Modules)
descreve um buffer e módulos executados em ordem, com origem/destino e direção
configuráveis. É documentação antiga: serve para esses conceitos, não para
certificar taxas/configurações da versão ATM10. Não foi lido código Java do mod.

## Por que a extensão foi necessária

`machine::get-slot/set-slot` só alcança os handles emprestados ao próprio mod.
`capabilities::get-item-slots` expõe ID e quantidade, sem os componentes do
ItemStack; `set-item-slot` não autoriza reescrever inventários estrangeiros.
Combinar duas gravações independentes não representa uma transferência atômica.
`server::spawn-drop` e `set-block` são ações enfileiradas, enquanto uma remoção
via `machine` já altera a view: não há confirmação transacional para consumir
primeiro e garantir a ação de mundo depois.

## Extensão implementada para inventários adjacentes

`machine::request-item-move` contém handle, slot do buffer, face, sentido e
quantidade. O resultado de enfileirar significa somente pedido aceito.
`take-item-move-result` confirma e consome a quantidade efetivamente movida.
O limite é 64 itens por pedido e 4.096 pedidos não consumidos por mod; uma
recusa não modifica inventários. IDs são transitórios, associados ao handle
vivo e descartados ao remover/recarregar a instância.

1. Runtime valida propriedade do handle, presença no empréstimo atual, slot,
   posição e limites de quantidade/número de pedidos. Não autoriza qualquer
   coordenada estrangeira arbitrária.
2. O jogo processa pedidos em ordem, **depois de devolver todas as views** e
   antes de salvar. Revalida dimensão, chunk carregado, handle e entidade ainda
   existentes; pedidos antigos não alcançam outro bloco após remoção/reload.
3. Uma rotina do jogo transfere o ItemStack real, com todos os componentes,
   usando as mesmas regras de inserção/extração por face dos funis. Verifica
   espaço, compatibilidade, limites por slot e aliases antes de retirar da
   origem. Origem/destino no mesmo inventário compartilhado não podem duplicar
   nem provocar deadlock. Sem espaço/origem válida, nenhuma escrita ocorre.
4. Capabilities dos inventários mod alterados são atualizadas após a operação.
   A confirmação informa a quantidade efetivamente movida. Terminação de
   módulos, estatísticas e eventuais custos dependem dessa quantidade, nunca
   de um pedido aceito. Custos exigem saldo do host na mesma transação; energia
   privada do guest não deve ser descontada antecipadamente.
5. Breaker, vacuum e dropper precisam de transações próprias que revalidem o
   bloco/entidade/drop e preservem componentes/loot. Não substituir por
   decrementos do buffer nem geração de cobblestone.

O jogo aplica a operação em `mod_move_item_adjacent`, usando as regras em
`crates/horizon-client/src/game/machines.rs`: views de containers/double chests,
permissões mod por face, `take_one`, `put_back`, `add_item`, marcação de chunks e
comparadores. A transferência preserva o ItemStack completo; snapshots de
capabilities de ID/quantidade não o substituem. Composters, que têm efeitos de
mundo além de slots, são recusados por esta operação de inventário.

O módulo guarda a direção em `minecraft:custom_data` sob
`modularrouters:direction`, aceitando seis faces ou, quando ausente, a orientação
do router. O getter tipado lê um caminho limitado de compostos NBT; não depende
de registrar um tipo novo no parser vanilla de comandos. Os IDs de widgets de
slot foram alinhados aos slots reais (buffer 0, módulos 1–9, upgrades 10–14).

## Evidência e aceite restante

O [resultado nos dois modos](atm10-resource-validation/modularrouters-game.json)
cobre WASM real no jogo: origem vazia, destino cheio/incompatível,
extração proibida, dois routers disputando o último item, canais compartilhados,
ItemStack com nome/carga/encantamento, remoção do destino antes da aplicação,
save/reabertura e ausência de duplicação/perda. O sandbox verifica confirmação
única, remoção, trap e IDs reutilizados em hot-reload. Os
[hashes e limites da validação](atm10-resource-validation/modularrouters-environment.json)
estão registrados. Ainda falta medir 1/100/1.000 routers realmente transferindo.

Breaker, vacuum e dropper precisam de transações reais sobre blocos/entidades e
loot. Módulos remotos, filtros, augments, controle de redstone, interface de
configuração e custos configuráveis ainda estão pendentes. Filtros/terminação
precisam observar o resultado de módulos anteriores na mesma rodada: a simples
aceitação de pedidos assíncronos não resolve essa semântica. A tarefa 4 e o
port completo não foram declarados concluídos por esta implementação parcial.

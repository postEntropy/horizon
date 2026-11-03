# Panela: reservatório, serviço e acesso

Estado da tarefa 4, implementação parcial. Evidências: [oráculo original](atm10-resource-validation/farmersdelight-original-serving.json),
[componente WASM](atm10-resource-validation/farmersdelight-component.json) e
[receitas carregadas](atm10-resource-validation/farmersdelight-recipe-graph.json).
Nenhum fonte Java do mod original foi lido.

O oráculo usa o mod original em NeoForge isolado, APIs públicas de mundo e o menu
criado pelo próprio bloco/entidade. Para testar serviço em lote, primeiro cozinha
uma receita de verdade e depois altera as quantidades por slots do menu. Preencher
só o reservatório não inicializa os metadados de recipiente mantidos pelo original.
O driver anterior continua usando `Container` vanilla quando disponível.

## Inventário e transações

| Slots | Função | Acesso do jogador | Automação configurada no port |
| --- | --- | --- | --- |
| 0–5 | Ingredientes | Inserção/extração | Entrada por cima |
| 6 | Reservatório, capacidade 64 | Bloqueado | Bloqueado em todas as faces |
| 7 | Recipientes | Inserção/extração | Entrada pelas laterais |
| 8 | Comida servida | Só extração | Extração por baixo |

A política de faces ainda precisa ser comparada com todas as faces do mod original.
O serviço observado independe de calor e de ingredientes. O oráculo confirmou a
capacidade 64 do reservatório e, para Cooked Rice, saída limitada a 16 por pilha.
Uma saída com 15 recebe só uma porção, consumindo só uma tigela.

Cozinhar usa `machine::process-items` para debitar ingredientes e acrescentar o
resultado ao slot 6 numa única transação. A simulação verifica o reservatório antes
de avançar o processo. Saída servida cheia não impede cozinhar se ainda couberem
refeições no reservatório. Recipientes não são debitados nessa etapa.

Servir usa outra transação: debita o reservatório e os recipientes e publica a
comida no slot 8. A quantidade é limitada pelo espaço, pela refeição armazenada e
pelos recipientes disponíveis. Falha não publica alteração parcial. Componentes
customizados na saída são recusados pela transação em vez de sobrescritos.

Timer continua no estado privado de 8 bytes; os slots persistem no inventário real
do host. Não se mantém uma segunda cópia das refeições no estado do guest.

## Política do menu

`machine::set-player-slot-access` configura gates de inserção/extração para slots
vivos pertencentes ao guest. Não altera o acesso do próprio processador, nem as
regras por face de funis. Slots sem configuração continuam acessíveis. O getter do
host recusa handles/slots inválidos e fecha acesso quando o proprietário parou.
Remoção descarta a política; criação/carregamento do guest a reaplica.

O motor do menu aplica gates adicionais às regras vanilla. Pickup, quick move,
swap, throw, pickup all e drag os respeitam; clone criativo também respeita a
proibição explícita de extração. Um gate permissivo não libera saídas vanilla para
inserção. O caminho de menus do servidor LAN/integrado também aplica a política
antes de processar pacotes, sem depender da previsão do cliente.

Ao preencher a última linha de menus de mods que não têm múltiplos de nove slots,
o mapeamento usa a quantidade de slots da linha completa como início do inventário
do jogador. A parte vazia não representa inventário e fica bloqueada.

## Pendências

- A matriz caixa-preta de 28 receitas confirmou os recipientes/resultados da
  tabela atual, sem conflito de recipiente por item de saída. Sete relações de
  ingredientes com crafting remainder foram testadas; o original consome esses
  ingredientes sem devolvê-los. Evidência: [validação L3](atm10-resource-validation/farmersdelight-cooking-l3-validation.json).
- Templates de alimentos: os carries atuais ainda usam itens simples em vários casos.
- Tags comuns ausentes/incompletas no carry isolado: com tags observadas, o cliente
  carrega 28/28 receitas; com apenas o carry instalado do Farmer, carrega 14/28.
- Os dois modos passaram no cenário de Cooked Rice com persistência, funis e
  cliques: [relatório conjunto](atm10-resource-validation/farmersdelight-game.json).
  O teste de gameplay cobre Cooked Rice, não todas as 28 receitas. Comparar todas
  as faces de automação e o restante dos recursos antes de certificar o mod.
- Medir performance; tempo de inicialização do oráculo não é benchmark de tick.

# ATM10: inventários compartilhados (tarefa 2)

## Limite da API anterior

`machine.get-slot` entrega o estado de uma cópia por bloco no começo do tick.
Guardar a cópia publicada anteriormente distingue uma extração de uma cópia
antiga, mas não serializa as operações: se um canal tem um diamante e dois
funis extraem de duas cópias antes do callback, os dois já receberam o diamante.
Somar deltas depois não desfaz essa duplicação. Substituições simultâneas por
itens/componentes diferentes também não podem ser resolvidas sem perda usando
somente snapshots. Não considerar esse algoritmo uma correção completa.

## Extensão pequena proposta

`machine.bind-shared-inventory(handle, channel) -> bool` vincula os slots de um
bloco próprio a um canal opaco, com escopo automático pelo ID do mod e pelo
mundo aberto. O host rejeita handles alheios e canais inválidos ou com tamanho
incompatível. O mod chama no create e ao alterar frequência/proprietário. Não
há cópia de inventário dentro do WASM nem sincronização periódica por 27 slots.

O jogo resolve o vínculo para uma instância compartilhada do inventário real.
Menus, comandos e funis operam nessa mesma instância; portanto uma extração
fica visível antes da operação seguinte, inclusive por outro endpoint. Os
componentes opacos fazem parte da pilha real e não são descartados na ligação.
Trocar de canal troca a referência, sem depositar o inventário do canal antigo
no novo. Remover/descarregar um endpoint não remove o canal nem derruba seus
itens no mundo. Inventários normais continuam independentes.

## Persistência e dimensões

O host mantém uma tabela canônica de canais no estado global de mods do jogo,
fora dos níveis trocados pelo servidor integrado. Chaves de posição dos endpoints
incluem dimensão; a chave de um canal dimensional deliberadamente não inclui
ela. EnderStorage inclui as três cores e o proprietário real quando privado.
Nunca usar o texto fixo `player` como proprietário.

Salvar a tabela canônica uma vez em `world/data/horizon/`, com escrita atômica,
na mesma rotina de persistência dos attachments. Blocos salvam apenas identidade
do canal/frequência/proprietário; não salvam cópias concorrentes de seus itens.
Carregar a tabela antes de ligar endpoints. Canais sem endpoints continuam no
arquivo. Reload do WASM preserva a tabela e recria somente os vínculos/handles.

## Critério de conclusão

A extensão só estará entregue após integração no runtime e no jogo, fixtures
WASM dos dois ports e validação de funis/save nos modos in-frame e servidor
integrado. A primitiva de armazenamento compartilhado isolada não prova esses
caminhos. As demais tarefas do plano permanecem no escopo do objetivo ativo.

## Referências públicas de comportamento

- [DimStorage, página do publicador](https://www.curseforge.com/minecraft/mc-mods/dimstorage): armazenamento com proprietário, frequência e trava.
- [Ender Storage, página do projeto](https://www.curseforge.com/minecraft/mc-mods/ender-storage-1-8): a combinação de cores identifica inventários compartilhados entre dimensões; usar diamante na trava seleciona a rede do jogador que o usou.

Nenhum fonte Java de mod foi lido. A implementação acima ainda precisa da
validação integrada registrada no plano; não é uma declaração de completude dos
mods, de seus tanques ou tablets/pouches.

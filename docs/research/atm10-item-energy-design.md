# Energia de itens — correção do Charging Gadgets

O [comportamento público do Charging Gadgets](https://www.curseforge.com/minecraft/mc-mods/charging-gadgets)
é queimar combustível para carregar um item que suporta FE. A
[documentação de capabilities do NeoForge](https://docs.neoforged.net/docs/inventories/capabilities/)
explica a consulta de suporte sem conhecer a representação privada do item.
Nenhum fonte Java de mod foi consultado.

A API atual expõe energia por posição de bloco e componentes opacos de itens,
mas não oferece um contrato comum para carregar um item de outro mod. Escrever
um contador privado do carregador não resolve isso. A extensão pequena proposta
fica em `item-data`, mantendo o inventário e seus componentes no host:

- O dono do namespace registra capacidade e limites de recebimento/extração de
  seus próprios itens. O registro não concede energia inicial. Registros são
  limitados e reconstruídos no reload; não podem redefinir itens de outros mods.
- Consulta de energia de um slot retorna `none` para itens não registrados,
  stacks com quantidade diferente de um e componentes inválidos.
- Recebimento/extração retornam a quantidade efetivamente transferida. Simulação
  não modifica nada. São respeitados capacidade, saldo, limite por chamada e
  limite de tamanho do componente; falha retorna zero sem alterar o item.
- O saldo é um componente Long `horizon:energy` do próprio ItemStack. Outros
  componentes são preservados. Movimentação por menus/funis, drops e saves usa
  o mesmo ItemStack; não existe um segundo saldo no carregador.
- Charging Gadgets desconta somente o valor confirmado pelo host. Item cheio,
  comum ou ausente não drena FE. Um consumidor usa a mesma extração tipada.

Não se inferem capacidades por nome de item. Ports de ferramentas/baterias
precisam registrar suas capacidades e usar a extração real; esta extensão não
certifica os ports Mining Gadgets, Building Gadgets ou Iron Jetpacks. A fixture
deve provar conservação entre estação e item, saturação, recursos insuficientes,
componentes preservados e save/load antes de declarar corrigido o carregamento.

# Roadmap: menus do Mekanism no Horizon

## Objetivo

Fazer com que os menus das máquinas de Mekanism e Mekanism: Generators no Horizon
sejam reconhecíveis e visualmente próximos aos do ATM10, mantendo slots, gauges,
botões e valores ligados ao estado real da máquina.

O trabalho cobre os menus desses dois ports. Não inclui portar a interface inteira
de outros mods nem reproduzir telas que ainda não têm comportamento funcional.

## Estado atual

Os ports já abrem telas declarativas e atualizam alguns slots e gauges. O SDK
agora aceita um widget de fundo e o cliente carrega imagens
`assets/<namespace>/gui/` no atlas. As telas de processamento e factories do
Mekanism usam `mekanism/gui/base`, presente no carry. Os layouts ainda precisam
de revisão de posições e comparação visual. Abas, tooltips ricos e controles de
configuração característicos do Mekanism continuam fora do contrato de UI.

## Etapas

### 1. Inventário visual e referência

- Agrupar as telas existentes por família: máquinas de processamento, geradores,
  armazenamento, tanques, logística e multiblocos.
- Montar uma lista de quais máquinas têm tela própria e quais compartilham o mesmo
  tipo de layout.
- Registrar capturas de referência do Mekanism original no ATM10 e capturas atuais
  do Horizon, com resolução e escala iguais.
- Conferir apenas o comportamento e os recursos públicos do mod; não usar código
  fonte Java do mod como referência de implementação.

**Pronto quando:** existe uma matriz de telas com nome da máquina, família,
dimensões, slots, indicadores, controles e estado atual no Horizon.

### 2. Fechar o suporte visual do SDK e do cliente

- Permitir que uma tela escolha um fundo carregado dos assets trazidos pelo carry,
  com fallback previsível quando o recurso não existir.
- Definir âncoras e escala para posicionar widgets sobre o fundo, em vez de depender
  só de coordenadas fixas.
- Melhorar os widgets de barra, fluido e progresso para exibir preenchimento,
  orientação, estado vazio/cheio e tooltip com valor/capacidade.
- Dar suporte a ícones de item e tooltips nos slots e a texto com alinhamento e
  cor configuráveis.
- Adicionar controles interativos necessários às telas do Mekanism, como abas,
  botões e alternâncias; encaminhar ações ao servidor/mod com estado sincronizado.
- Preservar navegação, escala da interface, foco, fechar tela e regras de clique
  do inventário vanilla.

**Pronto quando:** uma tela de teste consegue combinar fundo, slots, gauges,
ícones, tooltips e controles sem código especial no renderer para cada máquina.

**Progresso:** fundos declarativos em `assets/<namespace>/gui/` já são carregados
e desenhados. Falta validar visualmente escalas e o fallback quando o fundo
escolhido não existe.

### 3. Acertar a família de processamento

- Começar por Enrichment Chamber e Energized Smelter, que cobrem o fluxo comum
  de entrada, energia, progresso e saída.
- Aplicar o layout aos outros processadores simples e depois às variantes de
  factories, respeitando número e posição dos slots.
- Tratar máquinas com segunda entrada, combustível ou item retornável em layouts
  próprios, em vez de forçá-las no modelo de uma entrada.
- Usar nomes localizados, ícones e tooltips adequados; remover títulos e rótulos
  genéricos quando o fundo já comunicar a função.

**Pronto quando:** o ciclo visual de entrada → progresso → saída bate com a
referência, e cada slot continua aceitando e devolvendo os itens corretos.

**Progresso:** primeiro fundo compartilhado aplicado a processadores e factories;
posições e comparação com o original ainda não foram revisadas.

### 4. Acertar armazenamento, energia e fluidos

- Refazer Energy Cubes, tanques, tubos e cubos de armazenamento com seus fundos,
  indicadores, capacidade e tooltips próprios.
- Mostrar unidades e valores de forma legível, inclusive números grandes e estados
  vazios.
- Implementar configurações expostas pelo menu, como modo de saída ou interação
  de faces, somente quando a ação correspondente já existir no port.

**Pronto quando:** os valores visuais acompanham o estado salvo e sincronizado da
máquina e as configurações visíveis realmente alteram seu comportamento.

### 5. Acertar Mekanism: Generators e multiblocos

- Aplicar o mesmo sistema visual a geradores simples, com combustível, energia,
  produção e progresso relevantes para cada tipo.
- Tratar telas de fissão, fusão e turbina como layouts específicos, incluindo
  estado de formação e indicadores de fluidos/energia pertinentes.
- Diferenciar máquinas que não têm menu equivalente no mod original: não inventar
  controles; mostrar apenas informação útil que o port suporta.

**Pronto quando:** estados formados/não formados, níveis e produção refletem o
multibloco e os menus podem ser usados sem valores fictícios.

### 6. Revisão de cobertura e acabamento

- Percorrer a matriz da etapa 1 e marcar cada menu como implementado, parcial ou
  sem suporte funcional no port.
- Comparar capturas lado a lado e corrigir espaçamento, escala, cores, alinhamento,
  estados hover/pressionado e tooltips.
- Verificar resolução de janela e escala de GUI diferentes.
- Confirmar que abrir/fechar, clicar slots, mover itens e atualizar o estado da
  máquina continuam corretos nos caminhos local e cliente-servidor.

**Pronto quando:** todas as telas cobertas têm uma captura revisada, os controles
visíveis funcionam, e limitações restantes estão listadas por máquina.

## Ordem sugerida

1. Inventário visual e capturas de referência.
2. Suporte de fundos e widgets no SDK/cliente.
3. Enrichment Chamber e Energized Smelter como telas piloto.
4. Demais processadores e factories.
5. Energia, tanques e fluidos.
6. Geradores simples.
7. Fissão, fusão, turbina e acabamento geral.

## Critérios gerais de aceite

- O fundo, posição e escala dos elementos correspondem à tela equivalente do
  Mekanism no ATM10, dentro das limitações do renderer do Horizon.
- Cada barra, slot e rótulo representa dados reais da máquina; nada fica preso em
  valores ilustrativos.
- Slots aceitam, movem e devolvem itens sem duplicação ou perda.
- Cada botão, aba ou alternância visível executa uma ação real, ou não aparece.
- Telas atualizam em multiplayer/LAN e após salvar e reabrir o mundo.
- Os ports continuam usando os assets carregados do pack e não dependem de arquivos
  do jogo ou do mod adicionados ao repositório.

## Fora do escopo desta rodada

- Paridade visual de menus de mods que não sejam Mekanism e Mekanism: Generators.
- Telas sem implementação funcional da máquina correspondente.
- Reprodução de código ou implementação interna do mod original; a referência é
  o comportamento observável e os assets públicos carregados pelo pack.

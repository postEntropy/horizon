# ATM10: índice de views — tarefa 1

Implementado e medido em 2026-11-01. Base: `ac0a35ede47c92ec440dd13554cb3ad0061995d1`.
Os arquivos `before-*.json` e `after-*.json` são as saídas integrais do probe;
`environment.json` registra compilador, CPU e hashes dos mesmos cores WASM usados
nas duas medições. Não houve deploy ou alteração do pack instalado.

## Método

`port_tick_bench` em release, 27 slots vazios por baú, sem `ItemFacts`,
10 ticks de aquecimento e 200 amostras por processo. Inclui runtime host + WASM,
sem jogo, cópia/serialização do loop do cliente ou renderização. Cada combinação
foi executada uma vez, sequencialmente, na mesma sessão do desktop; nenhum build
ou teste estava concorrendo com as medições. Sem afinidade de CPU ou frequência
fixa; aplicativos de fundo permaneceram abertos. p95 é o percentil discreto
calculado pelo probe, não um intervalo de confiança.

Antes de medir, os dois cores foram reconstruídos com o SDK atual:

```sh
cargo build --manifest-path ports/Cargo.toml --target wasm32-unknown-unknown --release -p dimstorage-port -p enderstorage-port
cargo build --release -p horizon-mods --features runtime,pack --example port_tick_bench
```

Executar para cada `COUNT` em `1 100 1000`, trocando `PHASE` por `before`/`after`:

```sh
target/release/examples/port_tick_bench   "$HOME/.local/share/horizon/ports/all-the-mods-10-atm10/mods/dimstorage/carry"   ports/target/wasm32-unknown-unknown/release/dimstorage_port.wasm   dimstorage:dimensional_chest COUNT 200   docs/research/atm10-view-index-bench/PHASE-dimstorage-COUNT.json
```

Para EnderStorage, substituir `dimstorage` por `enderstorage` e o bloco por
`enderstorage:ender_chest`. O probe lê o manifesto instalado, mas empacota uma
cópia em diretório temporário e remove-a ao sair.

## Resultados

Tempos em microssegundos por tick; razão = p50 antes / depois.

| Mod | Baús | p50 antes | p50 depois | p95 antes | p95 depois | Razão |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| dimstorage | 1 | 2.425 | 3.016 | 2.485 | 3.066 | 0.80× |
| dimstorage | 100 | 266.977 | 261.366 | 435.005 | 275.583 | 1.02× |
| dimstorage | 1000 | 9579.418 | 2795.121 | 11664.581 | 3240.035 | 3.43× |
| enderstorage | 1 | 2.464 | 3.035 | 2.594 | 3.968 | 0.81× |
| enderstorage | 100 | 252.478 | 268.990 | 357.168 | 361.826 | 0.94× |
| enderstorage | 1000 | 9648.448 | 2814.477 | 10800.242 | 3144.794 | 3.43× |

Com 1.000 baús, a redução de p50 é de aproximadamente 71% nos dois mods.
De 100 para 1.000 baús, p50 agora cresce 10,69× em DimStorage e 10,46× em
EnderStorage, contra 35,88× e 38,22× antes. Nesse intervalo, o probe deixou de
mostrar o crescimento muito acima do linear causado pela busca de views.
A consulta por handle usa `HashMap` (O(1) esperado), construído em O(N) uma vez
por callback. Os mapas preservam capacidade após a devolução das views.

Para um único baú, p50 aumentou cerca de 0,6 µs; não há ganho universal em cargas
pequenas. São medições diagnósticas de uma execução por combinação, sujeitas a
ruído do desktop. Os ~2,8 ms restantes com 1.000 baús ainda incluem as 27 leituras
por baú no WASM e a reconciliação de capabilities do host. Isto não certifica
performance global do pack nem do loop do jogo.

Em todos os JSONs, `slot_after_extraction` continua contendo um diamante:
a duplicação/reposição não foi corrigida nesta tarefa e continua na tarefa 2.
Nenhum mod recebeu status de concluído.

## Validação e escopo

- `cargo test -p horizon-mods --features runtime,pack`: 47 passaram, 1 ignorado
  (`hook_overhead`, já ignorado anteriormente); doc-tests sem casos.
- `cargo clippy -p horizon-mods --features runtime,pack --all-targets -- -D warnings`: passou.
- `cargo clippy -p horizon-mods-test-machine --target wasm32-unknown-unknown -- -D warnings`: passou.
- Rustfmt dos três arquivos Rust alterados e `git diff --check`: passaram.

O teste de subconjuntos alterna handles fora da ordem de criação, insere um
segundo mod entre os handles e confirma leitura/escrita na máquina correta.
A fixture tenta ler/escrever handles ausentes e de outro proprietário; save
confirma que slots não continuam emprestados fora do tick. Os testes cobrem
retorno de múltiplas views após panic e fuel esgotado, preservando a mutação
anterior à falha e a view ainda não processada. A suíte também cobre reload e
transferência/reconciliação de capabilities entre mods.

Auditoria do ciclo de vida: retorno normal e falhas usam `take_views`, que limpa
o índice antes de retirar o vetor; falhas movem o vetor para `Mods::stranded`.
Reload descarta o `State` anterior e cria outro com índice vazio. A ordem das
views e o primeiro resultado para um handle duplicado permanecem preservados;
não há ponteiros ou referências armazenadas no índice. Buscas espaciais não
foram alteradas.

A suíte inicialmente revelou `docs/api/mods-reference.md` desatualizado em
relação à função `smithing-result` já existente na WIT da base. O documento foi
regenerado para liberar esse check; WIT, API pública e ports não foram alterados.

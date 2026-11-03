#!/usr/bin/env bash
set -e

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
ATM10_DIR="$HOME/.local/share/horizon/ports/all-the-mods-10-atm10"
ATM10_HOME="$ATM10_DIR/home"
RESOURCES="$REPO_ROOT/.local/horizon-client/26.3"

# Garante variáveis de display do ambiente Linux
export DISPLAY="${DISPLAY:-:0}"
if [ -n "$WAYLAND_DISPLAY" ]; then
    export WAYLAND_DISPLAY="$WAYLAND_DISPLAY"
fi

# Diretório de dados do jogo
export HORIZON_HOME="$ATM10_HOME"
export HORIZON_LAUNCHER_NO_SELF_UPDATE=1

# Compilar horizon-client rápido com suporte a mods caso necessário
if [ ! -f "$REPO_ROOT/target/fast/horizon-client" ]; then
    echo "Compilando horizon-client (profile fast) com suporte a mods..."
    cargo build --profile fast -p horizon-client --features mods
fi

# Coletar automaticamente todos os mods portados do ATM 10
DEV_MOD_ARGS=()
for mod_dir in "$ATM10_DIR"/mods/*/carry; do
    if [ -f "$mod_dir/horizon-mod.toml" ]; then
        DEV_MOD_ARGS+=("--dev-mod" "$mod_dir")
    fi
done

echo "🚀 Iniciando All the Mods 10 no Horizon..."
echo "📦 Total de mods carregados: $(( ${#DEV_MOD_ARGS[@]} / 2 )) mods nativos"
echo "🏠 Dados salvos em: $ATM10_HOME"

exec "$REPO_ROOT/target/fast/horizon-client" \
    --resources "$RESOURCES" \
    --world "ATM10" \
    --creative \
    --render-distance 12 \
    "${DEV_MOD_ARGS[@]}" \
    "$@"

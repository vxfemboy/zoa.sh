#!/usr/bin/env bash
# Deploy zoa.sh to the `zoa` Incus container (Void Linux, runit).
#
# The container has no Rust toolchain, so we build the release binary + WASM
# here on the host, push the binary and all runtime assets in, install/refresh
# a runit service, and restart it. The app is reached via the `caddy` container,
# which should reverse-proxy to  http://<zoa-container-ip>:<port>  (config.toml
# → [server] host/port). Caddy config is managed separately.
#
# Usage:   ./deploy.sh              # build + deploy
#          ./deploy.sh --no-build   # deploy the already-built binary/assets
#
# Override any of these via env, e.g.  ZOA_APP_DIR=/opt/zoa ./deploy.sh
set -euo pipefail
cd "$(dirname "$0")"

CONTAINER="${ZOA_CONTAINER:-zoa}"
APP_DIR="${ZOA_APP_DIR:-/srv/zoa}"
SERVICE="${ZOA_SERVICE:-zoa}"
RUN_USER="${ZOA_RUN_USER:-zoa}"   # dedicated unprivileged user inside the container
BIN="zoa-sh"

log()  { printf '\033[1;35m::\033[0m %s\n' "$*"; }
die()  { printf '\033[1;31m!!\033[0m %s\n' "$*" >&2; exit 1; }

# ── 0. sanity ───────────────────────────────────────────────────────────────
command -v incus >/dev/null || die "incus not found on this host"
incus info "$CONTAINER" >/dev/null 2>&1 || die "container '$CONTAINER' not found"
incus start "$CONTAINER" >/dev/null 2>&1 || true

# ── 1. build release binary + WASM on the host ──────────────────────────────
if [ "${1:-}" != "--no-build" ]; then
    log "building release (server + wasm)…"
    cargo build --release            # build.rs also builds the WASM into static/wasm/
fi
[ -x "target/release/$BIN" ]        || die "missing target/release/$BIN — build failed?"
[ -f static/wasm/zoa_sh_bg.wasm ]   || die "missing static/wasm — run ./build-wasm.sh"

# ── 2. prepare the container (dirs, user, service skeleton) ─────────────────
log "preparing $CONTAINER (dirs, user '$RUN_USER', runit service)…"
incus exec "$CONTAINER" -- sh -euc "
    mkdir -p '$APP_DIR' '/etc/sv/$SERVICE/log' '/var/log/$SERVICE'
    if [ '$RUN_USER' != root ] && ! id '$RUN_USER' >/dev/null 2>&1; then
        useradd -r -d '$APP_DIR' -s /usr/bin/nologin '$RUN_USER' 2>/dev/null \
          || useradd -r -d '$APP_DIR' -s /sbin/nologin '$RUN_USER'
    fi
"

# ── 3. push binary + runtime assets ─────────────────────────────────────────
# Binary is swapped via a temp name so a running server keeps its old inode
# until the restart below.
log "pushing binary…"
incus file push "target/release/$BIN" "$CONTAINER$APP_DIR/$BIN.new"
incus exec "$CONTAINER" -- sh -c "chmod +x '$APP_DIR/$BIN.new' && mv '$APP_DIR/$BIN.new' '$APP_DIR/$BIN'"

# Runtime assets the server reads relative to its CWD.
for path in static templates posts config.toml; do
    [ -e "$path" ] || continue
    log "pushing $path …"
    incus file push --recursive --create-dirs --quiet "$path" "$CONTAINER$APP_DIR/"
done

# ── 4. install / refresh the runit service ──────────────────────────────────
log "installing runit service '$SERVICE'…"
run_script="$(mktemp)"; log_script="$(mktemp)"
trap 'rm -f "$run_script" "$log_script"' EXIT

if [ "$RUN_USER" = root ]; then RUN_PREFIX=""; else RUN_PREFIX="chpst -u $RUN_USER "; fi
cat > "$run_script" <<EOF
#!/bin/sh
exec 2>&1
cd $APP_DIR
export SKIP_WASM=1
exec ${RUN_PREFIX}./$BIN
EOF
cat > "$log_script" <<EOF
#!/bin/sh
exec svlogd -tt /var/log/$SERVICE
EOF

incus file push "$run_script" "$CONTAINER/etc/sv/$SERVICE/run"
incus file push "$log_script" "$CONTAINER/etc/sv/$SERVICE/log/run"
incus exec "$CONTAINER" -- sh -euc "
    chmod +x '/etc/sv/$SERVICE/run' '/etc/sv/$SERVICE/log/run'
    chown -R '$RUN_USER' '$APP_DIR' 2>/dev/null || true
    ln -sfn '/etc/sv/$SERVICE' '/var/service/$SERVICE'
"

# ── 5. restart + health check ───────────────────────────────────────────────
log "restarting service…"
# runsvdir needs a moment to notice a freshly-linked service the first time.
incus exec "$CONTAINER" -- sh -c "sv start '$SERVICE' >/dev/null 2>&1 || true; sleep 2; sv restart '$SERVICE' || sv up '$SERVICE'"

port="$(grep -E '^[[:space:]]*port[[:space:]]*=' config.toml | head -1 | grep -oE '[0-9]+' || echo 8084)"
log "checking http://localhost:$port inside the container…"
health="incus exec $CONTAINER -- sh -c '
    if command -v curl >/dev/null; then curl -fsS -o /dev/null \"http://localhost:$port/\"
    elif command -v wget >/dev/null; then wget -qO /dev/null \"http://localhost:$port/\"
    else exit 3; fi'"
if eval "$health"; then
    printf '\033[1;32m:: up ✓\033[0m  %s:%s  (service '\''%s'\'', port %s)\n' "$CONTAINER" "$APP_DIR" "$SERVICE" "$port"
elif [ $? -eq 3 ]; then
    printf '\033[1;33m?? no curl/wget in container — skipping HTTP check. Service status:\033[0m\n'
    incus exec "$CONTAINER" -- sv status "$SERVICE" || true
else
    printf '\033[1;31m!! health check FAILED. Recent logs:\033[0m\n'
    incus exec "$CONTAINER" -- sh -c "tail -n 20 /var/log/$SERVICE/current 2>/dev/null; sv status $SERVICE" || true
fi

echo
echo "Caddy should proxy zoa.sh → http://$(incus list "$CONTAINER" -c4 --format csv | grep -oE '10\.[0-9.]+' | head -1):$port"
echo "Logs:  incus exec $CONTAINER -- tail -f /var/log/$SERVICE/current"

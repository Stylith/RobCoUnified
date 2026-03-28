#!/bin/zsh
set -euo pipefail

BINARY_PATH="${1:-/Users/hal-9000/nucleon-core/target/release/nucleon-native}"
PROFILE_DIR="$(mktemp -d /tmp/nucleon-profile.XXXXXX)"
LOG_FILE="$PROFILE_DIR/startup.log"
REPAINT_LOG="$PROFILE_DIR/repaint.log"
APP_LOG="/tmp/nucleon-profile-run.log"

mkdir -p "$PROFILE_DIR/users"
mkdir -p "$PROFILE_DIR/users/profile"
printf '{"profile":{"password_hash":"","is_admin":true,"auth_method":"no_password"}}\n' \
  > "$PROFILE_DIR/users/users.json"
cat <<'JSON' > "$PROFILE_DIR/users/profile/settings.json"
{
  "sound": true,
  "bootup": false,
  "theme": "Green (Default)",
  "default_open_mode": "desktop"
}
JSON

START_MS="$(python3 -c 'import time; print(int(time.time() * 1000))')"

NUCLEON_BASE_DIR="$PROFILE_DIR" \
NUCLEON_AUTOLOGIN_USER=profile \
NUCLEON_STARTUP_PROFILE_LOG="$LOG_FILE" \
NUCLEON_REPAINT_TRACE_LOG="$REPAINT_LOG" \
"$BINARY_PATH" >"$APP_LOG" 2>&1 &
PID=$!

READY_MS=""
for _ in {1..200}; do
  if [[ -f "$LOG_FILE" ]] && grep -q 'desktop_ready' "$LOG_FILE"; then
    READY_MS="$(awk '/desktop_ready/{print $1; exit}' "$LOG_FILE")"
    break
  fi
  sleep 0.1
done

sleep 10

AVG_CPU=""
if [[ "$(uname -s)" == "Darwin" ]]; then
  AVG_CPU="$(
    top -l 6 -pid "$PID" -stats pid,cpu,command 2>/dev/null \
      | awk -v pid="$PID" '$1 == pid {print $2}' \
      | tail -n 5 \
      | awk '{sum+=$1} END { if (NR > 0) printf "%.2f", sum / NR; else print "" }'
  )"
fi

if [[ -z "$AVG_CPU" ]]; then
  cpu_samples=()
  for _ in {1..10}; do
    cpu_samples+=("$(ps -o %cpu= -p "$PID" | tr -d ' ')")
    sleep 1
  done
  AVG_CPU="$(
    printf '%s\n' "${cpu_samples[@]}" \
      | awk '{sum+=$1} END { if (NR > 0) printf "%.2f", sum / NR; else print "0.00" }'
  )"
fi
RSS_KB="$(ps -o rss= -p "$PID" | tr -d ' ')"
RSS_MB="$(python3 -c 'import sys; print(f"{int(sys.argv[1]) / 1024:.1f}")' "$RSS_KB")"

if [[ -n "$READY_MS" ]]; then
  STARTUP_MS="$((READY_MS - START_MS))"
else
  STARTUP_MS="unreached"
fi

kill "$PID" >/dev/null 2>&1 || true
wait "$PID" 2>/dev/null || true

printf 'PROFILE_DIR=%s\n' "$PROFILE_DIR"
printf 'STARTUP_MS=%s\n' "$STARTUP_MS"
printf 'AVG_IDLE_CPU=%s\n' "$AVG_CPU"
printf 'RSS_MB=%s\n' "$RSS_MB"
printf 'STARTUP_LOG_BEGIN\n'
if [[ -f "$LOG_FILE" ]]; then
  cat "$LOG_FILE"
else
  echo "MISSING_LOG"
fi
printf 'STARTUP_LOG_END\n'
printf 'REPAINT_LOG_BEGIN\n'
if [[ -f "$REPAINT_LOG" ]]; then
  tail -n 40 "$REPAINT_LOG"
else
  echo "MISSING_REPAINT_LOG"
fi
printf 'REPAINT_LOG_END\n'
printf 'APP_LOG_BEGIN\n'
if [[ -f "$APP_LOG" ]]; then
  tail -n 40 "$APP_LOG"
else
  echo "MISSING_APP_LOG"
fi
printf 'APP_LOG_END\n'

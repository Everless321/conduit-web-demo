#!/usr/bin/env bash
# Launch privacy-filter + conduit-web-demo on a shared `container` network and
# wire the demo to privacy-filter by its CURRENT IP.
#
# NOTE on name-based DNS: Apple `container`'s `system dns create` domain is for
# HOST -> container access (a bare name resolves to a localhost-redirect IP), NOT
# for container -> container. Since the demo reaches privacy-filter container-to-
# container, we wire by network IP. IPs change across restarts, so this script
# re-reads the privacy-filter IP every run. Safe to re-run any time.
set -euo pipefail

NET=conduit-net
HERE="$(cd "$(dirname "$0")" && pwd)"
DATA="$HERE/data"
ADMIN_PW="${CONDUIT_ADMIN_PASSWORD:-demo-admin-pw}"
HOST_PORT="${HOST_PORT:-18088}"
FAIL_OPEN="${CONDUIT_PRIVACY_FAIL_OPEN:-false}"

container network inspect "$NET" >/dev/null 2>&1 || container network create "$NET"

echo "==> (re)starting privacy-filter"
container rm -f privacy-filter >/dev/null 2>&1 || true
container run -d --name privacy-filter --network "$NET" privacy-filter:dev >/dev/null

PFIP=""
for _ in $(seq 1 20); do
  PFIP="$(container ls | awk '$1=="privacy-filter"{print $6}' | cut -d/ -f1)"
  [ -n "$PFIP" ] && break
  sleep 0.5
done
[ -n "$PFIP" ] || { echo "ERROR: could not read privacy-filter IP"; exit 1; }
echo "    privacy-filter @ $PFIP:8088"

echo "==> (re)starting conduit-web-demo"
container rm -f conduit-web-demo >/dev/null 2>&1 || true
container run -d --name conduit-web-demo --network "$NET" \
  -p "${HOST_PORT}:8088" \
  -v "$DATA:/data" \
  -e CONDUIT_ADMIN_PASSWORD="$ADMIN_PW" \
  -e CONDUIT_PRIVACY_FILTER_URL="http://$PFIP:8088" \
  -e CONDUIT_PRIVACY_FAIL_OPEN="$FAIL_OPEN" \
  conduit-web-demo:0.1.0 >/dev/null

echo "    conduit-web-demo @ http://127.0.0.1:${HOST_PORT}/  (admin: $ADMIN_PW)"
echo "    privacy redaction: ON (fail_open=$FAIL_OPEN), filter URL: http://$PFIP:8088"

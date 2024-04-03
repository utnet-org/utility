#!/bin/sh
set -e

unc_HOME=${unc_HOME:-/srv/unc}
export unc_HOME

if [ -n "$INIT" ]; then
    uncd init ${CHAIN_ID:+--chain-id="$CHAIN_ID"} \
               ${ACCOUNT_ID:+--account-id="$ACCOUNT_ID"}
fi

if [ -n "$NODE_KEY" ]; then
    cat << EOF > "$unc_HOME/node_key.json"
{"account_id": "", "public_key": "", "secret_key": "$NODE_KEY"}
EOF
fi

ulimit -c unlimited

echo "Telemetry: ${TELEMETRY_URL}"
echo "Bootnodes: ${BOOT_NODES}"

exec uncd run ${TELEMETRY_URL:+--telemetry-url="$TELEMETRY_URL"} \
               ${BOOT_NODES:+--boot-nodes="$BOOT_NODES"} "$@"

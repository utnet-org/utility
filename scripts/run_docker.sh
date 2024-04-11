#!/bin/sh
set -e

UNC_HOME=${UNC_HOME:-/srv/unc}
export UNC_HOME

if [ -n "$INIT" ]; then
    uncd init ${CHAIN_ID:+--chain-id="$CHAIN_ID"} \
               ${ACCOUNT_ID:+--account-id="$ACCOUNT_ID"}
fi

if [ -n "$NODE_KEY" ]; then
    cat << EOF > "$UNC_HOME/node_key.json"
{"account_id": "", "public_key": "", "secret_key": "$NODE_KEY"}
EOF
fi

ulimit -c unlimited

echo "Telemetry: ${TELEMETRY_URL}"
echo "Bootnodes: ${BOOT_NODES}"

exec uncd run ${TELEMETRY_URL:+--telemetry-url="$TELEMETRY_URL"} \
               ${BOOT_NODES:+--boot-nodes="$BOOT_NODES"} "$@"

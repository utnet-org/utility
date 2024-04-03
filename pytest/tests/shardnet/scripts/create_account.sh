#!/bin/bash
set -eux

account_id=$(grep account_id /home/ubuntu/.unc/shardnet/validator_key.json | awk -F'"' '{print $4}')
mkdir -p /home/ubuntu/.unc-credentials/shardnet/
printf '{"account_id":"unc","public_key":"%s","private_key":"%s"}' \
    "${1:?}" "${2:?}" > /home/ubuntu/.unc-credentials/shardnet/unc.json
pk=$(grep public_key /home/ubuntu/.unc/shardnet/validator_key.json | awk -F'"' '{print $4}')
cp /home/ubuntu/.unc/shardnet/validator_key.json /home/ubuntu/.unc-credentials/shardnet/"$account_id".json
unc_ENV=shardnet unc --nodeUrl=http://127.0.0.1:3030 \
        create-account "$account_id" --masterAccount unc \
        --initialBalance 1000000 --publicKey "$pk"

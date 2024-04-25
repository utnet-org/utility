#!/bin/bash

unc-node  --home "~/.unc/Unc1" run  > "~/.unc/logfiles/logfile1" 2>&1 &
sleep 1
unc-node  --home "~/.unc/Unc2" run --boot-nodes "ed25519:9e9JtarsJc3JR1PcnU6ykQgUmEf6LCAQi4ZjZjMuxrip@127.0.0.1:24567" > "~/.unc/logfiles/logfile2" 2>&1 &
sleep 1
unc-node  --home "~/.unc/Unc3" run --boot-nodes "ed25519:9e9JtarsJc3JR1PcnU6ykQgUmEf6LCAQi4ZjZjMuxrip@127.0.0.1:24567" > "~/.unc/logfiles/logfile3" 2>&1 &
sleep 1
unc-node  --home "~/.unc/Unc4" run --boot-nodes "ed25519:9e9JtarsJc3JR1PcnU6ykQgUmEf6LCAQi4ZjZjMuxrip@127.0.0.1:24567" > "~/.unc/logfiles/logfile4" 2>&1 &
sleep 10

## create new accounts
unc account create-account fund-later use-auto-generation save-to-folder ~/.unc-credentials/implicit
unc account create-account fund-later use-auto-generation save-to-folder ~/.unc-credentials/implicit
unc account create-account fund-later use-auto-generation save-to-folder ~/.unc-credentials/implicit
## as follows:
## 16438e347058391fdfdd98f13d0bf4fd4d64267d59b67328579d51846565ce9b.json
## 8e42ce2442abe82f49be2cc44d7b6216f406621da3c453f17f286fe78952d389.json
## 41e2f1cc1b5133917ba8b9e49f74e9cb57e45b0f4c2672830659ab8287168a87.json


## send money to other accounts
unc tokens miner send-unc 16438e347058391fdfdd98f13d0bf4fd4d64267d59b67328579d51846565ce9b '35000000 unc' network-config testnet sign-with-keychain send
sleep 2
unc tokens miner send-unc 8e42ce2442abe82f49be2cc44d7b6216f406621da3c453f17f286fe78952d389 '25000000 unc' network-config testnet sign-with-keychain send
sleep 2
unc tokens miner send-unc 41e2f1cc1b5133917ba8b9e49f74e9cb57e45b0f4c2672830659ab8287168a87 '20000000 unc' network-config testnet sign-with-keychain send

## cargo install unc-validator
## pledge new accounts
sleep 2
unc validator pledging pledge-proposal 16438e347058391fdfdd98f13d0bf4fd4d64267d59b67328579d51846565ce9b ed25519:2JvmJLCnRfPLzUnYHZsEhSKcNLw7E2qFPAD8U3gmX2HU '30000000 unc' network-config testnet sign-with-keychain send
sleep 2
unc validator pledging pledge-proposal 8e42ce2442abe82f49be2cc44d7b6216f406621da3c453f17f286fe78952d389 ed25519:9CeceB9q57XdrFgE58byk9RpNyH4cotbRXFZSqLKcW6E '20000000 unc' network-config testnet sign-with-keychain send
sleep 2
unc validator pledging pledge-proposal 41e2f1cc1b5133917ba8b9e49f74e9cb57e45b0f4c2672830659ab8287168a87 ed25519:2VuiWqdedrmv9FNxRWFomr77hgykwXgrhptdKbHcgoFp '30000000 unc' network-config testnet sign-with-plaintext-private-key --signer-public-key ed25519:2VuiWqdedrmv9FNxRWFomr77hgykwXgrhptdKbHcgoFp --signer-private-key ed25519:3NWVkj5Gnz6obeGUBJK7NFeVzErm7uKGgnbnitQgVyXbDVjADTYLwNBPBBGKYqQJwcPTfBfB4wJwT8hhjxHDHFf8 send


## unpledge accounts
## unc validator pledging unpledge-proposal 41e2f1cc1b5133917ba8b9e49f74e9cb57e45b0f4c2672830659ab8287168a87 ed25519:2VuiWqdedrmv9FNxRWFomr77hgykwXgrhptdKbHcgoFp '30000000 unc' network-config testnet sign-with-plaintext-private-key --signer-public-key ed25519:2VuiWqdedrmv9FNxRWFomr77hgykwXgrhptdKbHcgoFp --signer-private-key ed25519:3NWVkj5Gnz6obeGUBJK7NFeVzErm7uKGgnbnitQgVyXbDVjADTYLwNBPBBGKYqQJwcPTfBfB4wJwT8hhjxHDHFf8 send


## view tx status
## unc transaction view-status EWHzhriCRTbDVd9SH6Vk88hSHqzJ7pipXW6eUhWTBkvS network-config testnet

##/// validator pledge
##cargo install unc-validator
##unc-validator validators network-config testnet now
##unc pledging validator-list network-config testnet

##unc-validator proposals network-config testnet
##///pledging
##unc-validator pledging view-pledge 60595eb3cb90fdeb0cd3743b90388f6a3fcd24fda09c9732f1f256e9a01ae5a9 network-config testnet now
##unc-validator pledging pledge-proposal 60595eb3cb90fdeb0cd3743b90388f6a3fcd24fda09c9732f1f256e9a01ae5a9 ed25519:7V7BLUwwYS92NPsLVDyGChUv8fbwj3c8ktPcx5sYwbWp '1500 UNC' network-config testnet sign-with-keychain send
##unc-validator pledging pledge-proposal miner ed25519:8FhzmFG24qXxJ9BJLHTxwhxYY4yu4NV8YPxtksmC86Nv '1500 UNC' network-config testnet sign-with-keychain send
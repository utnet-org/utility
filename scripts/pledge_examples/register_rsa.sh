#!/bin/bash
## Register Rsa2048 keys
sleep 5
unc extensions register-rsa-keys unc use-file ~/.unc/keys/batch_register_rsa1.json with-init-call network-config custom sign-with-access-key-file ~/.unc/keys/unc.json send
sleep 5
unc extensions register-rsa-keys unc use-file ~/.unc/keys/batch_register_rsa2.json with-init-call network-config custom sign-with-access-key-file ~/.unc/keys/unc.json send

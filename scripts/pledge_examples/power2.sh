#!/bin/bash
## Challenge Rsa2048 keys
sleep 5
utility-cli-rs/target/debug/unc extensions create-challenge-rsa 79d96a47f387ae8f8e92a7f5b42e75d86c31c680581ad77cef1115a5b76b6e3b use-file ~/.unc/keys/challenge3.json without-init-call network-config custom sign-with-access-key-file ~/.unc/Unc3/signer_key.json send
sleep 5
utility-cli-rs/target/debug/unc extensions create-challenge-rsa 79d96a47f387ae8f8e92a7f5b42e75d86c31c680581ad77cef1115a5b76b6e3b use-file ~/.unc/keys/challenge4.json without-init-call network-config custom sign-with-access-key-file ~/.unc/Unc3/signer_key.json send

#!/bin/bash
## Challenge Rsa2048 keys
sleep 5
unc extensions create-challenge-rsa 13735a00a19b0b572ed183f517d66c93f22b7cd216b6b0cfd2444191088a86af use-file ~/.unc/keys/challenge1.json without-init-call network-config custom sign-with-access-key-file ~/.unc/Unc2/signer_key.json send
sleep 5
unc extensions create-challenge-rsa 13735a00a19b0b572ed183f517d66c93f22b7cd216b6b0cfd2444191088a86af use-file ~/.unc/keys/challenge2.json without-init-call network-config custom sign-with-access-key-file ~/.unc/Unc2/signer_key.json send

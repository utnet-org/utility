#!/bin/bash
## Challenge Rsa2048 keys
sleep 5
unc extensions register-rsa-keys unc use-file ~/.unc/keys/batch_register_rsa3.json with-init-call network-config custom sign-with-access-key-file ~/.unc/keys/unc.json send
sleep 5
unc extensions create-challenge-rsa e2678f53a51a46a8c76639cf37ed6c6070b995ed759d6fff0fad1c25ee87057d use-file ~/.unc/keys/challenge5.json without-init-call network-config custom sign-with-access-key-file ~/.unc/Unc4/signer_key.json send
sleep 5
unc extensions create-challenge-rsa e2678f53a51a46a8c76639cf37ed6c6070b995ed759d6fff0fad1c25ee87057d use-file ~/.unc/keys/challenge6.json without-init-call network-config custom sign-with-access-key-file ~/.unc/Unc4/signer_key.json send

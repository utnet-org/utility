#!/bin/bash
#
# Downloads the WASM contracts necessary for all workloads and stores them in "res" folder.

cd res
ln -s ../../../../../runtime/unc-test-contracts/res/fungible_token.wasm fungible_token.wasm
ln -s ../../../../../runtime/unc-test-contracts/res/backwards_compatible_rs_contract.wasm congestion.wasm

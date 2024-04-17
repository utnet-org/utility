#!/bin/bash
#
# Downloads the WASM contracts necessary for all workloads and stores them in "res" folder.
# As seen in commands, you will need a fungible token contract to test with. There's one you
# can get from the `utnet/contracts-examples` repository.

cd res
ln -s ../../../../../runtime/unc-test-contracts/res/fungible_token.wasm fungible_token.wasm
ln -s ../../../../../runtime/unc-test-contracts/res/backwards_compatible_rs_contract.wasm congestion.wasm

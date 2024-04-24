# Loadtest

This test requires a few steps. Firstly, build the binary:

```shell
make unc-node-release
```

Secondly, initialise your own localnet:

```shell
./target/release/unc-node --home ~/.unc_tmp init --chain-id localnet
./target/release/unc-node --home ~/.unc_tmp run
```

Thirdly, create accounts and deploy the contract:

```shell
python3 setup.py --home ~/.unc_tmp
```

And lastly, run the test:

```shell
python3 loadtest.py --home ~/.unc_tmp --num_requests=1000
```

## Load Test version 2

The newer loadtest2.py script currently runs an intense load test with the FT contract.

Much like with the earlier version you will want to build a `unc-node`. This script can set up a (2
node) cluster for you (nice for testing):

```sh
ulimit -n 4096
env UTILITY_ROOT=../../../target/release/ python3 loadtest2.py --fungible-token-wasm=$PWD/locust/res/fungible_token.wasm --setup-cluster --accounts=1000 --executors=4
```

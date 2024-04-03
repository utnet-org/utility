A collection of smart-contract used in framework tests.

Rust contracts are built via `build.rs`, the Assembly Script contract
is build manually and committed to the git repository.
`res/unc_evm.wasm` and `res/ZombieOwnership.bin` are taken from
<https://github.com/unc/unc-evm/tree/a651e9be680b59cca9aad86c1f9e9e9b54ad9c06>
and it's for reproduce a performance issue encountered in EVM
contracts.

If you want to use a contract from rust core, add

```toml
[dev-dependencies]
unc-test-contracts = { path = "../unc-test-contracts" }
```

to the Cargo.toml and use `unc_test_contract::rs_contract()`.

If you want to use a contract from an integration test, you can read
the wasm file directly from the `./res` directory.  To populate
`./res`, you need to make sure that this crate was compiled.

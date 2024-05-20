# Utility Indexer

Utility Indexer is a micro-utility. which provides you with a stream of blocks that are recorded on Utility network. It is useful to handle real-time "events" on the chain.

## Rationale

As scaling dApps enter utility’s mainnet, an issue may arise: how do they quickly and efficiently access state from our deployed smart contracts, and cut out the cruft? Contracts may grow to have complex data structures and querying the network RPC may not be the optimal way to access state data. The Utility Indexer Framework allows for streams to be captured and indexed in a customized manner. The typical use-case is for this data to make its way to a relational database. Seeing as this is custom per project, there is engineering work involved in using this utility.

Utility Indexer is already in use for several new projects, namely, we index all the events for Utility Blockchain Explorer, and we also dig into Access Keys and index all of them for Utility Wallet passphrase recovery and multi-factor authentication. With Utility Indexer you can do high-level aggregation as well as low-level introspection of all the events inside the blockchain.

We are going to build more Indexers in the future, and will also consider building Indexer integrations with streaming solutions like Kafka, RabbitMQ, ZeroMQ, and NoSQL databases.

See the [example](https://github.com/utnet-org/utility/tree/master/tools/indexer/example) for further technical details.

## How to set up and test Utility Indexer

Before you proceed, make sure you have the following software installed:

* [rustup](https://rustup.rs/) or Rust version that is mentioned in `rust-toolchain` file in the root of utility.project.

### localnet

Clone [utility](https://github.com/utnet-org/utility)

To run the Utility Indexer connected to a network we need to have configs and keys prepopulated. To generate configs for localnet do the following

```bash
git clone git@github.com:utnet-org/utility.git
cd utility/tools/indexer/example
cargo run --release -- --home-dir ~/.unc/localnet init
```

The above commands should initialize necessary configs and keys to run localnet in `~/.unc/localnet`.

```bash
cargo run --release -- --home-dir ~/.unc/localnet/ run
```

After the node is started, you should see logs of every block produced in your localnet. Get back to the code to implement any custom handling of the data flowing into the indexer.

Use [unc-shell](https://github.com/unc/unc-shell) to submit transactions. For example, to create a new user you run the following command:

```bash
$ UNC_ENV=local unc --keyPath ~/.unc/localnet/validator_key.json \
       create_account new-account.test --masterAccount test
```

### testnet

To run the Utility Indexer connected to testnet or mainnet we need to have configs and keys prepopulated, you can get them with the Utility Indexer Example like above with a little change. Follow the instructions below to run non-validating node (leaving account ID empty).

```bash
cargo run --release -- --home-dir ~/.unc/testnet init --chain-id testnet --download
```

The above code will download the official genesis config and generate necessary configs. You can replace `testnet` in the command above to different network ID `mainnet`.

**NB!** According to changes in `utility/config generation we don't fill all the necessary fields in the config file. you need to download config you want and replace the generated one manually.

* [testnet config.json](https://s3-us-west-1.amazonaws.com/build.utility.com/utility.deploy/testnet/config.json)
* [mainnet config.json](https://s3-us-west-1.amazonaws.com/build.utility.com/utility.deploy/mainnet/config.json)

Replace `config.json` in your `--home-dir` (e.g. `~/.unc/testnet/config.json`) with downloaded one.

After that we can run Utility Indexer.

```bash
cargo run --release -- --home-dir ~/.unc/testnet run
```

After the network is synced, you should see logs of every block produced in Testnet. Get back to the code to implement any custom handling of the data flowing into the indexer.

You can choose Indexer Framework sync mode by setting what to stream:

* `LatestSynced` - Real-time syncing, always taking the latest finalized block to stream
* `FromInterruption` - Starts syncing from the block Utility Indexer was interrupted last time
* `BlockHeight(u64)` - Specific block height to start syncing from

 Refer to `main()` function in [Indexer Example](https://github.com/utnet-org/utility/blob/master/tools/indexer/example/src/main.rs)

Indexer Framework also exposes access to the internal APIs (see `Indexer::client_actors` method), so you can fetch data about any block, transaction, etc, yet by default, utility is configured to remove old data (garbage collection), so querying the data that was observed a few epochs before may return an error saying that the data is not found. If you only need blocks streaming, you don't need this tweak, but if you need access to the historical data right from your Indexer, consider updating `"archive"` setting in `config.json` to `true`:

```json
...
"archive": true,
...
```

## Who is using Utility Indexer?

*This list is not exhaustive, feel free to submit your project by sending a pull request.*

* [Indexer for Utility Wallet](https://github.com/utnet-org/unc-indexer-for-wallet)
* [Indexer for Utility Explorer](https://github.com/utnet-org/unc-indexer-for-explorer)

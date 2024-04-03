<br />
<br />

<p align="center">
<img src="docs/images/logo.gif" width="240">
</p>

<br />
<br />


## Reference implementation of UNC Protocol

![Buildkite](https://img.shields.io/buildkite/0eae07525f8e44a19b48fa937813e2c21ee04aa351361cd851)
![Stable Status][stable-release]
![Prerelease Status][prerelease]
[![codecov][codecov-badge]][codecov-url]
[![Discord chat][discord-badge]][discord-url]
[![Telegram Group][telegram-badge]][telegram-url]

[stable-release]: https://img.shields.io/github/v/release/utnet-org/utility?label=stable
[prerelease]: https://img.shields.io/github/v/release/utnet-org/utility?include_prereleases&label=prerelease
[ci-badge-master]: https://badge.buildkite.com/a81147cb62c585cc434459eedd1d25e521453120ead9ee6c64.svg?branch=master
[ci-url]: https://buildkite.com/utnet-org/utility
[codecov-badge]: https://codecov.io/gh/utnet-org/utility/branch/master/graph/badge.svg
[codecov-url]: https://codecov.io/gh/utnet-org/utility
[discord-badge]: https://img.shields.io/discord/490367152054992913.svg
[discord-url]: https://unc.chat
[telegram-badge]: https://cdn.jsdelivr.net/gh/Patrolavia/telegram-badge@8fe3382b3fd3a1c533ba270e608035a27e430c2e/chat.svg
[telegram-url]: https://t.me/cryptounc

## About UNC

UNC's purpose is to enable community-driven innovation to benefit people around the world.

To achieve this purpose, *UNC* provides a developer platform where developers and entrepreneurs can create apps that put users back in control of their data and assets, which is the foundation of ["Open Web" movement][open-web-url].

One of the components of *UNC* is the UNC Protocol, an infrastructure for server-less applications and smart contracts powered by a blockchain.
UNC Protocol is built to deliver usability and scalability of modern PaaS like Firebase at fraction of the prices that blockchains like Ethereum charge.

Overall, *UNC* provides a wide range of tools for developers to easily build applications:
 - [JS Client library][js-api] to connect to UNC Protocol from your applications.
 - [Rust][rust-sdk] and [JavaScript/TypeScript][js-sdk] SDKs to write smart contracts and stateful server-less functions.
 - [Numerous examples][examples-url] with links to hack on them right inside your browser.
 - [Lots of documentation][docs-url], with [Tutorials][tutorials-url] and [API docs][api-docs-url].

[open-web-url]: https://techcrunch.com/2016/04/10/1301496/
[js-api]: https://github.com/utnet-org/utility/unc-api-js
[rust-sdk]: https://github.com/utnet-org/utility/unc-sdk-rs
[js-sdk]: https://github.com/utnet-org/utility/unc-sdk-js
[examples-url]: https://utnet-org/utility.dev
[docs-url]: https://docs.utnet-org/utility.org
[tutorials-url]: https://docs.utnet-org/utility.org/tutorials/welcome
[api-docs-url]: https://docs.utnet-org/utility.org/api/rpc/introduction

## Join the Network

The easiest way to join the network, is by using the `make release` command, which you can install as follows:

```
# testnet node init
./target/release/uncd --home ~/.unc  init --chain-id testnet --download-genesis --download-config

# download snapshot data （optional）
## install rclone
```sh
# Mac 
$ brew install rclone

# Linux
$ sudo apt install rclone

$ mkdir -p ~/.config/rclone
$ touch ~/.config/rclone/rclone.conf

## rclone config
[unc_aws]
type = s3
provider = AWS
download_url = https://unc-oss.s3.us-west-1.amazonaws.com
region = us-west-1
acl = public-read
server_side_encryption = AES256
storage_class = STANDARD

## download data
$ rclone copy --no-check-certificate unc_aws:unc-oss/latest ./
$ latest=$(cat latest)
$ rclone copy --no-check-certificate --progress --transfers=6  unc_aws:unc-oss/${latest:?} ~/.unc/data

# node run
$ ./target/release/uncd --home ~/.unc  run
```



To learn how to become validator, checkout [documentation](https://docs.utnet-org/utility.org/docs/develop/node/validator/staking-and-delegation).

## Contributing

The workflow and details of setup to contribute are described in [CONTRIBUTING.md](CONTRIBUTING.md), and security policy is described in [SECURITY.md](SECURITY.md).
To propose new protocol changes or standards use [Specification & Standards repository](https://github.com/utility/UEPs).


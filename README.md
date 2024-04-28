<br />
<br />

<p align="center">
<img src="docs/images/logo.gif" width="240">
</p>

<br />
<br />

## Reference implementation of Utility

![Buildkite](https://img.shields.io/buildkite/0eae07525f8e44a19b48fa937813e2c21ee04aa351361cd851)
![Stable Status][stable-release]
![Prerelease Status][prerelease]
[![codecov][codecov-badge]][codecov-url]
[![Discord chat][discord-badge]][discord-url]
[![Telegram Group][telegram-badge]][telegram-url]

[stable-release]: https://img.shields.io/github/v/release/utnet-org/utility?label=stable
[prerelease]: https://img.shields.io/github/v/release/utnet-org/utility?include_prereleases&label=prerelease
[codecov-badge]: https://codecov.io/gh/utnet-org/utility/branch/master/graph/badge.svg
[codecov-url]: https://codecov.io/gh/utnet-org/utility
[discord-badge]: https://img.shields.io/discord/490367152054992913.svg
[discord-url]: https://unc.chat
[telegram-badge]: https://cdn.jsdelivr.net/gh/Patrolavia/telegram-badge@8fe3382b3fd3a1c533ba270e608035a27e430c2e/chat.svg
[telegram-url]: https://t.me/cryptounc

## About Utility

Utility's purpose is to enable community-driven innovation to benefit people around the world.

To achieve this purpose, *Utility* provides a developer platform where developers and entrepreneurs can create apps that put users back in control of their data and assets, which is the foundation of ["Open Web" movement][open-web-url].

One of the components of *Utility* is the Utility Protocol, an infrastructure for server-less applications and smart contracts powered by a blockchain.
Utility Protocol is built to deliver usability and scalability of modern PaaS like Firebase at fraction of the prices that blockchains like Ethereum charge.

Overall, *Utility* provides a wide range of tools for developers to easily build applications:

- [JS Client library][js-api] to connect to Utility Protocol from your applications.
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

The easiest way to join the network, is by using the `make release` command  or download release binaries, which you can install as follows:

```sh
# testnet node init directly use binaries
unc-node --home ~/.unc  init --chain-id testnet --download-genesis --download-config

# download snapshot data （optional）
## install rclone 1.66.0 or beyond
```sh
# Mac 
$ brew install rclone

# Linux
$ sudo apt install rclone

$ mkdir -p ~/.config/rclone
$ touch ~/.config/rclone/rclone.conf

## rclone config
[unc_cf]
type = s3
provider = Cloudflare
endpoint= https://ec9b597fa02615ca6a0e62b7ff35d0cc.r2.cloudflarestorage.com
access_key_id = 2ff213c3730df215a7cc56e28914092e
secret_access_key = b28609e3869b43339c1267b59cf25aa5deff4097737d3848e1491e0729c3ff6c
acl = public-read

## download data 
$ rclone copy --no-check-certificate unc_cf:unc/latest ./
$ latest=$(cat latest)
$ rclone copy --no-check-certificate --progress --transfers=6  unc_cf:unc/${latest:?}.tar.gz /tmp

$ un archive snapshot
tar -zxvf /tmp/${latest:?}.tar.gz -C /tmp  && mv /tmp/${latest:?}/data ~/.unc

## on ～/.unc dir touch file `validator_key.json`  (optional)
{
    "account_id": "miner-addr"
    "public_key":"ed25519:2yMvZrTtjgFMtcpE12G3tdt7KsYKdKE6jufRnz4Yyxw3",
    "private_key":"ed25519:3NVx4sHxBJciEH2wZoMig8YiMx1Q84Ur2RWTd2GQ7JNfWdyDxwwYrUR6XtJR3YcYeWh9NzVEmsnYe2keB97mVExZ"
}

# node run
$ unc-node --home ~/.unc  run
```

To learn how to become validator, checkout [documentation](https://docs.utnet-org/utility.org/docs/develop/node/validator/staking-and-delegation).

## Contributing

The workflow and details of setup to contribute are described in [CONTRIBUTING.md](CONTRIBUTING.md), and security policy is described in [SECURITY.md](SECURITY.md).
To propose new protocol changes or standards use [Specification & Standards repository](https://github.com/utility/UEPs).

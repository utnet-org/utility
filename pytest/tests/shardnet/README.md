# Shardnet tools

## repledge.py

Manages restaking of shardnet network participants. Uses `repledged` to regularly repledge if a node is kicked.
Runs `repledged` on each of the remote machines. Gets the `repledged` binary from AWS.

Optionally creates accounts for the remote nodes, but requires public and private keys of account `unc`.

## Example

```
python3 tests/shardnet/repledge.py
    --delay-sec 60
    --unc-pk $unc_PUBLIC_KEY
    --unc-sk $unc_PRIVATE_KEY
```

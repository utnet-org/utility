#!/usr/bin/env python3
# Runs randomized staking transactions and makes some basic checks on the final `pledging` values
# In each epoch sends two sets of staking transactions, one when (last_height % 12 == 4), called "fake", and
# one when (last_height % 12 == 7), called "real" (because the former will be overwritten by the later).
# Before the "fake" tx we expect the pledges to be equal to the largest of the last three "real" pledges for
# each node. Before "real" txs it is the largest of the same value, and the last "fake" pledge.

import sys, time, base58, random, logging
import pathlib

sys.path.append(str(pathlib.Path(__file__).resolve().parents[2] / 'lib'))

from cluster import start_cluster
from configured_logger import logger
from transaction import sign_staking_tx

TIMEOUT = 360
TIMEOUT_PER_ITER = 30

FAKE_OFFSET = 6
REAL_OFFSET = 12
EPOCH_LENGTH = 18

all_pledges = []
fake_pledges = [0, 0, 0]
next_nonce = 3

# other tests can set `sequence` to some sequence of triplets of pledges
# `do_moar_pledges` first uses the elements of `sequence` for pledges before switching to
# random. See `staking_repro1.py` for an example
sequence = []


def get_validators():
    return set([x['account_id'] for x in nodes[0].get_status()['validators']])


def get_pledges():
    return [
        int(nodes[2].get_account("test%s" % i)['result']['pledging'])
        for i in range(3)
    ]


def get_expected_pledges():
    global all_pledges
    return [
        max(fake_pledges[i], max([x[i]
                                 for x in all_pledges[-3:]]))
        for i in range(3)
    ]


def do_moar_pledges(last_block_hash, update_expected):
    global next_nonce, all_pledges, fake_pledges, sequence

    if len(sequence) == 0:
        pledges = [0, 0, 0]
        # have 1-2 validators with pledge, and the remaining without
        # make numbers dibisable by 1M so that we can easily distinguish a situation when the current pledging amt has some reward added to it (not divisable by 1M) vs not (divisable by 1M)
        pledges[random.randint(0, 2)] = random.randint(
            70000000000000000000000000, 100000000000000000000000000) * 1000000
        pledges[random.randint(0, 2)] = random.randint(
            70000000000000000000000000, 100000000000000000000000000) * 1000000
    else:
        pledges = sequence[0]
        sequence = sequence[1:]

    vals = get_validators()
    val_id = int(list(vals)[0][4:])
    for i in range(3):
        tx = sign_staking_tx(nodes[i].signer_key, nodes[i].validator_key,
                             pledges[i], next_nonce,
                             base58.b58decode(last_block_hash.encode('utf8')))
        nodes[val_id].send_tx(tx)
        next_nonce += 1

    if update_expected:
        fake_pledges = [0, 0, 0]
        all_pledges.append(pledges)
    else:
        fake_pledges = pledges

    logger.info("Sent %s staking txs: %s" %
                ("REAL" if update_expected else "fake", pledges))


def doit(seq=[]):
    global nodes, all_pledges, sequence
    sequence = seq

    config = None
    nodes = start_cluster(
        2, 1, 1, config, [["epoch_length", EPOCH_LENGTH],
                          ["block_producer_kickout_threshold", 40],
                          ["chunk_producer_kickout_threshold", 40]], {
                              0: {
                                  "view_client_throttle_period": {
                                      "secs": 0,
                                      "nanos": 0
                                  },
                                  "consensus": {
                                      "state_sync_timeout": {
                                          "secs": 2,
                                          "nanos": 0
                                      }
                                  }
                              },
                              1: {
                                  "view_client_throttle_period": {
                                      "secs": 0,
                                      "nanos": 0
                                  },
                                  "consensus": {
                                      "state_sync_timeout": {
                                          "secs": 2,
                                          "nanos": 0
                                      }
                                  }
                              },
                              2: {
                                  "tracked_shards": [0],
                                  "view_client_throttle_period": {
                                      "secs": 0,
                                      "nanos": 0
                                  },
                                  "consensus": {
                                      "state_sync_timeout": {
                                          "secs": 2,
                                          "nanos": 0
                                      }
                                  },
                                  "store.state_snapshot_enabled": True,
                              }
                          })

    started = time.time()
    last_iter = started

    height, hash_ = nodes[2].get_latest_block()
    for i in range(3):
        nodes[i].stop_checking_store()

    logger.info("Initial pledges: %s" % get_pledges())
    all_pledges.append(get_pledges())

    do_moar_pledges(hash_, True)
    last_fake_pledges_height = FAKE_OFFSET
    last_pledged_height = REAL_OFFSET

    while True:
        if time.time() - started >= TIMEOUT:
            break

        assert time.time() - last_iter < TIMEOUT_PER_ITER

        height, hash_ = nodes[0].get_latest_block()
        logging.info(
            f"Node 0 at height {height}; time since last staking iteration: {time.time() - last_iter} seconds"
        )
        send_fakes = send_reals = False

        if (height + EPOCH_LENGTH - FAKE_OFFSET) // EPOCH_LENGTH > (
                last_fake_pledges_height + EPOCH_LENGTH -
                FAKE_OFFSET) // EPOCH_LENGTH:
            last_iter = time.time()

            send_fakes = True

        if (height + EPOCH_LENGTH - REAL_OFFSET) // EPOCH_LENGTH > (
                last_pledged_height + EPOCH_LENGTH -
                REAL_OFFSET) // EPOCH_LENGTH:

            send_reals = True

        if send_fakes or send_reals:
            cur_pledges = get_pledges()
            logger.info("Current pledges: %s" % cur_pledges)
            if len(all_pledges) > 1:
                expected_pledges = get_expected_pledges()
                logger.info("Expect  pledges: %s" % expected_pledges)
                for (cur, expected) in zip(cur_pledges, expected_pledges):
                    if cur % 1000000 == 0:
                        assert cur == expected
                    else:
                        assert expected <= cur <= expected * 1.1

            do_moar_pledges(hash_, update_expected=send_reals)

        if send_fakes:
            last_fake_pledges_height += EPOCH_LENGTH

        elif send_reals:
            last_pledged_height += EPOCH_LENGTH

        time.sleep(1)


if __name__ == "__main__":
    doit()

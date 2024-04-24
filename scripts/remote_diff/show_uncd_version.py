#!/usr/bin/env python3
"""
Shows results of '/home/ubuntu/unc-node -V' on provided google cloud machines.
Usage: ./show_uncd_version.py project host1 host2 host3 ...
Example for testnet canaries:
    ./show_uncd_version.py unc-core testnet-canary-rpc-01-europe-north1-a-1f3e1e97 \
    testnet-canary-rpc-02-europe-west2-a-031e15e8 testnet-canary-rpc-archive-01-asia-east2-a-b25465d1 \
    testnet-canary-validator-01-us-west1-a-f160e149
"""
import sys
from utils import display_table, run_on_machine


def get_uncd_info(project, host, user='ubuntu'):
    return run_on_machine("./unc-node -V", user, host, project)


def display_uncd_info(hosts, uncd_info, user='ubuntu'):
    display_table([[host] + uncd_info.split(' ')[1:]
                   for (host, uncd_info) in zip(hosts, uncd_infos)])


if __name__ == '__main__':
    project = sys.argv[1]
    hosts = sys.argv[2:]
    uncd_infos = [get_uncd_info(project, host) for host in hosts]
    display_uncd_info(hosts, uncd_infos)

#!/usr/bin/env bash

cd test-binaries || exit 1;

ensure_repo () {
  if [[ -e $1 ]]; then
    cd $1;
    git pull
  else
    git clone --depth=1 https://github.com/utnet-org/$1;
  fi
}

ensure_repo utility-sdk-rs;
ensure_repo core-contracts;

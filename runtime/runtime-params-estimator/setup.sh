#!/usr/bin/env bash

cd test-binaries || exit 1;

ensure_repo () {
  if [[ -e $1 ]]; then
    cd $1;
    git pull
  else
    git clone --depth=1 https://github.com/unc/$1;
  fi
}

ensure_repo unc-sdk-rs;
ensure_repo core-contracts;

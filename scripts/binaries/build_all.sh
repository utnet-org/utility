#!/bin/bash
export ENGINE="${ENGINE:-docker}"
export ARCH="${ARCH:-x64}"
export ZIP="true"

for distro in ubuntu-2004 ubuntu-2204 ubuntu-2404 fedora-39 fedora-40 debian-11 debian-12 arch; do
    DISTRO=${distro} ./scripts/binaries/build.sh
done

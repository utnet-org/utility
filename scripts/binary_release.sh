#!/bin/bash
set -xeo pipefail

release="${1:-uncd-release}"

case "$release" in
  uncd-release|nightly-release|perf-release|assertions-release)
    ;;
  *)  
    echo "Unsupported release type '$release'. Please provide no argument for normal release or provide nightly-release for nightly."
    exit 1
    ;;
esac

BRANCH=$(git branch --show-current)
COMMIT=$(git rev-parse HEAD)

os=$(uname)
arch=$(uname -m)
os_and_arch=${os}-${arch}

function tar_binary {
  mkdir -p $1/${os_and_arch}
  cp target/release/$1 $1/${os_and_arch}/
  tar -C $1 -czvf $1.tar.gz ${os_and_arch}
}

make $release

function upload_binary {
  if [ "$release" = "uncd-release" ]
  then
    tar_binary $1
    tar_file=$1.tar.gz
    aws s3 cp --acl public-read target/release/$1 s3://unc-oss/${os}/${BRANCH}/$1
    aws s3 cp --acl public-read target/release/$1 s3://unc-oss/${os}/${BRANCH}/${COMMIT}/$1
    aws s3 cp --acl public-read target/release/$1 s3://unc-oss/${os}/${BRANCH}/${COMMIT}/stable/$1

    aws s3 cp --acl public-read target/release/$1 s3://unc-oss/${os_and_arch}/${BRANCH}/$1
    aws s3 cp --acl public-read target/release/$1 s3://unc-oss/${os_and_arch}/${BRANCH}/${COMMIT}/$1
    aws s3 cp --acl public-read target/release/$1 s3://unc-oss/${os_and_arch}/${BRANCH}/${COMMIT}/stable/$1

    aws s3 cp --acl public-read ${tar_file} s3://unc-oss/${os_and_arch}/${BRANCH}/${tar_file}
    aws s3 cp --acl public-read ${tar_file} s3://unc-oss/${os_and_arch}/${BRANCH}/${COMMIT}/${tar_file}
    aws s3 cp --acl public-read ${tar_file} s3://unc-oss/${os_and_arch}/${BRANCH}/${COMMIT}/stable/${tar_file}

  else
    folder="${release%-release}"
    aws s3 cp --acl public-read target/release/$1 s3://unc-oss/${os}/${BRANCH}/${COMMIT}/${folder}/$1
    aws s3 cp --acl public-read target/release/$1 s3://unc-oss/${os_and_arch}/${BRANCH}/${COMMIT}/${folder}/$1
  fi
}

upload_binary uncd

# disabled until we clarify why we need this binary in S3
# if [ "$release" != "assertions-release" ]
# then
#   upload_binary store-validator
# fi

# if [ "$release" = "release" ]
# then
#   upload_binary unc-sandbox
# fi

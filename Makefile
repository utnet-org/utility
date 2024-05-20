export DOCKER_BUILDKIT = 1
export CARGO_BUILD_RUSTFLAGS = -D warnings
export UNC_RELEASE_BUILD = no
export CARGO_TARGET_DIR = target


# By default, build a regular release
all: release


docker-utility: DOCKER_TAG ?= utility
docker-utility:
	docker build -t $(DOCKER_TAG) -f Dockerfile --build-arg=make_target=unc-node-release --progress=plain .

docker-utility-sandbox: DOCKER_TAG ?= utility-sandbox
docker-utility-sandbox:
	docker build -t $(DOCKER_TAG) -f Dockerfile --build-arg=make_target=unc-node-sandbox-release --progress=plain .

docker-utility-nightly: DOCKER_TAG ?= utility-nightly
docker-utility-nightly:
	docker build -t $(DOCKER_TAG) -f Dockerfile --build-arg=make_target=unc-node-nightly-release --progress=plain .


release: unc-node-release
	cargo build -p store-validator --release
	cargo build -p genesis-populate --release
	$(MAKE) sandbox-release

unc-node: unc-node-release
	@echo 'unc-node binary ready in ./target/release/unc-node'

unc-node-release: UNC_RELEASE_BUILD=release
unc-node-release:
	cargo build -p unc-node --release

unc-node-debug:
	cargo build -p unc-node

debug: unc-node-debug
	cargo build -p store-validator
	cargo build -p genesis-populate
	$(MAKE) sandbox


perf-release: UNC_RELEASE_BUILD=release
perf-release:
	CARGO_PROFILE_RELEASE_DEBUG=true cargo build -p unc-node --release --features performance_stats
	cargo build -p store-validator --release --features unc-infra.performance_stats


perf-debug:
	cargo build -p unc-node --features performance_stats
	cargo build -p store-validator --features unc-infra.performance_stats


nightly-release: unc-node-nightly-release
	cargo build -p store-validator --release --features unc-infra/nightly,unc-infra.performance_stats
	cargo build -p genesis-populate --release --features unc-infra/nightly,unc-infra.performance_stats

unc-node-nightly-release:
	cargo build -p unc-node --release --features nightly,performance_stats


nightly-debug:
	cargo build -p unc-node --features nightly,performance_stats
	cargo build -p store-validator --features unc-infra/nightly,unc-infra.performance_stats
	cargo build -p genesis-populate --features unc-infra/nightly,unc-infra.performance_stats


assertions-release: UNC_RELEASE_BUILD=release
assertions-release:
	CARGO_PROFILE_RELEASE_DEBUG=true CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true cargo build -p unc-node --release --features performance_stats

sandbox: CARGO_TARGET_DIR=sandbox
sandbox: unc-node-sandbox
	mkdir -p target/debug
	ln -f sandbox/debug/unc-node target/debug/unc-node-sandbox
	@ln -f sandbox/debug/unc-node target/debug/unc-sandbox

unc-node-sandbox:
	cargo build -p unc-node --features sandbox


sandbox-release: CARGO_TARGET_DIR=sandbox
sandbox-release: unc-node-sandbox-release
	mkdir -p target/release
	ln -f sandbox/release/unc-node target/release/unc-node-sandbox
	@ln -f sandbox/release/unc-node target/release/unc-sandbox

unc-node-sandbox-release:
	cargo build -p unc-node --features sandbox --release


.PHONY: docker-utility docker-utility-nightly release unc-node debug
.PHONY: perf-release perf-debug nightly-release nightly-debug assertions-release sandbox
.PHONY: sandbox-release

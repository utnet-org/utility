export DOCKER_BUILDKIT = 1
export CARGO_BUILD_RUSTFLAGS = -D warnings
export UNC_RELEASE_BUILD = no
export CARGO_TARGET_DIR = target


# By default, build a regular release
all: release


docker-framework: DOCKER_TAG ?= framework
docker-framework:
	docker build -t $(DOCKER_TAG) -f Dockerfile --build-arg=make_target=uncd-release         --progress=plain .

docker-framework-sandbox: DOCKER_TAG ?= framework-sandbox
docker-framework-sandbox:
	docker build -t $(DOCKER_TAG) -f Dockerfile --build-arg=make_target=uncd-sandbox-release --progress=plain .

docker-framework-nightly: DOCKER_TAG ?= framework-nightly
docker-framework-nightly:
	docker build -t $(DOCKER_TAG) -f Dockerfile --build-arg=make_target=uncd-nightly-release --progress=plain .


release: uncd-release
	cargo build -p store-validator --release
	cargo build -p genesis-populate --release
	$(MAKE) sandbox-release

uncd: uncd-release
	@echo 'uncd binary ready in ./target/release/uncd'

uncd-release: UNC_RELEASE_BUILD=release
uncd-release:
	cargo build -p uncd --release

uncd-debug:
	cargo build -p uncd

debug: uncd-debug
	cargo build -p store-validator
	cargo build -p genesis-populate
	$(MAKE) sandbox


perf-release: UNC_RELEASE_BUILD=release
perf-release:
	CARGO_PROFILE_RELEASE_DEBUG=true cargo build -p uncd --release --features performance_stats
	cargo build -p store-validator --release --features framework/performance_stats


perf-debug:
	cargo build -p uncd --features performance_stats
	cargo build -p store-validator --features framework/performance_stats


nightly-release: uncd-nightly-release
	cargo build -p store-validator --release --features framework/nightly,framework/performance_stats
	cargo build -p genesis-populate --release --features framework/nightly,framework/performance_stats

uncd-nightly-release:
	cargo build -p uncd --release --features nightly,performance_stats


nightly-debug:
	cargo build -p uncd --features nightly,performance_stats
	cargo build -p store-validator --features framework/nightly,framework/performance_stats
	cargo build -p genesis-populate --features framework/nightly,framework/performance_stats


assertions-release: UNC_RELEASE_BUILD=release
assertions-release:
	CARGO_PROFILE_RELEASE_DEBUG=true CARGO_PROFILE_RELEASE_DEBUG_ASSERTIONS=true cargo build -p uncd --release --features performance_stats

sandbox: CARGO_TARGET_DIR=sandbox
sandbox: uncd-sandbox
	mkdir -p target/debug
	ln -f sandbox/debug/uncd target/debug/uncd-sandbox
	@ln -f sandbox/debug/uncd target/debug/unc-sandbox

uncd-sandbox:
	cargo build -p uncd --features sandbox


sandbox-release: CARGO_TARGET_DIR=sandbox
sandbox-release: uncd-sandbox-release
	mkdir -p target/release
	ln -f sandbox/release/uncd target/release/uncd-sandbox
	@ln -f sandbox/release/uncd target/release/unc-sandbox

uncd-sandbox-release:
	cargo build -p uncd --features sandbox --release


.PHONY: docker-framework docker-framework-nightly release uncd debug
.PHONY: perf-release perf-debug nightly-release nightly-debug assertions-release sandbox
.PHONY: sandbox-release

# syntax=docker/dockerfile-upstream:experimental

FROM ubuntu:22.04 as build

RUN apt-get update -qq && apt-get install -y \
    git \
    cmake \
    g++ \
    pkg-config \
    libssl-dev \
    curl \
    llvm \
    clang \
    && rm -rf /var/lib/apt/lists/*

COPY ./rust-toolchain.toml /tmp/rust-toolchain.toml

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH

RUN curl https://sh.rustup.rs -sSf | \
    sh -s -- -y --no-modify-path --default-toolchain none

VOLUME [ /workdir ]
WORKDIR /workdir
COPY . .

ENV PORTABLE=ON
ARG make_target=unc-node-release
RUN make CARGO_TARGET_DIR=/tmp/target \
    "${make_target:?make_target not set}"

# Docker image
FROM ubuntu:22.04

EXPOSE 3030 12345

RUN apt-get update -qq && apt-get install -y \
    libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY scripts/run_docker.sh /usr/local/bin/run.sh
COPY --from=build /tmp/target/release/unc-node /usr/local/bin/unc-node
RUN chmod +x /usr/local/bin/unc-node

# Opencontainers annotations
LABEL org.opencontainers.image.authors="The Utility Project Team" \
	org.opencontainers.image.url="https://www.utnet.org/" \
	org.opencontainers.image.source="https://github.com/utnet-org/utility" \
	org.opencontainers.image.version="0.8.0" \
	org.opencontainers.image.revision="1" \
	org.opencontainers.image.vendor="The Utility Project" \
	org.opencontainers.image.licenses="GPL-2.0-or-later" \
	org.opencontainers.image.title="Utility Node" \
	org.opencontainers.image.description="Utility Chain Docker Node"

CMD ["/usr/local/bin/run.sh"]

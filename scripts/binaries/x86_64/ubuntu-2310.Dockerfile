FROM ubuntu:23.10 as builder

# This installs all dependencies that we need (besides Rust).
RUN apt update -y && \
    apt install build-essential git clang curl libssl-dev llvm libudev-dev make cmake protobuf-compiler -y

# This installs Rust
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs > rust_install.sh && chmod u+x rust_install.sh && ./rust_install.sh -y

ADD . ./workdir
WORKDIR "/workdir"

# This installs the right toolchain
RUN $HOME/.cargo/bin/rustup show

# This builds the binary.
#RUN $HOME/.cargo/bin/cargo build --locked --release -p unc-node
ARG make_target=release
RUN make CARGO_TARGET_DIR=/tmp/target \
    "${make_target:?make_target not set}"

# Create output folder
RUN mkdir -p output

VOLUME ["/output"]
CMD cp /tmp/target/release/unc-node /output

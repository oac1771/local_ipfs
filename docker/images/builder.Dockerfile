FROM debian:bullseye-slim as builder

ENV PATH="/root/.cargo/bin:${PATH}"

ARG RUST_VERSION

RUN apt-get update
RUN apt-get install -y curl \
                       protobuf-compiler \
                       build-essential \    
                       libclang-dev \ 
                       wget

# Install Rust
RUN curl -sSf https://sh.rustup.rs/ | bash -s -- -y
RUN rustup install ${RUST_VERSION}
RUN rustup default ${RUST_VERSION}-aarch64-unknown-linux-gnu

# Install Ipfs Cli
RUN wget https://dist.ipfs.io/go-ipfs/v0.9.0/go-ipfs_v0.9.0_linux-arm64.tar.gz
RUN tar -xvzf go-ipfs_v0.9.0_linux-arm64.tar.gz
RUN cd go-ipfs && ./install.sh
RUN rm -rf go-ipfs_v0.9.0_linux-arm64.tar.gz
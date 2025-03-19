FROM debian:bullseye-slim as builder

ENV PATH="/root/.cargo/bin:${PATH}"

ARG RUST_VERSION

RUN apt-get update
RUN apt-get install -y curl \
                       protobuf-compiler \
                       build-essential \    
                       libclang-dev

RUN curl -sSf https://sh.rustup.rs/ | bash -s -- -y
RUN rustup install ${RUST_VERSION}
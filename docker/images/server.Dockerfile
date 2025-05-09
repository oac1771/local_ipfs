ARG BUILDER_IMAGE

FROM ${BUILDER_IMAGE} as builder
LABEL stage=intermediate

COPY . .

RUN cargo build -p server --release

##############################################################################
FROM docker.io/library/ubuntu:20.04
LABEL stage=app

RUN apt-get update && apt install -y openssl

COPY --from=builder /target/release/server /usr/local/bin
COPY --from=builder /usr/local/bin/ipfs /usr/local/bin

ARG BUILDER_IMAGE

FROM ${BUILDER_IMAGE} as builder
LABEL stage=intermediate

COPY . .

RUN cargo build --workspace --release

##############################################################################
FROM docker.io/library/ubuntu:20.04
LABEL stage=app

COPY --from=builder /target/release/server /usr/local/bin

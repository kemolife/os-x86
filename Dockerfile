# Build with: docker build --platform=linux/amd64 -t os-x86 .
# amd64 is required — gcc-multilib / binutils-i686-linux-gnu are not packaged
# for arm64 Ubuntu. The platform is set at build time, not hardcoded here.
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    nasm \
    make \
    gcc \
    gcc-multilib \
    binutils-i686-linux-gnu \
    gdb \
    qemu-system-x86 \
    curl \
    && rm -rf /var/lib/apt/lists/*

ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:$PATH

RUN curl https://sh.rustup.rs -sSf | sh -s -- -y \
    --default-toolchain nightly \
    --profile minimal \
    --no-modify-path \
    && rustup component add rust-src --toolchain nightly

WORKDIR /os

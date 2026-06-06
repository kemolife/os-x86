FROM --platform=linux/amd64 ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update && apt-get install -y \
    nasm \
    make \
    gcc \
    gcc-multilib \
    binutils-i686-linux-gnu \
    gdb \
    qemu-system-x86 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /os

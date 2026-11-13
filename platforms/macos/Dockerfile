FROM debian:12-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    clang lld llvm gdc gcc g++ python3 scons xz-utils p7zip-full \
    ca-certificates patch qemu-user gcc-aarch64-linux-gnu g++-aarch64-linux-gnu \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /work

FROM debian:12-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    nasm gcc g++ gcc-mingw-w64-x86-64-posix g++-mingw-w64-x86-64-posix binutils-mingw-w64-x86-64 \
    wine64 xvfb xauth libgl1-mesa-dri libglx-mesa0 python3 ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /work

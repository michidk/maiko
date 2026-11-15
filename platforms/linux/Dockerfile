FROM redguard-recomp-tools:bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsdl2-dev libsdl2-mixer-dev mesa-vulkan-drivers libvulkan1 libegl1 libssl-dev pkg-config gdb xdotool ffmpeg \
    && rm -rf /var/lib/apt/lists/*

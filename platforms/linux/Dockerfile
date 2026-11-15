FROM redguard-recomp-tools:bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
    libasound2-dev libudev-dev libxkbcommon-x11-0 mesa-vulkan-drivers libvulkan1 libegl1 libssl-dev pkg-config gdb xdotool ffmpeg \
    && rm -rf /var/lib/apt/lists/*

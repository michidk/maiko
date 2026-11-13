FROM redguard-recomp-tools:bookworm
RUN apt-get update && apt-get install -y --no-install-recommends \
    libsdl2-dev libsdl2-mixer-dev libgl-dev libglu1-mesa-dev libssl-dev pkg-config gdb xdotool ffmpeg \
    && rm -rf /var/lib/apt/lists/*

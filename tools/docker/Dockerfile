FROM debian:12-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    nasm gcc g++ gcc-mingw-w64-x86-64-posix g++-mingw-w64-x86-64-posix binutils-mingw-w64-x86-64 \
    xvfb xauth libgl1-mesa-dri libglx-mesa0 python3 ca-certificates \
    && python3 -c "import urllib.request; base='https://dl.winehq.org/wine-builds/debian/pool/main/w/wine/'; urllib.request.urlretrieve(base+'wine-stable-amd64_10.0.0.0~bookworm-1_amd64.deb','/tmp/wine-amd64.deb'); urllib.request.urlretrieve(base+'wine-stable_10.0.0.0~bookworm-1_amd64.deb','/tmp/wine-common.deb')" \
    && python3 -c "import hashlib; checks={'/tmp/wine-amd64.deb':'57ea31156f508dfba175eb2db3467346aaa9b0ca948785959ac67778cfdac42b','/tmp/wine-common.deb':'69ca6a124fa8dfaf9cc2467d2fbcaf4e2edf7b5f641d66765fd3b22a120210e4'}; [(_ for _ in ()).throw(ValueError(p)) for p,h in checks.items() if hashlib.sha256(open(p,'rb').read()).hexdigest()!=h]" \
    && apt-get install -y --no-install-recommends /tmp/wine-amd64.deb \
    && dpkg-deb -x /tmp/wine-common.deb / \
    && mkdir -p /usr/lib/wine \
    && ln -s /opt/wine-stable/bin/wine64 /usr/lib/wine/wine64 \
    && ln -s /opt/wine-stable/bin/wineserver /usr/lib/wine/wineserver64 \
    && rm -rf /var/lib/apt/lists/* /tmp/wine-*.deb
LABEL org.maiko.wine-version="10.0"
WORKDIR /work

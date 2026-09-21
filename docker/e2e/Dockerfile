# E2E verification box for the three hard install/update/sideload scenarios
# covering quiver.core + quiver.desktop together.
#
# Adapted from Crowbar's docker/dev-linux box (same purpose: a disposable
# Linux desktop sandbox with a real X11 session over noVNC, since a Tauri app
# needs a real GTK/webkit2gtk display to actually run, not a headless mock).
# Base: ubuntu:24.04, matching GitHub Actions' ubuntu-latest so apt package
# versions (webkit2gtk-4.1, etc.) line up with what both repos' own CI
# already proves builds.
#
# The build context is THIS directory, not the repo root. The repo itself is
# bind-mounted at run time (see docker-compose.yml), and a repo-root context
# would hand the daemon a 5 GB tarball (node_modules + src-tauri/target) on
# every image build for the sake of one shell script.
FROM ubuntu:24.04

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    DISPLAY=:1 \
    RESOLUTION=1280x800x24 \
    VNC_PORT=5900 \
    NOVNC_PORT=6080

# --- Desktop environment + remote access ---------------------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
    xfce4 \
    xfce4-terminal \
    xvfb \
    x11vnc \
    x11-utils \
    x11-xserver-utils \
    xauth \
    xdotool \
    wmctrl \
    scrot \
    dbus-x11 \
    python3-websockify \
    fonts-dejavu-core \
    fonts-liberation \
    git \
    curl \
    wget \
    unzip \
    jq \
    procps \
    psmisc \
    lsof \
    openssl \
    python3 \
    python3-venv \
    ca-certificates \
    sudo \
    && rm -rf /var/lib/apt/lists/*

ARG NOVNC_VERSION=1.5.0
RUN git clone --branch v${NOVNC_VERSION} --depth 1 \
    https://github.com/novnc/noVNC.git /opt/novnc \
    && ln -s /opt/novnc/vnc.html /opt/novnc/index.html

# --- Tauri v2 Linux build prerequisites -----------------------------------
RUN apt-get update && apt-get install -y --no-install-recommends \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    libgtk-3-dev \
    libssl-dev \
    libxdo-dev \
    build-essential \
    file \
    patchelf \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# --- Toolchains, pinned to match each repo's own CI ------------------------
ARG GO_VERSION=1.26.2
RUN GOARCH_DEB="$(dpkg --print-architecture)" \
    && curl -fsSL https://go.dev/dl/go${GO_VERSION}.linux-${GOARCH_DEB}.tar.gz | tar -C /usr/local -xz
ENV PATH="/usr/local/go/bin:${PATH}"

ARG RUST_VERSION=1.88
ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH="/usr/local/cargo/bin:${PATH}"
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | \
    sh -s -- -y --default-toolchain ${RUST_VERSION} --profile default

RUN curl -fsSL https://bun.sh/install | bash
ENV PATH="/root/.bun/bin:${PATH}"

# webkit2gtk's own sandbox needs privileges a default container does not have,
# and its GPU/DMA-BUF paths need a GPU a container does not have either. Set
# here rather than per-scenario so every process that ever launches the app --
# a scenario driver, an arrow's own relaunch step, a human poking around over
# VNC -- gets identical treatment.
ENV WEBKIT_DISABLE_COMPOSITING_MODE=1 \
    WEBKIT_DISABLE_DMABUF_RENDERER=1 \
    WEBKIT_FORCE_SANDBOX=0 \
    GDK_BACKEND=x11 \
    LIBGL_ALWAYS_SOFTWARE=1 \
    NO_AT_BRIDGE=1

WORKDIR /workspace

COPY start.sh /usr/local/bin/start.sh
RUN chmod +x /usr/local/bin/start.sh

EXPOSE 6080

ENTRYPOINT ["/usr/local/bin/start.sh"]

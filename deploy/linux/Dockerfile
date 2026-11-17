# LanLink Linux 运行时容器
#
# 用途：在 webkit2gtk 版本不达标的机器上（如 Ubuntu 18.04 只有 4.0）运行 Tauri v2 应用。
# Tauri v2 硬性要求 webkit2gtk 4.1 + libsoup3，Ubuntu 22.04 起才有。
#
# 构建（在有 docker 的 Linux 机器上，二进制来自 CI 产物）：
#   tar -xzf lanlink-linux-x86_64.tar.gz
#   docker build -t lanlink:linux .
#
# 运行见 run.sh

FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive \
    LANG=C.UTF-8 \
    # 容器里 WebKit 的 bwrap 沙箱通常起不来，关掉它（仅测试环境）
    WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 \
    # 无 GPU 时避免合成器初始化失败导致白窗
    WEBKIT_DISABLE_COMPOSITING_MODE=1 \
    LIBGL_ALWAYS_SOFTWARE=1

RUN apt-get update && apt-get install -y --no-install-recommends \
      libwebkit2gtk-4.1-0 \
      libjavascriptcoregtk-4.1-0 \
      libgtk-3-0 \
      libsoup-3.0-0 \
      libglib2.0-0 \
      librsvg2-2 \
      libssl3 \
      libx11-6 \
      dbus-x11 \
      dbus \
      xdg-utils \
      fonts-noto-cjk \
      fonts-dejavu-core \
      ca-certificates \
      file \
    && rm -rf /var/lib/apt/lists/*

COPY entrypoint.sh /usr/local/bin/lanlink-entrypoint
COPY lanlink-desktop /usr/local/bin/lanlink-desktop
RUN chmod 755 /usr/local/bin/lanlink-entrypoint /usr/local/bin/lanlink-desktop

ENTRYPOINT ["/usr/local/bin/lanlink-entrypoint"]

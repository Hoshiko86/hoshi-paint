# Source from an existing checkout; used only in the managed Linux environment.
export CARGO_HOME=/workspace/.hoshi-tools/cargo
export RUSTUP_HOME=/workspace/.hoshi-tools/rustup
export PATH="$CARGO_HOME/bin:/workspace/.hoshi-tools/fontconfig/usr/bin:$PATH"
export PKG_CONFIG_PATH="/workspace/.hoshi-tools/fontconfig/usr/lib/x86_64-linux-gnu/pkgconfig:/workspace/.hoshi-tools/fontconfig/usr/share/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export PKG_CONFIG_SYSROOT_DIR=/workspace/.hoshi-tools/fontconfig
export LD_LIBRARY_PATH="/workspace/.hoshi-tools/fontconfig/usr/lib/x86_64-linux-gnu${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
export XDG_CACHE_HOME=/workspace/.hoshi-tools/cache
export CARGO_BUILD_JOBS=4
mkdir -p "$XDG_CACHE_HOME"

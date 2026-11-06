#!/usr/bin/env bash
# Rootless prerequisites for the managed Debian 13 development machine.
set -euo pipefail
. /etc/os-release
if [ "$ID" != debian ] || [ "$VERSION_ID" != 13 ]; then
  printf 'This helper targets Debian 13. Use your OS development packages instead.\n' >&2
  exit 1
fi
root=/workspace/.hoshi-tools
mkdir -p "$root/apt/lists/partial" "$root/apt/archives/partial" "$root/apt/etc" "$root/apt/log" "$root/linux-packages" "$root/fontconfig"
printf '%s\n' 'deb [signed-by=/usr/share/keyrings/debian-archive-keyring.gpg] https://deb.debian.org/debian trixie main' > "$root/apt/etc/sources.list"
apt_options=(
  -o "Dir=$root/apt"
  -o "Dir::Etc::sourcelist=$root/apt/etc/sources.list"
  -o Dir::Etc::sourceparts=-
  -o "Dir::State::lists=$root/apt/lists"
  -o Dir::State::status=/var/lib/dpkg/status
  -o "Dir::Cache::archives=$root/apt/archives"
  -o "APT::Sandbox::User=$(id -un)"
)
if [ -n "${HTTPS_PROXY:-}" ]; then apt_options+=(-o "Acquire::https::Proxy=$HTTPS_PROXY"); fi
apt-get "${apt_options[@]}" update
cd "$root/linux-packages"
packages=(libfontconfig-dev libfontconfig1 libfreetype-dev libfreetype6 libpng-dev libpng16-16t64 uuid-dev libuuid1 libbz2-dev libbz2-1.0 libbrotli-dev libbrotli1 zlib1g-dev zlib1g xvfb xserver-common xauth libxfont2)
downloads=()
# Select versions from this signed index rather than unrelated installed versions.
for package in "${packages[@]}"; do downloads+=("$package/trixie"); done
apt-get "${apt_options[@]}" download "${downloads[@]}"
for archive in ./*.deb; do dpkg-deb -x "$archive" "$root/fontconfig"; done
sha256sum ./*.deb > SHA256SUMS
printf 'Verified Debian packages extracted locally. Source scripts/cloud-env.sh before building.\n'

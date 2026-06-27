#!/usr/bin/env bash
# Assemble a fbootd .deb from a prebuilt binary.
# Usage: build-deb.sh <binary-path> <debian-arch> <version> <output-dir>
#   debian-arch: amd64 | arm64
set -euo pipefail

binary="$1"
arch="$2"
version="$3"
outdir="$4"

here="$(cd "$(dirname "$0")" && pwd)"
pkgroot="$(mktemp -d)"
trap 'rm -rf "$pkgroot"' EXIT

install -D -m 0755 "$binary"               "$pkgroot/usr/bin/fbootd"
install -D -m 0644 "$here/fbootd.service"  "$pkgroot/lib/systemd/system/fbootd.service"
install -D -m 0644 "$here/fbootd.toml"     "$pkgroot/etc/fbootd.toml"

mkdir -p "$pkgroot/DEBIAN"
install -m 0755 "$here/postinst" "$pkgroot/DEBIAN/postinst"
install -m 0755 "$here/prerm"    "$pkgroot/DEBIAN/prerm"
install -m 0755 "$here/postrm"   "$pkgroot/DEBIAN/postrm"

# Mark the shipped config as a conffile so admin edits survive upgrades.
printf '/etc/fbootd.toml\n' > "$pkgroot/DEBIAN/conffiles"

size_kb="$(du -ks "$pkgroot/usr" "$pkgroot/etc" "$pkgroot/lib" | awk '{s+=$1} END {print s}')"

cat > "$pkgroot/DEBIAN/control" <<EOF
Package: fbootd
Version: ${version}
Section: net
Priority: optional
Architecture: ${arch}
Maintainer: minco <mail@drchi.co.kr>
Installed-Size: ${size_kb}
Depends: adduser
Recommends: ipmitool
Description: fbootd PXE/iPXE network boot daemon
 fbootd is a self-contained PXE/iPXE network boot server providing
 ProxyDHCP, TFTP, an HTTP boot endpoint, a JSON/WebSocket dashboard API,
 and IPMI power control for bare-metal provisioning.
EOF

mkdir -p "$outdir"
deb="$outdir/fbootd_${version}_${arch}.deb"
dpkg-deb --root-owner-group --build "$pkgroot" "$deb"
echo "Built $deb"

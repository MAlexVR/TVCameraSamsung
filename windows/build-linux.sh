#!/usr/bin/env bash
# SPDX-License-Identifier: GPL-3.0-only
set -eu
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if ! command -v x86_64-w64-mingw32-gcc >/dev/null; then
  printf '%s\n' 'Missing MinGW linker. Install Fedora mingw64-gcc through the normal system policy.' >&2
  exit 2
fi
if ! rustup target list --toolchain 1.88.0 --installed | grep -Fxq x86_64-pc-windows-gnu; then
  printf '%s\n' 'Missing Rust target: rustup target add --toolchain 1.88.0 x86_64-pc-windows-gnu' >&2
  exit 2
fi
export CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER=x86_64-w64-mingw32-gcc
cargo +1.88.0 build --manifest-path "$here/Cargo.toml" --release --offline --locked \
  --target x86_64-pc-windows-gnu --target-dir "$here/build"
cp -- "$here/build/x86_64-pc-windows-gnu/release/samsung-camera-windows.exe" "$here/samsung-camera-windows.exe"
printf '%s\n' 'Built Windows x64 camera utilities; signed driver installation and native hardware tests are still required.'

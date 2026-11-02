#!/bin/sh
# SPDX-License-Identifier: GPL-3.0-only
# Help intentionally needs no Python, polkit, or systemd installation.
case "${1:---help}" in
  --help|-h)
    printf '%s\n' 'Usage: install.sh --check FIRMWARE | --install FIRMWARE' \
      'Check is read-only. --install explicitly authorizes privileged automatic initialization.' \
      'Use a trusted release; firmware is supplied separately. Existing installs are refused.'
    exit 0 ;;
esac
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 1
exec /usr/bin/python3 -I -B "$HERE/install.py" "$@"

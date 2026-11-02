# Version 1.0.0

## Included

- Linux x86-64 guarded RAM firmware initializer with opt-in installer/systemd integration. Existing OS video/audio drivers remain in use.
- Windows x64 experimental read-only diagnostic with per-user installer. **Not a firmware loader or functional Windows camera driver.**
- Source archive, GPL-3.0 license, build information and SHA-256 checksums.

## Important limits

Windows execution/hardware support remains unverified and the EXE is unsigned.
No vendor firmware, private media, credentials or signing keys are included.
Linux requires separately supplied pinned firmware and compatible runtime libraries.
Installers have offline safety checks; native installation on fresh Linux/Windows
hosts is not established by structural tests. Existing working Linux setup, LED
and image behavior were not changed or retested for this release.

## Next step

Run the Windows diagnostic and provide its generated report before implementing
Windows firmware initialization. Never disable system protections to run a package.

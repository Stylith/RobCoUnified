NucleonOS 0.4.4 highlights since 0.4.3:

- embed the native app icon into the binary so the macOS `.app` no longer falls back to the default eframe icon when launched outside the repo root
- start terminal shells as normal login shells instead of suppressing startup files, so Homebrew-installed commands resolve again in bundled macOS builds
- set explicit macOS bundle icon metadata in the release workflow so the packaged `.app` advertises the shipped `NucleonOS.icns` resource cleanly

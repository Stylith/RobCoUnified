Nucleon Desktop First-Party Optional Addons Repository

This directory is the staged contents for the future standalone `nucleon-desktop`
first-party addons repository.

Current purpose:
- host optional first-party addon manifests outside the core shell repo
- publish a repository index consumable by the installer feed
- keep shell-critical addons in the main repo while optional addons move out first

Current optional addons staged here:
- `games.red-menace`
- `games.zeta-invaders`
- `tools.nuke-codes`

Bundle format:

- each addon bundle contains `manifest.json`
- shell-hosted WASM addons also contain an `addon.wasm` module
- additional assets/data files live beside those files inside the same addon directory

Repository layout:
- `index.json`
- `games/<slug>/manifest.json`
- `tools/<slug>/manifest.json`

Current artifact contract:
- artifact format is `addon-dir`
- artifact URL points to the addon directory itself
- installer verifies SHA-256
- installed addon is still hosted inside the shell UI rather than launching as a separate OS window

Not included yet:
- packaged binaries
- dynamic plugin loading
- third-party trust model
- signed update workflow

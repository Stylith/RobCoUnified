RobCoOS 0.4.3 highlights since 0.4.2:

- migrate old macOS app-bundle runtime data from `RobCoOS.app/Contents/MacOS` into Application Support instead of leaving installed apps and users behind
- merge legacy `users.json` into the new writable runtime directory so a temporary bootstrap admin does not hide existing accounts
- resolve package manager and runtime tool binaries by absolute path on macOS app launches, so Finder-launched builds can detect Homebrew, Python, and blueutil without depending on shell PATH
- keep the new changelog-backed release workflow in place for future tags

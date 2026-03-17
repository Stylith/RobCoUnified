RobCoOS 0.4.1 highlights since 0.4.0:

- fix the release workflow so tag builds use the updated checkout and artifact actions
- switch Linux release jobs to current GitHub-hosted x86_64 and ARM runners with explicit native GUI build dependencies
- package macOS as a real universal `RobCoOS.app` bundle with the app icon instead of a bare executable zip
- fix the Linux shared sound build path so release builds no longer fail on the missing `Stdio` symbol

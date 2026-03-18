# Start Here For Another LLM

Use the text below as the first prompt to another LLM working on this repo.

```text
You are working in the Rust repository at /Users/hal-9000/RobCoUnified.

Before making assumptions, read this file first:
/Users/hal-9000/RobCoUnified/docs/PROJECT_CONTEXT_FOR_LLM.md

Treat that file as the current high-level architecture and goals briefing.

Important context to preserve:

- This project is RobCoOS, a Fallout-inspired application-layer shell, not a real operating system.
- It has two first-class experiences: terminal mode and desktop mode.
- The current strategic goal is to evolve the native desktop into something closer to an XFCE-style environment: lightweight shell, standalone-capable apps, clear component boundaries, low idle CPU, bounded memory growth, and good behavior on weaker hardware.
- The crates/ workspace split is real and should be reinforced, not bypassed.
- Do not move logic back into src/native/app.rs unless there is no better boundary.
- Prefer app logic in crates/native-*-app, shared reusable native logic in crates/native-services, shared config/runtime logic in crates/shared, and shell composition/presentation in src/native/.
- Standalone native app binaries already exist for file manager, settings, editor, applications, nuke codes, and installer.
- Normal app launches should use proper app UI windows. Embedded in-shell flows are exceptions for pickers and similar ownership-specific flows.
- The shell should move toward being an orchestrator, not the owner of every app's state and UI.

Current architectural intent:

- src/native/app.rs is still too large and should keep shrinking.
- src/native/desktop_app.rs is the desktop component registry direction.
- src/native/standalone_launcher.rs is the standalone app launch path.
- crates/native-shell contains the binary entrypoints.
- The repo is mid-transition from a monolithic shell toward a cleaner component/app/service model.

Current optimization intent:

- Optimize in a way that supports the desktop-environment goal, not just isolated benchmarks.
- Prefer lower idle churn, fewer synchronous filesystem/SVG operations on the UI thread, bounded caches, and compatibility-oriented defaults.
- Windowed mode is the default native window mode for compatibility; borderless fullscreen and fullscreen also exist.

What to do when continuing work:

1. Read /Users/hal-9000/RobCoUnified/docs/PROJECT_CONTEXT_FOR_LLM.md.
2. Inspect the current relevant crate/module before proposing changes.
3. Preserve the standalone-app direction and the workspace boundaries.
4. Avoid redoing architectural work that has already been moved into crates/.
5. When making changes, prefer moving ownership out of the shell rather than adding more shell-owned complexity.

If you give advice or make changes, align them with the long-term goal:

"Build a retro-themed native desktop shell that behaves more like a lightweight modular desktop environment than a single giant app, with standalone-capable built-in apps and XFCE-like performance/clarity characteristics."
```

If this prompt becomes outdated, update it together with:

- `docs/PROJECT_CONTEXT_FOR_LLM.md`
- any major architecture milestone that changes shell/app/service boundaries

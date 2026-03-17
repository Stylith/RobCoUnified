RobCoOS 0.4.0 highlights since 0.3.2:

- promote the native shell to the release target and ship native-only release assets
- switch macOS release packaging to a single universal binary zip
- split the native codebase into shared, service, shell, and app-focused workspace crates
- move editor, file manager, terminal, installer, settings, programs, default apps, connections, edit menus, document browser, about, and nuke codes behind cleaner app boundaries
- centralize native desktop menu, taskbar, session, launcher, file, settings, and status behavior behind shared services
- refresh the README and user manual to match the workspace-native architecture

RobCoOS 0.3.0 highlights since 0.2.1:

- split Logs and ROBCO Word Processor into separate flows with separate storage
- upgraded the editor in terminal and desktop with Save As, search, line numbers, and in-app document opening
- fixed Save As so folder selection and explicit save work correctly in both UI modes
- moved word processor documents into the platform-native Documents folder under a RobCoOS subfolder
- added desktop file manager multiselect and stronger Open With / default-app behavior
- improved navigation consistency across menus, back actions, and settings flows
- hardened the hacking minigame generator and added difficulty/auth-flow integration
- added platform-aware release packaging:
  - Windows `.exe` with embedded icon
  - macOS `.app` bundle in release zips
  - Linux binary + `.desktop` entry + icon in release zips

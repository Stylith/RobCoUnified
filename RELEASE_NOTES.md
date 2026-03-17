RobCoOS 0.4.2 highlights since 0.4.1:

- fix the macOS `.app` runtime data path so bundled builds create and read users/settings from Application Support instead of inside the app bundle
- embed the retro font into the native UI and PTY renderer so standalone app bundles keep the intended typeface without needing repo assets
- stabilize the native release gate by making document-category service tests independent of global user-scoped config state

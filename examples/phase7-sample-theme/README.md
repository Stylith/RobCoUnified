Phase 7 sample theme bundle for icon and cursor theming.

Contents:
- `manifest.json`: addon manifest with `kind: "theme"`
- `theme.json`: `ThemePack` payload consumed by the current loader
- `assets/icons_color/`: full-color icon overrides
- `assets/icons_mono/`: white-channel icon overrides for monochrome mode
- `assets/cursors/cursors.json`: arrow cursor override

Use:
1. Copy this directory into your addon root as a folder addon.
2. Open Tweaks and select the `Signal Forge` theme pack.
3. In the `Cursors` section, leave `Cursor Theme` on `Follow Theme` or set it explicitly to `Signal Forge`.
4. Confirm the desktop uses the custom folder, settings, and app icons.
5. Confirm the software cursor uses the custom arrow sprite.

Current runtime note:
The desktop Tweaks color controls still detach the selected theme pack when you manually
change color mode. Because of that, the easiest way to inspect both `icons_color` and
`icons_mono` right now is to edit the bundle's `color_style` in `theme.json`, then reload
or reselect the theme pack.

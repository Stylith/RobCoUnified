# SVG and Sprite Sheet Guide for Your Own Art

## Lowest-pain workflow

1. Pick one grid size, usually 16x16 or 32x32
2. Make one SVG sheet per category
3. Keep every frame inside a clean grid cell
4. Export each SVG sheet to PNG
5. Write one JSON metadata file per sheet
6. Let Rust load the PNG + JSON
7. Tint frames at draw time using the app theme

## Recommended sheets

- mario_sheet
- barrels_sheet
- level_sheet
- ui_sheet
- effects_sheet

## Important art rules

- monochrome only
- keep feet/baselines aligned between frames
- keep frame sizes consistent
- avoid color-dependent gameplay information
- prefer silhouette clarity over detail

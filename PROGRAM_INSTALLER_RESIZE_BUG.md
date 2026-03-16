# Program Installer Resize Bug Handoff

## Summary

The native desktop `Program Installer` window still has a resize/autogrow bug in the
`SearchResults` view.

Observed behavior:

- moving the mouse around the lower part of the search results area can cause the
  window to expand vertically
- in some revisions it expands and then snaps back, which means the UI is fighting
  between two sizing paths
- in the latest revisions it can still stretch larger than intended even after some
  of the horizontal overflow was fixed
- the bug is not fixed

This is **not** a normal user resize. It is an automatic resize triggered by hover
and repaint/layout feedback.

The user is frustrated because many partial tweaks changed the symptom without fixing
the actual cause.

## Main Files

- `src/native/app.rs`
- `src/native/installer_screen.rs`

Important note:

- The desktop installer UI currently lives in `src/native/app.rs`, not
  `installer_screen.rs`.
- `src/native/installer_screen.rs` mainly contains installer state/backend logic,
  including async search.

## Confirmed Facts

### 1. The bug is specific to installer result views

The bad behavior is in:

- `DesktopInstallerView::SearchResults`
- likely also `DesktopInstallerView::Installed`

The installer home screen does not show the same problem.

### 2. The bug is tied to scroll/layout feedback

This was confirmed by behavior changes when altering the result list scroll area:

- forcing visible scrollbars reduced the bug a lot
- disabling drag-to-scroll helped somewhat
- changing scrollbar style changed severity
- the worst trigger zone is near the lower/middle-lower results area
- at one point the behavior was obviously tied to the bottom edge / resize zone
- even after that was improved, the results view can still enlarge itself

So the scroll area is involved, but it is not the only cause.

### 3. Row/card overflow was real and partially fixed

Diagnostics showed earlier:

- `clip 758`
- `row 752`
- `group 764`
- `content 764x...`

That proved the result card itself was wider than the scroll area clip rect.

Cause:

- `ui.group(...)` adds `6px` inner margins on both sides in `egui`

That was replaced with a tighter custom frame. Later diagnostics showed:

- `clip 961`
- `row 955`
- `group 955`
- `content 955x1289`
- `inner 961x927`

So the horizontal overflow source is no longer the row frame itself.

More recent screenshots show values like:

- `search body 768x519 clip 758 row 752 group 752 content 752x1217`
- `search body 768x927 clip 758 row 752 group 752 content 752x1289`
- `search body 1023x865 clip 1013 row 1007 group 1007 content 1007x1217`

These matter because:

- `group <= row <= clip` is now true
- `content.x <= inner.x` is also effectively under control
- the remaining problem is no longer explained by row/card width overflow

### 4. Installer-specific window clamp logic caused "snap back"

At one point, custom logic in `draw_installer()`:

- stopped saving restore size unless dragging resize edges
- conditionally applied `window.resize(|r| r.max_size(content_size))`

That produced the "expands then snaps back" behavior. It was a symptom of two sizing
systems fighting.

That custom outer-window logic has since mostly been removed so the installer follows
the editor-style desktop window path more closely.

### 5. The issue is still present after row overflow fix

Even after:

- row width matched clip width
- group/frame width matched row width
- content width was no longer larger than inner width

the window can still expand vertically on hover in search results.

That strongly suggests the remaining bug is not a simple width overflow anymore.

### 6. Workspace max-size / constrain did not solve it

Another attempted fix added:

- `.max_size(workspace_rect.size())`
- `.constrain_to(workspace_rect)`

to the installer `egui::Window`.

This did **not** solve the bug. The window could still stretch large inside the
workspace, so this is not an "off-screen growth only" problem.

## Current Relevant Code Paths

### `draw_installer()` in `src/native/app.rs`

This builds the native desktop installer window.

Current important properties:

- uses `egui::Window`
- `title_bar(false)`
- `frame(Self::desktop_window_frame())`
- `resizable(true)`
- `min_size([500.0, 400.0])`
- currently also `max_size(workspace_rect.size())`
- currently also `constrain_to(workspace_rect)`
- `default_pos(...)`
- `default_size(...)`
- header/body/status built using `TopBottomPanel::show_inside(...)` and
  `CentralPanel::show_inside(...)`

The installer currently also writes debug metrics into the status line.

Important current behavior in `draw_installer()`:

- the installer now follows the editor-style outer window path more closely
- previous installer-only restore/clamp fighting logic was removed
- activation uses `response.contains_pointer()`

Short excerpt of current outer window setup:

```rust
let workspace_rect = Self::desktop_workspace_rect(ctx);
let mut window = egui::Window::new("Program Installer")
    .id(Id::new(("native_installer", generation)))
    .open(&mut open)
    .title_bar(false)
    .frame(Self::desktop_window_frame())
    .resizable(true)
    .min_size([500.0, 400.0])
    .max_size(workspace_rect.size())
    .constrain_to(workspace_rect)
    .default_pos(default_pos)
    .default_size([default_size.x, default_size.y]);
```

### `draw_installer_search_results()` in `src/native/app.rs`

Current structure:

- top panel for back button/title
- bottom panel for paging
- `CentralPanel::show_inside(...)`
- inside that, a vertical `ScrollArea`
- rows rendered with `show_rows(...)`
- per-row custom `Frame::none().stroke(...).inner_margin(2.0)`

Important current settings:

- `ScrollArea::vertical()`
- `.auto_shrink([false, false])`
- `.max_height(body_height)`
- `.drag_to_scroll(false)`
- `.scroll_bar_visibility(AlwaysVisible)`
- `ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid()`
- rows are rendered by `show_rows(...)`
- row width is currently derived from `ui.clip_rect().width()`
- result cards no longer use `ui.group(...)`; they use a custom
  `Frame::none().stroke(...).inner_margin(2.0)`
- debug metrics include `clip`, `row`, `group`, `content`, `inner`, `overflow`

Short excerpt of current results list setup:

```rust
egui::CentralPanel::default()
    .frame(egui::Frame::none())
    .show_inside(ui, |ui| {
        let body_height = ui.available_height().max(120.0);
        ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();
        let out = egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(body_height)
            .drag_to_scroll(false)
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            .show_rows(ui, row_height, visible_count, |ui, row_range| {
                let clip_width = ui.clip_rect().width();
                let row_width = (clip_width - 6.0).floor().max(240.0);
                // row rendering...
            });
    });
```

## Important Egui Finding

From `egui 0.29.1`, `Resize` keeps auto-expanding a resizable window to fit the last
frame's content:

```rust
// We are not being actively resized, so auto-expand to include size of last frame.
state.desired_size = state.desired_size.max(state.last_content_size);
```

And later:

```rust
state.desired_size[d] = state.desired_size[d].max(state.last_content_size[d]);
```

This means any content path that reports a larger height can make the window grow even
without explicit user resizing.

That is the core feedback-loop mechanism.

Another relevant `egui` note from `lib.rs`:

- resizable auto-sized windows/panels can behave badly in immediate mode
- recommended mitigations include:
  - turn off resizing
  - use `ScrollArea`
  - use a justified/fill layout

This matches the overall shape of the bug, but the installer already uses a scroll
area, so the remaining issue is more specific than "just add a scroll area".

## Current Diagnostics

The status line currently shows metrics like:

```text
search body 768x519 clip 758 row 752 group 752 content 752x1217 inner 758x519 ...
search body 971x927 clip 961 row 955 group 955 content 955x1289 inner 961x927 ...
search body 1023x865 clip 1013 row 1007 group 1007 content 1007x1217 ...
```

Interpretation:

- row width and group width are now under control
- content height is still much larger than inner height, which is expected for a
  scrolled list
- the remaining problem is likely vertical autosize feedback from the window/resize
  system rather than horizontal row overflow
- when the window grows, `content.y` remains the full list height and `body/inner`
  rise to meet it, which is consistent with the outer window accepting larger content
  size from the results view

### Latest Confirmed Screenshot Metrics

These are the most important concrete values from the latest screenshots:

Normal/less-expanded state:

```text
search body 768x519
clip 758
row 752
group 752
content 752x1217
```

Expanded state:

```text
search body 768x927
clip 758
row 752
group 752
content 752x1289
```

Later larger state after more changes:

```text
search body 1023x865
clip 1013
row 1007
group 1007
content 1007x1217
```

What these values prove:

- `group` is no longer wider than `row`
- `row` is no longer wider than `clip`
- the earlier horizontal overflow bug from `ui.group(...)` was real and has been fixed
- the remaining bug is primarily vertical
- in the bad state, the visible body/inner height increases while content height stays
  large and scrollable
- therefore the outer window is still accepting a larger content-driven height during
  hover/repaint

### Behavior Sequence Seen By User

The user consistently reports this sequence:

1. Open Program Installer in desktop mode.
2. Search for a package term and enter `SearchResults`.
3. Move or wiggle the mouse near the lower or middle-lower results area.
4. The window grows vertically on its own.
5. In some revisions it keeps stretching.
6. In some revisions it stretches and snaps back, which means the window is fighting
   between two size sources.

Important:

- this is not caused by explicit mouse dragging
- this is not a first-open-only bug
- this is specific to the results views, not the installer home screen
- it is reproducible on hover alone

## Current Exact State In `src/native/app.rs`

Another model should assume the following is already true in the working tree unless it
has changed it itself:

### Outer installer window

- `egui::Window::new("Program Installer")`
- `.title_bar(false)`
- `.frame(Self::desktop_window_frame())`
- `.resizable(true)`
- `.min_size([500.0, 400.0])`
- `.max_size(workspace_rect.size())`
- `.constrain_to(workspace_rect)`
- `.default_pos(default_pos)`
- `.default_size([default_size.x, default_size.y])`
- if maximized:
  - `.movable(false)`
  - `.resizable(false)`
  - `.fixed_pos(workspace_rect.min)`
  - `.fixed_size(workspace_rect.size())`
- restore behavior uses:
  - `let restore = self.take_desktop_window_restore_dims(DesktopWindow::Installer);`
  - `live_restore` from `desktop_window_state`

### Outer installer content structure

Inside `window.show(ctx, |ui| { ... })`, the installer currently uses:

- `TopBottomPanel::top(...)` for the header
- `TopBottomPanel::bottom(...)` for the status line
- optional `TopBottomPanel::bottom(...)` for confirm dialog
- optional `egui::Area` overlay for success/failure notice
- `CentralPanel::default().show_inside(...)` for the main per-view content

### Search results view

Current structure of `draw_installer_search_results(...)`:

- `TopBottomPanel::top("inst_search_top")`
- optional `TopBottomPanel::bottom("inst_search_bottom")`
- `CentralPanel::default().show_inside(...)`
- inside central panel:
  - `let body_height = ui.available_height().max(120.0);`
  - `ui.style_mut().spacing.scroll = egui::style::ScrollStyle::solid();`
  - `egui::ScrollArea::vertical()`
    - `.auto_shrink([false, false])`
    - `.max_height(body_height)`
    - `.drag_to_scroll(false)`
    - `.scroll_bar_visibility(AlwaysVisible)`
    - `.show_rows(ui, row_height, visible_count, |ui, row_range| { ... })`

### Row rendering details

Inside each `show_rows` callback:

- `let clip_width = ui.clip_rect().width();`
- `let row_width = (clip_width - 6.0).floor().max(240.0);`
- each row allocates:
  - `ui.allocate_space(egui::vec2(row_width, row_height - 4.0))`
- each row is rendered inside:
  - `ui.scope_builder(UiBuilder::new().max_rect(row_rect), |ui| { ... })`
- each card uses:
  - `egui::Frame::none()`
  - `.stroke(egui::Stroke::new(1.0, palette.fg))`
  - `.inner_margin(egui::Margin::same(2.0))`
- it no longer uses `ui.group(...)`
- package description preview is truncated and single-line in this list view

### Current debug text contents

The installer status line currently gets overwritten with metrics roughly in this form:

```text
search body {body_w}x{body_h} clip {clip_w} row {row_w} group {group_w}
content {content_w}x{content_h} inner {inner_w}x{inner_h}
overflow {overflow_w}x{overflow_h} offset {x},{y} | win ... restore ... ptr ... edge_hover ... edge_drag ...
```

If another model changes diagnostics, it should preserve at least:

- body
- clip
- row
- group
- content
- inner
- overflow
- actual window rect
- pointer position

because those are what already ruled out several earlier wrong hypotheses.

## What Clearly Did Not Work

These are important because another model should not repeat them as if they are new:

- only tweaking scrollbar visibility
- only tweaking scrollbar style
- only setting `drag_to_scroll(false)`
- only clamping row width
- only replacing `ui.group`
- only using `show_rows(...)`
- only adding `.max_height(...)` to the `ScrollArea`
- only adding workspace `max_size` / `constrain_to`
- installer-specific restore-size guards that reapply old rects
- installer-specific `window.resize(|r| r.max_size(...))` logic
- switching pointer detection to `contains_pointer()`
- merely changing edge-hover checks to edge-drag checks
- merely copying generic "ScrollArea + auto_shrink + max_height" advice from another LLM
- merely matching the editor outer window path without addressing the results layout

## Things Already Tried

These were tried and did not fully fix it:

- multiple outer installer window layout rewrites
- row width clamps
- row fixed-height rendering
- `show_rows(...)`
- max-size clamps on nested `Ui`s
- installer-specific restore-size guards
- installer-specific max-size clamp on hover
- float/solid/always-visible scrollbar changes
- drag-to-scroll off
- replacing `ui.group` with a custom frame
- switching activation logic to `response.contains_pointer()`
- reusing saved rects
- removing installer-specific outer rect clamp logic again
- later re-adding workspace-level `max_size` / `constrain_to`
- comparing against editor/file-manager outer window path

Some of these reduced the bug or changed its character, but none fully solved it.

## Most Likely Remaining Cause

The most likely remaining cause is:

- the installer results view is still participating in `egui::Window` content sizing,
  so a repaint/hover near the lower edge changes the measured content height enough to
  let `Resize` grow the window

More concretely:

- the outer `egui::Window` still accepts larger `last_content_size.y` from the results
  view
- the results view still reports enough vertical size during hover/repaint for the
  window `Resize` state to increase `desired_size`
- once larger, the window can remain larger or fight/snap depending on whatever guard
  logic is active in that revision

Potentially important suspects:

- `TopBottomPanel::show_inside(...)` + `CentralPanel::show_inside(...)` interaction
  inside a resizable `egui::Window`
- `show_rows(...)` still reporting a large content height, which may be fine for
  scrolling but may still feed into window measurement in this layout
- the lower edge hover may overlap the window resize interaction region, making
  `egui::Window` think resize affordance is active even when the user is not dragging
- the results `CentralPanel` / paging panel composition may still be contributing a
  large `min_size()` to the window even though the visible viewport is smaller

## Stronger Hypotheses Worth Testing

These are the most promising next steps, not more small knob changes:

1. Move the results list out of `TopBottomPanel::show_inside(...)` / `CentralPanel::show_inside(...)`
   composition and render it in a strictly allocated child `Ui` with an explicit fixed
   rectangle for the frame.

2. Stop using `show_rows(...)` for this window and manually virtualize visible rows in
   a viewport whose outer height is explicitly fixed, so the child content no longer
   reports the full list height through the same layout path.

3. Temporarily disable vertical resizing for the installer window only
   (`resizable([true, false])` or equivalent) just to prove whether `egui::Resize`
   is the remaining mechanism. This is a diagnostic step, not necessarily the final UX.

4. Add diagnostics for the outer window `Resize` state if possible:
   - `desired_size`
   - `last_content_size`
   - actual window rect

5. Check whether a widget near the bottom of results is still interacting with the
   bottom resize affordance of the window, even after previous edge-hover logic was
   removed.

6. Read the `egui` `Resize` implementation directly and trace which child `min_size()`
   path is still feeding the larger `last_content_size.y` into the installer window.
   This is likely more valuable than additional UI-level guessing.

## Important Correction For Other LLMs

Another model may suggest a generic immediate-mode fix:

- "wrap the results in a `ScrollArea`"
- "set `.auto_shrink([false; 2])`"
- "add a window `max_height`"

Those suggestions are **not enough** for this repository, because the installer
already does all of the following in `src/native/app.rs`:

- uses a vertical `ScrollArea` for the results list
- uses `.auto_shrink([false, false])`
- uses `.max_height(body_height)`
- uses always-visible scrollbars
- disables drag-to-scroll
- constrains the outer installer window to the workspace and applies a workspace max
  size

So if another LLM starts by proposing those changes as if they are missing, it is
working from the wrong assumption.

The remaining bug is deeper than "no scroll area" or "window can grow off-screen".
It is specifically about how the results view still participates in `egui::Window`
content sizing during hover/repaint.

## Explicitly Rejected Proposal

The following proposal has already been suggested and is **not sufficient** for this
repository:

- add `egui::ScrollArea::vertical().max_height(...).auto_shrink([false; 2])`
  around the results list
- add `Window::max_height(...)` to the installer window

Why it is rejected:

- the results list already uses a vertical `ScrollArea`
- it already uses `.auto_shrink([false, false])`
- it already uses a bounded height (`.max_height(body_height)`)
- the outer installer window already has workspace-level max-size/constrain logic

Therefore, another model should not spend time reapplying that patch as if it is new.
The remaining issue is after those mitigations are already in place.

## Suggested Next Steps

These are the best next steps for another model or engineer:

1. Remove the current status-line diagnostics and replace them with a temporary overlay
   panel that prints:
   - window rect
   - window `Resize` desired size if accessible
   - central panel rect
   - scroll area outer rect
   - scroll area inner rect
   - content size
   - pointer position
   - whether the pointer is inside the window resize hit zone

2. Compare `draw_installer()` against the editor/file-manager desktop windows and
   identify exactly which layout pattern prevents their content from feeding back into
   autosize.

3. Strong candidate fix:
   render the results list into a strictly allocated child region whose height is
   explicitly fixed from the parent window rect for that frame, and ensure that child
   region does not contribute a larger `min_size()` back to the window.

4. If needed, avoid `show_rows(...)` and manually virtualize the visible rows in a
   fixed-height viewport, so the child content no longer reports full-list height to
   the window layout system.

5. Check whether the bottom window resize interaction zone overlaps the results area in
   a way that allows hover-only edge interaction to influence the window.

6. If another model wants a quick binary test:
   - temporarily make the installer window non-vertically-resizable
   - if the bug disappears completely, the remaining issue is definitely the
     `egui::Resize` autosize path rather than just the internal scroll area

7. If another model wants the cleanest high-signal experiment:
   - remove `show_rows(...)` entirely for this view
   - create a fixed-height viewport `Ui`
   - manually compute visible rows from scroll offset
   - render only those rows in that viewport
   - verify whether the bug disappears when the list no longer reports full content
     height through `show_rows`

## Current User-Supplied Assessment

The user explicitly believes:

- the bug is still not fixed
- many prior changes were guesses rather than diagnosis
- the text editor had a window bug fixed before and its structural solution should be
  used as a comparison point

This is worth respecting: another model should start from the diagnostics and the
editor/file-manager patterns, not from generic "add a ScrollArea" advice.

## Exact Ask For Another LLM

If another LLM is given this file, it should focus on:

1. Explaining exactly which current child layout is still feeding too much height into
   the installer window.
2. Producing a fix that is specific to the current `src/native/app.rs` structure, not a
   generic immediate-mode GUI recipe.
3. Avoiding proposals that are already present in code:
   - `ScrollArea`
   - `auto_shrink([false, false])`
   - bounded list height
   - workspace-constrained outer window
4. Using the latest metrics to reason from facts, not assumptions.

## Current State of Working Tree

There are active local changes related to this debugging in:

- `src/native/app.rs`
- `src/native/installer_screen.rs`

No commit was made for this debugging tail yet.

//! Custom iced widget that implements the inner window manager.
//!
//! `DesktopWindowHost` renders desktop windows with title-bar chrome (title,
//! close/min/max buttons, borders) and handles drag, resize, focus, and
//! event forwarding to child content widgets.

#![allow(dead_code)]

use super::message::{Message, WindowHeaderButton};
use super::retro_theme::current_retro_colors;
use super::shared_types::DesktopWindow;
use super::shell::{WindowLifecycle, WindowRect};
use iced::advanced::layout::{self, Layout};
use iced::advanced::renderer::{self, Quad, Renderer as _};
use iced::advanced::text::Renderer as _;
use iced::advanced::widget::{self, Tree};
use iced::advanced::{Clipboard, Shell, Widget};
use iced::event::{self, Event};
use iced::mouse;
use iced::{Border, Element, Length, Point, Rectangle, Size};

// ── Constants ────────────────────────────────────────────────────────────────

const TITLE_BAR_HEIGHT: f32 = 28.0;
const BORDER_WIDTH: f32 = 2.0;
const BUTTON_WIDTH: f32 = 28.0;
const RESIZE_HANDLE: f32 = 8.0;

// ── Hit testing ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum HitZone {
    TitleBar,
    CloseButton,
    MinimizeButton,
    MaximizeButton,
    Content,
    ResizeRight,
    ResizeBottom,
    ResizeCorner,
}

#[derive(Debug, Clone, Copy)]
struct HitInfo {
    window_idx: usize,
    zone: HitZone,
}

// ── Interaction state (stored in Tree::State) ────────────────────────────────

#[derive(Debug, Clone)]
struct DragState {
    window_id: DesktopWindow,
    /// Cursor position when the drag started.
    start_cursor: Point,
    /// Window rect at drag start.
    origin_rect: WindowRect,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResizeEdge {
    Right,
    Bottom,
    Corner,
}

#[derive(Debug, Clone)]
struct ResizeState {
    window_id: DesktopWindow,
    edge: ResizeEdge,
    start_cursor: Point,
    origin_rect: WindowRect,
}

#[derive(Debug, Default)]
struct WmState {
    drag: Option<DragState>,
    resize: Option<ResizeState>,
}

// ── WindowChild ──────────────────────────────────────────────────────────────

/// One window to be rendered by the host widget.
pub struct WindowChild<'a> {
    pub id: DesktopWindow,
    pub rect: WindowRect,
    pub title: String,
    pub lifecycle: WindowLifecycle,
    pub is_active: bool,
    pub resizable: bool,
    /// The iced Element rendered inside this window's content area.
    pub content: Element<'a, Message>,
}

// ── DesktopWindowHost ────────────────────────────────────────────────────────

/// The custom widget that renders and manages inner desktop windows.
///
/// Children are given in **front-to-back** z-order: `children[0]` is the
/// topmost window. Drawing goes back-to-front; hit-testing goes front-to-back.
pub struct DesktopWindowHost<'a> {
    children: Vec<WindowChild<'a>>,
}

impl<'a> DesktopWindowHost<'a> {
    pub fn new(children: Vec<WindowChild<'a>>) -> Self {
        Self { children }
    }
}

/// Convert to an `Element`.
impl<'a> From<DesktopWindowHost<'a>> for Element<'a, Message> {
    fn from(host: DesktopWindowHost<'a>) -> Self {
        Element::new(host)
    }
}

// ── Widget impl ──────────────────────────────────────────────────────────────

impl<'a> Widget<Message, iced::Theme, iced::Renderer> for DesktopWindowHost<'a> {
    fn tag(&self) -> widget::tree::Tag {
        widget::tree::Tag::of::<WmState>()
    }

    fn state(&self) -> widget::tree::State {
        widget::tree::State::new(WmState::default())
    }

    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(|c| Tree::new(&c.content)).collect()
    }

    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(
            &self.children.iter().map(|c| &c.content).collect::<Vec<_>>(),
        );
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(&self, tree: &mut Tree, renderer: &iced::Renderer, limits: &layout::Limits) -> layout::Node {
        let size = limits.max();

        // Each child gets a layout node at its absolute position/size.
        let child_nodes: Vec<layout::Node> = self.children.iter().enumerate().map(|(i, child)| {
            let content_w = child.rect.w - 2.0 * BORDER_WIDTH;
            let content_h = child.rect.h - TITLE_BAR_HEIGHT - BORDER_WIDTH;

            let content_limits = layout::Limits::new(
                Size::ZERO,
                Size::new(content_w.max(0.0), content_h.max(0.0)),
            );

            let mut content_node = self.children[i]
                .content
                .as_widget()
                .layout(&mut tree.children[i], renderer, &content_limits);

            // Position content inside the window frame.
            content_node = content_node.move_to(Point::new(
                child.rect.x + BORDER_WIDTH,
                child.rect.y + TITLE_BAR_HEIGHT,
            ));

            content_node
        }).collect();

        layout::Node::with_children(size, child_nodes)
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        let palette = current_retro_colors();
        let fg = palette.fg.to_iced();
        let dim = palette.dim.to_iced();
        let bg = palette.bg.to_iced();
        let panel_bg = palette.panel.to_iced();
        let active_bg = palette.active_bg.to_iced();
        let selected_bg = palette.selected_bg.to_iced();
        let selected_fg = palette.selected_fg.to_iced();

        let child_layouts: Vec<Layout<'_>> = layout.children().collect();

        // Draw back-to-front (last child in Vec = bottom window, drawn first).
        for i in (0..self.children.len()).rev() {
            let child = &self.children[i];
            if child.lifecycle == WindowLifecycle::Minimized {
                continue;
            }

            let r = child.rect;
            let title_bar_color = if child.is_active { active_bg } else { panel_bg };
            let border_color = if child.is_active { fg } else { dim };

            // ── Window border ───────────────────────────────────────
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(
                        Point::new(r.x, r.y),
                        Size::new(r.w, r.h),
                    ),
                    border: Border {
                        color: border_color,
                        width: BORDER_WIDTH,
                        radius: 0.0.into(),
                    },
                    ..Quad::default()
                },
                bg,
            );

            // ── Title bar background ────────────────────────────────
            renderer.fill_quad(
                Quad {
                    bounds: Rectangle::new(
                        Point::new(r.x + BORDER_WIDTH, r.y + BORDER_WIDTH),
                        Size::new(
                            r.w - 2.0 * BORDER_WIDTH,
                            TITLE_BAR_HEIGHT - BORDER_WIDTH,
                        ),
                    ),
                    border: Border::default(),
                    ..Quad::default()
                },
                title_bar_color,
            );

            // ── Title text ──────────────────────────────────────────
            let text_bounds = Rectangle::new(
                Point::new(r.x + BORDER_WIDTH + 8.0, r.y + BORDER_WIDTH),
                Size::new(
                    r.w - 2.0 * BORDER_WIDTH - 3.0 * BUTTON_WIDTH - 16.0,
                    TITLE_BAR_HEIGHT - BORDER_WIDTH,
                ),
            );
            renderer.fill_text(
                iced::advanced::Text {
                    content: child.title.clone(),
                    bounds: Size::new(text_bounds.width, text_bounds.height),
                    size: 14.0.into(),
                    line_height: iced::widget::text::LineHeight::default(),
                    font: iced::Font::MONOSPACE,
                    horizontal_alignment: iced::alignment::Horizontal::Left,
                    vertical_alignment: iced::alignment::Vertical::Center,
                    shaping: iced::widget::text::Shaping::Basic,
                    wrapping: iced::widget::text::Wrapping::None,
                },
                Point::new(text_bounds.x, text_bounds.y),
                fg,
                text_bounds,
            );

            // ── Chrome buttons (minimize, maximize, close) ──────────
            let btn_y = r.y + BORDER_WIDTH;
            let btn_h = TITLE_BAR_HEIGHT - BORDER_WIDTH;
            let close_x = r.x + r.w - BORDER_WIDTH - BUTTON_WIDTH;
            let max_x = close_x - BUTTON_WIDTH;
            let min_x = max_x - BUTTON_WIDTH;

            // Check hover state for button highlights
            let cursor_pos = cursor.position();

            for (bx, label) in [(min_x, "_"), (max_x, "+"), (close_x, "X")] {
                let btn_rect = Rectangle::new(
                    Point::new(bx, btn_y),
                    Size::new(BUTTON_WIDTH, btn_h),
                );

                let hovered = cursor_pos.map_or(false, |p| btn_rect.contains(p));

                if hovered {
                    renderer.fill_quad(
                        Quad {
                            bounds: btn_rect,
                            border: Border::default(),
                            ..Quad::default()
                        },
                        selected_bg,
                    );
                }

                let label_color = if hovered { selected_fg } else { fg };
                renderer.fill_text(
                    iced::advanced::Text {
                        content: label.to_string(),
                        bounds: Size::new(BUTTON_WIDTH, btn_h),
                        size: 14.0.into(),
                        line_height: iced::widget::text::LineHeight::default(),
                        font: iced::Font::MONOSPACE,
                        horizontal_alignment: iced::alignment::Horizontal::Center,
                        vertical_alignment: iced::alignment::Vertical::Center,
                        shaping: iced::widget::text::Shaping::Basic,
                        wrapping: iced::widget::text::Wrapping::None,
                    },
                    Point::new(bx, btn_y),
                    label_color,
                    btn_rect,
                );
            }

            // ── Content area ────────────────────────────────────────
            if i < child_layouts.len() {
                let content_bounds = Rectangle::new(
                    Point::new(r.x + BORDER_WIDTH, r.y + TITLE_BAR_HEIGHT),
                    Size::new(
                        r.w - 2.0 * BORDER_WIDTH,
                        r.h - TITLE_BAR_HEIGHT - BORDER_WIDTH,
                    ),
                );

                renderer.with_layer(content_bounds, |renderer| {
                    child.content.as_widget().draw(
                        &tree.children[i],
                        renderer,
                        theme,
                        style,
                        child_layouts[i],
                        cursor,
                        &content_bounds,
                    );
                });
            }
        }
    }

    fn on_event(
        &mut self,
        tree: &mut Tree,
        event: Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Message>,
        _viewport: &Rectangle,
    ) -> event::Status {
        let state = tree.state.downcast_mut::<WmState>();

        match &event {
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                // Finish drag.
                if let Some(drag) = state.drag.take() {
                    if let Some(pos) = cursor.position() {
                        let dx = pos.x - drag.start_cursor.x;
                        let dy = pos.y - drag.start_cursor.y;
                        let new_x = drag.origin_rect.x + dx;
                        let new_y = drag.origin_rect.y + dy;
                        shell.publish(Message::WindowMoved {
                            window: drag.window_id,
                            x: new_x,
                            y: new_y,
                        });
                    }
                    return event::Status::Captured;
                }
                // Finish resize.
                if let Some(resize) = state.resize.take() {
                    if let Some(pos) = cursor.position() {
                        let dx = pos.x - resize.start_cursor.x;
                        let dy = pos.y - resize.start_cursor.y;
                        let (min_w, min_h) = (200.0_f32, 150.0_f32);
                        let new_w = match resize.edge {
                            ResizeEdge::Right | ResizeEdge::Corner => {
                                (resize.origin_rect.w + dx).max(min_w)
                            }
                            _ => resize.origin_rect.w,
                        };
                        let new_h = match resize.edge {
                            ResizeEdge::Bottom | ResizeEdge::Corner => {
                                (resize.origin_rect.h + dy).max(min_h)
                            }
                            _ => resize.origin_rect.h,
                        };
                        shell.publish(Message::WindowResized {
                            window: resize.window_id,
                            w: new_w,
                            h: new_h,
                        });
                    }
                    return event::Status::Captured;
                }
            }

            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                // Live drag: publish intermediate positions.
                if let Some(drag) = &state.drag {
                    if let Some(pos) = cursor.position() {
                        let dx = pos.x - drag.start_cursor.x;
                        let dy = pos.y - drag.start_cursor.y;
                        shell.publish(Message::WindowMoved {
                            window: drag.window_id,
                            x: drag.origin_rect.x + dx,
                            y: drag.origin_rect.y + dy,
                        });
                    }
                    return event::Status::Captured;
                }
                // Live resize: publish intermediate sizes.
                if let Some(resize) = &state.resize {
                    if let Some(pos) = cursor.position() {
                        let dx = pos.x - resize.start_cursor.x;
                        let dy = pos.y - resize.start_cursor.y;
                        let (min_w, min_h) = (200.0_f32, 150.0_f32);
                        let new_w = match resize.edge {
                            ResizeEdge::Right | ResizeEdge::Corner => {
                                (resize.origin_rect.w + dx).max(min_w)
                            }
                            _ => resize.origin_rect.w,
                        };
                        let new_h = match resize.edge {
                            ResizeEdge::Bottom | ResizeEdge::Corner => {
                                (resize.origin_rect.h + dy).max(min_h)
                            }
                            _ => resize.origin_rect.h,
                        };
                        shell.publish(Message::WindowResized {
                            window: resize.window_id,
                            w: new_w,
                            h: new_h,
                        });
                    }
                    return event::Status::Captured;
                }
            }

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor.position() {
                    if let Some(hit) = self.hit_test(pos) {
                        let child = &self.children[hit.window_idx];

                        // Focus the window (bring to front).
                        shell.publish(Message::FocusWindow(child.id));

                        match hit.zone {
                            HitZone::CloseButton => {
                                shell.publish(Message::WindowHeaderButtonClicked {
                                    window: child.id,
                                    button: WindowHeaderButton::Close,
                                });
                                return event::Status::Captured;
                            }
                            HitZone::MinimizeButton => {
                                shell.publish(Message::WindowHeaderButtonClicked {
                                    window: child.id,
                                    button: WindowHeaderButton::Minimize,
                                });
                                return event::Status::Captured;
                            }
                            HitZone::MaximizeButton => {
                                let btn = if child.lifecycle == WindowLifecycle::Maximized {
                                    WindowHeaderButton::Restore
                                } else {
                                    WindowHeaderButton::Maximize
                                };
                                shell.publish(Message::WindowHeaderButtonClicked {
                                    window: child.id,
                                    button: btn,
                                });
                                return event::Status::Captured;
                            }
                            HitZone::TitleBar => {
                                state.drag = Some(DragState {
                                    window_id: child.id,
                                    start_cursor: pos,
                                    origin_rect: child.rect,
                                });
                                return event::Status::Captured;
                            }
                            HitZone::ResizeRight => {
                                state.resize = Some(ResizeState {
                                    window_id: child.id,
                                    edge: ResizeEdge::Right,
                                    start_cursor: pos,
                                    origin_rect: child.rect,
                                });
                                return event::Status::Captured;
                            }
                            HitZone::ResizeBottom => {
                                state.resize = Some(ResizeState {
                                    window_id: child.id,
                                    edge: ResizeEdge::Bottom,
                                    start_cursor: pos,
                                    origin_rect: child.rect,
                                });
                                return event::Status::Captured;
                            }
                            HitZone::ResizeCorner => {
                                state.resize = Some(ResizeState {
                                    window_id: child.id,
                                    edge: ResizeEdge::Corner,
                                    start_cursor: pos,
                                    origin_rect: child.rect,
                                });
                                return event::Status::Captured;
                            }
                            HitZone::Content => {
                                // Fall through to forward to child widget.
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        // Forward events to child widgets (front-to-back).
        let child_layouts: Vec<Layout<'_>> = layout.children().collect();
        for (i, child) in self.children.iter_mut().enumerate() {
            if child.lifecycle == WindowLifecycle::Minimized {
                continue;
            }
            if i >= child_layouts.len() {
                continue;
            }

            let content_bounds = Rectangle::new(
                Point::new(child.rect.x + BORDER_WIDTH, child.rect.y + TITLE_BAR_HEIGHT),
                Size::new(
                    child.rect.w - 2.0 * BORDER_WIDTH,
                    child.rect.h - TITLE_BAR_HEIGHT - BORDER_WIDTH,
                ),
            );

            // Only forward if cursor is within content bounds.
            if let Some(pos) = cursor.position() {
                if content_bounds.contains(pos) {
                    let status = child.content.as_widget_mut().on_event(
                        &mut tree.children[i],
                        event.clone(),
                        child_layouts[i],
                        cursor,
                        renderer,
                        clipboard,
                        shell,
                        &content_bounds,
                    );
                    if status == event::Status::Captured {
                        return status;
                    }
                }
            }
        }

        event::Status::Ignored
    }

    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let state = tree.state.downcast_ref::<WmState>();

        if state.drag.is_some() {
            return mouse::Interaction::Grabbing;
        }
        if let Some(resize) = &state.resize {
            return match resize.edge {
                ResizeEdge::Right => mouse::Interaction::ResizingHorizontally,
                ResizeEdge::Bottom => mouse::Interaction::ResizingVertically,
                ResizeEdge::Corner => mouse::Interaction::ResizingDiagonallyDown,
            };
        }

        if let Some(pos) = cursor.position() {
            if let Some(hit) = self.hit_test(pos) {
                return match hit.zone {
                    HitZone::TitleBar => mouse::Interaction::Grab,
                    HitZone::CloseButton | HitZone::MinimizeButton | HitZone::MaximizeButton => {
                        mouse::Interaction::Pointer
                    }
                    HitZone::ResizeRight => mouse::Interaction::ResizingHorizontally,
                    HitZone::ResizeBottom => mouse::Interaction::ResizingVertically,
                    HitZone::ResizeCorner => mouse::Interaction::ResizingDiagonallyDown,
                    HitZone::Content => {
                        // Delegate to child widget.
                        let child_layouts: Vec<Layout<'_>> = layout.children().collect();
                        let idx = hit.window_idx;
                        if idx < child_layouts.len() {
                            self.children[idx].content.as_widget().mouse_interaction(
                                &tree.children[idx],
                                child_layouts[idx],
                                cursor,
                                viewport,
                                renderer,
                            )
                        } else {
                            mouse::Interaction::None
                        }
                    }
                };
            }
        }

        mouse::Interaction::None
    }
}

// ── Hit testing helpers ──────────────────────────────────────────────────────

impl<'a> DesktopWindowHost<'a> {
    /// Front-to-back hit test. Returns the first (topmost) window hit.
    fn hit_test(&self, pos: Point) -> Option<HitInfo> {
        for (idx, child) in self.children.iter().enumerate() {
            if child.lifecycle == WindowLifecycle::Minimized {
                continue;
            }
            let r = child.rect;
            let window_rect = Rectangle::new(
                Point::new(r.x, r.y),
                Size::new(r.w, r.h),
            );
            if !window_rect.contains(pos) {
                continue;
            }

            // Check resize handles first (only if resizable and not maximized).
            if child.resizable && child.lifecycle != WindowLifecycle::Maximized {
                let at_right = pos.x >= r.x + r.w - RESIZE_HANDLE;
                let at_bottom = pos.y >= r.y + r.h - RESIZE_HANDLE;
                if at_right && at_bottom {
                    return Some(HitInfo { window_idx: idx, zone: HitZone::ResizeCorner });
                }
                if at_right && pos.y > r.y + TITLE_BAR_HEIGHT {
                    return Some(HitInfo { window_idx: idx, zone: HitZone::ResizeRight });
                }
                if at_bottom {
                    return Some(HitInfo { window_idx: idx, zone: HitZone::ResizeBottom });
                }
            }

            // Title bar zone.
            if pos.y < r.y + TITLE_BAR_HEIGHT {
                // Check chrome buttons (right-aligned).
                let close_x = r.x + r.w - BORDER_WIDTH - BUTTON_WIDTH;
                let max_x = close_x - BUTTON_WIDTH;
                let min_x = max_x - BUTTON_WIDTH;

                if pos.x >= close_x && pos.x < r.x + r.w - BORDER_WIDTH {
                    return Some(HitInfo { window_idx: idx, zone: HitZone::CloseButton });
                }
                if pos.x >= max_x && pos.x < close_x {
                    return Some(HitInfo { window_idx: idx, zone: HitZone::MaximizeButton });
                }
                if pos.x >= min_x && pos.x < max_x {
                    return Some(HitInfo { window_idx: idx, zone: HitZone::MinimizeButton });
                }

                return Some(HitInfo { window_idx: idx, zone: HitZone::TitleBar });
            }

            // Content zone.
            return Some(HitInfo { window_idx: idx, zone: HitZone::Content });
        }
        None
    }
}

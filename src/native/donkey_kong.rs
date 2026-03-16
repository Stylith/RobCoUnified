use eframe::egui::{
    self, Align2, Color32, Context, FontFamily, FontId, Key, Pos2, Rect, Stroke, TextureHandle,
    Vec2,
};
use serde::Deserialize;
use std::collections::HashMap;

pub const BUILTIN_DONKEY_KONG_GAME: &str = "Donkey Kong";

const WORLD_W: f32 = 224.0;
const WORLD_H: f32 = 256.0;
const PLAYER_W: f32 = 14.0;
const PLAYER_H: f32 = 14.0;
const BARREL_SIZE: f32 = 14.0;
const MOVE_SPEED: f32 = 68.0;
const CLIMB_SPEED: f32 = 56.0;
const JUMP_SPEED: f32 = 145.0;
const GRAVITY: f32 = 360.0;
const BARREL_SPEED: f32 = 44.0;
const BARREL_DROP_SPEED: f32 = 84.0;
const BARREL_SPAWN_SECS: f32 = 2.9;

#[derive(Clone)]
pub struct DonkeyKongTheme {
    pub primary: Color32,
    pub enemy: Color32,
    pub ui: Color32,
    pub neutral: Color32,
}

#[derive(Clone)]
pub struct DonkeyKongConfig {
    pub scale: f32,
    pub theme: DonkeyKongTheme,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AtlasId {
    Mario,
    Barrels,
    Level,
    Ui,
    Effects,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum FrameId {
    MarioIdle,
    MarioRun1,
    MarioRun2,
    MarioJump,
    MarioClimb1,
    MarioClimb2,
    BarrelRoll1,
    BarrelRoll2,
    BarrelBroken,
    Fire,
    Girder,
    Ladder,
    PlatformEdge,
    Goal,
    Heart,
    ScoreIcon,
    LifeIcon,
    Spark,
    Explosion1,
    Explosion2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AnimationId {
    MarioIdle,
    MarioRun,
    MarioJump,
    MarioClimb,
    BarrelRoll,
    FireIdle,
    Explosion,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
enum TintRole {
    Primary,
    Enemy,
    Ui,
    Neutral,
}

#[allow(dead_code)]
#[derive(Clone)]
struct Frame {
    atlas: AtlasId,
    uv: Rect,
    tint_role: TintRole,
}

#[derive(Clone)]
struct Animation {
    frames: Vec<FrameId>,
    tick: u32,
}

#[allow(dead_code)]
#[derive(Clone)]
struct AtlasTexture {
    texture: TextureHandle,
}

#[allow(dead_code)]
#[derive(Clone, Default)]
struct SpriteCatalog {
    textures: HashMap<AtlasId, AtlasTexture>,
    frames: HashMap<FrameId, Frame>,
    animations: HashMap<AnimationId, Animation>,
}

#[derive(Clone, Copy)]
struct Platform {
    x1: f32,
    x2: f32,
    y: f32,
    next_drop_x: f32,
}

#[derive(Clone, Copy)]
struct Ladder {
    x: f32,
    y_top: f32,
    y_bottom: f32,
}

#[derive(Clone, Copy)]
struct Player {
    pos: Vec2,
    vel: Vec2,
    facing: f32,
    climbing: bool,
    on_ground: bool,
}

#[derive(Clone, Copy)]
struct Barrel {
    pos: Vec2,
    vel: Vec2,
    platform_idx: usize,
    falling: bool,
}

#[derive(Clone)]
struct GameState {
    player: Player,
    barrels: Vec<Barrel>,
    score: u32,
    lives: u8,
    won: bool,
    game_over: bool,
    animation_ticks: f32,
    score_timer: f32,
    spawn_timer: f32,
}

#[derive(Clone, Copy, Default)]
pub struct GameInput {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump: bool,
}

#[derive(Deserialize)]
struct SheetMeta {
    frames: HashMap<String, MetaFrame>,
    animations: HashMap<String, MetaAnimation>,
}

#[derive(Deserialize)]
struct MetaFrame {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    tint_role: String,
}

#[derive(Deserialize)]
struct MetaAnimation {
    frames: Vec<String>,
    tick: u32,
}

#[derive(Clone)]
pub struct DonkeyKongGame {
    config: DonkeyKongConfig,
    theme: DonkeyKongTheme,
    catalog: SpriteCatalog,
    state: GameState,
}

impl DonkeyKongGame {
    pub fn new(ctx: &Context, config: DonkeyKongConfig) -> Self {
        let catalog = SpriteCatalog::load(ctx);
        Self {
            config: config.clone(),
            theme: config.theme,
            catalog,
            state: initial_state(),
        }
    }

    pub fn reset(&mut self) {
        self.state = initial_state();
    }

    pub fn set_theme(&mut self, theme: DonkeyKongTheme) {
        self.theme = theme;
    }

    pub fn update(&mut self, input: GameInput, dt: f32) {
        let dt = dt.clamp(1.0 / 240.0, 1.0 / 20.0);
        self.state.animation_ticks += dt * 60.0;

        if self.state.won || self.state.game_over {
            if input.jump {
                self.reset();
            }
            return;
        }

        self.state.score_timer += dt;
        if self.state.score_timer >= 1.0 {
            self.state.score_timer -= 1.0;
            self.state.score = self.state.score.saturating_add(1);
        }

        self.update_player(input, dt);
        self.update_barrels(dt);
        self.spawn_barrels(dt);
        self.check_goal();

        if self.player_hit_barrel() {
            self.lose_life();
        }
    }

    pub fn draw(&self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, Color32::BLACK);

        let world = fit_world_rect(rect, self.config.scale.max(1.0));
        painter.rect_stroke(world, 0.0, Stroke::new(2.0, self.theme.neutral));

        self.draw_ui(&painter, world);
        self.draw_level(&painter, world);
        self.draw_goal(&painter, world);
        self.draw_barrels(&painter, world);
        self.draw_player(&painter, world);

        if self.state.won {
            self.draw_overlay(&painter, world, "STAGE CLEAR", "Press Space to restart");
        } else if self.state.game_over {
            self.draw_overlay(&painter, world, "GAME OVER", "Press Space to restart");
        }
    }

    fn update_player(&mut self, input: GameInput, dt: f32) {
        let ladders = ladders();
        let platforms = platforms();
        let player = &mut self.state.player;

        let ladder_overlap = ladders.iter().copied().find(|ladder| {
            (player.pos.x + PLAYER_W * 0.5 - ladder.x).abs() <= 8.0
                && player.pos.y + PLAYER_H >= ladder.y_top
                && player.pos.y <= ladder.y_bottom
        });

        let moving_h = (input.left as i32 - input.right as i32) as f32;
        if moving_h.abs() > 0.0 {
            player.facing = if moving_h > 0.0 { -1.0 } else { 1.0 };
        }

        if ladder_overlap.is_some() && (input.up || input.down) {
            player.climbing = true;
            player.vel.x = 0.0;
        } else if player.climbing && ladder_overlap.is_none() {
            player.climbing = false;
        }

        if player.climbing {
            let Some(ladder) = ladder_overlap else {
                player.climbing = false;
                return;
            };
            player.pos.x = ladder.x - PLAYER_W * 0.5;
            player.vel.y = if input.up {
                -CLIMB_SPEED
            } else if input.down {
                CLIMB_SPEED
            } else {
                0.0
            };
            player.pos.y += player.vel.y * dt;
            player.pos.y = player
                .pos
                .y
                .clamp(ladder.y_top - PLAYER_H + 2.0, ladder.y_bottom - 2.0);
            if input.jump {
                player.climbing = false;
                player.vel.y = -JUMP_SPEED;
            }
            player.on_ground = false;
            return;
        }

        player.vel.x = -moving_h * MOVE_SPEED;
        if input.jump && player.on_ground {
            player.vel.y = -JUMP_SPEED;
            player.on_ground = false;
        }

        player.vel.y += GRAVITY * dt;
        player.pos += player.vel * dt;

        player.pos.x = player.pos.x.clamp(0.0, WORLD_W - PLAYER_W);

        player.on_ground = false;
        let feet = player.pos.y + PLAYER_H;
        for platform in platforms.iter().copied() {
            if player.pos.x + PLAYER_W >= platform.x1 && player.pos.x <= platform.x2 {
                let within_snap = feet >= platform.y - 4.0 && feet <= platform.y + 6.0;
                if player.vel.y >= 0.0 && within_snap {
                    player.pos.y = platform.y - PLAYER_H;
                    player.vel.y = 0.0;
                    player.on_ground = true;
                    break;
                }
            }
        }

        if player.pos.y > WORLD_H {
            self.lose_life();
        }
    }

    fn update_barrels(&mut self, dt: f32) {
        let platforms = platforms();
        for barrel in &mut self.state.barrels {
            if barrel.falling {
                barrel.pos.y += BARREL_DROP_SPEED * dt;
                if barrel.platform_idx + 1 < platforms.len() {
                    let next = platforms[barrel.platform_idx + 1];
                    if barrel.pos.y + BARREL_SIZE >= next.y {
                        barrel.platform_idx += 1;
                        barrel.falling = false;
                        barrel.pos.y = next.y - BARREL_SIZE;
                        barrel.vel.x = if barrel.platform_idx % 2 == 0 {
                            BARREL_SPEED
                        } else {
                            -BARREL_SPEED
                        };
                    }
                }
            } else {
                barrel.pos.x += barrel.vel.x * dt;
                let platform = platforms[barrel.platform_idx];
                let should_drop_right =
                    barrel.vel.x > 0.0 && barrel.pos.x + BARREL_SIZE >= platform.next_drop_x;
                let should_drop_left = barrel.vel.x < 0.0 && barrel.pos.x <= platform.next_drop_x;
                if should_drop_right || should_drop_left {
                    barrel.falling = true;
                    barrel.vel.x = 0.0;
                }
            }
        }

        self.state
            .barrels
            .retain(|barrel| barrel.pos.y < WORLD_H + 24.0);
    }

    fn spawn_barrels(&mut self, dt: f32) {
        self.state.spawn_timer += dt;
        if self.state.spawn_timer < BARREL_SPAWN_SECS {
            return;
        }
        self.state.spawn_timer = 0.0;
        if self.state.barrels.len() >= 6 {
            return;
        }
        let top = platforms()[0];
        self.state.barrels.push(Barrel {
            pos: egui::vec2(top.x1 + 6.0, top.y - BARREL_SIZE),
            vel: egui::vec2(BARREL_SPEED, 0.0),
            platform_idx: 0,
            falling: false,
        });
    }

    fn check_goal(&mut self) {
        let goal = goal_rect();
        let player = Rect::from_min_size(
            pos_to_screen(self.state.player.pos),
            egui::vec2(PLAYER_W, PLAYER_H),
        );
        if player.intersects(goal) {
            self.state.won = true;
            self.state.score = self.state.score.saturating_add(500);
        }
    }

    fn player_hit_barrel(&self) -> bool {
        let player = Rect::from_min_size(
            pos_to_screen(self.state.player.pos),
            egui::vec2(PLAYER_W, PLAYER_H),
        );
        self.state.barrels.iter().any(|barrel| {
            let rect = Rect::from_min_size(
                pos_to_screen(barrel.pos),
                egui::vec2(BARREL_SIZE, BARREL_SIZE),
            );
            player.intersects(rect)
        })
    }

    fn lose_life(&mut self) {
        if self.state.lives > 1 {
            self.state.lives -= 1;
            let mut reset = initial_state();
            reset.score = self.state.score;
            reset.lives = self.state.lives;
            self.state = reset;
        } else {
            self.state.lives = 0;
            self.state.game_over = true;
            self.state.barrels.clear();
        }
    }

    fn draw_ui(&self, painter: &egui::Painter, rect: Rect) {
        let text = format!("SCORE {:05}", self.state.score);
        painter.text(
            rect.left_top() + egui::vec2(8.0, 8.0),
            Align2::LEFT_TOP,
            text,
            FontId::new(14.0, FontFamily::Monospace),
            self.theme.ui,
        );
        painter.text(
            rect.right_top() - egui::vec2(8.0, -8.0),
            Align2::RIGHT_TOP,
            format!("LIVES {}", self.state.lives),
            FontId::new(14.0, FontFamily::Monospace),
            self.theme.ui,
        );
    }

    fn draw_level(&self, painter: &egui::Painter, rect: Rect) {
        for platform in platforms() {
            let mut x = platform.x1;
            while x <= platform.x2 - 16.0 {
                self.paint_frame(
                    painter,
                    rect,
                    FrameId::Girder,
                    egui::pos2(x, platform.y - 16.0),
                    16.0,
                    false,
                );
                x += 16.0;
            }
        }
        for ladder in ladders() {
            let mut y = ladder.y_top;
            while y <= ladder.y_bottom - 16.0 {
                self.paint_frame(
                    painter,
                    rect,
                    FrameId::Ladder,
                    egui::pos2(ladder.x - 8.0, y),
                    16.0,
                    false,
                );
                y += 16.0;
            }
        }
    }

    fn draw_goal(&self, painter: &egui::Painter, rect: Rect) {
        let goal = goal_rect();
        self.paint_frame(painter, rect, FrameId::Goal, goal.min, goal.width(), false);
    }

    fn draw_barrels(&self, painter: &egui::Painter, rect: Rect) {
        let frame = self.animation_frame(AnimationId::BarrelRoll);
        for barrel in &self.state.barrels {
            self.paint_frame(
                painter,
                rect,
                frame,
                pos_to_screen(barrel.pos),
                BARREL_SIZE,
                barrel.vel.x < 0.0,
            );
        }
    }

    fn draw_player(&self, painter: &egui::Painter, rect: Rect) {
        let animation = if self.state.player.climbing {
            AnimationId::MarioClimb
        } else if !self.state.player.on_ground {
            AnimationId::MarioJump
        } else if self.state.player.vel.x.abs() > 1.0 {
            AnimationId::MarioRun
        } else {
            AnimationId::MarioIdle
        };
        let frame = self.animation_frame(animation);
        self.paint_frame(
            painter,
            rect,
            frame,
            pos_to_screen(self.state.player.pos),
            PLAYER_W.max(PLAYER_H),
            self.state.player.facing < 0.0,
        );
    }

    fn draw_overlay(&self, painter: &egui::Painter, rect: Rect, title: &str, subtitle: &str) {
        let overlay = Rect::from_center_size(rect.center(), egui::vec2(140.0, 44.0));
        painter.rect_filled(overlay, 0.0, Color32::BLACK.gamma_multiply(0.92));
        painter.rect_stroke(overlay, 0.0, Stroke::new(2.0, self.theme.ui));
        painter.text(
            overlay.center_top() + egui::vec2(0.0, 10.0),
            Align2::CENTER_TOP,
            title,
            FontId::new(16.0, FontFamily::Monospace),
            self.theme.ui,
        );
        painter.text(
            overlay.center_bottom() - egui::vec2(0.0, 10.0),
            Align2::CENTER_BOTTOM,
            subtitle,
            FontId::new(12.0, FontFamily::Monospace),
            self.theme.neutral,
        );
    }

    fn animation_frame(&self, animation_id: AnimationId) -> FrameId {
        let animation = self
            .catalog
            .animations
            .get(&animation_id)
            .expect("animation missing");
        let len = animation.frames.len().max(1);
        let idx = ((self.state.animation_ticks as u32 / animation.tick.max(1)) as usize) % len;
        animation.frames[idx]
    }

    fn paint_frame(
        &self,
        painter: &egui::Painter,
        world_rect: Rect,
        frame_id: FrameId,
        world_pos: Pos2,
        size: f32,
        flip_x: bool,
    ) {
        let px = world_rect.left() + (world_pos.x / WORLD_W) * world_rect.width();
        let py = world_rect.top() + (world_pos.y / WORLD_H) * world_rect.height();
        let pw = (size / WORLD_W) * world_rect.width();
        let ph = (size / WORLD_H) * world_rect.height();
        let dest = Rect::from_min_size(egui::pos2(px, py), egui::vec2(pw, ph));
        self.paint_placeholder_frame(painter, dest, frame_id, flip_x);
    }

    fn paint_placeholder_frame(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        frame_id: FrameId,
        flip_x: bool,
    ) {
        let stroke_primary = Stroke::new(2.0, self.theme.primary);
        let stroke_enemy = Stroke::new(2.0, self.theme.enemy);
        let stroke_ui = Stroke::new(2.0, self.theme.ui);
        let stroke_neutral = Stroke::new(2.0, self.theme.neutral);
        match frame_id {
            FrameId::Girder => {
                painter.line_segment(
                    [rect.left_bottom(), rect.right_bottom()],
                    Stroke::new(3.0, self.theme.neutral),
                );
                painter.line_segment(
                    [rect.left_top(), rect.right_bottom()],
                    Stroke::new(1.5, self.theme.neutral),
                );
                painter.line_segment(
                    [rect.right_top(), rect.left_bottom()],
                    Stroke::new(1.5, self.theme.neutral),
                );
            }
            FrameId::Ladder => {
                let left = rect.left() + rect.width() * 0.3;
                let right = rect.right() - rect.width() * 0.3;
                painter.line_segment(
                    [
                        egui::pos2(left, rect.top()),
                        egui::pos2(left, rect.bottom()),
                    ],
                    stroke_neutral,
                );
                painter.line_segment(
                    [
                        egui::pos2(right, rect.top()),
                        egui::pos2(right, rect.bottom()),
                    ],
                    stroke_neutral,
                );
                for idx in 1..4 {
                    let y = rect.top() + rect.height() * (idx as f32 / 4.0);
                    painter.line_segment(
                        [egui::pos2(left, y), egui::pos2(right, y)],
                        Stroke::new(1.5, self.theme.neutral),
                    );
                }
            }
            FrameId::Goal => {
                painter.rect_stroke(rect.shrink(1.0), 0.0, stroke_ui);
                painter.circle_filled(
                    rect.center(),
                    rect.width().min(rect.height()) * 0.18,
                    self.theme.ui,
                );
            }
            FrameId::BarrelRoll1 | FrameId::BarrelRoll2 | FrameId::BarrelBroken => {
                painter.circle_stroke(
                    rect.center(),
                    rect.width().min(rect.height()) * 0.38,
                    stroke_enemy,
                );
                painter.line_segment(
                    [
                        egui::pos2(
                            rect.left() + rect.width() * 0.28,
                            rect.top() + rect.height() * 0.3,
                        ),
                        egui::pos2(
                            rect.right() - rect.width() * 0.28,
                            rect.bottom() - rect.height() * 0.3,
                        ),
                    ],
                    Stroke::new(1.5, self.theme.enemy),
                );
                painter.line_segment(
                    [
                        egui::pos2(
                            rect.left() + rect.width() * 0.28,
                            rect.bottom() - rect.height() * 0.3,
                        ),
                        egui::pos2(
                            rect.right() - rect.width() * 0.28,
                            rect.top() + rect.height() * 0.3,
                        ),
                    ],
                    Stroke::new(1.5, self.theme.enemy),
                );
            }
            FrameId::Fire | FrameId::Spark | FrameId::Explosion1 | FrameId::Explosion2 => {
                let c = rect.center();
                let r = rect.width().min(rect.height()) * 0.32;
                painter.line_segment(
                    [egui::pos2(c.x, c.y - r), egui::pos2(c.x + r * 0.45, c.y)],
                    stroke_enemy,
                );
                painter.line_segment(
                    [egui::pos2(c.x + r * 0.45, c.y), egui::pos2(c.x, c.y + r)],
                    stroke_enemy,
                );
                painter.line_segment(
                    [egui::pos2(c.x, c.y + r), egui::pos2(c.x - r * 0.45, c.y)],
                    stroke_enemy,
                );
                painter.line_segment(
                    [egui::pos2(c.x - r * 0.45, c.y), egui::pos2(c.x, c.y - r)],
                    stroke_enemy,
                );
            }
            FrameId::Heart | FrameId::ScoreIcon | FrameId::LifeIcon => {
                painter.rect_stroke(rect.shrink(2.0), 0.0, stroke_ui);
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    match frame_id {
                        FrameId::Heart => "H",
                        FrameId::ScoreIcon => "S",
                        _ => "L",
                    },
                    FontId::new(rect.height() * 0.55, FontFamily::Monospace),
                    self.theme.ui,
                );
            }
            FrameId::MarioIdle
            | FrameId::MarioRun1
            | FrameId::MarioRun2
            | FrameId::MarioJump
            | FrameId::MarioClimb1
            | FrameId::MarioClimb2 => {
                let body = rect.shrink2(egui::vec2(rect.width() * 0.22, rect.height() * 0.18));
                painter.circle_filled(
                    egui::pos2(body.center().x, body.top() + body.height() * 0.16),
                    body.width() * 0.18,
                    self.theme.primary,
                );
                painter.rect_filled(
                    Rect::from_min_max(
                        egui::pos2(
                            body.left() + body.width() * 0.28,
                            body.top() + body.height() * 0.24,
                        ),
                        egui::pos2(
                            body.right() - body.width() * 0.28,
                            body.bottom() - body.height() * 0.18,
                        ),
                    ),
                    0.0,
                    self.theme.primary,
                );
                let dir = if flip_x { -1.0 } else { 1.0 };
                let arm_y = body.top() + body.height() * 0.42;
                painter.line_segment(
                    [
                        egui::pos2(body.center().x, arm_y),
                        egui::pos2(
                            body.center().x + dir * body.width() * 0.28,
                            arm_y - body.height() * 0.08,
                        ),
                    ],
                    stroke_primary,
                );
                painter.line_segment(
                    [
                        egui::pos2(body.center().x, arm_y),
                        egui::pos2(
                            body.center().x - dir * body.width() * 0.25,
                            arm_y + body.height() * 0.05,
                        ),
                    ],
                    stroke_primary,
                );
                let leg_top = body.bottom() - body.height() * 0.18;
                let (left_leg_dx, right_leg_dx) = match frame_id {
                    FrameId::MarioRun1 => (-0.18, 0.26),
                    FrameId::MarioRun2 => (0.22, -0.14),
                    FrameId::MarioJump => (-0.2, 0.2),
                    FrameId::MarioClimb1 => (0.0, 0.0),
                    FrameId::MarioClimb2 => (0.08, -0.08),
                    _ => (-0.1, 0.1),
                };
                painter.line_segment(
                    [
                        egui::pos2(body.center().x - body.width() * 0.1, leg_top),
                        egui::pos2(body.center().x + body.width() * left_leg_dx, body.bottom()),
                    ],
                    stroke_primary,
                );
                painter.line_segment(
                    [
                        egui::pos2(body.center().x + body.width() * 0.1, leg_top),
                        egui::pos2(body.center().x + body.width() * right_leg_dx, body.bottom()),
                    ],
                    stroke_primary,
                );
            }
            _ => {
                painter.rect_stroke(rect.shrink(1.0), 0.0, stroke_neutral);
            }
        }
    }

    #[allow(dead_code)]
    fn tint(&self, role: TintRole) -> Color32 {
        match role {
            TintRole::Primary => self.theme.primary,
            TintRole::Enemy => self.theme.enemy,
            TintRole::Ui => self.theme.ui,
            TintRole::Neutral => self.theme.neutral,
        }
    }
}

impl SpriteCatalog {
    fn load(ctx: &Context) -> Self {
        let mut catalog = Self::default();
        catalog.load_sheet(
            ctx,
            AtlasId::Mario,
            include_str!("../../assets/donkey_kong/meta/mario_sheet.json"),
            include_bytes!(concat!(env!("OUT_DIR"), "/donkey_kong/mario_sheet.png")),
        );
        catalog.load_sheet(
            ctx,
            AtlasId::Barrels,
            include_str!("../../assets/donkey_kong/meta/barrels_sheet.json"),
            include_bytes!(concat!(env!("OUT_DIR"), "/donkey_kong/barrels_sheet.png")),
        );
        catalog.load_sheet(
            ctx,
            AtlasId::Level,
            include_str!("../../assets/donkey_kong/meta/level_sheet.json"),
            include_bytes!(concat!(env!("OUT_DIR"), "/donkey_kong/level_sheet.png")),
        );
        catalog.load_sheet(
            ctx,
            AtlasId::Ui,
            include_str!("../../assets/donkey_kong/meta/ui_sheet.json"),
            include_bytes!(concat!(env!("OUT_DIR"), "/donkey_kong/ui_sheet.png")),
        );
        catalog.load_sheet(
            ctx,
            AtlasId::Effects,
            include_str!("../../assets/donkey_kong/meta/effects_sheet.json"),
            include_bytes!(concat!(env!("OUT_DIR"), "/donkey_kong/effects_sheet.png")),
        );
        catalog
    }

    fn load_sheet(&mut self, ctx: &Context, atlas_id: AtlasId, json: &str, png: &[u8]) {
        let meta: SheetMeta = serde_json::from_str(json).expect("invalid donkey kong metadata");
        let image = image::load_from_memory(png)
            .expect("invalid donkey kong png")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let texture = ctx.load_texture(
            format!("donkey_kong_{atlas_id:?}"),
            egui::ColorImage::from_rgba_unmultiplied(
                [width as usize, height as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::NEAREST,
        );
        self.textures.insert(atlas_id, AtlasTexture { texture });

        for (name, frame) in meta.frames {
            let Some(frame_id) = frame_id_from_name(&name) else {
                continue;
            };
            self.frames.insert(
                frame_id,
                Frame {
                    atlas: atlas_id,
                    uv: Rect::from_min_max(
                        egui::pos2(
                            frame.x as f32 / width as f32,
                            frame.y as f32 / height as f32,
                        ),
                        egui::pos2(
                            (frame.x + frame.w) as f32 / width as f32,
                            (frame.y + frame.h) as f32 / height as f32,
                        ),
                    ),
                    tint_role: tint_role_from_name(&frame.tint_role),
                },
            );
        }

        for (name, animation) in meta.animations {
            let Some(animation_id) = animation_id_from_name(&name) else {
                continue;
            };
            self.animations.insert(
                animation_id,
                Animation {
                    frames: animation
                        .frames
                        .iter()
                        .filter_map(|frame| frame_id_from_name(frame))
                        .collect(),
                    tick: animation.tick.max(1),
                },
            );
        }
    }
}

pub fn input_from_ctx(ctx: &Context) -> GameInput {
    GameInput {
        left: ctx.input(|i| i.key_down(Key::ArrowLeft) || i.key_down(Key::A)),
        right: ctx.input(|i| i.key_down(Key::ArrowRight) || i.key_down(Key::D)),
        up: ctx.input(|i| i.key_down(Key::ArrowUp) || i.key_down(Key::W)),
        down: ctx.input(|i| i.key_down(Key::ArrowDown) || i.key_down(Key::S)),
        jump: ctx.input(|i| i.key_pressed(Key::Space)),
    }
}

fn initial_state() -> GameState {
    let start = platforms()[platforms().len() - 1];
    GameState {
        player: Player {
            pos: egui::vec2(start.x1 + 10.0, start.y - PLAYER_H),
            vel: Vec2::ZERO,
            facing: 1.0,
            climbing: false,
            on_ground: true,
        },
        barrels: Vec::new(),
        score: 0,
        lives: 3,
        won: false,
        game_over: false,
        animation_ticks: 0.0,
        score_timer: 0.0,
        spawn_timer: 1.0,
    }
}

fn fit_world_rect(outer: Rect, scale_hint: f32) -> Rect {
    let max_scale_x = outer.width() / WORLD_W;
    let max_scale_y = outer.height() / WORLD_H;
    let scale = max_scale_x.min(max_scale_y).min(scale_hint.max(1.0) * 4.0);
    let size = egui::vec2(WORLD_W * scale, WORLD_H * scale);
    Rect::from_center_size(outer.center(), size)
}

fn pos_to_screen(pos: Vec2) -> Pos2 {
    egui::pos2(pos.x, pos.y)
}

fn goal_rect() -> Rect {
    Rect::from_min_size(egui::pos2(186.0, 34.0), egui::vec2(16.0, 16.0))
}

fn platforms() -> &'static [Platform] {
    &[
        Platform {
            x1: 28.0,
            x2: 196.0,
            y: 54.0,
            next_drop_x: 196.0,
        },
        Platform {
            x1: 16.0,
            x2: 188.0,
            y: 92.0,
            next_drop_x: 16.0,
        },
        Platform {
            x1: 28.0,
            x2: 208.0,
            y: 130.0,
            next_drop_x: 208.0,
        },
        Platform {
            x1: 16.0,
            x2: 196.0,
            y: 168.0,
            next_drop_x: 16.0,
        },
        Platform {
            x1: 20.0,
            x2: 204.0,
            y: 206.0,
            next_drop_x: 204.0,
        },
        Platform {
            x1: 12.0,
            x2: 212.0,
            y: 238.0,
            next_drop_x: 12.0,
        },
    ]
}

fn ladders() -> &'static [Ladder] {
    &[
        Ladder {
            x: 52.0,
            y_top: 206.0,
            y_bottom: 238.0,
        },
        Ladder {
            x: 160.0,
            y_top: 168.0,
            y_bottom: 206.0,
        },
        Ladder {
            x: 76.0,
            y_top: 130.0,
            y_bottom: 168.0,
        },
        Ladder {
            x: 178.0,
            y_top: 92.0,
            y_bottom: 130.0,
        },
        Ladder {
            x: 90.0,
            y_top: 54.0,
            y_bottom: 92.0,
        },
    ]
}

fn tint_role_from_name(name: &str) -> TintRole {
    match name {
        "Primary" => TintRole::Primary,
        "Enemy" => TintRole::Enemy,
        "Ui" => TintRole::Ui,
        _ => TintRole::Neutral,
    }
}

fn frame_id_from_name(name: &str) -> Option<FrameId> {
    Some(match name {
        "MarioIdle" => FrameId::MarioIdle,
        "MarioRun1" => FrameId::MarioRun1,
        "MarioRun2" => FrameId::MarioRun2,
        "MarioJump" => FrameId::MarioJump,
        "MarioClimb1" => FrameId::MarioClimb1,
        "MarioClimb2" => FrameId::MarioClimb2,
        "BarrelRoll1" => FrameId::BarrelRoll1,
        "BarrelRoll2" => FrameId::BarrelRoll2,
        "BarrelBroken" => FrameId::BarrelBroken,
        "Fire" => FrameId::Fire,
        "Girder" => FrameId::Girder,
        "Ladder" => FrameId::Ladder,
        "PlatformEdge" => FrameId::PlatformEdge,
        "Goal" => FrameId::Goal,
        "Heart" => FrameId::Heart,
        "ScoreIcon" => FrameId::ScoreIcon,
        "LifeIcon" => FrameId::LifeIcon,
        "Spark" => FrameId::Spark,
        "Explosion1" => FrameId::Explosion1,
        "Explosion2" => FrameId::Explosion2,
        _ => return None,
    })
}

fn animation_id_from_name(name: &str) -> Option<AnimationId> {
    Some(match name {
        "MarioIdle" => AnimationId::MarioIdle,
        "MarioRun" => AnimationId::MarioRun,
        "MarioJump" => AnimationId::MarioJump,
        "MarioClimb" => AnimationId::MarioClimb,
        "BarrelRoll" => AnimationId::BarrelRoll,
        "FireIdle" => AnimationId::FireIdle,
        "Explosion" => AnimationId::Explosion,
        _ => return None,
    })
}

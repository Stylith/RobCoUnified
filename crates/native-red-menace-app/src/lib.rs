use std::{
    cell::{Ref, RefCell},
    fs,
    path::{Path, PathBuf},
};

use egui::{
    pos2, vec2, Align2, Color32, ColorImage, Context, FontFamily, FontId, Key, Pos2, Rect,
    Sense, Stroke, TextureHandle, TextureOptions, Ui, Vec2,
};
use image::RgbaImage;
use rand::{rngs::SmallRng, Rng, SeedableRng};

const WORLD_W: f32 = 826.0;
const WORLD_H: f32 = 700.0;
const FRAME_DT: f32 = 1.0 / 60.0;
const TITLE_ANIM_SPEED: f32 = 0.65;
const INTRO_SECS: f32 = 4.5;

// Values aligned with the original Flash runtime config in RedMenaceConfig.xml.
const LIVES_INIT: u8 = 3;
const POWER_ARMOR_DURATION: f32 = 5.0;
const PLAYER_RUN_SPEED_PER_FRAME: f32 = 1.3;
const PLAYER_CLIMB_SPEED_PER_FRAME: f32 = 1.35;
const PLAYER_JUMP_SPEED_PER_FRAME: f32 = 0.75;
const PLAYER_FALL_SPEED_Y_PER_FRAME: f32 = 1.8;
const PLAYER_FALL_SPEED_X_PER_FRAME: f32 = 0.12;
const SCORE_BOMB: i32 = 100;
const BONUS_TIMER_INIT: i32 = 6000;
const BONUS_TIMER_LOST_PER_SECOND: i32 = 100;
const HEIGHT_TO_REACH_PER_STAGE: i32 = 25;
const STAGE_COUNT: u8 = 3;

const HERO_JUMP_START_SPEED_PER_FRAME: f32 = 5.6;
const HERO_JUMP_PHASE_1_DURATION: f32 = 0.06;
const HERO_JUMP_PHASE_2_DURATION: f32 = 0.08;
const HERO_JUMP_EASE_COEFF: f32 = 1.0;
const HERO_JUMP_TO_FALL_SPEED: f32 = 1.4;

const STAGE1_ATTACK_INIT_DELAY: f32 = 3.6;
const STAGE1_SHOW_BOMBS_DURATION: f32 = 1.15;
const STAGE1_DELAY_BETWEEN_LEFT_RIGHT_BOMBS: f32 = 1.15;
const STAGE1_THROW_DURATION: f32 = 0.55;
const STAGE1_THROW_SPAWN_AT: f32 = 0.2;
const STAGE1_FLYING_BOMB_CHANCE: f64 = 0.5;

const HERO_SIZE: Vec2 = Vec2::new(34.0, 56.0);
const BOSS_SIZE: Vec2 = Vec2::new(118.0, 92.0);
const GIRL_SIZE: Vec2 = Vec2::new(26.0, 38.0);
const HELMET_SIZE: Vec2 = Vec2::new(28.0, 24.0);
const FLYING_BOMB_RADIUS: f32 = 10.0;
const ROLLING_BOMB_RADIUS: f32 = 14.0;
const LADDER_USE_RADIUS: f32 = 18.0;
const HERO_DRAW_SIZE: Vec2 = Vec2::new(60.0, 68.0);
const HERO_POWER_ARMOR_DRAW_SIZE: Vec2 = Vec2::new(76.0, 76.0);
const BOSS_DRAW_SIZE: Vec2 = Vec2::new(148.0, 126.0);
const GIRL_DRAW_SIZE: Vec2 = Vec2::new(58.0, 54.0);
const HELMET_DRAW_SIZE: Vec2 = Vec2::new(34.0, 28.0);
const FLYING_BOMB_DRAW_SIZE: Vec2 = Vec2::new(26.0, 30.0);
const ROLLING_BOMB_DRAW_SIZE: Vec2 = Vec2::new(24.0, 24.0);
const HUD_ICON_DRAW_SIZE: Vec2 = Vec2::new(18.0, 20.0);

const PLACEHOLDER_MARKER_SIZE: Vec2 = Vec2::new(34.0, 56.0);
const PLACEHOLDER_MARKER_FLOOR_Y: f32 = WORLD_H - 118.0;
const PLACEHOLDER_MARKER_SPEED: f32 = PLAYER_RUN_SPEED_PER_FRAME * 60.0;
const STAGE1_GIRL_GIRDER: usize = 0;
const STAGE1_BOSS_GIRDER: usize = 1;
const STAGE1_BOTTOM_GIRDER: usize = 6;
const STAGE1_HERO_SPAWN_X: f32 = 300.0;
const HERO_STAND_FRAMES: &[u16] = &[1];
const HERO_RUN_FRAMES: &[u16] = &[6, 12, 18, 24];
const HERO_CLIMB_FRAMES: &[u16] = &[35, 41];
const HERO_JUMP_FRAMES: &[u16] = &[50, 55];
const HERO_FALL_FRAMES: &[u16] = &[60];
const HERO_POWER_ARMOR_STAND_FRAMES: &[u16] = &[370];
const HERO_POWER_ARMOR_RUN_FRAMES: &[u16] = &[350, 357, 363];
const BOSS_IDLE_FRAMES: &[u16] = &[1];
const GIRL_WAVE_FRAMES: &[u16] = &[1];
const FLYING_BOMB_FRAMES: &[u16] = &[1, 5, 9, 13];
const ROLLING_BOMB_FRAMES: &[u16] = &[1, 6, 11, 16];
const HELMET_FRAMES: &[u16] = &[1];
const LIFE_FRAMES: &[u16] = &[1];
const PAUSE_FRAMES: &[u16] = &[1];
const BOSS_SHOW_BOMBS_FRAMES: &[u16] = &[20, 24, 28];
const BOSS_THROW_LEFT_FRAMES: &[u16] = &[30, 33, 35];
const BOSS_THROW_RIGHT_FRAMES: &[u16] = &[30, 33, 35];
const STAGE_FLOOR_505_FRAMES: &[u16] = &[1];
const STAGE_FLOOR_507_FRAMES: &[u16] = &[1];
const STAGE_FLOOR_510_FRAMES: &[u16] = &[1];
const STAGE_FLOOR_513_FRAMES: &[u16] = &[1];
const STAGE_FLOOR_516_FRAMES: &[u16] = &[1];
const STAGE_FLOOR_518_FRAMES: &[u16] = &[1];
const STAGE_LADDER_521_FRAMES: &[u16] = &[1];
const STAGE_LADDER_524_FRAMES: &[u16] = &[1];
const STAGE_LADDER_527_FRAMES: &[u16] = &[1];
const STAGE_LADDER_530_FRAMES: &[u16] = &[1];
const STAGE_LADDER_533_FRAMES: &[u16] = &[1];
const STAGE_SUPPORT_325_FRAMES: &[u16] = &[1];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamePhase {
    Title,
    Intro,
    Transition,
    Level,
    GameOver,
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub ui: Color32,
    pub neutral: Color32,
    pub background: Color32,
    pub warning: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            ui: Color32::from_rgb(56, 255, 120),
            neutral: Color32::from_rgb(92, 164, 112),
            background: Color32::BLACK,
            warning: Color32::from_rgb(255, 110, 110),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct RedMenaceConfig {
    pub scale: f32,
    pub theme: Theme,
}

impl Default for RedMenaceConfig {
    fn default() -> Self {
        Self {
            scale: 1.0,
            theme: Theme::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GameInput {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub action: bool,
    pub action_pressed: bool,
    pub start: bool,
}

#[derive(Clone, Copy, Debug)]
struct PersistentData {
    lives: u8,
    stage: u8,
    level: u32,
    score: i32,
    bonus_timer: i32,
    high_score: i32,
}

impl PersistentData {
    fn new(high_score: i32) -> Self {
        Self {
            lives: LIVES_INIT,
            stage: 0,
            level: 1,
            score: 0,
            bonus_timer: BONUS_TIMER_INIT,
            high_score,
        }
    }

    fn current_height(&self) -> i32 {
        (((self.level - 1) * STAGE_COUNT as u32 + self.stage as u32 + 1) as i32)
            * HEIGHT_TO_REACH_PER_STAGE
    }

    fn complete_stage(&mut self) {
        self.score += self.bonus_timer.max(0);
        self.bonus_timer = BONUS_TIMER_INIT;
        if self.stage + 1 < STAGE_COUNT {
            self.stage += 1;
        } else {
            self.stage = 0;
            self.level += 1;
        }
    }

    fn lose_life(&mut self) -> bool {
        if self.lives > 0 {
            self.lives -= 1;
        }
        self.lives == 0
    }
}

#[derive(Clone, Copy, Debug)]
struct DemoMarker {
    pos: Vec2,
    jump_timer: f32,
}

impl DemoMarker {
    fn new() -> Self {
        Self {
            pos: vec2(WORLD_W * 0.5, PLACEHOLDER_MARKER_FLOOR_Y),
            jump_timer: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct GirderSegment {
    start: Vec2,
    end: Vec2,
}

impl GirderSegment {
    fn min_x(&self) -> f32 {
        self.start.x.min(self.end.x)
    }

    fn max_x(&self) -> f32 {
        self.start.x.max(self.end.x)
    }

    fn contains_x(&self, x: f32) -> bool {
        x >= self.min_x() && x <= self.max_x()
    }

    fn y_at_x(&self, x: f32) -> f32 {
        let span = (self.end.x - self.start.x).abs().max(f32::EPSILON);
        let t = ((x - self.start.x) / span).clamp(0.0, 1.0);
        egui::lerp(self.start.y..=self.end.y, t)
    }
}

#[derive(Clone, Copy, Debug)]
struct LadderSegment {
    x: f32,
    top_y: f32,
    bottom_y: f32,
    upper_girder: usize,
    lower_girder: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeroMode {
    Grounded(usize),
    Jumping,
    Falling,
    Climbing(usize),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HeroVisualState {
    Stand,
    Run,
    Climb,
    Jump,
    Fall,
    PowerArmorStand,
    PowerArmorRun,
}

#[derive(Clone, Copy, Debug)]
struct HeroRuntime {
    pos: Vec2,
    facing: f32,
    mode: HeroMode,
    jump_speed: f32,
    power_armor_timer: f32,
    visual: HeroVisualState,
}

impl HeroRuntime {
    fn new(girders: &[GirderSegment]) -> Self {
        let x = STAGE1_HERO_SPAWN_X;
        let girder = STAGE1_BOTTOM_GIRDER.min(girders.len().saturating_sub(1));
        let y = girders[girder].y_at_x(x) - HERO_SIZE.y * 0.5;
        Self {
            pos: vec2(x, y),
            facing: 1.0,
            mode: HeroMode::Grounded(girder),
            jump_speed: 0.0,
            power_armor_timer: 0.0,
            visual: HeroVisualState::Stand,
        }
    }

    fn rect(&self) -> Rect {
        Rect::from_center_size(pos2(self.pos.x, self.pos.y), HERO_SIZE)
    }

    fn feet(&self) -> Vec2 {
        self.pos + vec2(0.0, HERO_SIZE.y * 0.5)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BombKind {
    Flying,
    Rolling,
}

#[derive(Clone, Copy, Debug)]
enum RollingBombMode {
    OnGirder { girder: usize, dir: f32 },
    OnLadder { ladder: usize, dir: f32 },
    Falling { target_girder: usize, dir: f32 },
}

#[derive(Clone, Copy, Debug)]
enum BombMotion {
    Flying { vel: Vec2, end: Vec2 },
    Rolling(RollingBombMode),
}

#[derive(Clone, Copy, Debug)]
struct StageBomb {
    kind: BombKind,
    pos: Vec2,
    prev_pos: Vec2,
    motion: BombMotion,
    radius: f32,
    age_frames: f32,
    jump_marked: bool,
    bonus_awarded: bool,
    remove: bool,
}

#[derive(Clone, Copy, Debug)]
enum BossAttackState {
    Cooldown(f32),
    Showing {
        timer: f32,
        left: BombKind,
        right: BombKind,
    },
    ThrowingLeft {
        timer: f32,
        left: BombKind,
        right: BombKind,
        spawned: bool,
    },
    RightDelay {
        timer: f32,
        right: BombKind,
    },
    ThrowingRight {
        timer: f32,
        right: BombKind,
        spawned: bool,
    },
}

#[derive(Clone, Copy, Debug, Default)]
struct StageOneOutcome {
    score_delta: i32,
    damaged: bool,
    cleared: bool,
}

#[derive(Clone, Debug)]
struct StageOneState {
    frame_accumulator: f32,
    hero: HeroRuntime,
    girders: Vec<GirderSegment>,
    ladders: Vec<LadderSegment>,
    bombs: Vec<StageBomb>,
    boss_pos: Vec2,
    girl_pos: Vec2,
    helmet_pos: Option<Vec2>,
    attack_state: BossAttackState,
}

impl StageOneState {
    fn new() -> Self {
        let girders = build_stage_one_girders();
        let ladders = build_stage_one_ladders(&girders);
        let boss_x = 310.0;
        let girl_x = 510.0;
        let helmet_x = 690.0;
        let boss_pos = vec2(
            boss_x,
            girders[STAGE1_BOSS_GIRDER].y_at_x(boss_x) - BOSS_SIZE.y * 0.42,
        );
        let girl_pos = vec2(
            girl_x,
            girders[STAGE1_GIRL_GIRDER].y_at_x(girl_x) - GIRL_SIZE.y * 0.55,
        );
        let helmet_pos = vec2(helmet_x, girders[2].y_at_x(helmet_x) - HELMET_SIZE.y * 0.9);
        Self {
            frame_accumulator: 0.0,
            hero: HeroRuntime::new(&girders),
            girders,
            ladders,
            bombs: Vec::new(),
            boss_pos,
            girl_pos,
            helmet_pos: Some(helmet_pos),
            attack_state: BossAttackState::Cooldown(STAGE1_ATTACK_INIT_DELAY),
        }
    }

    fn update_with_dt(
        &mut self,
        input: &GameInput,
        dt: f32,
        level: u32,
        rng: &mut SmallRng,
    ) -> StageOneOutcome {
        self.frame_accumulator += dt * 60.0;
        let mut outcome = StageOneOutcome::default();
        while self.frame_accumulator >= 1.0 && !outcome.damaged && !outcome.cleared {
            self.frame_accumulator -= 1.0;
            let step = self.step_frame(input, level, rng);
            outcome.score_delta += step.score_delta;
            outcome.damaged |= step.damaged;
            outcome.cleared |= step.cleared;
        }
        outcome
    }

    fn step_frame(
        &mut self,
        input: &GameInput,
        level: u32,
        rng: &mut SmallRng,
    ) -> StageOneOutcome {
        let mut outcome = StageOneOutcome::default();
        let girl_zone = Rect::from_center_size(pos2(self.girl_pos.x, self.girl_pos.y), GIRL_SIZE);
        if self.hero.rect().intersects(girl_zone) {
            outcome.cleared = true;
            return outcome;
        }

        self.hero.power_armor_timer = (self.hero.power_armor_timer - FRAME_DT).max(0.0);
        self.update_boss_attack(level, rng);
        self.update_hero(input);
        self.update_bombs(level, rng);

        if let Some(helmet_pos) = self.helmet_pos {
            if self.hero.rect().contains(pos2(helmet_pos.x, helmet_pos.y)) {
                self.hero.power_armor_timer = POWER_ARMOR_DURATION;
                self.helmet_pos = None;
            }
        }

        let hero_rect = self.hero.rect();
        for bomb in &mut self.bombs {
            if bomb.remove {
                continue;
            }

            if bomb.kind == BombKind::Rolling && !bomb.bonus_awarded {
                let overlaps_x = (self.hero.pos.x - bomb.pos.x).abs() < HERO_SIZE.x * 0.6;
                let airborne = matches!(self.hero.mode, HeroMode::Jumping | HeroMode::Falling);
                let hero_above = hero_rect.bottom() < bomb.pos.y - bomb.radius * 0.6;
                if airborne && overlaps_x && hero_above {
                    bomb.jump_marked = true;
                }
                if bomb.jump_marked && !bomb.bonus_awarded && (self.hero.pos.x - bomb.pos.x).abs() > HERO_SIZE.x
                {
                    bomb.bonus_awarded = true;
                    outcome.score_delta += SCORE_BOMB;
                }
            }

            if circle_intersects_rect(bomb.pos, bomb.radius, hero_rect) {
                if self.hero.power_armor_timer > 0.0 {
                    bomb.remove = true;
                    outcome.score_delta += SCORE_BOMB;
                } else {
                    outcome.damaged = true;
                    break;
                }
            }
        }

        self.bombs.retain(|bomb| !bomb.remove);

        if hero_rect.intersects(girl_zone) {
            outcome.cleared = true;
        }

        outcome
    }

    fn update_boss_attack(&mut self, level: u32, rng: &mut SmallRng) {
        self.attack_state = match self.attack_state {
            BossAttackState::Cooldown(mut timer) => {
                timer -= FRAME_DT;
                if timer <= 0.0 {
                    let left = random_bomb_kind(rng);
                    let right = random_bomb_kind(rng);
                    BossAttackState::Showing {
                        timer: STAGE1_SHOW_BOMBS_DURATION,
                        left,
                        right,
                    }
                } else {
                    BossAttackState::Cooldown(timer)
                }
            }
            BossAttackState::Showing {
                mut timer,
                left,
                right,
            } => {
                timer -= FRAME_DT;
                if timer <= 0.0 {
                    BossAttackState::ThrowingLeft {
                        timer: STAGE1_THROW_DURATION,
                        left,
                        right,
                        spawned: false,
                    }
                } else {
                    BossAttackState::Showing { timer, left, right }
                }
            }
            BossAttackState::ThrowingLeft {
                mut timer,
                left,
                right,
                mut spawned,
            } => {
                timer -= FRAME_DT;
                if !spawned && timer <= STAGE1_THROW_SPAWN_AT {
                    self.spawn_hand_bomb(true, left, level, rng);
                    spawned = true;
                }
                if timer <= 0.0 {
                    BossAttackState::RightDelay {
                        timer: STAGE1_DELAY_BETWEEN_LEFT_RIGHT_BOMBS,
                        right,
                    }
                } else {
                    BossAttackState::ThrowingLeft {
                        timer,
                        left,
                        right,
                        spawned,
                    }
                }
            }
            BossAttackState::RightDelay { mut timer, right } => {
                timer -= FRAME_DT;
                if timer <= 0.0 {
                    BossAttackState::ThrowingRight {
                        timer: STAGE1_THROW_DURATION,
                        right,
                        spawned: false,
                    }
                } else {
                    BossAttackState::RightDelay { timer, right }
                }
            }
            BossAttackState::ThrowingRight {
                mut timer,
                right,
                mut spawned,
            } => {
                timer -= FRAME_DT;
                if !spawned && timer <= STAGE1_THROW_SPAWN_AT {
                    self.spawn_hand_bomb(false, right, level, rng);
                    spawned = true;
                }
                if timer <= 0.0 {
                    BossAttackState::Cooldown(stage1_bomb_freq(level))
                } else {
                    BossAttackState::ThrowingRight {
                        timer,
                        right,
                        spawned,
                    }
                }
            }
        };
    }

    fn spawn_hand_bomb(&mut self, left_hand: bool, kind: BombKind, level: u32, rng: &mut SmallRng) {
        let hand_pos = self.hand_pos(left_hand);
        let bomb = match kind {
            BombKind::Flying => {
                let end = random_flying_bomb_endpoint(left_hand, rng);
                let direction = (end - hand_pos).normalized();
                StageBomb {
                    kind,
                    pos: hand_pos,
                    prev_pos: hand_pos,
                    motion: BombMotion::Flying {
                        vel: direction * stage1_flying_bomb_speed(level),
                        end,
                    },
                    radius: FLYING_BOMB_RADIUS,
                    age_frames: 0.0,
                    jump_marked: false,
                    bonus_awarded: false,
                    remove: false,
                }
            }
            BombKind::Rolling => {
                let top_girder = STAGE1_BOSS_GIRDER;
                let start_x = hand_pos.x.clamp(
                    self.girders[top_girder].min_x() + 8.0,
                    self.girders[top_girder].max_x() - 8.0,
                );
                let start_y = self.girders[top_girder].y_at_x(start_x);
                StageBomb {
                    kind,
                    pos: vec2(start_x, start_y),
                    prev_pos: vec2(start_x, start_y),
                    motion: BombMotion::Rolling(RollingBombMode::OnGirder {
                        girder: top_girder,
                        dir: if left_hand { -1.0 } else { 1.0 },
                    }),
                    radius: ROLLING_BOMB_RADIUS,
                    age_frames: 0.0,
                    jump_marked: false,
                    bonus_awarded: false,
                    remove: false,
                }
            }
        };
        self.bombs.push(bomb);
    }

    fn hand_pos(&self, left_hand: bool) -> Vec2 {
        self.boss_pos + if left_hand { vec2(-42.0, 38.0) } else { vec2(42.0, 38.0) }
    }

    fn update_hero(&mut self, input: &GameInput) {
        let mut horizontal: f32 = 0.0;
        if input.left {
            horizontal -= 1.0;
        }
        if input.right {
            horizontal += 1.0;
        }
        if horizontal != 0.0 {
            self.hero.facing = horizontal.signum();
        }

        match self.hero.mode {
            HeroMode::Grounded(girder_idx) => {
                if let Some(ladder_idx) = self.ladder_for_grounded_hero(input, girder_idx) {
                    self.hero.mode = HeroMode::Climbing(ladder_idx);
                    self.hero.pos.x = self.ladders[ladder_idx].x;
                    self.sync_hero_visual(horizontal);
                    return;
                }

                if input.action_pressed {
                    self.hero.mode = HeroMode::Jumping;
                    self.hero.jump_speed = -HERO_JUMP_START_SPEED_PER_FRAME;
                    self.hero.pos.y += self.hero.jump_speed;
                    if horizontal < 0.0 {
                        self.hero.pos.x -= PLAYER_JUMP_SPEED_PER_FRAME;
                    } else if horizontal > 0.0 {
                        self.hero.pos.x += PLAYER_JUMP_SPEED_PER_FRAME;
                    }
                } else {
                    self.hero.pos.x += horizontal * PLAYER_RUN_SPEED_PER_FRAME;
                    self.hero.pos.x = self.hero.pos.x.clamp(
                        self.girders[girder_idx].min_x() + HERO_SIZE.x * 0.5,
                        self.girders[girder_idx].max_x() - HERO_SIZE.x * 0.5,
                    );
                    self.hero.pos.y =
                        self.girders[girder_idx].y_at_x(self.hero.pos.x) - HERO_SIZE.y * 0.5;

                    let near_left_edge =
                        (self.hero.pos.x - self.girders[girder_idx].min_x()) <= HERO_SIZE.x * 0.25;
                    let near_right_edge =
                        (self.girders[girder_idx].max_x() - self.hero.pos.x) <= HERO_SIZE.x * 0.25;
                    if (near_left_edge && horizontal < 0.0) || (near_right_edge && horizontal > 0.0)
                    {
                        self.hero.mode = HeroMode::Falling;
                    }
                }
            }
            HeroMode::Jumping => {
                let prev = self.hero.pos;
                if self.hero.jump_speed < 0.0 {
                    self.hero.jump_speed *= 1.0 - HERO_JUMP_PHASE_1_DURATION;
                    if self.hero.jump_speed > -HERO_JUMP_EASE_COEFF {
                        self.hero.jump_speed = self.hero.jump_speed.abs() * 0.45;
                    }
                } else if self.hero.jump_speed > 0.0
                    && self.hero.jump_speed <= HERO_JUMP_TO_FALL_SPEED
                {
                    self.hero.jump_speed *= 1.0 + HERO_JUMP_PHASE_2_DURATION;
                }

                self.hero.pos.y += self.hero.jump_speed;
                if horizontal < 0.0 {
                    self.hero.pos.x -= PLAYER_JUMP_SPEED_PER_FRAME;
                } else if horizontal > 0.0 {
                    self.hero.pos.x += PLAYER_JUMP_SPEED_PER_FRAME;
                }
                self.hero.pos.x = self
                    .hero
                    .pos
                    .x
                    .clamp(HERO_SIZE.x * 0.5, WORLD_W - HERO_SIZE.x * 0.5);
                if self.try_land_hero(prev) {
                    self.sync_hero_visual(horizontal);
                    return;
                }
                if self.hero.jump_speed >= HERO_JUMP_TO_FALL_SPEED {
                    self.hero.mode = HeroMode::Falling;
                }
            }
            HeroMode::Falling => {
                let prev = self.hero.pos;
                self.hero.pos.y += PLAYER_FALL_SPEED_Y_PER_FRAME;
                if horizontal < 0.0 {
                    self.hero.pos.x -= PLAYER_FALL_SPEED_X_PER_FRAME;
                } else if horizontal > 0.0 {
                    self.hero.pos.x += PLAYER_FALL_SPEED_X_PER_FRAME;
                }
                self.hero.pos.x = self
                    .hero
                    .pos
                    .x
                    .clamp(HERO_SIZE.x * 0.5, WORLD_W - HERO_SIZE.x * 0.5);
                if self.try_land_hero(prev) {
                    self.sync_hero_visual(horizontal);
                    return;
                }
                self.hero.pos.y = self.hero.pos.y.min(WORLD_H - HERO_SIZE.y * 0.5);
            }
            HeroMode::Climbing(ladder_idx) => {
                let ladder = self.ladders[ladder_idx];
                self.hero.pos.x = ladder.x;
                if input.up {
                    self.hero.pos.y -= PLAYER_CLIMB_SPEED_PER_FRAME;
                }
                if input.down {
                    self.hero.pos.y += PLAYER_CLIMB_SPEED_PER_FRAME;
                }
                self.hero.pos.y = self.hero.pos.y.clamp(
                    ladder.top_y - HERO_SIZE.y * 0.5,
                    ladder.bottom_y - HERO_SIZE.y * 0.5,
                );

                if self.hero.feet().y <= ladder.top_y {
                    self.hero.pos.y =
                        self.girders[ladder.upper_girder].y_at_x(ladder.x) - HERO_SIZE.y * 0.5;
                    self.hero.mode = HeroMode::Grounded(ladder.upper_girder);
                } else if self.hero.feet().y >= ladder.bottom_y {
                    self.hero.pos.y =
                        self.girders[ladder.lower_girder].y_at_x(ladder.x) - HERO_SIZE.y * 0.5;
                    self.hero.mode = HeroMode::Grounded(ladder.lower_girder);
                } else if horizontal != 0.0 && (input.up || input.down) {
                    self.hero.facing = horizontal.signum();
                }
            }
        }
        self.sync_hero_visual(horizontal);
    }

    fn sync_hero_visual(&mut self, horizontal: f32) {
        self.hero.visual = match self.hero.mode {
            HeroMode::Grounded(_) => {
                if self.hero.power_armor_timer > 0.0 {
                    if horizontal != 0.0 {
                        HeroVisualState::PowerArmorRun
                    } else {
                        HeroVisualState::PowerArmorStand
                    }
                } else if horizontal != 0.0 {
                    HeroVisualState::Run
                } else {
                    HeroVisualState::Stand
                }
            }
            HeroMode::Jumping => {
                if self.hero.power_armor_timer > 0.0 {
                    HeroVisualState::PowerArmorRun
                } else {
                    HeroVisualState::Jump
                }
            }
            HeroMode::Falling => {
                if self.hero.power_armor_timer > 0.0 {
                    HeroVisualState::PowerArmorRun
                } else {
                    HeroVisualState::Fall
                }
            }
            HeroMode::Climbing(_) => {
                if self.hero.power_armor_timer > 0.0 {
                    HeroVisualState::PowerArmorRun
                } else {
                    HeroVisualState::Climb
                }
            }
        };
    }

    fn ladder_for_grounded_hero(&self, input: &GameInput, girder_idx: usize) -> Option<usize> {
        let feet_y = self.hero.feet().y;
        if input.up {
            return self
                .ladders
                .iter()
                .enumerate()
                .find(|(_, ladder)| {
                    ladder.lower_girder == girder_idx
                        && (self.hero.pos.x - ladder.x).abs() <= LADDER_USE_RADIUS
                        && (feet_y - ladder.bottom_y).abs() <= PLAYER_CLIMB_SPEED_PER_FRAME * 2.0
                })
                .map(|(idx, _)| idx);
        }
        if input.down {
            return self
                .ladders
                .iter()
                .enumerate()
                .find(|(_, ladder)| {
                    ladder.upper_girder == girder_idx
                        && (self.hero.pos.x - ladder.x).abs() <= LADDER_USE_RADIUS
                        && (feet_y - ladder.top_y).abs() <= PLAYER_CLIMB_SPEED_PER_FRAME * 2.0
                })
                .map(|(idx, _)| idx);
        }
        None
    }

    fn try_land_hero(&mut self, prev_pos: Vec2) -> bool {
        let prev_feet_y = prev_pos.y + HERO_SIZE.y * 0.5;
        let feet_y = self.hero.feet().y;
        let x = self.hero.pos.x;
        let mut landing = None;

        for (idx, girder) in self.girders.iter().enumerate() {
            if !girder.contains_x(x) {
                continue;
            }
            let girder_y = girder.y_at_x(x);
            if prev_feet_y <= girder_y + 1.0 && feet_y >= girder_y - 1.0 {
                if landing.map_or(true, |(_, best_y): (usize, f32)| girder_y < best_y) {
                    landing = Some((idx, girder_y));
                }
            }
        }

        if let Some((idx, y)) = landing {
            self.hero.pos.y = y - HERO_SIZE.y * 0.5;
            self.hero.mode = HeroMode::Grounded(idx);
            self.hero.jump_speed = 0.0;
            return true;
        }
        false
    }

    fn update_bombs(&mut self, level: u32, rng: &mut SmallRng) {
        let rolling_speed = stage1_rolling_bomb_speed(level);
        let ladder_probability = stage1_rolling_ladder_chance(level);

        for bomb in &mut self.bombs {
            bomb.prev_pos = bomb.pos;
            bomb.age_frames += 1.0;
            match bomb.motion {
                BombMotion::Flying { vel, end } => {
                    bomb.pos += vel * FRAME_DT;
                    if (bomb.pos - end).length() <= vel.length() * FRAME_DT
                        || bomb.pos.x < -40.0
                        || bomb.pos.x > WORLD_W + 80.0
                        || bomb.pos.y > WORLD_H + 80.0
                    {
                        bomb.remove = true;
                    }
                }
                BombMotion::Rolling(RollingBombMode::OnGirder { girder, dir }) => {
                    let next_x = bomb.pos.x + dir * rolling_speed;
                    if next_x < self.girders[girder].min_x() || next_x > self.girders[girder].max_x()
                    {
                        if girder + 1 < self.girders.len() {
                            let clamped_x = next_x.clamp(
                                self.girders[girder + 1].min_x() + bomb.radius,
                                self.girders[girder + 1].max_x() - bomb.radius,
                            );
                            bomb.pos.x = clamped_x;
                            bomb.motion = BombMotion::Rolling(RollingBombMode::Falling {
                                target_girder: girder + 1,
                                dir: -dir,
                            });
                        } else {
                            bomb.remove = true;
                        }
                    } else {
                        bomb.pos.x = next_x;
                        bomb.pos.y = self.girders[girder].y_at_x(bomb.pos.x);

                        if let Some((ladder_idx, ladder)) = self
                            .ladders
                            .iter()
                            .enumerate()
                            .find(|(_, ladder)| {
                                ladder.upper_girder == girder
                                    && (bomb.pos.x - ladder.x).abs() <= rolling_speed * 1.5
                            })
                        {
                            if rng.gen_bool(ladder_probability) {
                                bomb.pos.x = ladder.x;
                                bomb.motion = BombMotion::Rolling(RollingBombMode::OnLadder {
                                    ladder: ladder_idx,
                                    dir,
                                });
                            }
                        }
                    }
                }
                BombMotion::Rolling(RollingBombMode::OnLadder { ladder, dir }) => {
                    bomb.pos.x = self.ladders[ladder].x;
                    bomb.pos.y += rolling_speed;
                    if bomb.pos.y >= self.ladders[ladder].bottom_y {
                        let lower = self.ladders[ladder].lower_girder;
                        bomb.pos.y = self.girders[lower].y_at_x(bomb.pos.x);
                        bomb.motion = BombMotion::Rolling(RollingBombMode::OnGirder {
                            girder: lower,
                            dir: -dir,
                        });
                    }
                }
                BombMotion::Rolling(RollingBombMode::Falling { target_girder, dir }) => {
                    bomb.pos.y += rolling_speed;
                    let landing_y = self.girders[target_girder].y_at_x(bomb.pos.x);
                    if bomb.pos.y >= landing_y {
                        bomb.pos.y = landing_y;
                        bomb.motion = BombMotion::Rolling(RollingBombMode::OnGirder {
                            girder: target_girder,
                            dir,
                        });
                    }
                }
            }
        }
    }
}

struct SpriteFrame {
    texture: TextureHandle,
}

struct SpriteClip {
    frames: Vec<SpriteFrame>,
    pixel_size: Vec2,
}

impl SpriteClip {
    fn frame_at(&self, phase: f32, ticks_per_frame: f32) -> &SpriteFrame {
        let idx = if self.frames.len() <= 1 {
            0
        } else {
            ((phase / ticks_per_frame.max(1.0)).floor() as usize) % self.frames.len()
        };
        &self.frames[idx]
    }

    fn first(&self) -> &SpriteFrame {
        &self.frames[0]
    }
}

struct RedMenaceTextures {
    hero_stand: SpriteClip,
    hero_run: SpriteClip,
    hero_climb: SpriteClip,
    hero_jump: SpriteClip,
    hero_fall: SpriteClip,
    hero_power_armor_stand: SpriteClip,
    hero_power_armor_run: SpriteClip,
    boss_idle: SpriteClip,
    girl_wave: SpriteClip,
    flying_bomb: SpriteClip,
    rolling_bomb: SpriteClip,
    helmet: SpriteClip,
    life_icon: SpriteClip,
    pause_icon: SpriteClip,
    boss_show_bombs: SpriteClip,
    boss_throw_left: SpriteClip,
    boss_throw_right: SpriteClip,
    stage_floor_505: SpriteClip,
    stage_floor_507: SpriteClip,
    stage_floor_510: SpriteClip,
    stage_floor_513: SpriteClip,
    stage_floor_516: SpriteClip,
    stage_floor_518: SpriteClip,
    stage_ladder_521: SpriteClip,
    stage_ladder_524: SpriteClip,
    stage_ladder_527: SpriteClip,
    stage_ladder_530: SpriteClip,
    stage_ladder_533: SpriteClip,
    stage_support_325: SpriteClip,
}

impl RedMenaceTextures {
    fn load(ctx: &Context) -> Self {
        Self {
            hero_stand: load_clip(ctx, "hero-stand", "pipboy_anim", HERO_STAND_FRAMES),
            hero_run: load_clip(ctx, "hero-run", "pipboy_anim", HERO_RUN_FRAMES),
            hero_climb: load_clip(ctx, "hero-climb", "pipboy_anim", HERO_CLIMB_FRAMES),
            hero_jump: load_clip(ctx, "hero-jump", "pipboy_anim", HERO_JUMP_FRAMES),
            hero_fall: load_clip(ctx, "hero-fall", "pipboy_anim", HERO_FALL_FRAMES),
            hero_power_armor_stand: load_clip(
                ctx,
                "hero-pa-stand",
                "pipboy_anim",
                HERO_POWER_ARMOR_STAND_FRAMES,
            ),
            hero_power_armor_run: load_clip(
                ctx,
                "hero-pa-run",
                "pipboy_anim",
                HERO_POWER_ARMOR_RUN_FRAMES,
            ),
            boss_idle: load_clip(ctx, "boss-stage1", "boss_stage1", BOSS_IDLE_FRAMES),
            girl_wave: load_clip(ctx, "girl-wave", "girl", GIRL_WAVE_FRAMES),
            flying_bomb: load_clip(ctx, "bomb-thrown", "bomb_thrown", FLYING_BOMB_FRAMES),
            rolling_bomb: load_clip(ctx, "bomb-rolled", "bomb_rolled", ROLLING_BOMB_FRAMES),
            helmet: load_clip(ctx, "helmet", "helmet", HELMET_FRAMES),
            life_icon: load_clip(ctx, "life-icon", "life", LIFE_FRAMES),
            pause_icon: load_clip(ctx, "pause-icon", "pause", PAUSE_FRAMES),
            boss_show_bombs: load_clip(
                ctx,
                "boss-show-bombs",
                "boss_stage1",
                BOSS_SHOW_BOMBS_FRAMES,
            ),
            boss_throw_left: load_clip(
                ctx,
                "boss-throw-left",
                "boss_stage1",
                BOSS_THROW_LEFT_FRAMES,
            ),
            boss_throw_right: load_clip(
                ctx,
                "boss-throw-right",
                "boss_stage1",
                BOSS_THROW_RIGHT_FRAMES,
            ),
            stage_floor_505: load_clip(
                ctx,
                "stage-floor-505",
                "stage_floor_505",
                STAGE_FLOOR_505_FRAMES,
            ),
            stage_floor_507: load_clip(
                ctx,
                "stage-floor-507",
                "stage_floor_507",
                STAGE_FLOOR_507_FRAMES,
            ),
            stage_floor_510: load_clip(
                ctx,
                "stage-floor-510",
                "stage_floor_510",
                STAGE_FLOOR_510_FRAMES,
            ),
            stage_floor_513: load_clip(
                ctx,
                "stage-floor-513",
                "stage_floor_513",
                STAGE_FLOOR_513_FRAMES,
            ),
            stage_floor_516: load_clip(
                ctx,
                "stage-floor-516",
                "stage_floor_516",
                STAGE_FLOOR_516_FRAMES,
            ),
            stage_floor_518: load_clip(
                ctx,
                "stage-floor-518",
                "stage_floor_518",
                STAGE_FLOOR_518_FRAMES,
            ),
            stage_ladder_521: load_clip(
                ctx,
                "stage-ladder-521",
                "stage_ladder_521",
                STAGE_LADDER_521_FRAMES,
            ),
            stage_ladder_524: load_clip(
                ctx,
                "stage-ladder-524",
                "stage_ladder_524",
                STAGE_LADDER_524_FRAMES,
            ),
            stage_ladder_527: load_clip(
                ctx,
                "stage-ladder-527",
                "stage_ladder_527",
                STAGE_LADDER_527_FRAMES,
            ),
            stage_ladder_530: load_clip(
                ctx,
                "stage-ladder-530",
                "stage_ladder_530",
                STAGE_LADDER_530_FRAMES,
            ),
            stage_ladder_533: load_clip(
                ctx,
                "stage-ladder-533",
                "stage_ladder_533",
                STAGE_LADDER_533_FRAMES,
            ),
            stage_support_325: load_clip(
                ctx,
                "stage-support-325",
                "stage_support_325",
                STAGE_SUPPORT_325_FRAMES,
            ),
        }
    }
}

pub struct RedMenaceGame {
    config: RedMenaceConfig,
    theme: Theme,
    phase: GamePhase,
    phase_timer: f32,
    animation_ticks: f32,
    bonus_timer_accumulator: f32,
    data: PersistentData,
    marker: DemoMarker,
    new_high_score: bool,
    stage_one: Option<StageOneState>,
    rng: SmallRng,
    textures: RefCell<Option<RedMenaceTextures>>,
}

impl RedMenaceGame {
    pub fn new(config: RedMenaceConfig) -> Self {
        let theme = config.theme;
        Self {
            config,
            theme,
            phase: GamePhase::Title,
            phase_timer: 0.0,
            animation_ticks: 0.0,
            bonus_timer_accumulator: 0.0,
            data: PersistentData::new(0),
            marker: DemoMarker::new(),
            new_high_score: false,
            stage_one: None,
            rng: SmallRng::from_entropy(),
            textures: RefCell::new(None),
        }
    }

    pub fn reset(&mut self) {
        let high_score = self.data.high_score;
        self.phase = GamePhase::Title;
        self.phase_timer = 0.0;
        self.animation_ticks = 0.0;
        self.bonus_timer_accumulator = 0.0;
        self.data = PersistentData::new(high_score);
        self.marker = DemoMarker::new();
        self.new_high_score = false;
        self.stage_one = None;
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn phase(&self) -> GamePhase {
        self.phase
    }

    pub fn stage(&self) -> u8 {
        self.data.stage + 1
    }

    pub fn level(&self) -> u32 {
        self.data.level
    }

    pub fn score(&self) -> i32 {
        self.data.score
    }

    pub fn high_score(&self) -> i32 {
        self.data.high_score
    }

    pub fn lives(&self) -> u8 {
        self.data.lives
    }

    pub fn bonus_timer(&self) -> i32 {
        self.data.bonus_timer
    }

    pub fn update(&mut self, input: &GameInput, dt: f32) {
        let dt = dt.clamp(1.0 / 240.0, 1.0 / 20.0);
        self.animation_ticks += dt * 60.0;
        self.marker.jump_timer = (self.marker.jump_timer - dt).max(0.0);

        match self.phase {
            GamePhase::Title => {
                if input.start || input.action_pressed {
                    self.start_intro();
                }
            }
            GamePhase::Intro => {
                self.phase_timer = (self.phase_timer - dt).max(0.0);
                if self.phase_timer <= f32::EPSILON || input.start || input.action_pressed {
                    self.enter_transition();
                }
            }
            GamePhase::Transition => {
                if input.start || input.action_pressed {
                    self.enter_level();
                }
            }
            GamePhase::Level => self.update_level(input, dt),
            GamePhase::GameOver => {
                if input.start || input.action_pressed {
                    self.reset();
                }
            }
        }
    }

    pub fn draw(&self, ui: &mut Ui) {
        let outer = ui.available_rect_before_wrap();
        let outer = if outer.width() > 1.0 && outer.height() > 1.0 {
            ui.allocate_rect(outer, Sense::hover()).rect
        } else {
            ui.allocate_exact_size(
                vec2(WORLD_W * self.config.scale, WORLD_H * self.config.scale),
                Sense::hover(),
            )
            .0
        };
        let painter = ui.painter_at(outer);
        let world = fit_world_rect(outer, self.config.scale);
        ui.ctx().request_repaint();
        self.ensure_textures(ui.ctx());

        painter.rect_filled(outer, 6.0, self.theme.background);
        painter.rect_stroke(world, 0.0, Stroke::new(2.0, self.theme.neutral));

        match self.phase {
            GamePhase::Title => self.draw_title(&painter, world),
            GamePhase::Intro => self.draw_intro(&painter, world),
            GamePhase::Transition => self.draw_transition(&painter, world),
            GamePhase::Level => self.draw_level(&painter, world),
            GamePhase::GameOver => self.draw_game_over(&painter, world),
        }
    }

    fn start_intro(&mut self) {
        self.phase = GamePhase::Intro;
        self.phase_timer = INTRO_SECS;
        self.marker = DemoMarker::new();
        self.new_high_score = false;
        self.stage_one = None;
    }

    fn enter_transition(&mut self) {
        self.phase = GamePhase::Transition;
        self.phase_timer = 0.0;
        self.stage_one = None;
    }

    fn enter_level(&mut self) {
        self.phase = GamePhase::Level;
        self.phase_timer = 0.0;
        self.data.bonus_timer = BONUS_TIMER_INIT;
        self.bonus_timer_accumulator = 0.0;
        if self.data.stage == 0 {
            self.stage_one = Some(StageOneState::new());
        } else {
            self.stage_one = None;
            self.marker = DemoMarker::new();
        }
    }

    fn reset_level_after_hit(&mut self) {
        self.data.bonus_timer = BONUS_TIMER_INIT;
        self.bonus_timer_accumulator = 0.0;
        if self.data.stage == 0 {
            self.stage_one = Some(StageOneState::new());
        } else {
            self.marker = DemoMarker::new();
        }
    }

    fn update_level(&mut self, input: &GameInput, dt: f32) {
        if self.data.stage == 0 {
            let outcome = self
                .stage_one
                .get_or_insert_with(StageOneState::new)
                .update_with_dt(input, dt, self.data.level, &mut self.rng);
            self.apply_level_outcome(outcome);
        } else {
            self.update_placeholder_level(input, dt);
        }

        if self.phase != GamePhase::Level {
            return;
        }

        self.bonus_timer_accumulator += BONUS_TIMER_LOST_PER_SECOND as f32 * dt;
        let lost = self.bonus_timer_accumulator.floor() as i32;
        if lost > 0 {
            self.data.bonus_timer = (self.data.bonus_timer - lost).max(0);
            self.bonus_timer_accumulator -= lost as f32;
        }

        if self.phase == GamePhase::Level && self.data.bonus_timer <= 0 {
            if self.data.lose_life() {
                self.enter_game_over();
            } else {
                self.reset_level_after_hit();
            }
        }
    }

    fn update_placeholder_level(&mut self, input: &GameInput, dt: f32) {
        let mut dir = 0.0;
        if input.left {
            dir -= 1.0;
        }
        if input.right {
            dir += 1.0;
        }
        self.marker.pos.x += dir * PLACEHOLDER_MARKER_SPEED * dt;
        self.marker.pos.x = self
            .marker
            .pos
            .x
            .clamp(PLACEHOLDER_MARKER_SIZE.x, WORLD_W - PLACEHOLDER_MARKER_SIZE.x);

        if input.action_pressed {
            self.marker.jump_timer = 0.22;
            self.data.complete_stage();
            self.bonus_timer_accumulator = 0.0;
            self.enter_transition();
            return;
        }

        if input.start {
            if self.data.lose_life() {
                self.enter_game_over();
            } else {
                self.reset_level_after_hit();
            }
        }
    }

    fn apply_level_outcome(&mut self, outcome: StageOneOutcome) {
        if outcome.score_delta != 0 {
            self.data.score += outcome.score_delta;
        }
        if outcome.cleared {
            self.data.complete_stage();
            self.bonus_timer_accumulator = 0.0;
            self.enter_transition();
            return;
        }
        if outcome.damaged {
            if self.data.lose_life() {
                self.enter_game_over();
            } else {
                self.reset_level_after_hit();
            }
        }
    }

    fn enter_game_over(&mut self) {
        self.phase = GamePhase::GameOver;
        self.stage_one = None;
        if self.data.score > self.data.high_score {
            self.data.high_score = self.data.score;
            self.new_high_score = true;
        } else {
            self.new_high_score = false;
        }
    }

    fn ensure_textures(&self, ctx: &Context) {
        if self.textures.borrow().is_none() {
            *self.textures.borrow_mut() = Some(RedMenaceTextures::load(ctx));
        }
    }

    fn textures(&self) -> Ref<'_, RedMenaceTextures> {
        Ref::map(self.textures.borrow(), |textures| {
            textures.as_ref().expect("red menace textures should be loaded")
        })
    }

    fn draw_title(&self, painter: &egui::Painter, world: Rect) {
        let pulse = (self.animation_ticks * TITLE_ANIM_SPEED).sin() * 0.18 + 0.82;
        let title_color = self.theme.ui.gamma_multiply(pulse.clamp(0.3, 1.0));

        painter.text(
            world.center_top() + vec2(0.0, world.height() * 0.24),
            Align2::CENTER_CENTER,
            "RED MENACE",
            FontId::new(44.0, FontFamily::Monospace),
            title_color,
        );
        painter.text(
            world.center_top() + vec2(0.0, world.height() * 0.40),
            Align2::CENTER_CENTER,
            "START GAME",
            FontId::new(20.0, FontFamily::Monospace),
            self.theme.neutral,
        );
        painter.text(
            world.center_top() + vec2(0.0, world.height() * 0.48),
            Align2::CENTER_CENTER,
            "Press Enter",
            FontId::new(15.0, FontFamily::Monospace),
            self.theme.neutral.gamma_multiply(0.9),
        );
        painter.text(
            world.center_bottom() - vec2(0.0, 48.0),
            Align2::CENTER_CENTER,
            "Stage 1 gameplay slice now ported in Rust",
            FontId::new(12.0, FontFamily::Monospace),
            self.theme.neutral.gamma_multiply(0.7),
        );
    }

    fn draw_intro(&self, painter: &egui::Painter, world: Rect) {
        let progress = 1.0 - self.phase_timer / INTRO_SECS;
        let progress = progress.clamp(0.0, 1.0);
        let title_y = egui::lerp(world.top() + 120.0..=world.center().y - 24.0, progress);
        let boss_offset = (self.animation_ticks * 0.08).sin() * 18.0;

        painter.text(
            pos2(world.center().x, title_y),
            Align2::CENTER_CENTER,
            "INTRO",
            FontId::new(34.0, FontFamily::Monospace),
            self.theme.ui,
        );
        painter.circle_stroke(
            pos2(world.center().x + boss_offset, world.center().y + 32.0),
            world.width().min(world.height()) * 0.10,
            Stroke::new(3.0, self.theme.neutral),
        );
        painter.text(
            world.center_bottom() - vec2(0.0, 64.0),
            Align2::CENTER_CENTER,
            format!("Sequence {:.1}s", self.phase_timer),
            FontId::new(14.0, FontFamily::Monospace),
            self.theme.neutral,
        );
    }

    fn draw_transition(&self, painter: &egui::Painter, world: Rect) {
        painter.text(
            world.center_top() + vec2(0.0, world.height() * 0.25),
            Align2::CENTER_CENTER,
            "HOW HIGH CAN YOU GET?",
            FontId::new(30.0, FontFamily::Monospace),
            self.theme.ui,
        );
        painter.text(
            world.center(),
            Align2::CENTER_CENTER,
            format!("{}M", self.data.current_height()),
            FontId::new(54.0, FontFamily::Monospace),
            self.theme.ui,
        );
        painter.text(
            world.center_bottom() - vec2(0.0, 84.0),
            Align2::CENTER_CENTER,
            "READY",
            FontId::new(20.0, FontFamily::Monospace),
            self.theme.neutral,
        );
        painter.text(
            world.center_bottom() - vec2(0.0, 54.0),
            Align2::CENTER_CENTER,
            format!("Stage {}", self.data.stage + 1),
            FontId::new(14.0, FontFamily::Monospace),
            self.theme.neutral.gamma_multiply(0.9),
        );
        painter.text(
            world.center_bottom() - vec2(0.0, 32.0),
            Align2::CENTER_CENTER,
            "Press Enter",
            FontId::new(14.0, FontFamily::Monospace),
            self.theme.neutral.gamma_multiply(0.9),
        );
    }

    fn draw_level(&self, painter: &egui::Painter, world: Rect) {
        self.draw_hud(painter, world);
        if self.data.stage == 0 {
            if let Some(stage) = &self.stage_one {
                self.draw_stage_one(painter, world, stage);
            }
        } else {
            self.draw_placeholder_level(painter, world);
        }
    }

    fn draw_stage_one(&self, painter: &egui::Painter, world: Rect, stage: &StageOneState) {
        let textures = self.textures();
        let stage_tint = self.theme.ui;

        for (pos, height) in [
            (vec2(166.0, 448.0), 456.0),
            (vec2(760.0, 498.0), 288.0),
            (vec2(546.0, 183.0), 78.0),
        ] {
            draw_static_clip_centered(
                painter,
                world,
                &textures.stage_support_325,
                pos,
                clip_size_for_height(&textures.stage_support_325, height),
                stage_tint.gamma_multiply(0.82),
                false,
            );
        }

        for &(x, top, bottom) in &[
            (196.0, 226.0, 292.0),
            (560.0, 222.0, 292.0),
            (708.0, 370.0, 514.0),
            (758.0, 370.0, 566.0),
        ] {
            draw_chain(painter, world, x, top, bottom, self.theme.neutral.gamma_multiply(0.9));
        }

        for (clip, pos, width, flip_x) in [
            (&textures.stage_floor_505, vec2(494.0, 148.0), 180.0, false),
            (&textures.stage_floor_518, vec2(319.0, 220.0), 318.0, false),
            (&textures.stage_floor_516, vec2(632.0, 239.0), 286.0, false),
            (&textures.stage_floor_507, vec2(386.0, 392.0), 352.0, false),
            (&textures.stage_floor_513, vec2(676.0, 370.0), 344.0, false),
            (&textures.stage_floor_510, vec2(456.0, 553.0), 620.0, false),
            (&textures.stage_floor_513, vec2(488.0, 646.0), 704.0, false),
        ] {
            draw_static_clip_centered(
                painter,
                world,
                clip,
                pos,
                clip_size_for_width(clip, width),
                stage_tint,
                flip_x,
            );
        }

        for ladder in &stage.ladders {
            let ladder_height = (ladder.bottom_y - ladder.top_y).abs() + 12.0;
            let clip = if ladder_height < 84.0 {
                &textures.stage_ladder_521
            } else if ladder_height < 112.0 {
                &textures.stage_ladder_524
            } else if ladder_height < 126.0 {
                &textures.stage_ladder_527
            } else if ladder_height < 154.0 {
                &textures.stage_ladder_530
            } else {
                &textures.stage_ladder_533
            };
            draw_static_clip_centered(
                painter,
                world,
                clip,
                vec2(ladder.x, (ladder.top_y + ladder.bottom_y) * 0.5),
                clip_size_for_height(clip, ladder_height),
                stage_tint,
                false,
            );
        }

        draw_boss_stack(painter, world, stage.boss_pos + vec2(-88.0, -10.0), self.theme.ui);
        let (boss_clip, boss_phase, boss_ticks_per_frame, left_hand_bomb, right_hand_bomb) =
            match stage.attack_state {
                BossAttackState::Cooldown(_) => {
                    (&textures.boss_idle, self.animation_ticks, 18.0, None, None)
                }
                BossAttackState::Showing { timer, left, right } => (
                    &textures.boss_show_bombs,
                    (STAGE1_SHOW_BOMBS_DURATION - timer) * 60.0,
                    7.0,
                    Some(left),
                    Some(right),
                ),
                BossAttackState::ThrowingLeft {
                    timer,
                    left,
                    right,
                    spawned,
                } => (
                    &textures.boss_throw_left,
                    (STAGE1_THROW_DURATION - timer) * 60.0,
                    6.0,
                    (!spawned).then_some(left),
                    Some(right),
                ),
                BossAttackState::RightDelay { timer, right } => (
                    &textures.boss_show_bombs,
                    (STAGE1_DELAY_BETWEEN_LEFT_RIGHT_BOMBS - timer) * 60.0,
                    7.0,
                    None,
                    Some(right),
                ),
                BossAttackState::ThrowingRight {
                    timer,
                    right,
                    spawned,
                } => (
                    &textures.boss_throw_right,
                    (STAGE1_THROW_DURATION - timer) * 60.0,
                    6.0,
                    None,
                    (!spawned).then_some(right),
                ),
            };
        draw_clip_centered(
            painter,
            world,
            boss_clip,
            stage.boss_pos + vec2(0.0, 4.0),
            BOSS_DRAW_SIZE,
            boss_phase,
            boss_ticks_per_frame,
            self.theme.ui,
            false,
        );

        if let Some(left) = left_hand_bomb {
            self.draw_boss_hand_bomb(painter, world, &textures, stage.hand_pos(true), left);
        }
        if let Some(right) = right_hand_bomb {
            self.draw_boss_hand_bomb(painter, world, &textures, stage.hand_pos(false), right);
        }

        draw_clip_centered(
            painter,
            world,
            &textures.girl_wave,
            stage.girl_pos + vec2(0.0, 2.0),
            GIRL_DRAW_SIZE,
            self.animation_ticks,
            20.0,
            self.theme.ui,
            false,
        );

        if let Some(helmet_pos) = stage.helmet_pos {
            draw_clip_centered(
                painter,
                world,
                &textures.helmet,
                helmet_pos,
                HELMET_DRAW_SIZE,
                0.0,
                1.0,
                self.theme.ui,
                false,
            );
        }

        for bomb in &stage.bombs {
            let (clip, draw_size, flip_x) = match bomb.kind {
                BombKind::Flying => (&textures.flying_bomb, FLYING_BOMB_DRAW_SIZE, bomb.pos.x < bomb.prev_pos.x),
                BombKind::Rolling => (&textures.rolling_bomb, ROLLING_BOMB_DRAW_SIZE, false),
            };
            draw_clip_centered(
                painter,
                world,
                clip,
                bomb.pos,
                draw_size,
                bomb.age_frames,
                6.0,
                self.theme.ui,
                flip_x,
            );
        }

        let (hero_clip, hero_size, hero_phase, hero_ticks_per_frame) = match stage.hero.visual {
            HeroVisualState::Stand => (&textures.hero_stand, HERO_DRAW_SIZE, self.animation_ticks, 24.0),
            HeroVisualState::Run => (&textures.hero_run, HERO_DRAW_SIZE, self.animation_ticks, 8.0),
            HeroVisualState::Climb => (&textures.hero_climb, HERO_DRAW_SIZE, self.animation_ticks, 12.0),
            HeroVisualState::Jump => (&textures.hero_jump, HERO_DRAW_SIZE, self.animation_ticks, 10.0),
            HeroVisualState::Fall => (&textures.hero_fall, HERO_DRAW_SIZE, self.animation_ticks, 10.0),
            HeroVisualState::PowerArmorStand => (
                &textures.hero_power_armor_stand,
                HERO_POWER_ARMOR_DRAW_SIZE,
                self.animation_ticks,
                24.0,
            ),
            HeroVisualState::PowerArmorRun => (
                &textures.hero_power_armor_run,
                HERO_POWER_ARMOR_DRAW_SIZE,
                self.animation_ticks,
                10.0,
            ),
        };
        draw_clip_centered(
            painter,
            world,
            hero_clip,
            stage.hero.pos + vec2(0.0, 1.0),
            hero_size,
            hero_phase,
            hero_ticks_per_frame,
            self.theme.ui,
            stage.hero.facing > 0.0,
        );
    }

    fn draw_boss_hand_bomb(
        &self,
        painter: &egui::Painter,
        world: Rect,
        textures: &RedMenaceTextures,
        pos: Vec2,
        bomb_kind: BombKind,
    ) {
        let (clip, size) = match bomb_kind {
            BombKind::Flying => (&textures.flying_bomb, FLYING_BOMB_DRAW_SIZE),
            BombKind::Rolling => (&textures.rolling_bomb, ROLLING_BOMB_DRAW_SIZE),
        };
        draw_clip_centered(
            painter,
            world,
            clip,
            pos,
            size,
            self.animation_ticks,
            8.0,
            self.theme.ui,
            false,
        );
    }

    fn draw_placeholder_level(&self, painter: &egui::Painter, world: Rect) {
        painter.text(
            world.center_top() + vec2(0.0, 18.0),
            Align2::CENTER_TOP,
            format!("STAGE {} PLACEHOLDER", self.data.stage + 1),
            FontId::new(18.0, FontFamily::Monospace),
            self.theme.ui,
        );
        painter.text(
            world.center_top() + vec2(0.0, 42.0),
            Align2::CENTER_TOP,
            "Space clears stage, Enter loses life",
            FontId::new(13.0, FontFamily::Monospace),
            self.theme.neutral,
        );

        let floor_y = PLACEHOLDER_MARKER_FLOOR_Y + 18.0;
        let line_start = world_point(world, vec2(54.0, floor_y));
        let line_end = world_point(world, vec2(WORLD_W - 54.0, floor_y));
        painter.line_segment([line_start, line_end], Stroke::new(1.5, self.theme.neutral));

        let bob = if self.marker.jump_timer > 0.0 {
            (self.marker.jump_timer / 0.22) * 34.0
        } else {
            0.0
        };
        let marker_rect =
            world_rect_from_entity(world, self.marker.pos - vec2(0.0, bob), PLACEHOLDER_MARKER_SIZE);
        painter.rect_stroke(marker_rect, 0.0, Stroke::new(2.0, self.theme.ui));
        painter.line_segment(
            [marker_rect.center_top(), marker_rect.center_bottom()],
            Stroke::new(2.0, self.theme.ui),
        );
        painter.line_segment(
            [marker_rect.left_center(), marker_rect.right_center()],
            Stroke::new(2.0, self.theme.ui),
        );
    }

    fn draw_game_over(&self, painter: &egui::Painter, world: Rect) {
        painter.text(
            world.center_top() + vec2(0.0, world.height() * 0.24),
            Align2::CENTER_CENTER,
            "GAME OVER",
            FontId::new(40.0, FontFamily::Monospace),
            self.theme.warning,
        );
        if self.new_high_score {
            painter.text(
                world.center_top() + vec2(0.0, world.height() * 0.36),
                Align2::CENTER_CENTER,
                "High Score",
                FontId::new(20.0, FontFamily::Monospace),
                self.theme.ui,
            );
        }
        painter.text(
            world.center(),
            Align2::CENTER_CENTER,
            format!("{:06}", self.data.high_score),
            FontId::new(50.0, FontFamily::Monospace),
            self.theme.ui,
        );
        painter.text(
            world.center_bottom() - vec2(0.0, 74.0),
            Align2::CENTER_CENTER,
            "PLAY AGAIN?",
            FontId::new(20.0, FontFamily::Monospace),
            self.theme.neutral,
        );
        painter.text(
            world.center_bottom() - vec2(0.0, 46.0),
            Align2::CENTER_CENTER,
            "Press Enter",
            FontId::new(14.0, FontFamily::Monospace),
            self.theme.neutral.gamma_multiply(0.9),
        );
    }

    fn draw_hud(&self, painter: &egui::Painter, world: Rect) {
        if self.phase == GamePhase::Level && self.data.stage == 0 {
            let textures = self.textures();
            let label_font = FontId::new(14.0, FontFamily::Monospace);
            let value_font = FontId::new(16.0, FontFamily::Monospace);
            let y_label = world.top() + 12.0;
            let y_value = world.top() + 32.0;
            let x_left = world.left() + world.width() * 0.13;
            let x_mid = world.left() + world.width() * 0.43;
            let x_high = world.left() + world.width() * 0.72;

            painter.text(
                pos2(x_left, y_label),
                Align2::CENTER_TOP,
                "1UP",
                label_font.clone(),
                self.theme.ui,
            );
            painter.text(
                pos2(x_left, y_value),
                Align2::CENTER_TOP,
                self.data.score.max(0).to_string(),
                value_font.clone(),
                self.theme.ui,
            );
            painter.text(
                pos2(x_mid, y_label),
                Align2::CENTER_TOP,
                "BONUS TIMER",
                label_font.clone(),
                self.theme.ui,
            );
            painter.text(
                pos2(x_mid, y_value),
                Align2::CENTER_TOP,
                self.data.bonus_timer.max(0).to_string(),
                value_font.clone(),
                self.theme.ui.gamma_multiply(if self.data.bonus_timer <= 500 { 0.75 } else { 1.0 }),
            );
            painter.text(
                pos2(x_high, y_label),
                Align2::CENTER_TOP,
                "HIGH SCORE",
                label_font,
                self.theme.ui,
            );
            painter.text(
                pos2(x_high, y_value),
                Align2::CENTER_TOP,
                self.data.high_score.max(0).to_string(),
                value_font,
                self.theme.ui,
            );

            let mut life_x = world.right() - 104.0;
            for idx in 0..LIVES_INIT {
                let rect = Rect::from_center_size(
                    pos2(life_x, world.top() + 20.0),
                    world_size(world, HUD_ICON_DRAW_SIZE),
                );
                draw_clip_in_rect(
                    painter,
                    rect,
                    &textures.life_icon,
                    textures.life_icon.first(),
                    if idx < self.data.lives {
                        self.theme.ui
                    } else {
                        self.theme.ui.gamma_multiply(0.35)
                    },
                    false,
                );
                life_x += 22.0;
            }
            draw_clip_in_rect(
                painter,
                Rect::from_center_size(pos2(world.right() - 28.0, world.top() + 24.0), vec2(26.0, 26.0)),
                &textures.pause_icon,
                textures.pause_icon.first(),
                self.theme.ui,
                false,
            );
            return;
        }

        let left = world.left() + 22.0;
        let top = world.top() + 18.0;
        let right = world.right() - 22.0;
        painter.text(
            pos2(left, top),
            Align2::LEFT_TOP,
            format!("SCORE {:06}", self.data.score.max(0)),
            FontId::new(18.0, FontFamily::Monospace),
            self.theme.ui,
        );
        painter.text(
            pos2(left, top + 24.0),
            Align2::LEFT_TOP,
            format!("HIGH {:06}", self.data.high_score.max(0)),
            FontId::new(16.0, FontFamily::Monospace),
            self.theme.neutral,
        );
        painter.text(
            pos2(world.center().x, top),
            Align2::CENTER_TOP,
            format!("L{:02}-{:01}", self.data.level, self.data.stage + 1),
            FontId::new(18.0, FontFamily::Monospace),
            self.theme.ui,
        );
        painter.text(
            pos2(world.center().x, top + 24.0),
            Align2::CENTER_TOP,
            format!("{}M", self.data.current_height()),
            FontId::new(16.0, FontFamily::Monospace),
            self.theme.neutral,
        );
        painter.text(
            pos2(right, top),
            Align2::RIGHT_TOP,
            format!("TIME {:04}", self.data.bonus_timer.max(0)),
            FontId::new(18.0, FontFamily::Monospace),
            self.theme.ui,
        );
        painter.text(
            pos2(right, top + 24.0),
            Align2::RIGHT_TOP,
            format!("LIVES {}", self.data.lives),
            FontId::new(16.0, FontFamily::Monospace),
            self.theme.neutral,
        );
    }
}

pub fn input_from_ctx(ctx: &Context) -> GameInput {
    GameInput {
        left: ctx.input(|i| i.key_down(Key::ArrowLeft) || i.key_down(Key::A)),
        right: ctx.input(|i| i.key_down(Key::ArrowRight) || i.key_down(Key::D)),
        up: ctx.input(|i| i.key_down(Key::ArrowUp) || i.key_down(Key::W)),
        down: ctx.input(|i| i.key_down(Key::ArrowDown) || i.key_down(Key::S)),
        action: ctx.input(|i| i.key_down(Key::Space)),
        action_pressed: ctx.input(|i| i.key_pressed(Key::Space)),
        start: ctx.input(|i| i.key_pressed(Key::Enter)),
    }
}

fn random_bomb_kind(rng: &mut SmallRng) -> BombKind {
    if rng.gen_bool(STAGE1_FLYING_BOMB_CHANCE) {
        BombKind::Flying
    } else {
        BombKind::Rolling
    }
}

fn random_flying_bomb_endpoint(left_hand: bool, rng: &mut SmallRng) -> Vec2 {
    let endpoints = [
        vec2(130.0, 760.0),
        vec2(395.0, 760.0),
        vec2(890.0, 640.0),
    ];
    if left_hand {
        if rng.gen_bool(0.5) {
            endpoints[0]
        } else {
            endpoints[1]
        }
    } else if rng.gen_bool(0.5) {
        endpoints[1]
    } else {
        endpoints[2]
    }
}

fn build_stage_one_girders() -> Vec<GirderSegment> {
    vec![
        GirderSegment {
            start: vec2(410.0, 148.0),
            end: vec2(575.0, 148.0),
        },
        GirderSegment {
            start: vec2(168.0, 220.0),
            end: vec2(472.0, 220.0),
        },
        GirderSegment {
            start: vec2(500.0, 220.0),
            end: vec2(760.0, 260.0),
        },
        GirderSegment {
            start: vec2(252.0, 412.0),
            end: vec2(520.0, 374.0),
        },
        GirderSegment {
            start: vec2(540.0, 386.0),
            end: vec2(808.0, 354.0),
        },
        GirderSegment {
            start: vec2(168.0, 522.0),
            end: vec2(744.0, 586.0),
        },
        GirderSegment {
            start: vec2(168.0, 670.0),
            end: vec2(806.0, 624.0),
        },
    ]
}

fn build_stage_one_ladders(girders: &[GirderSegment]) -> Vec<LadderSegment> {
    let specs = [
        (0usize, 1usize, 435.0),
        (1usize, 3usize, 286.0),
        (2usize, 4usize, 740.0),
        (3usize, 5usize, 536.0),
        (5usize, 6usize, 350.0),
        (4usize, 5usize, 742.0),
    ];

    specs
        .into_iter()
        .map(|(upper, lower, x)| LadderSegment {
            x,
            top_y: girders[upper].y_at_x(x),
            bottom_y: girders[lower].y_at_x(x),
            upper_girder: upper,
            lower_girder: lower,
        })
        .collect()
}

fn stage1_bomb_freq(level: u32) -> f32 {
    (5.75f32 * (1.0f32 - 0.22f32).powf(level.saturating_sub(1) as f32)).max(2.2f32)
}

fn stage1_flying_bomb_speed(level: u32) -> f32 {
    (110.0f32 * (1.0f32 + 0.05f32).powf(level.saturating_sub(1) as f32)).min(160.0f32)
}

fn stage1_rolling_bomb_speed(level: u32) -> f32 {
    (2.2f32 * (1.0f32 + 0.1f32).powf(level.saturating_sub(1) as f32)).min(3.6f32)
}

fn stage1_rolling_ladder_chance(level: u32) -> f64 {
    ((6.0f32 * (1.0f32 + 0.18f32).powf(level.saturating_sub(1) as f32)).min(15.0f32)
        / 100.0f32) as f64
}

fn asset_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/png")
}

fn asset_frame_path(dir: &str, frame: u16) -> PathBuf {
    asset_root().join(dir).join(format!("{frame}.png"))
}

fn load_png_image(path: &Path) -> RgbaImage {
    let bytes = fs::read(path).unwrap_or_else(|err| {
        panic!("failed to read red menace asset {}: {err}", path.display())
    });
    image::load_from_memory(&bytes)
        .unwrap_or_else(|err| panic!("invalid red menace png {}: {err}", path.display()))
        .into_rgba8()
}

#[derive(Clone, Copy)]
struct PixelBounds {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

impl PixelBounds {
    fn width(self) -> u32 {
        self.max_x - self.min_x + 1
    }

    fn height(self) -> u32 {
        self.max_y - self.min_y + 1
    }
}

fn opaque_bounds(image: &RgbaImage) -> Option<PixelBounds> {
    let mut min_x = image.width();
    let mut min_y = image.height();
    let mut max_x = 0;
    let mut max_y = 0;
    let mut found = false;

    for (x, y, pixel) in image.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
        found = true;
    }

    found.then_some(PixelBounds {
        min_x,
        min_y,
        max_x,
        max_y,
    })
}

fn union_bounds(images: &[RgbaImage]) -> PixelBounds {
    let mut bounds: Option<PixelBounds> = None;
    for image in images {
        if let Some(image_bounds) = opaque_bounds(image) {
            bounds = Some(match bounds {
                Some(current) => PixelBounds {
                    min_x: current.min_x.min(image_bounds.min_x),
                    min_y: current.min_y.min(image_bounds.min_y),
                    max_x: current.max_x.max(image_bounds.max_x),
                    max_y: current.max_y.max(image_bounds.max_y),
                },
                None => image_bounds,
            });
        }
    }
    bounds.unwrap_or(PixelBounds {
        min_x: 0,
        min_y: 0,
        max_x: 0,
        max_y: 0,
    })
}

fn load_clip(ctx: &Context, name: &str, dir: &str, frames: &[u16]) -> SpriteClip {
    let images: Vec<_> = frames
        .iter()
        .map(|frame| load_png_image(&asset_frame_path(dir, *frame)))
        .collect();
    let bounds = union_bounds(&images);
    let pixel_size = vec2(bounds.width() as f32, bounds.height() as f32);
    let frames = images
        .into_iter()
        .zip(frames.iter())
        .map(|(image, frame)| {
            let cropped = image::imageops::crop_imm(
                &image,
                bounds.min_x,
                bounds.min_y,
                bounds.width(),
                bounds.height(),
            )
            .to_image();
            let size = [cropped.width() as usize, cropped.height() as usize];
            let color_image = ColorImage::from_rgba_unmultiplied(size, cropped.as_raw());
            SpriteFrame {
                texture: ctx.load_texture(
                    format!("red-menace-{name}-{frame}"),
                    color_image,
                    TextureOptions::NEAREST,
                ),
            }
        })
        .collect();
    SpriteClip { frames, pixel_size }
}

fn circle_intersects_rect(center: Vec2, radius: f32, rect: Rect) -> bool {
    let nearest_x = center.x.clamp(rect.left(), rect.right());
    let nearest_y = center.y.clamp(rect.top(), rect.bottom());
    (center - vec2(nearest_x, nearest_y)).length() <= radius
}

fn draw_chain(
    painter: &egui::Painter,
    world: Rect,
    x: f32,
    top: f32,
    bottom: f32,
    color: Color32,
) {
    let mut y = top;
    while y < bottom {
        let center = world_point(world, vec2(x, y));
        painter.circle_stroke(center, 5.0, Stroke::new(1.5, color));
        y += 14.0;
    }
}

fn draw_boss_stack(painter: &egui::Painter, world: Rect, origin: Vec2, color: Color32) {
    for row in 0..4 {
        for col in 0..3 {
            let offset = vec2(col as f32 * 18.0, row as f32 * 16.0 + (col % 2) as f32 * 1.5);
            painter.circle_stroke(
                world_point(world, origin + offset),
                7.0,
                Stroke::new(2.0, color),
            );
        }
    }
}

fn fit_size_with_aspect(max_size: Vec2, content_size: Vec2) -> Vec2 {
    if content_size.x <= f32::EPSILON || content_size.y <= f32::EPSILON {
        return max_size;
    }
    let texture_aspect = content_size.x / content_size.y;
    let box_aspect = (max_size.x / max_size.y.max(f32::EPSILON)).max(f32::EPSILON);
    if texture_aspect > box_aspect {
        vec2(max_size.x, max_size.x / texture_aspect)
    } else {
        vec2(max_size.y * texture_aspect, max_size.y)
    }
}

fn clip_size_for_width(clip: &SpriteClip, width: f32) -> Vec2 {
    if clip.pixel_size.x <= f32::EPSILON {
        return vec2(width, width);
    }
    vec2(width, width * clip.pixel_size.y / clip.pixel_size.x)
}

fn clip_size_for_height(clip: &SpriteClip, height: f32) -> Vec2 {
    if clip.pixel_size.y <= f32::EPSILON {
        return vec2(height, height);
    }
    vec2(height * clip.pixel_size.x / clip.pixel_size.y, height)
}

fn draw_clip_in_rect(
    painter: &egui::Painter,
    rect: Rect,
    clip: &SpriteClip,
    frame: &SpriteFrame,
    tint: Color32,
    flip_x: bool,
) {
    let draw_size = fit_size_with_aspect(rect.size(), clip.pixel_size);
    let draw_rect = Rect::from_center_size(rect.center(), draw_size);
    let uv = if flip_x {
        Rect::from_min_max(pos2(1.0, 0.0), pos2(0.0, 1.0))
    } else {
        Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))
    };
    painter.image(frame.texture.id(), draw_rect, uv, tint);
}

fn draw_static_clip_centered(
    painter: &egui::Painter,
    world: Rect,
    clip: &SpriteClip,
    pos: Vec2,
    size: Vec2,
    tint: Color32,
    flip_x: bool,
) {
    let rect = Rect::from_center_size(world_point(world, pos), world_size(world, size));
    draw_clip_in_rect(painter, rect, clip, clip.first(), tint, flip_x);
}

fn draw_clip_centered(
    painter: &egui::Painter,
    world: Rect,
    clip: &SpriteClip,
    pos: Vec2,
    max_size: Vec2,
    phase: f32,
    ticks_per_frame: f32,
    tint: Color32,
    flip_x: bool,
) {
    let rect = Rect::from_center_size(world_point(world, pos), world_size(world, max_size));
    let frame = clip.frame_at(phase, ticks_per_frame);
    draw_clip_in_rect(painter, rect, clip, frame, tint, flip_x);
}

fn fit_world_rect(outer: Rect, scale_hint: f32) -> Rect {
    let scale = (outer.width() / WORLD_W)
        .min(outer.height() / WORLD_H)
        .min(scale_hint.max(1.0) * 4.0)
        .max(0.1);
    Rect::from_center_size(outer.center(), vec2(WORLD_W * scale, WORLD_H * scale))
}

fn world_point(world: Rect, pos: Vec2) -> Pos2 {
    pos2(
        world.left() + pos.x / WORLD_W * world.width(),
        world.top() + pos.y / WORLD_H * world.height(),
    )
}

fn world_size(world: Rect, size: Vec2) -> Vec2 {
    vec2(
        size.x / WORLD_W * world.width(),
        size.y / WORLD_H * world.height(),
    )
}

fn world_rect_from_entity(world: Rect, pos: Vec2, size: Vec2) -> Rect {
    Rect::from_center_size(world_point(world, pos), world_size(world, size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_stage_one_assets_decode() {
        let checks = [
            ("pipboy_anim", HERO_STAND_FRAMES),
            ("pipboy_anim", HERO_RUN_FRAMES),
            ("pipboy_anim", HERO_CLIMB_FRAMES),
            ("pipboy_anim", HERO_JUMP_FRAMES),
            ("pipboy_anim", HERO_FALL_FRAMES),
            ("pipboy_anim", HERO_POWER_ARMOR_STAND_FRAMES),
            ("pipboy_anim", HERO_POWER_ARMOR_RUN_FRAMES),
            ("boss_stage1", BOSS_IDLE_FRAMES),
            ("girl", GIRL_WAVE_FRAMES),
            ("bomb_thrown", FLYING_BOMB_FRAMES),
            ("bomb_rolled", ROLLING_BOMB_FRAMES),
            ("helmet", HELMET_FRAMES),
            ("life", LIFE_FRAMES),
            ("pause", PAUSE_FRAMES),
            ("boss_stage1", BOSS_SHOW_BOMBS_FRAMES),
            ("boss_stage1", BOSS_THROW_LEFT_FRAMES),
            ("boss_stage1", BOSS_THROW_RIGHT_FRAMES),
            ("stage_floor_505", STAGE_FLOOR_505_FRAMES),
            ("stage_floor_507", STAGE_FLOOR_507_FRAMES),
            ("stage_floor_510", STAGE_FLOOR_510_FRAMES),
            ("stage_floor_513", STAGE_FLOOR_513_FRAMES),
            ("stage_floor_516", STAGE_FLOOR_516_FRAMES),
            ("stage_floor_518", STAGE_FLOOR_518_FRAMES),
            ("stage_ladder_521", STAGE_LADDER_521_FRAMES),
            ("stage_ladder_524", STAGE_LADDER_524_FRAMES),
            ("stage_ladder_527", STAGE_LADDER_527_FRAMES),
            ("stage_ladder_530", STAGE_LADDER_530_FRAMES),
            ("stage_ladder_533", STAGE_LADDER_533_FRAMES),
            ("stage_support_325", STAGE_SUPPORT_325_FRAMES),
        ];

        for (dir, frames) in checks {
            for frame in frames {
                let image = load_png_image(&asset_frame_path(dir, *frame));
                assert!(image.width() > 0 && image.height() > 0);
            }
        }
    }

    #[test]
    fn game_starts_on_title_screen() {
        let game = RedMenaceGame::new(RedMenaceConfig::default());
        assert_eq!(game.phase(), GamePhase::Title);
        assert_eq!(game.level(), 1);
        assert_eq!(game.stage(), 1);
        assert_eq!(game.lives(), LIVES_INIT);
        assert_eq!(game.bonus_timer(), BONUS_TIMER_INIT);
    }

    #[test]
    fn enter_starts_intro_phase() {
        let mut game = RedMenaceGame::new(RedMenaceConfig::default());
        game.update(
            &GameInput {
                start: true,
                ..GameInput::default()
            },
            1.0 / 60.0,
        );
        assert_eq!(game.phase(), GamePhase::Intro);
    }

    #[test]
    fn intro_auto_advances_to_transition() {
        let mut game = RedMenaceGame::new(RedMenaceConfig::default());
        game.start_intro();
        for _ in 0..((INTRO_SECS * 60.0) as usize + 2) {
            game.update(&GameInput::default(), 1.0 / 60.0);
        }
        assert_eq!(game.phase(), GamePhase::Transition);
    }

    #[test]
    fn completing_third_stage_advances_level() {
        let mut game = RedMenaceGame::new(RedMenaceConfig::default());
        game.phase = GamePhase::Level;
        game.data.stage = 2;
        game.data.level = 4;
        game.data.bonus_timer = 500;
        game.update(
            &GameInput {
                action_pressed: true,
                ..GameInput::default()
            },
            1.0 / 60.0,
        );
        assert_eq!(game.phase(), GamePhase::Transition);
        assert_eq!(game.level(), 5);
        assert_eq!(game.stage(), 1);
        assert_eq!(game.score(), 500);
    }

    #[test]
    fn losing_last_life_enters_game_over() {
        let mut game = RedMenaceGame::new(RedMenaceConfig::default());
        game.phase = GamePhase::Level;
        game.data.stage = 1;
        game.data.lives = 1;
        game.data.score = 1234;
        game.update(
            &GameInput {
                start: true,
                ..GameInput::default()
            },
            1.0 / 60.0,
        );
        assert_eq!(game.phase(), GamePhase::GameOver);
        assert_eq!(game.high_score(), 1234);
    }

    #[test]
    fn stage_one_clears_when_hero_reaches_goal() {
        let mut stage = StageOneState::new();
        stage.hero.pos = stage.girl_pos;
        let outcome = stage.step_frame(&GameInput::default(), 1, &mut SmallRng::seed_from_u64(1));
        assert!(outcome.cleared);
    }
}

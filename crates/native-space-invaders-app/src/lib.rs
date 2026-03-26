use egui::{
    self, pos2, vec2, Align2, Color32, Context, FontFamily, FontId, Key, Pos2, Rect, Sense, Stroke,
    TextureHandle, Ui, Vec2,
};
use rand::{rngs::SmallRng, seq::SliceRandom, Rng, SeedableRng};
use robcos_hosted_addon_contract::{
    HostedAddonFrame, HostedAddonSize, HostedColor, HostedDrawCommand, HostedInputEvent,
    HostedTextAlign,
};
use std::{collections::HashMap, path::PathBuf};

pub const BUILTIN_ZETA_INVADERS_GAME: &str = "Zeta Invaders";

const WORLD_W: f32 = 224.0;
const WORLD_H: f32 = 256.0;
const ZETA_STAGE_W: f32 = 826.0;
const ZETA_STAGE_H: f32 = 700.0;
const SWARM_MARGIN: f32 = 16.0;
const PLAYER_SIZE: Vec2 = Vec2::new(14.0, 12.0);
const ALIEN_SIZE: Vec2 = Vec2::new(12.0, 10.0);
const UFO_SIZE: Vec2 = Vec2::new(14.0, 8.0);
const BULLET_HITBOX_SIZE: Vec2 = Vec2::new(2.0, 6.0);
const PLAYER_BULLET_DRAW_SIZE: Vec2 = Vec2::new(0.9, 7.5);
const ALIEN_BULLET_DRAW_SIZE: Vec2 = Vec2::new(1.2, 7.5);
const BARN_PIECE_SIZE: Vec2 =
    Vec2::new(12.0 * WORLD_W / ZETA_STAGE_W, 12.0 * WORLD_H / ZETA_STAGE_H);
const BARN_COLS: usize = 6;
const BARN_ROWS: usize = 3;
const PLAYER_Y: f32 = 226.0;
const BARN_Y: f32 = 540.0 * WORLD_H / ZETA_STAGE_H;
const PLAYER_SPEED: f32 = 110.0;
const PLAYER_BULLET_SPEED: f32 = 210.0;
const ALIEN_SLOW_BULLET_SPEED: f32 = PLAYER_BULLET_SPEED * (260.0 / 420.0);
const ALIEN_FAST_BULLET_SPEED: f32 = PLAYER_BULLET_SPEED * (345.0 / 420.0);
const ALIEN_SLOW_BULLET_CHANCE: f64 = 0.7;
const UFO_SPEED: f32 = 48.0;
const UFO_SCORE: u32 = 400;
const ALIEN_STEP_X: f32 = 3.0;
const ALIEN_STEP_DOWN: f32 = 4.0;
const ALIEN_SPACING_X: f32 = 16.0;
const ALIEN_SPACING_Y: f32 = 16.0;
const READY_SECS: f32 = 0.9;
const RESPAWN_SECS: f32 = 0.85;
const PLAYER_FLASH_SECS: f32 = 0.18;
const PLAYER_RELOAD_SECS: f32 = 0.26;
const PLAYER_EXPLOSION_SECS: f32 = 0.55;
const EFFECT_SECS: f32 = 0.32;
const TITLE_ANIM_SPEED: f32 = 1.0;
const UFO_ANIM_SPEED: f32 = 0.75;
const MAX_ALIEN_BULLETS: usize = 3;
const ALIEN_ROWS: usize = 5;
const ALIEN_COLS: usize = 11;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GamePhase {
    Title,
    Ready,
    Playing,
    Paused,
    Respawning,
    GameOver,
}

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub player: Color32,
    pub alien: Color32,
    pub enemy: Color32,
    pub bullet: Color32,
    pub ui: Color32,
    pub barrier: Color32,
    pub neutral: Color32,
}

impl Default for Theme {
    fn default() -> Self {
        let green = Color32::from_rgb(120, 255, 120);
        let dim_green = Color32::from_rgb(72, 138, 72);
        Self {
            player: green,
            alien: green,
            enemy: green,
            bullet: green,
            ui: green,
            barrier: green,
            neutral: dim_green,
        }
    }
}

impl Theme {
    fn tint(self, role: TintRole) -> Color32 {
        match role {
            TintRole::Player => self.player,
            TintRole::Alien => self.alien,
            TintRole::Enemy => self.enemy,
            TintRole::Bullet => self.bullet,
            TintRole::Ui => self.ui,
            TintRole::Barrier => self.barrier,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpaceInvadersConfig {
    pub scale: f32,
    pub theme: Theme,
}

impl Default for SpaceInvadersConfig {
    fn default() -> Self {
        Self {
            scale: 2.0,
            theme: Theme::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GameInput {
    pub left: bool,
    pub right: bool,
    pub fire: bool,
    pub start: bool,
    pub pause: bool,
}

#[derive(Clone)]
pub struct SpaceInvadersGame {
    config: SpaceInvadersConfig,
    theme: Theme,
    rng: SmallRng,
    fire_held: bool,
    high_score: u32,
    state: GameState,
}

#[derive(Clone)]
pub struct AtlasTextures {
    textures: HashMap<FrameId, TextureHandle>,
    catalog: SpriteCatalog,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum FrameId {
    PlayerIdle,
    PlayerMoveLeft,
    PlayerMoveRight,
    PlayerShoot,
    PlayerExplosion1,
    PlayerExplosion2,
    AlienSquid1,
    AlienSquid2,
    AlienCrab1,
    AlienCrab2,
    AlienOcto1,
    AlienOcto2,
    AlienExplosion,
    PlayerBullet,
    AlienBullet1,
    AlienBullet2,
    Spark1,
    Spark2,
    ExplosionSmall1,
    ExplosionSmall2,
    ExplosionSmall3,
    ExplosionSmall4,
    ExplosionSmall5,
    ExplosionSmall6,
    BarrierFull,
    BarrierDamage1,
    BarrierDamage2,
    BarrierDamage3,
    BarrierChunk,
    BarnPiece00,
    BarnPiece01,
    BarnPiece02,
    BarnPiece03,
    BarnPiece04,
    BarnPiece05,
    BarnPiece06,
    BarnPiece07,
    BarnPiece08,
    BarnPiece09,
    BarnPiece10,
    BarnPiece11,
    BarnPiece12,
    BarnPiece13,
    BarnPiece14,
    BarnPiece15,
    BarnPiece16,
    BarnPiece17,
    UfoIdle,
    UfoFlash,
    UfoExplosion,
    LifeIcon,
    ScoreIcon,
    WaveIcon,
    ReadyIcon,
    Title01,
    Title02,
    Title03,
    Title04,
    Title05,
    Title06,
    Title07,
    Title08,
    Title09,
    Title10,
    Title11,
    Title12,
    Title13,
    Title14,
    Title15,
    Title16,
    Title17,
    Title18,
    Title19,
    Title20,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum AnimationId {
    PlayerExplode,
    AlienSquid,
    AlienCrab,
    AlienOcto,
    AlienBullet,
    Spark,
    ExplosionSmall,
    UfoMove,
    TitleScreen,
}

#[derive(Clone, Copy, Debug)]
enum TintRole {
    Player,
    Alien,
    Enemy,
    Bullet,
    Ui,
    Barrier,
}

#[derive(Clone)]
struct Frame {
    tint_role: TintRole,
}

#[derive(Clone)]
struct Animation {
    frames: Vec<FrameId>,
    tick: u32,
}

#[derive(Clone, Default)]
struct SpriteCatalog {
    frames: HashMap<FrameId, Frame>,
    animations: HashMap<AnimationId, Animation>,
}

#[derive(Clone, Copy)]
struct Player {
    pos: Vec2,
    visual_dir: i8,
    flash_timer: f32,
    reload_timer: f32,
}

#[derive(Clone, Copy)]
struct Bullet {
    pos: Vec2,
    prev_pos: Vec2,
    vel: Vec2,
    age: f32,
    kind: BulletKind,
}

#[derive(Clone, Copy)]
enum BulletKind {
    Player,
    AlienSlow,
    AlienFast,
}

#[derive(Clone, Copy, Debug)]
enum AlienKind {
    Squid,
    Crab,
    Octo,
}

#[derive(Clone, Copy)]
struct Alien {
    row: usize,
    col: usize,
    kind: AlienKind,
    alive: bool,
}

#[derive(Clone)]
struct AlienFormation {
    aliens: Vec<Alien>,
    offset: Vec2,
    direction: f32,
    step_timer: f32,
    anim_frame_idx: usize,
}

#[derive(Clone)]
struct Barn {
    origin: Vec2,
    pieces_alive: [bool; BARN_ROWS * BARN_COLS],
}

#[derive(Clone, Copy)]
enum EffectKind {
    Spark,
    ExplosionSmall,
}

#[derive(Clone, Copy)]
struct Effect {
    pos: Vec2,
    timer: f32,
    duration: f32,
    kind: EffectKind,
}

#[derive(Clone, Copy)]
enum UfoState {
    Flying,
    Exploding { timer: f32 },
}

#[derive(Clone, Copy)]
struct Ufo {
    pos: Vec2,
    direction: f32,
    score_value: u32,
    state: UfoState,
}

#[derive(Clone, Copy)]
struct PlayerExplosion {
    pos: Vec2,
    timer: f32,
    duration: f32,
}

#[derive(Clone)]
struct GameState {
    player: Player,
    player_bullet: Option<Bullet>,
    alien_bullets: Vec<Bullet>,
    formation: AlienFormation,
    barns: Vec<Barn>,
    effects: Vec<Effect>,
    ufo: Option<Ufo>,
    player_explosion: Option<PlayerExplosion>,
    phase: GamePhase,
    phase_timer: f32,
    score: u32,
    lives: u8,
    wave: u32,
    animation_ticks: f32,
    ufo_cooldown: f32,
    alien_shot_timer: f32,
}

impl SpaceInvadersGame {
    pub fn new(config: SpaceInvadersConfig) -> Self {
        let theme = config.theme;
        Self {
            config,
            theme,
            rng: SmallRng::from_entropy(),
            fire_held: false,
            high_score: 0,
            state: GameState::new(),
        }
    }

    pub fn reset(&mut self) {
        self.state = GameState::new();
        self.fire_held = false;
    }

    fn begin_run(&mut self) {
        self.state = GameState::new();
        self.state.phase = GamePhase::Ready;
        self.state.phase_timer = READY_SECS;
        self.fire_held = false;
    }

    pub fn set_theme(&mut self, theme: Theme) {
        self.theme = theme;
    }

    pub fn phase(&self) -> GamePhase {
        self.state.phase
    }

    pub fn score(&self) -> u32 {
        self.state.score
    }

    pub fn lives(&self) -> u8 {
        self.state.lives
    }

    pub fn wave(&self) -> u32 {
        self.state.wave
    }

    pub fn high_score(&self) -> u32 {
        self.high_score
    }

    pub fn update(&mut self, input: &GameInput, dt: f32) {
        let dt = dt.clamp(1.0 / 240.0, 1.0 / 20.0);
        let fire_pressed = input.fire && !self.fire_held;
        self.fire_held = input.fire;

        if self.state.phase == GamePhase::Title {
            self.state.animation_ticks += dt * 60.0;
            if input.start {
                self.begin_run();
            }
            return;
        }

        if input.pause {
            match self.state.phase {
                GamePhase::Playing => {
                    self.state.phase = GamePhase::Paused;
                    return;
                }
                GamePhase::Paused => {
                    self.state.phase = GamePhase::Playing;
                    return;
                }
                _ => {}
            }
        }

        if self.state.phase == GamePhase::Paused {
            return;
        }

        self.state.animation_ticks += dt * 60.0;
        self.state.player.flash_timer = (self.state.player.flash_timer - dt).max(0.0);
        self.state.player.reload_timer = (self.state.player.reload_timer - dt).max(0.0);

        self.update_effects(dt);
        self.update_player_explosion(dt);
        self.update_ufo(dt);

        if self.state.phase == GamePhase::GameOver {
            if input.start {
                self.begin_run();
            } else if fire_pressed {
                self.reset();
            }
            return;
        }

        if matches!(self.state.phase, GamePhase::Ready | GamePhase::Respawning) {
            self.state.phase_timer = (self.state.phase_timer - dt).max(0.0);
            if self.state.phase_timer <= f32::EPSILON {
                self.state.phase = GamePhase::Playing;
                self.state.player.pos = player_spawn_pos();
                self.state.player.visual_dir = 0;
            } else {
                return;
            }
        }

        self.update_player(input, fire_pressed, dt);
        self.update_swarm(dt);
        self.update_player_bullet(dt);
        self.update_alien_bullets(dt);
        self.maybe_spawn_alien_bullet(dt);
        self.resolve_collisions();

        if self.swarm_reached_player_zone() {
            self.state.phase = GamePhase::GameOver;
        } else if self.alive_aliens() == 0 {
            self.start_next_wave();
        }
    }

    pub fn draw(&self, ui: &mut Ui, atlas: &AtlasTextures) {
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

        painter.rect_filled(outer, 6.0, Color32::BLACK);

        if self.state.phase == GamePhase::Title {
            self.draw_title_screen(&painter, world, atlas);
            return;
        }

        self.paint_starfield(&painter, world);
        painter.rect_stroke(world, 0.0, Stroke::new(2.0, self.theme.neutral));

        self.draw_hud(&painter, world, atlas);
        self.draw_barns(&painter, world, atlas);
        self.draw_ufo(&painter, world, atlas);
        self.draw_aliens(&painter, world, atlas);
        self.draw_player_bullet(&painter, world, atlas);
        self.draw_alien_bullets(&painter, world, atlas);
        self.draw_effects(&painter, world, atlas);
        self.draw_player(&painter, world, atlas);
        self.draw_overlay(&painter, world, atlas);
    }

    pub fn hosted_frame(&self) -> HostedAddonFrame {
        let mut commands = Vec::new();

        if self.state.phase == GamePhase::Title {
            commands.push(HostedDrawCommand::Image {
                x: 0.0,
                y: 0.0,
                width: WORLD_W,
                height: WORLD_H,
                asset_path: hosted_asset_path(self.title_frame()).to_string(),
            });
            return HostedAddonFrame {
                size: HostedAddonSize {
                    width: WORLD_W,
                    height: WORLD_H,
                },
                clear: Some(hosted_color(Color32::BLACK)),
                commands,
                status_line: Some(
                    "ARROWS/A D MOVE   SPACE FIRE   ENTER START   ESC/P PAUSE".to_string(),
                ),
            };
        }

        push_starfield_commands(&mut commands, self.theme);
        push_rect_border(
            &mut commands,
            Rect::from_min_max(pos2(0.0, 0.0), pos2(WORLD_W, WORLD_H)),
            self.theme.neutral,
            2.0,
        );
        self.push_hosted_hud(&mut commands);
        self.push_hosted_barns(&mut commands);
        self.push_hosted_ufo(&mut commands);
        self.push_hosted_aliens(&mut commands);
        self.push_hosted_player_bullet(&mut commands);
        self.push_hosted_alien_bullets(&mut commands);
        self.push_hosted_effects(&mut commands);
        self.push_hosted_player(&mut commands);
        self.push_hosted_overlay(&mut commands);

        HostedAddonFrame {
            size: HostedAddonSize {
                width: WORLD_W,
                height: WORLD_H,
            },
            clear: Some(hosted_color(Color32::BLACK)),
            commands,
            status_line: Some(format!(
                "SCORE {:05}   HI {:05}   WAVE {:02}   LIVES {}",
                self.state.score, self.high_score, self.state.wave, self.state.lives
            )),
        }
    }

    fn title_frame(&self) -> FrameId {
        title_animation_frame(self.state.animation_ticks * TITLE_ANIM_SPEED)
    }

    fn push_hosted_hud(&self, commands: &mut Vec<HostedDrawCommand>) {
        let score_icon = Rect::from_center_size(pos2(14.0, 10.0), vec2(12.0, 12.0));
        push_image(commands, score_icon, FrameId::ScoreIcon);
        push_text(
            commands,
            score_icon.right() + 8.0,
            score_icon.center().y,
            &format!("{:05}", self.state.score),
            14.0,
            self.theme.ui,
            Align2::LEFT_CENTER,
        );

        push_text(
            commands,
            WORLD_W * 0.5,
            10.0,
            &format!("HI {:05}", self.high_score),
            14.0,
            self.theme.ui,
            Align2::CENTER_CENTER,
        );

        let wave_icon =
            Rect::from_center_size(pos2(WORLD_W * 0.5 - 14.0, 24.0), vec2(12.0, 12.0));
        push_image(commands, wave_icon, FrameId::WaveIcon);
        push_text(
            commands,
            wave_icon.right() + 8.0,
            wave_icon.center().y,
            &format!("W{:02}", self.state.wave),
            12.0,
            self.theme.neutral,
            Align2::LEFT_CENTER,
        );

        let mut life_x = WORLD_W - 16.0;
        for _ in 0..self.state.lives {
            push_image(
                commands,
                Rect::from_center_size(pos2(life_x, 10.0), vec2(12.0, 12.0)),
                FrameId::LifeIcon,
            );
            life_x -= 18.0;
        }
    }

    fn push_hosted_barns(&self, commands: &mut Vec<HostedDrawCommand>) {
        for barn in &self.state.barns {
            for idx in 0..barn.pieces_alive.len() {
                if !barn.pieces_alive[idx] {
                    continue;
                }
                let piece_rect = barn.piece_rect(idx);
                let (frame, _) = barn_piece_visual(idx);
                push_image(commands, piece_rect, frame);
            }
        }
    }

    fn push_hosted_ufo(&self, commands: &mut Vec<HostedDrawCommand>) {
        let Some(ufo) = self.state.ufo else {
            return;
        };
        let frame = match ufo.state {
            UfoState::Flying => ufo_animation_frame(
                self.state.animation_ticks * UFO_ANIM_SPEED
                    + if ufo.direction < 0.0 { 4.0 } else { 0.0 },
            ),
            UfoState::Exploding { .. } => FrameId::UfoExplosion,
        };
        push_image(commands, game_rect_from_entity(ufo.pos, UFO_SIZE), frame);
    }

    fn push_hosted_aliens(&self, commands: &mut Vec<HostedDrawCommand>) {
        for alien in self
            .state
            .formation
            .aliens
            .iter()
            .copied()
            .filter(|alien| alien.alive)
        {
            let frame = match alien.kind {
                AlienKind::Squid => alien_squid_frame(self.state.formation.anim_frame_idx),
                AlienKind::Crab => alien_crab_frame(self.state.formation.anim_frame_idx),
                AlienKind::Octo => alien_octo_frame(self.state.formation.anim_frame_idx),
            };
            push_image(
                commands,
                game_rect_from_entity(
                    alien_world_pos(alien, self.state.formation.offset),
                    ALIEN_SIZE,
                ),
                frame,
            );
        }
    }

    fn push_hosted_player_bullet(&self, commands: &mut Vec<HostedDrawCommand>) {
        let Some(bullet) = self.state.player_bullet else {
            return;
        };
        push_image(
            commands,
            game_rect_from_entity(bullet.pos, PLAYER_BULLET_DRAW_SIZE),
            FrameId::PlayerBullet,
        );
    }

    fn push_hosted_alien_bullets(&self, commands: &mut Vec<HostedDrawCommand>) {
        for bullet in &self.state.alien_bullets {
            let anim_ticks = match bullet.kind {
                BulletKind::AlienSlow => bullet.age * 42.0,
                BulletKind::AlienFast => bullet.age * 72.0,
                BulletKind::Player => bullet.age * 60.0,
            };
            push_image(
                commands,
                game_rect_from_entity(bullet.pos, ALIEN_BULLET_DRAW_SIZE),
                alien_bullet_frame(anim_ticks),
            );
        }
    }

    fn push_hosted_effects(&self, commands: &mut Vec<HostedDrawCommand>) {
        for effect in &self.state.effects {
            let progress = 1.0 - effect.timer / effect.duration.max(f32::EPSILON);
            let frame = match effect.kind {
                EffectKind::Spark => spark_frame(progress * 24.0),
                EffectKind::ExplosionSmall => explosion_small_frame(progress * 24.0),
            };
            push_image(commands, game_rect_from_entity(effect.pos, vec2(14.0, 14.0)), frame);
        }
    }

    fn push_hosted_player(&self, commands: &mut Vec<HostedDrawCommand>) {
        if let Some(explosion) = self.state.player_explosion {
            let progress = 1.0 - explosion.timer / explosion.duration.max(f32::EPSILON);
            push_image(
                commands,
                game_rect_from_entity(explosion.pos, PLAYER_SIZE),
                player_explode_frame(progress * 24.0),
            );
            return;
        }

        if self.state.phase == GamePhase::Respawning {
            return;
        }

        let frame = if self.state.player.flash_timer > 0.0 {
            FrameId::PlayerShoot
        } else if self.state.player.visual_dir < 0 {
            FrameId::PlayerMoveLeft
        } else if self.state.player.visual_dir > 0 {
            FrameId::PlayerMoveRight
        } else {
            FrameId::PlayerIdle
        };
        push_image(
            commands,
            game_rect_from_entity(self.state.player.pos, PLAYER_SIZE),
            frame,
        );
    }

    fn push_hosted_overlay(&self, commands: &mut Vec<HostedDrawCommand>) {
        let (title, subtitle) = match self.state.phase {
            GamePhase::Title => return,
            GamePhase::Ready => ("READY", "Clear the wave"),
            GamePhase::Paused => ("PAUSED", "Press P or Esc to resume"),
            GamePhase::Respawning => ("HIT", "Get back in position"),
            GamePhase::GameOver => ("GAME OVER", "Press fire to restart"),
            GamePhase::Playing => return,
        };

        let overlay = Rect::from_center_size(
            pos2(WORLD_W * 0.5, WORLD_H * 0.5),
            vec2(WORLD_W * 0.54, 54.0),
        );
        push_rect(commands, overlay, Color32::BLACK);
        push_rect_border(commands, overlay, self.theme.ui, 2.0);
        let icon_rect = Rect::from_center_size(
            pos2(overlay.center().x, overlay.top() + 16.0),
            vec2(18.0, 18.0),
        );
        push_image(commands, icon_rect, FrameId::ReadyIcon);
        push_text(
            commands,
            overlay.center().x,
            overlay.top() + 32.0,
            title,
            16.0,
            self.theme.ui,
            Align2::CENTER_TOP,
        );
        push_text(
            commands,
            overlay.center().x,
            overlay.bottom() - 10.0,
            subtitle,
            12.0,
            self.theme.neutral,
            Align2::CENTER_BOTTOM,
        );
    }

    fn draw_title_screen(&self, painter: &egui::Painter, world: Rect, atlas: &AtlasTextures) {
        let frame = atlas.animation_frame_for_tick_clamped(
            AnimationId::TitleScreen,
            self.state.animation_ticks * TITLE_ANIM_SPEED,
        );
        atlas.paint_frame(painter, world, frame, self.theme, false);
    }

    fn update_player(&mut self, input: &GameInput, fire_pressed: bool, dt: f32) {
        let mut dir = 0.0;
        if input.left {
            dir -= 1.0;
        }
        if input.right {
            dir += 1.0;
        }
        self.state.player.pos.x += dir * PLAYER_SPEED * dt;
        self.state.player.pos.x = self.state.player.pos.x.clamp(12.0, WORLD_W - 12.0);
        self.state.player.visual_dir = dir.signum() as i8;

        if fire_pressed
            && self.state.player.reload_timer <= f32::EPSILON
            && self.state.player_bullet.is_none()
        {
            let bullet_pos = self.state.player.pos + vec2(0.0, -PLAYER_SIZE.y * 0.65);
            self.state.player_bullet = Some(Bullet {
                pos: bullet_pos,
                prev_pos: bullet_pos,
                vel: vec2(0.0, -PLAYER_BULLET_SPEED),
                age: 0.0,
                kind: BulletKind::Player,
            });
            self.state.player.flash_timer = PLAYER_FLASH_SECS;
            self.state.player.reload_timer = PLAYER_RELOAD_SECS;
        }
    }

    fn add_score(&mut self, points: u32) {
        self.state.score = self.state.score.saturating_add(points);
        self.high_score = self.high_score.max(self.state.score);
    }

    fn update_swarm(&mut self, dt: f32) {
        self.state.formation.step_timer += dt;
        let interval = self.alien_step_interval();
        while self.state.formation.step_timer >= interval {
            self.state.formation.step_timer -= interval;
            self.advance_swarm_step();
        }
    }

    fn update_player_bullet(&mut self, dt: f32) {
        if let Some(bullet) = &mut self.state.player_bullet {
            bullet.prev_pos = bullet.pos;
            bullet.pos += bullet.vel * dt;
            bullet.age += dt;
            if bullet.pos.y < -12.0 {
                self.state.player_bullet = None;
            }
        }
    }

    fn update_alien_bullets(&mut self, dt: f32) {
        for bullet in &mut self.state.alien_bullets {
            bullet.prev_pos = bullet.pos;
            bullet.pos += bullet.vel * dt;
            bullet.age += dt;
        }
        self.state
            .alien_bullets
            .retain(|bullet| bullet.pos.y <= WORLD_H + 14.0);
    }

    fn maybe_spawn_alien_bullet(&mut self, dt: f32) {
        self.state.alien_shot_timer -= dt;
        if self.state.alien_shot_timer > 0.0 || self.state.alien_bullets.len() >= MAX_ALIEN_BULLETS
        {
            return;
        }

        let Some(origin) = self.random_bottom_shooter_pos() else {
            return;
        };
        let bullet_kind = if self.rng.gen_bool(ALIEN_SLOW_BULLET_CHANCE) {
            BulletKind::AlienSlow
        } else {
            BulletKind::AlienFast
        };
        let bullet_pos = origin + vec2(0.0, ALIEN_SIZE.y * 0.45);
        self.state.alien_bullets.push(Bullet {
            pos: bullet_pos,
            prev_pos: bullet_pos,
            vel: vec2(
                0.0,
                match bullet_kind {
                    BulletKind::AlienSlow => ALIEN_SLOW_BULLET_SPEED,
                    BulletKind::AlienFast => ALIEN_FAST_BULLET_SPEED,
                    BulletKind::Player => PLAYER_BULLET_SPEED,
                } + self.state.wave as f32 * 6.0,
            ),
            age: 0.0,
            kind: bullet_kind,
        });
        self.state.alien_shot_timer = self.alien_shot_interval();
    }

    fn update_ufo(&mut self, dt: f32) {
        match &mut self.state.ufo {
            Some(ufo) => match &mut ufo.state {
                UfoState::Flying => {
                    ufo.pos.x += ufo.direction * UFO_SPEED * dt;
                    if ufo.pos.x < -20.0 || ufo.pos.x > WORLD_W + 20.0 {
                        self.state.ufo = None;
                        self.state.ufo_cooldown = self.rng.gen_range(10.0..18.0);
                    }
                }
                UfoState::Exploding { timer } => {
                    *timer -= dt;
                    if *timer <= 0.0 {
                        self.state.ufo = None;
                        self.state.ufo_cooldown = self.rng.gen_range(12.0..20.0);
                    }
                }
            },
            None => {
                if self.state.phase != GamePhase::Playing {
                    return;
                }
                self.state.ufo_cooldown -= dt;
                if self.state.ufo_cooldown <= 0.0 {
                    let direction = if self.rng.gen_bool(0.5) { 1.0 } else { -1.0 };
                    let start_x = if direction > 0.0 {
                        -10.0
                    } else {
                        WORLD_W + 10.0
                    };
                    self.state.ufo = Some(Ufo {
                        pos: vec2(start_x, 24.0),
                        direction,
                        score_value: UFO_SCORE,
                        state: UfoState::Flying,
                    });
                }
            }
        }
    }

    fn update_effects(&mut self, dt: f32) {
        for effect in &mut self.state.effects {
            effect.timer -= dt;
        }
        self.state.effects.retain(|effect| effect.timer > 0.0);
    }

    fn update_player_explosion(&mut self, dt: f32) {
        if let Some(explosion) = &mut self.state.player_explosion {
            explosion.timer -= dt;
            if explosion.timer <= 0.0 {
                self.state.player_explosion = None;
            }
        }
    }

    fn resolve_collisions(&mut self) {
        self.resolve_bullet_vs_bullet();
        self.resolve_player_bullet_hits();
        self.resolve_alien_bullet_hits();
    }

    fn resolve_bullet_vs_bullet(&mut self) {
        let Some(player_bullet) = self.state.player_bullet else {
            return;
        };

        let player_rect = swept_rect(
            player_bullet.prev_pos,
            player_bullet.pos,
            BULLET_HITBOX_SIZE,
        );
        let hit_index = self.state.alien_bullets.iter().position(|bullet| {
            swept_rect(bullet.prev_pos, bullet.pos, BULLET_HITBOX_SIZE).intersects(player_rect)
        });

        if let Some(index) = hit_index {
            let bullet = self.state.alien_bullets.remove(index);
            self.state.player_bullet = None;
            self.spawn_effect(midpoint(player_bullet.pos, bullet.pos), EffectKind::Spark);
        }
    }

    fn resolve_player_bullet_hits(&mut self) {
        let Some(bullet) = self.state.player_bullet else {
            return;
        };
        let bullet_rect = swept_rect(bullet.prev_pos, bullet.pos, BULLET_HITBOX_SIZE);

        if self.hit_ufo(bullet_rect) {
            self.state.player_bullet = None;
            return;
        }

        if self.hit_alien(bullet_rect) {
            self.state.player_bullet = None;
            return;
        }

        if self.hit_barn(bullet_rect, true) {
            self.state.player_bullet = None;
        }
    }

    fn resolve_alien_bullet_hits(&mut self) {
        let bullets = std::mem::take(&mut self.state.alien_bullets);
        let mut remaining = Vec::with_capacity(bullets.len());
        let player_rect = entity_rect(self.state.player.pos, PLAYER_SIZE);
        let mut player_hit = false;

        for bullet in bullets {
            let bullet_rect = swept_rect(bullet.prev_pos, bullet.pos, BULLET_HITBOX_SIZE);

            if self.hit_barn(bullet_rect, false) {
                continue;
            }

            if self.state.phase == GamePhase::Playing && bullet_rect.intersects(player_rect) {
                self.on_player_hit();
                player_hit = true;
                break;
            }

            remaining.push(bullet);
        }

        self.state.alien_bullets = if player_hit { Vec::new() } else { remaining };
    }

    fn hit_ufo(&mut self, bullet_rect: Rect) -> bool {
        let Some(ufo) = &mut self.state.ufo else {
            return false;
        };
        if !matches!(ufo.state, UfoState::Flying) {
            return false;
        }
        let ufo_rect = entity_rect(ufo.pos, UFO_SIZE);
        if !ufo_rect.intersects(bullet_rect) {
            return false;
        }

        let score_value = ufo.score_value;
        let explosion_pos = ufo.pos;
        ufo.state = UfoState::Exploding { timer: 0.45 };
        self.add_score(score_value);
        self.spawn_effect(explosion_pos, EffectKind::Spark);
        true
    }

    fn hit_alien(&mut self, bullet_rect: Rect) -> bool {
        let formation_offset = self.state.formation.offset;
        let hit_index = self.state.formation.aliens.iter().position(|alien| {
            alien.alive && alien_rect_with_offset(*alien, formation_offset).intersects(bullet_rect)
        });

        let Some(index) = hit_index else {
            return false;
        };

        let alien = &mut self.state.formation.aliens[index];
        alien.alive = false;
        let alien_pos = alien_world_pos(*alien, formation_offset);
        let score_value = alien_kind_score(alien.kind);
        self.add_score(score_value);
        self.spawn_effect(alien_pos, EffectKind::ExplosionSmall);
        true
    }

    fn hit_barn(&mut self, bullet_rect: Rect, from_below: bool) -> bool {
        for barn in &mut self.state.barns {
            let Some(idx) = barn.destroy_piece_from_hit(bullet_rect, from_below) else {
                continue;
            };
            let center = barn.piece_rect(idx).center().to_vec2();
            self.spawn_effect(center, EffectKind::Spark);
            return true;
        }
        false
    }

    fn on_player_hit(&mut self) {
        self.state.player_bullet = None;
        self.state.alien_bullets.clear();
        self.state.player_explosion = Some(PlayerExplosion {
            pos: self.state.player.pos,
            timer: PLAYER_EXPLOSION_SECS,
            duration: PLAYER_EXPLOSION_SECS,
        });

        if self.state.lives > 1 {
            self.state.lives -= 1;
            self.state.phase = GamePhase::Respawning;
            self.state.phase_timer = RESPAWN_SECS;
        } else {
            self.state.lives = 0;
            self.state.phase = GamePhase::GameOver;
        }
    }

    fn advance_swarm_step(&mut self) {
        let Some(bounds) = self.swarm_bounds() else {
            return;
        };

        let direction = self.state.formation.direction;
        if direction > 0.0 && bounds.right() + ALIEN_STEP_X >= WORLD_W - SWARM_MARGIN {
            self.state.formation.offset.y += ALIEN_STEP_DOWN;
            self.state.formation.direction = -1.0;
        } else if direction < 0.0 && bounds.left() - ALIEN_STEP_X <= SWARM_MARGIN {
            self.state.formation.offset.y += ALIEN_STEP_DOWN;
            self.state.formation.direction = 1.0;
        } else {
            self.state.formation.offset.x += direction * ALIEN_STEP_X;
        }

        self.state.formation.anim_frame_idx = (self.state.formation.anim_frame_idx + 1) % 2;
    }

    fn alien_step_interval(&self) -> f32 {
        let alive_ratio = self.alive_aliens() as f32 / (ALIEN_ROWS * ALIEN_COLS) as f32;
        let wave_factor = (0.82 - (self.state.wave.saturating_sub(1) as f32 * 0.04)).max(0.28);
        wave_factor * (0.55 + alive_ratio * 0.85)
    }

    fn alien_shot_interval(&self) -> f32 {
        (1.05 - self.state.wave as f32 * 0.06).clamp(0.34, 1.05)
    }

    fn alive_aliens(&self) -> usize {
        self.state
            .formation
            .aliens
            .iter()
            .filter(|alien| alien.alive)
            .count()
    }

    fn swarm_bounds(&self) -> Option<Rect> {
        let mut min = pos2(f32::INFINITY, f32::INFINITY);
        let mut max = pos2(f32::NEG_INFINITY, f32::NEG_INFINITY);

        for alien in self
            .state
            .formation
            .aliens
            .iter()
            .copied()
            .filter(|alien| alien.alive)
        {
            let rect = alien_rect_with_offset(alien, self.state.formation.offset);
            min.x = min.x.min(rect.left());
            min.y = min.y.min(rect.top());
            max.x = max.x.max(rect.right());
            max.y = max.y.max(rect.bottom());
        }

        if min.x.is_finite() {
            Some(Rect::from_min_max(min, max))
        } else {
            None
        }
    }

    fn swarm_reached_player_zone(&self) -> bool {
        self.swarm_bounds()
            .map(|bounds| bounds.bottom() >= PLAYER_Y - 18.0)
            .unwrap_or(false)
    }

    fn random_bottom_shooter_pos(&mut self) -> Option<Vec2> {
        let mut shooters = Vec::new();
        for col in 0..ALIEN_COLS {
            if let Some(alien) = self
                .state
                .formation
                .aliens
                .iter()
                .copied()
                .filter(|alien| alien.alive && alien.col == col)
                .max_by_key(|alien| alien.row)
            {
                shooters.push(alien_world_pos(alien, self.state.formation.offset));
            }
        }
        shooters.choose(&mut self.rng).copied()
    }

    fn spawn_effect(&mut self, pos: Vec2, kind: EffectKind) {
        self.state.effects.push(Effect {
            pos,
            timer: EFFECT_SECS,
            duration: EFFECT_SECS,
            kind,
        });
    }

    fn start_next_wave(&mut self) {
        self.state.wave = self.state.wave.saturating_add(1);
        self.state.player_bullet = None;
        self.state.alien_bullets.clear();
        self.state.effects.clear();
        self.state.player_explosion = None;
        self.state.phase = GamePhase::Ready;
        self.state.phase_timer = READY_SECS;
        self.state.formation = build_formation();
        self.state.barns = build_barns();
        self.state.player.pos = player_spawn_pos();
        self.state.player.visual_dir = 0;
        self.state.player.flash_timer = 0.0;
        self.state.player.reload_timer = 0.0;
        self.state.alien_shot_timer = self.alien_shot_interval();
        self.state.ufo = None;
        self.state.ufo_cooldown = self.rng.gen_range(8.0..14.0);
    }

    fn draw_hud(&self, painter: &egui::Painter, world: Rect, atlas: &AtlasTextures) {
        let score_icon = world_icon_rect(world, vec2(14.0, 10.0));
        atlas.paint_frame(painter, score_icon, FrameId::ScoreIcon, self.theme, false);
        painter.text(
            score_icon.right_center() + vec2(8.0, 0.0),
            Align2::LEFT_CENTER,
            format!("{:05}", self.state.score),
            FontId::new(14.0, FontFamily::Monospace),
            self.theme.ui,
        );

        painter.text(
            world_point(world, vec2(WORLD_W * 0.5, 10.0)),
            Align2::CENTER_CENTER,
            format!("HI {:05}", self.high_score),
            FontId::new(14.0, FontFamily::Monospace),
            self.theme.ui,
        );

        let wave_icon = world_icon_rect(world, vec2(WORLD_W * 0.5 - 14.0, 24.0));
        atlas.paint_frame(painter, wave_icon, FrameId::WaveIcon, self.theme, false);
        painter.text(
            wave_icon.right_center() + vec2(8.0, 0.0),
            Align2::LEFT_CENTER,
            format!("W{:02}", self.state.wave),
            FontId::new(12.0, FontFamily::Monospace),
            self.theme.neutral,
        );

        let mut life_x = WORLD_W - 16.0;
        for _ in 0..self.state.lives {
            let rect = world_icon_rect(world, vec2(life_x, 10.0));
            atlas.paint_frame(painter, rect, FrameId::LifeIcon, self.theme, false);
            life_x -= 18.0;
        }
    }

    fn draw_barns(&self, painter: &egui::Painter, world: Rect, atlas: &AtlasTextures) {
        for barn in &self.state.barns {
            for idx in 0..barn.pieces_alive.len() {
                if !barn.pieces_alive[idx] {
                    continue;
                }
                let piece_rect = barn.piece_rect(idx);
                let world_rect = world_rect_from_game_rect(world, piece_rect);
                let (frame, flip_x) = barn_piece_visual(idx);
                atlas.paint_frame(painter, world_rect, frame, self.theme, flip_x);
            }
        }
    }

    fn draw_ufo(&self, painter: &egui::Painter, world: Rect, atlas: &AtlasTextures) {
        let Some(ufo) = self.state.ufo else {
            return;
        };
        let frame = match ufo.state {
            UfoState::Flying => atlas.animation_frame_for_tick(
                AnimationId::UfoMove,
                self.state.animation_ticks * UFO_ANIM_SPEED
                    + if ufo.direction < 0.0 { 4.0 } else { 0.0 },
            ),
            UfoState::Exploding { .. } => FrameId::UfoExplosion,
        };
        atlas.paint_frame(
            painter,
            world_rect_from_entity(world, ufo.pos, UFO_SIZE),
            frame,
            self.theme,
            ufo.direction < 0.0,
        );
    }

    fn draw_aliens(&self, painter: &egui::Painter, world: Rect, atlas: &AtlasTextures) {
        for alien in self
            .state
            .formation
            .aliens
            .iter()
            .copied()
            .filter(|alien| alien.alive)
        {
            let frame = match alien.kind {
                AlienKind::Squid => atlas.animation_frame_by_index(
                    AnimationId::AlienSquid,
                    self.state.formation.anim_frame_idx,
                ),
                AlienKind::Crab => atlas.animation_frame_by_index(
                    AnimationId::AlienCrab,
                    self.state.formation.anim_frame_idx,
                ),
                AlienKind::Octo => atlas.animation_frame_by_index(
                    AnimationId::AlienOcto,
                    self.state.formation.anim_frame_idx,
                ),
            };
            atlas.paint_frame(
                painter,
                world_rect_from_entity(
                    world,
                    alien_world_pos(alien, self.state.formation.offset),
                    ALIEN_SIZE,
                ),
                frame,
                self.theme,
                false,
            );
        }
    }

    fn draw_player_bullet(&self, painter: &egui::Painter, world: Rect, atlas: &AtlasTextures) {
        let Some(bullet) = self.state.player_bullet else {
            return;
        };
        atlas.paint_frame(
            painter,
            world_rect_from_entity(world, bullet.pos, PLAYER_BULLET_DRAW_SIZE),
            FrameId::PlayerBullet,
            self.theme,
            false,
        );
    }

    fn draw_alien_bullets(&self, painter: &egui::Painter, world: Rect, atlas: &AtlasTextures) {
        for bullet in &self.state.alien_bullets {
            let anim_ticks = match bullet.kind {
                BulletKind::AlienSlow => bullet.age * 42.0,
                BulletKind::AlienFast => bullet.age * 72.0,
                BulletKind::Player => bullet.age * 60.0,
            };
            let frame = atlas.animation_frame_for_tick(AnimationId::AlienBullet, anim_ticks);
            atlas.paint_frame(
                painter,
                world_rect_from_entity(world, bullet.pos, ALIEN_BULLET_DRAW_SIZE),
                frame,
                self.theme,
                false,
            );
        }
    }

    fn draw_effects(&self, painter: &egui::Painter, world: Rect, atlas: &AtlasTextures) {
        for effect in &self.state.effects {
            let progress = 1.0 - effect.timer / effect.duration.max(f32::EPSILON);
            let frame = match effect.kind {
                EffectKind::Spark => {
                    atlas.animation_frame_for_tick(AnimationId::Spark, progress * 24.0)
                }
                EffectKind::ExplosionSmall => {
                    atlas.animation_frame_for_tick(AnimationId::ExplosionSmall, progress * 24.0)
                }
            };
            atlas.paint_frame(
                painter,
                world_rect_from_entity(world, effect.pos, vec2(14.0, 14.0)),
                frame,
                self.theme,
                false,
            );
        }
    }

    fn draw_player(&self, painter: &egui::Painter, world: Rect, atlas: &AtlasTextures) {
        if let Some(explosion) = self.state.player_explosion {
            let progress = 1.0 - explosion.timer / explosion.duration.max(f32::EPSILON);
            let frame = atlas.animation_frame_for_tick(AnimationId::PlayerExplode, progress * 24.0);
            atlas.paint_frame(
                painter,
                world_rect_from_entity(world, explosion.pos, PLAYER_SIZE),
                frame,
                self.theme,
                false,
            );
            return;
        }

        if self.state.phase == GamePhase::Respawning {
            return;
        }

        let frame = if self.state.player.flash_timer > 0.0 {
            FrameId::PlayerShoot
        } else if self.state.player.visual_dir < 0 {
            FrameId::PlayerMoveLeft
        } else if self.state.player.visual_dir > 0 {
            FrameId::PlayerMoveRight
        } else {
            FrameId::PlayerIdle
        };
        atlas.paint_frame(
            painter,
            world_rect_from_entity(world, self.state.player.pos, PLAYER_SIZE),
            frame,
            self.theme,
            false,
        );
    }

    fn draw_overlay(&self, painter: &egui::Painter, world: Rect, atlas: &AtlasTextures) {
        let (title, subtitle) = match self.state.phase {
            GamePhase::Title => return,
            GamePhase::Ready => ("READY", "Clear the wave"),
            GamePhase::Paused => ("PAUSED", "Press P or Esc to resume"),
            GamePhase::Respawning => ("HIT", "Get back in position"),
            GamePhase::GameOver => ("GAME OVER", "Press fire to restart"),
            GamePhase::Playing => return,
        };

        let overlay = Rect::from_center_size(world.center(), vec2(world.width() * 0.54, 54.0));
        painter.rect_filled(overlay, 0.0, Color32::BLACK.gamma_multiply(0.92));
        painter.rect_stroke(overlay, 0.0, Stroke::new(2.0, self.theme.ui));
        let icon_rect =
            Rect::from_center_size(overlay.center_top() + vec2(0.0, 16.0), vec2(18.0, 18.0));
        atlas.paint_frame(painter, icon_rect, FrameId::ReadyIcon, self.theme, false);
        painter.text(
            overlay.center_top() + vec2(0.0, 32.0),
            Align2::CENTER_TOP,
            title,
            FontId::new(16.0, FontFamily::Monospace),
            self.theme.ui,
        );
        painter.text(
            overlay.center_bottom() - vec2(0.0, 10.0),
            Align2::CENTER_BOTTOM,
            subtitle,
            FontId::new(12.0, FontFamily::Monospace),
            self.theme.neutral,
        );
    }

    fn paint_starfield(&self, painter: &egui::Painter, world: Rect) {
        for (idx, star) in STAR_FIELD.iter().enumerate() {
            let pos = world_point(world, *star);
            let color = if idx % 3 == 0 {
                self.theme.neutral
            } else {
                self.theme.ui.gamma_multiply(0.5)
            };
            painter.circle_filled(pos, 1.0 + (idx % 2) as f32 * 0.5, color);
        }
    }
}

impl AtlasTextures {
    pub fn new(ctx: &Context) -> Self {
        macro_rules! load_frame_asset {
            ($atlas:expr, $ctx:expr, $frame_id:expr, $tint_role:expr, $file_name:literal) => {
                $atlas.load_frame(
                    $ctx,
                    $frame_id,
                    $tint_role,
                    $file_name,
                    include_bytes!(concat!("../assets/png/", $file_name)),
                );
            };
        }

        let mut atlas = Self {
            textures: HashMap::new(),
            catalog: SpriteCatalog::default(),
        };
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::PlayerIdle,
            TintRole::Player,
            "player_idle.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::PlayerMoveLeft,
            TintRole::Player,
            "player_move_left.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::PlayerMoveRight,
            TintRole::Player,
            "player_move_right.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::PlayerShoot,
            TintRole::Player,
            "player_shoot.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::PlayerExplosion1,
            TintRole::Enemy,
            "player_explosion_1.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::PlayerExplosion2,
            TintRole::Enemy,
            "player_explosion_2.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::AlienSquid1,
            TintRole::Alien,
            "alien_squid_1.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::AlienSquid2,
            TintRole::Alien,
            "alien_squid_2.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::AlienCrab1,
            TintRole::Alien,
            "alien_crab_1.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::AlienCrab2,
            TintRole::Alien,
            "alien_crab_2.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::AlienOcto1,
            TintRole::Alien,
            "alien_octo_1.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::AlienOcto2,
            TintRole::Alien,
            "alien_octo_2.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::AlienExplosion,
            TintRole::Enemy,
            "alien_explosion.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::PlayerBullet,
            TintRole::Bullet,
            "player_bullet.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::AlienBullet1,
            TintRole::Enemy,
            "alien_bullet_1.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::AlienBullet2,
            TintRole::Enemy,
            "alien_bullet_2.png"
        );
        load_frame_asset!(atlas, ctx, FrameId::Spark1, TintRole::Enemy, "spark_1.png");
        load_frame_asset!(atlas, ctx, FrameId::Spark2, TintRole::Enemy, "spark_2.png");
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::ExplosionSmall1,
            TintRole::Enemy,
            "explosion_small_1.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::ExplosionSmall2,
            TintRole::Enemy,
            "explosion_small_2.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::ExplosionSmall3,
            TintRole::Enemy,
            "explosion_small_3.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::ExplosionSmall4,
            TintRole::Enemy,
            "explosion_small_4.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::ExplosionSmall5,
            TintRole::Enemy,
            "explosion_small_5.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::ExplosionSmall6,
            TintRole::Enemy,
            "explosion_small_6.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::BarrierFull,
            TintRole::Barrier,
            "barrier_full.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::BarrierDamage1,
            TintRole::Barrier,
            "barrier_damage_1.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::BarrierDamage2,
            TintRole::Barrier,
            "barrier_damage_2.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::BarrierDamage3,
            TintRole::Barrier,
            "barrier_damage_3.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::BarrierChunk,
            TintRole::Barrier,
            "barrier_chunk.png"
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece00,
            TintRole::Barrier,
            "barn_piece_00.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece01,
            TintRole::Barrier,
            "barn_piece_01.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece02,
            TintRole::Barrier,
            "barn_piece_02.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece03,
            TintRole::Barrier,
            "barn_piece_03.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece04,
            TintRole::Barrier,
            "barn_piece_04.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece05,
            TintRole::Barrier,
            "barn_piece_05.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece06,
            TintRole::Barrier,
            "barn_piece_06.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece07,
            TintRole::Barrier,
            "barn_piece_07.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece08,
            TintRole::Barrier,
            "barn_piece_08.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece09,
            TintRole::Barrier,
            "barn_piece_09.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece10,
            TintRole::Barrier,
            "barn_piece_10.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece11,
            TintRole::Barrier,
            "barn_piece_11.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece12,
            TintRole::Barrier,
            "barn_piece_12.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece13,
            TintRole::Barrier,
            "barn_piece_13.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece14,
            TintRole::Barrier,
            "barn_piece_14.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece15,
            TintRole::Barrier,
            "barn_piece_15.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece16,
            TintRole::Barrier,
            "barn_piece_16.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        atlas.load_frame(
            ctx,
            FrameId::BarnPiece17,
            TintRole::Barrier,
            "barn_piece_17.png",
            include_bytes!("../assets/png/barrier_full.png"),
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::UfoIdle,
            TintRole::Enemy,
            "ufo_idle.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::UfoFlash,
            TintRole::Enemy,
            "ufo_flash.png"
        );
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::UfoExplosion,
            TintRole::Enemy,
            "ufo_explosion.png"
        );
        load_frame_asset!(atlas, ctx, FrameId::LifeIcon, TintRole::Ui, "life_icon.png");
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::ScoreIcon,
            TintRole::Ui,
            "score_icon.png"
        );
        load_frame_asset!(atlas, ctx, FrameId::WaveIcon, TintRole::Ui, "wave_icon.png");
        load_frame_asset!(
            atlas,
            ctx,
            FrameId::ReadyIcon,
            TintRole::Ui,
            "ready_icon.png"
        );
        load_frame_asset!(atlas, ctx, FrameId::Title01, TintRole::Ui, "title_01.png");
        load_frame_asset!(atlas, ctx, FrameId::Title02, TintRole::Ui, "title_02.png");
        load_frame_asset!(atlas, ctx, FrameId::Title03, TintRole::Ui, "title_03.png");
        load_frame_asset!(atlas, ctx, FrameId::Title04, TintRole::Ui, "title_04.png");
        load_frame_asset!(atlas, ctx, FrameId::Title05, TintRole::Ui, "title_05.png");
        load_frame_asset!(atlas, ctx, FrameId::Title06, TintRole::Ui, "title_06.png");
        load_frame_asset!(atlas, ctx, FrameId::Title07, TintRole::Ui, "title_07.png");
        load_frame_asset!(atlas, ctx, FrameId::Title08, TintRole::Ui, "title_08.png");
        load_frame_asset!(atlas, ctx, FrameId::Title09, TintRole::Ui, "title_09.png");
        load_frame_asset!(atlas, ctx, FrameId::Title10, TintRole::Ui, "title_10.png");
        load_frame_asset!(atlas, ctx, FrameId::Title11, TintRole::Ui, "title_11.png");
        load_frame_asset!(atlas, ctx, FrameId::Title12, TintRole::Ui, "title_12.png");
        load_frame_asset!(atlas, ctx, FrameId::Title13, TintRole::Ui, "title_13.png");
        load_frame_asset!(atlas, ctx, FrameId::Title14, TintRole::Ui, "title_14.png");
        load_frame_asset!(atlas, ctx, FrameId::Title15, TintRole::Ui, "title_15.png");
        load_frame_asset!(atlas, ctx, FrameId::Title16, TintRole::Ui, "title_16.png");
        load_frame_asset!(atlas, ctx, FrameId::Title17, TintRole::Ui, "title_17.png");
        load_frame_asset!(atlas, ctx, FrameId::Title18, TintRole::Ui, "title_18.png");
        load_frame_asset!(atlas, ctx, FrameId::Title19, TintRole::Ui, "title_19.png");
        load_frame_asset!(atlas, ctx, FrameId::Title20, TintRole::Ui, "title_20.png");

        atlas.load_animation(
            AnimationId::PlayerExplode,
            &[FrameId::PlayerExplosion1, FrameId::PlayerExplosion2],
            5,
        );
        atlas.load_animation(
            AnimationId::AlienSquid,
            &[FrameId::AlienSquid1, FrameId::AlienSquid2],
            12,
        );
        atlas.load_animation(
            AnimationId::AlienCrab,
            &[FrameId::AlienCrab1, FrameId::AlienCrab2],
            12,
        );
        atlas.load_animation(
            AnimationId::AlienOcto,
            &[FrameId::AlienOcto1, FrameId::AlienOcto2],
            12,
        );
        atlas.load_animation(
            AnimationId::AlienBullet,
            &[FrameId::AlienBullet1, FrameId::AlienBullet2],
            4,
        );
        atlas.load_animation(AnimationId::Spark, &[FrameId::Spark1, FrameId::Spark2], 4);
        atlas.load_animation(
            AnimationId::ExplosionSmall,
            &[
                FrameId::ExplosionSmall1,
                FrameId::ExplosionSmall2,
                FrameId::ExplosionSmall3,
                FrameId::ExplosionSmall4,
                FrameId::ExplosionSmall5,
                FrameId::ExplosionSmall6,
            ],
            5,
        );
        atlas.load_animation(
            AnimationId::UfoMove,
            &[FrameId::UfoIdle, FrameId::UfoFlash],
            8,
        );
        atlas.load_animation(
            AnimationId::TitleScreen,
            &[
                FrameId::Title01,
                FrameId::Title02,
                FrameId::Title03,
                FrameId::Title04,
                FrameId::Title05,
                FrameId::Title06,
                FrameId::Title07,
                FrameId::Title08,
                FrameId::Title09,
                FrameId::Title10,
                FrameId::Title11,
                FrameId::Title12,
                FrameId::Title13,
                FrameId::Title14,
                FrameId::Title15,
                FrameId::Title16,
                FrameId::Title17,
                FrameId::Title18,
                FrameId::Title19,
                FrameId::Title20,
            ],
            4,
        );
        atlas
    }

    fn paint_frame(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        frame_id: FrameId,
        theme: Theme,
        flip_x: bool,
    ) {
        let frame = self
            .catalog
            .frames
            .get(&frame_id)
            .expect("space invaders frame missing");
        let texture = self
            .textures
            .get(&frame_id)
            .expect("space invaders texture missing");
        let uv = if flip_x {
            Rect::from_min_max(pos2(1.0, 0.0), pos2(0.0, 1.0))
        } else {
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))
        };
        let mut mesh = egui::epaint::Mesh::with_texture(texture.id());
        mesh.add_rect_with_uv(rect, uv, theme.tint(frame.tint_role));
        painter.add(egui::Shape::mesh(mesh));
    }

    fn animation_frame_for_tick(&self, animation_id: AnimationId, ticks: f32) -> FrameId {
        let animation = self
            .catalog
            .animations
            .get(&animation_id)
            .expect("space invaders animation missing");
        let idx = ((ticks as u32) / animation.tick.max(1)) as usize % animation.frames.len();
        animation.frames[idx]
    }

    fn animation_frame_for_tick_clamped(&self, animation_id: AnimationId, ticks: f32) -> FrameId {
        let animation = self
            .catalog
            .animations
            .get(&animation_id)
            .expect("space invaders animation missing");
        let idx = (((ticks.max(0.0)) as u32) / animation.tick.max(1)) as usize;
        animation.frames[idx.min(animation.frames.len().saturating_sub(1))]
    }

    fn animation_frame_by_index(&self, animation_id: AnimationId, index: usize) -> FrameId {
        let animation = self
            .catalog
            .animations
            .get(&animation_id)
            .expect("space invaders animation missing");
        animation.frames[index % animation.frames.len()]
    }

    fn load_frame(
        &mut self,
        ctx: &Context,
        frame_id: FrameId,
        tint_role: TintRole,
        file_name: &str,
        fallback_png: &[u8],
    ) {
        let bytes = load_runtime_asset_bytes(file_name).unwrap_or_else(|| fallback_png.to_vec());
        let image = image::load_from_memory(&bytes)
            .unwrap_or_else(|err| panic!("invalid space invaders png {file_name}: {err}"))
            .into_rgba8();
        let (width, height) = image.dimensions();
        let texture = ctx.load_texture(
            format!("space_invaders_{frame_id:?}"),
            egui::ColorImage::from_rgba_unmultiplied(
                [width as usize, height as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::NEAREST,
        );
        self.textures.insert(frame_id, texture);
        self.catalog.frames.insert(frame_id, Frame { tint_role });
    }

    fn load_animation(&mut self, animation_id: AnimationId, frames: &[FrameId], tick: u32) {
        self.catalog.animations.insert(
            animation_id,
            Animation {
                frames: frames.to_vec(),
                tick: tick.max(1),
            },
        );
    }
}

fn load_runtime_asset_bytes(file_name: &str) -> Option<Vec<u8>> {
    std::fs::read(runtime_asset_path(file_name)).ok()
}

fn runtime_asset_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("png")
        .join(file_name)
}

impl GameState {
    fn new() -> Self {
        Self {
            player: Player {
                pos: player_spawn_pos(),
                visual_dir: 0,
                flash_timer: 0.0,
                reload_timer: 0.0,
            },
            player_bullet: None,
            alien_bullets: Vec::new(),
            formation: build_formation(),
            barns: build_barns(),
            effects: Vec::new(),
            ufo: None,
            player_explosion: None,
            phase: GamePhase::Title,
            phase_timer: 0.0,
            score: 0,
            lives: 3,
            wave: 1,
            animation_ticks: 0.0,
            ufo_cooldown: 9.0,
            alien_shot_timer: 1.15,
        }
    }
}

impl Barn {
    fn piece_rect(&self, idx: usize) -> Rect {
        let row = idx / BARN_COLS;
        let col = idx % BARN_COLS;
        const COLUMN_X_OFFSETS: [f32; BARN_COLS] = [
            0.0 * WORLD_W / ZETA_STAGE_W,
            12.0 * WORLD_W / ZETA_STAGE_W,
            24.0 * WORLD_W / ZETA_STAGE_W,
            60.0 * WORLD_W / ZETA_STAGE_W,
            48.0 * WORLD_W / ZETA_STAGE_W,
            36.0 * WORLD_W / ZETA_STAGE_W,
        ];
        const ROW_Y_OFFSETS: [f32; BARN_ROWS] = [
            0.0 * WORLD_H / ZETA_STAGE_H,
            12.0 * WORLD_H / ZETA_STAGE_H,
            24.0 * WORLD_H / ZETA_STAGE_H,
        ];
        Rect::from_min_size(
            pos2(
                self.origin.x + COLUMN_X_OFFSETS[col],
                self.origin.y + ROW_Y_OFFSETS[row],
            ),
            BARN_PIECE_SIZE,
        )
    }

    fn destroy_piece_from_hit(&mut self, bullet_rect: Rect, from_below: bool) -> Option<usize> {
        let bullet_center_x = bullet_rect.center().x;
        let mut hits: Vec<usize> = (0..self.pieces_alive.len())
            .filter(|&idx| self.pieces_alive[idx] && self.piece_rect(idx).intersects(bullet_rect))
            .collect();
        if hits.is_empty() {
            return None;
        }

        hits.sort_by(|a, b| {
            let row_a = a / BARN_COLS;
            let row_b = b / BARN_COLS;
            let row_order = if from_below {
                row_b.cmp(&row_a)
            } else {
                row_a.cmp(&row_b)
            };
            row_order.then_with(|| {
                let ax = self.piece_rect(*a).center().x;
                let bx = self.piece_rect(*b).center().x;
                (ax - bullet_center_x)
                    .abs()
                    .partial_cmp(&(bx - bullet_center_x).abs())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        let idx = hits[0];
        self.pieces_alive[idx] = false;
        Some(idx)
    }
}

pub fn input_from_hosted_events(events: &[HostedInputEvent]) -> GameInput {
    GameInput {
        left: hosted_key_active(events, &["arrow-left", "a"]),
        right: hosted_key_active(events, &["arrow-right", "d"]),
        fire: hosted_key_active(events, &["space"]),
        start: hosted_key_active(events, &["enter"]),
        pause: hosted_key_active(events, &["p", "escape"]),
    }
}

pub fn input_from_ctx(ctx: &Context) -> GameInput {
    GameInput {
        left: ctx.input(|i| i.key_down(Key::ArrowLeft) || i.key_down(Key::A)),
        right: ctx.input(|i| i.key_down(Key::ArrowRight) || i.key_down(Key::D)),
        fire: ctx.input(|i| i.key_down(Key::Space)),
        start: ctx.input(|i| i.key_pressed(Key::Enter)),
        pause: ctx.input(|i| i.key_pressed(Key::P) || i.key_pressed(Key::Escape)),
    }
}

fn hosted_key_active(events: &[HostedInputEvent], accepted: &[&str]) -> bool {
    events.iter().any(|event| match event {
        HostedInputEvent::Key { key, pressed } => {
            *pressed && accepted.iter().any(|accepted_key| key == accepted_key)
        }
        _ => false,
    })
}

fn player_spawn_pos() -> Vec2 {
    vec2(WORLD_W * 0.5, PLAYER_Y)
}

fn build_formation() -> AlienFormation {
    let mut aliens = Vec::with_capacity(ALIEN_ROWS * ALIEN_COLS);
    for row in 0..ALIEN_ROWS {
        let kind = match row {
            0 => AlienKind::Squid,
            1 | 2 => AlienKind::Crab,
            _ => AlienKind::Octo,
        };
        for col in 0..ALIEN_COLS {
            aliens.push(Alien {
                row,
                col,
                kind,
                alive: true,
            });
        }
    }
    AlienFormation {
        aliens,
        offset: vec2(24.0, 34.0),
        direction: 1.0,
        step_timer: 0.0,
        anim_frame_idx: 0,
    }
}

fn build_barns() -> Vec<Barn> {
    let start_x = 100.0 * WORLD_W / ZETA_STAGE_W;
    let barn_width = 72.0 * WORLD_W / ZETA_STAGE_W;
    let end_x = (ZETA_STAGE_W - 100.0) * WORLD_W / ZETA_STAGE_W - barn_width;
    let step_x = (end_x - start_x) / 3.0;
    (0..4)
        .map(|i| Barn {
            origin: vec2(start_x + step_x * i as f32, BARN_Y),
            pieces_alive: [true; BARN_ROWS * BARN_COLS],
        })
        .collect()
}

fn alien_world_pos(alien: Alien, formation_offset: Vec2) -> Vec2 {
    formation_offset
        + vec2(
            alien.col as f32 * ALIEN_SPACING_X + ALIEN_SIZE.x * 0.5,
            alien.row as f32 * ALIEN_SPACING_Y + ALIEN_SIZE.y * 0.5,
        )
}

fn alien_rect_with_offset(alien: Alien, formation_offset: Vec2) -> Rect {
    entity_rect(alien_world_pos(alien, formation_offset), ALIEN_SIZE)
}

fn entity_rect(pos: Vec2, size: Vec2) -> Rect {
    Rect::from_center_size(pos2(pos.x, pos.y), size)
}

fn swept_rect(from: Vec2, to: Vec2, size: Vec2) -> Rect {
    let half = size * 0.5;
    Rect::from_min_max(
        pos2(from.x.min(to.x) - half.x, from.y.min(to.y) - half.y),
        pos2(from.x.max(to.x) + half.x, from.y.max(to.y) + half.y),
    )
}

fn midpoint(a: Vec2, b: Vec2) -> Vec2 {
    vec2((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

fn alien_kind_score(kind: AlienKind) -> u32 {
    match kind {
        AlienKind::Squid => 30,
        AlienKind::Crab => 20,
        AlienKind::Octo => 10,
    }
}

fn barn_piece_frame(idx: usize) -> FrameId {
    match idx {
        0 => FrameId::BarnPiece00,
        1 => FrameId::BarnPiece01,
        2 => FrameId::BarnPiece02,
        3 => FrameId::BarnPiece03,
        4 => FrameId::BarnPiece04,
        5 => FrameId::BarnPiece05,
        6 => FrameId::BarnPiece06,
        7 => FrameId::BarnPiece07,
        8 => FrameId::BarnPiece08,
        9 => FrameId::BarnPiece09,
        10 => FrameId::BarnPiece10,
        11 => FrameId::BarnPiece11,
        12 => FrameId::BarnPiece12,
        13 => FrameId::BarnPiece13,
        14 => FrameId::BarnPiece14,
        15 => FrameId::BarnPiece15,
        16 => FrameId::BarnPiece16,
        _ => FrameId::BarnPiece17,
    }
}

fn barn_piece_visual(idx: usize) -> (FrameId, bool) {
    let row = idx / BARN_COLS;
    let col = idx % BARN_COLS;
    if col < BARN_COLS / 2 {
        (barn_piece_frame(row + col * BARN_ROWS), false)
    } else {
        // The right half is positioned outer-to-inner in piece_rect(),
        // so its art mapping must follow columns 0,1,2 again rather than
        // reversing 2,1,0.
        let mirrored_col = col - (BARN_COLS / 2);
        (barn_piece_frame(row + mirrored_col * BARN_ROWS), true)
    }
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

fn world_rect_from_game_rect(world: Rect, rect: Rect) -> Rect {
    let min = world_point(world, rect.min.to_vec2());
    let max = world_point(world, rect.max.to_vec2());
    Rect::from_min_max(min, max)
}

fn world_icon_rect(world: Rect, pos: Vec2) -> Rect {
    world_rect_from_entity(world, pos, vec2(12.0, 12.0))
}

fn game_rect_from_entity(pos: Vec2, size: Vec2) -> Rect {
    Rect::from_center_size(pos2(pos.x, pos.y), size)
}

fn push_starfield_commands(commands: &mut Vec<HostedDrawCommand>, theme: Theme) {
    for (idx, star) in STAR_FIELD.iter().enumerate() {
        let color = if idx % 3 == 0 {
            theme.neutral
        } else {
            theme.ui.gamma_multiply(0.5)
        };
        push_rect(
            commands,
            Rect::from_center_size(pos2(star.x, star.y), vec2(2.0, 2.0)),
            color,
        );
    }
}

fn push_rect(commands: &mut Vec<HostedDrawCommand>, rect: Rect, color: Color32) {
    commands.push(HostedDrawCommand::Rect {
        x: rect.left(),
        y: rect.top(),
        width: rect.width(),
        height: rect.height(),
        fill: hosted_color(color),
    });
}

fn push_rect_border(
    commands: &mut Vec<HostedDrawCommand>,
    rect: Rect,
    color: Color32,
    thickness: f32,
) {
    let t = thickness.max(1.0);
    push_rect(
        commands,
        Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + t)),
        color,
    );
    push_rect(
        commands,
        Rect::from_min_max(pos2(rect.left(), rect.bottom() - t), rect.max),
        color,
    );
    push_rect(
        commands,
        Rect::from_min_max(pos2(rect.left(), rect.top() + t), pos2(rect.left() + t, rect.bottom() - t)),
        color,
    );
    push_rect(
        commands,
        Rect::from_min_max(pos2(rect.right() - t, rect.top() + t), pos2(rect.right(), rect.bottom() - t)),
        color,
    );
}

fn push_text(
    commands: &mut Vec<HostedDrawCommand>,
    x: f32,
    y: f32,
    text: &str,
    size: f32,
    color: Color32,
    align: Align2,
) {
    commands.push(HostedDrawCommand::Text {
        x,
        y,
        text: text.to_string(),
        color: hosted_color(color),
        size,
        align: hosted_text_align(align),
    });
}

fn push_image(commands: &mut Vec<HostedDrawCommand>, rect: Rect, frame_id: FrameId) {
    commands.push(HostedDrawCommand::Image {
        x: rect.left(),
        y: rect.top(),
        width: rect.width(),
        height: rect.height(),
        asset_path: hosted_asset_path(frame_id).to_string(),
    });
}

fn hosted_color(color: Color32) -> HostedColor {
    HostedColor {
        r: color.r(),
        g: color.g(),
        b: color.b(),
        a: color.a(),
    }
}

fn hosted_text_align(align: Align2) -> HostedTextAlign {
    match align {
        Align2::LEFT_TOP => HostedTextAlign::LeftTop,
        Align2::LEFT_CENTER => HostedTextAlign::LeftCenter,
        Align2::CENTER_TOP => HostedTextAlign::CenterTop,
        Align2::CENTER_CENTER => HostedTextAlign::CenterCenter,
        Align2::CENTER_BOTTOM => HostedTextAlign::CenterBottom,
        _ => HostedTextAlign::LeftTop,
    }
}

fn title_animation_frame(ticks: f32) -> FrameId {
    title_frame_by_index(((ticks.max(0.0) as u32) / 4) as usize)
}

fn title_frame_by_index(index: usize) -> FrameId {
    const FRAMES: [FrameId; 20] = [
        FrameId::Title01,
        FrameId::Title02,
        FrameId::Title03,
        FrameId::Title04,
        FrameId::Title05,
        FrameId::Title06,
        FrameId::Title07,
        FrameId::Title08,
        FrameId::Title09,
        FrameId::Title10,
        FrameId::Title11,
        FrameId::Title12,
        FrameId::Title13,
        FrameId::Title14,
        FrameId::Title15,
        FrameId::Title16,
        FrameId::Title17,
        FrameId::Title18,
        FrameId::Title19,
        FrameId::Title20,
    ];
    FRAMES[index.min(FRAMES.len().saturating_sub(1))]
}

fn alien_squid_frame(index: usize) -> FrameId {
    [FrameId::AlienSquid1, FrameId::AlienSquid2][index % 2]
}

fn alien_crab_frame(index: usize) -> FrameId {
    [FrameId::AlienCrab1, FrameId::AlienCrab2][index % 2]
}

fn alien_octo_frame(index: usize) -> FrameId {
    [FrameId::AlienOcto1, FrameId::AlienOcto2][index % 2]
}

fn player_explode_frame(ticks: f32) -> FrameId {
    [FrameId::PlayerExplosion1, FrameId::PlayerExplosion2][((ticks as usize) / 5) % 2]
}

fn alien_bullet_frame(ticks: f32) -> FrameId {
    [FrameId::AlienBullet1, FrameId::AlienBullet2][((ticks as usize) / 4) % 2]
}

fn spark_frame(ticks: f32) -> FrameId {
    [FrameId::Spark1, FrameId::Spark2][((ticks as usize) / 4) % 2]
}

fn explosion_small_frame(ticks: f32) -> FrameId {
    const FRAMES: [FrameId; 6] = [
        FrameId::ExplosionSmall1,
        FrameId::ExplosionSmall2,
        FrameId::ExplosionSmall3,
        FrameId::ExplosionSmall4,
        FrameId::ExplosionSmall5,
        FrameId::ExplosionSmall6,
    ];
    FRAMES[((ticks as usize) / 5).min(FRAMES.len().saturating_sub(1))]
}

fn ufo_animation_frame(ticks: f32) -> FrameId {
    [FrameId::UfoIdle, FrameId::UfoFlash][((ticks as usize) / 8) % 2]
}

fn hosted_asset_path(frame_id: FrameId) -> &'static str {
    match frame_id {
        FrameId::PlayerIdle => "assets/png/player_idle.png",
        FrameId::PlayerMoveLeft => "assets/png/player_move_left.png",
        FrameId::PlayerMoveRight => "assets/png/player_move_right.png",
        FrameId::PlayerShoot => "assets/png/player_shoot.png",
        FrameId::PlayerExplosion1 => "assets/png/player_explosion_1.png",
        FrameId::PlayerExplosion2 => "assets/png/player_explosion_2.png",
        FrameId::AlienSquid1 => "assets/png/alien_squid_1.png",
        FrameId::AlienSquid2 => "assets/png/alien_squid_2.png",
        FrameId::AlienCrab1 => "assets/png/alien_crab_1.png",
        FrameId::AlienCrab2 => "assets/png/alien_crab_2.png",
        FrameId::AlienOcto1 => "assets/png/alien_octo_1.png",
        FrameId::AlienOcto2 => "assets/png/alien_octo_2.png",
        FrameId::AlienExplosion => "assets/png/alien_explosion.png",
        FrameId::PlayerBullet => "assets/png/player_bullet.png",
        FrameId::AlienBullet1 => "assets/png/alien_bullet_1.png",
        FrameId::AlienBullet2 => "assets/png/alien_bullet_2.png",
        FrameId::Spark1 => "assets/png/spark_1.png",
        FrameId::Spark2 => "assets/png/spark_2.png",
        FrameId::ExplosionSmall1 => "assets/png/explosion_small_1.png",
        FrameId::ExplosionSmall2 => "assets/png/explosion_small_2.png",
        FrameId::ExplosionSmall3 => "assets/png/explosion_small_3.png",
        FrameId::ExplosionSmall4 => "assets/png/explosion_small_4.png",
        FrameId::ExplosionSmall5 => "assets/png/explosion_small_5.png",
        FrameId::ExplosionSmall6 => "assets/png/explosion_small_6.png",
        FrameId::BarrierFull => "assets/png/barrier_full.png",
        FrameId::BarrierDamage1 => "assets/png/barrier_damage_1.png",
        FrameId::BarrierDamage2 => "assets/png/barrier_damage_2.png",
        FrameId::BarrierDamage3 => "assets/png/barrier_damage_3.png",
        FrameId::BarrierChunk => "assets/png/barrier_chunk.png",
        FrameId::BarnPiece00 => "assets/png/barn_piece_00.png",
        FrameId::BarnPiece01 => "assets/png/barn_piece_01.png",
        FrameId::BarnPiece02 => "assets/png/barn_piece_02.png",
        FrameId::BarnPiece03 => "assets/png/barn_piece_03.png",
        FrameId::BarnPiece04 => "assets/png/barn_piece_04.png",
        FrameId::BarnPiece05 => "assets/png/barn_piece_05.png",
        FrameId::BarnPiece06 => "assets/png/barn_piece_06.png",
        FrameId::BarnPiece07 => "assets/png/barn_piece_07.png",
        FrameId::BarnPiece08 => "assets/png/barn_piece_08.png",
        FrameId::BarnPiece09 => "assets/png/barn_piece_09.png",
        FrameId::BarnPiece10 => "assets/png/barn_piece_10.png",
        FrameId::BarnPiece11 => "assets/png/barn_piece_11.png",
        FrameId::BarnPiece12 => "assets/png/barn_piece_12.png",
        FrameId::BarnPiece13 => "assets/png/barn_piece_13.png",
        FrameId::BarnPiece14 => "assets/png/barn_piece_14.png",
        FrameId::BarnPiece15 => "assets/png/barn_piece_15.png",
        FrameId::BarnPiece16 => "assets/png/barn_piece_16.png",
        FrameId::BarnPiece17 => "assets/png/barn_piece_17.png",
        FrameId::UfoIdle => "assets/png/ufo_idle.png",
        FrameId::UfoFlash => "assets/png/ufo_flash.png",
        FrameId::UfoExplosion => "assets/png/ufo_explosion.png",
        FrameId::LifeIcon => "assets/png/life_icon.png",
        FrameId::ScoreIcon => "assets/png/score_icon.png",
        FrameId::WaveIcon => "assets/png/wave_icon.png",
        FrameId::ReadyIcon => "assets/png/ready_icon.png",
        FrameId::Title01 => "assets/png/title_01.png",
        FrameId::Title02 => "assets/png/title_02.png",
        FrameId::Title03 => "assets/png/title_03.png",
        FrameId::Title04 => "assets/png/title_04.png",
        FrameId::Title05 => "assets/png/title_05.png",
        FrameId::Title06 => "assets/png/title_06.png",
        FrameId::Title07 => "assets/png/title_07.png",
        FrameId::Title08 => "assets/png/title_08.png",
        FrameId::Title09 => "assets/png/title_09.png",
        FrameId::Title10 => "assets/png/title_10.png",
        FrameId::Title11 => "assets/png/title_11.png",
        FrameId::Title12 => "assets/png/title_12.png",
        FrameId::Title13 => "assets/png/title_13.png",
        FrameId::Title14 => "assets/png/title_14.png",
        FrameId::Title15 => "assets/png/title_15.png",
        FrameId::Title16 => "assets/png/title_16.png",
        FrameId::Title17 => "assets/png/title_17.png",
        FrameId::Title18 => "assets/png/title_18.png",
        FrameId::Title19 => "assets/png/title_19.png",
        FrameId::Title20 => "assets/png/title_20.png",
    }
}

const STAR_FIELD: [Vec2; 18] = [
    Vec2::new(14.0, 18.0),
    Vec2::new(38.0, 34.0),
    Vec2::new(70.0, 26.0),
    Vec2::new(96.0, 58.0),
    Vec2::new(128.0, 22.0),
    Vec2::new(174.0, 40.0),
    Vec2::new(200.0, 28.0),
    Vec2::new(18.0, 84.0),
    Vec2::new(52.0, 104.0),
    Vec2::new(88.0, 118.0),
    Vec2::new(136.0, 88.0),
    Vec2::new(186.0, 110.0),
    Vec2::new(30.0, 152.0),
    Vec2::new(74.0, 172.0),
    Vec2::new(120.0, 146.0),
    Vec2::new(160.0, 164.0),
    Vec2::new(196.0, 186.0),
    Vec2::new(208.0, 70.0),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_game_starts_on_title_screen() {
        let game = SpaceInvadersGame::new(SpaceInvadersConfig::default());
        assert_eq!(game.phase(), GamePhase::Title);
        assert_eq!(game.alive_aliens(), ALIEN_ROWS * ALIEN_COLS);
        assert_eq!(game.lives(), 3);
    }

    #[test]
    fn pressing_enter_on_title_starts_ready_phase() {
        let mut game = SpaceInvadersGame::new(SpaceInvadersConfig::default());
        game.update(
            &GameInput {
                start: true,
                ..GameInput::default()
            },
            1.0 / 60.0,
        );
        assert_eq!(game.phase(), GamePhase::Ready);
        assert_eq!(game.wave(), 1);
    }

    #[test]
    fn alien_wave_advances_when_swarm_is_cleared() {
        let mut game = SpaceInvadersGame::new(SpaceInvadersConfig::default());
        game.state.phase = GamePhase::Playing;
        for alien in &mut game.state.formation.aliens {
            alien.alive = false;
        }
        game.update(&GameInput::default(), 1.0 / 60.0);
        assert_eq!(game.wave(), 2);
        assert_eq!(game.phase(), GamePhase::Ready);
        assert_eq!(game.alive_aliens(), ALIEN_ROWS * ALIEN_COLS);
    }

    #[test]
    fn pause_toggles_playing_and_freezes_gameplay() {
        let mut game = SpaceInvadersGame::new(SpaceInvadersConfig::default());
        game.state.phase = GamePhase::Playing;
        let initial_x = game.state.player.pos.x;

        game.update(
            &GameInput {
                right: true,
                pause: true,
                ..GameInput::default()
            },
            0.1,
        );
        assert_eq!(game.phase(), GamePhase::Paused);
        assert_eq!(game.state.player.pos.x, initial_x);

        game.update(
            &GameInput {
                right: true,
                ..GameInput::default()
            },
            0.1,
        );
        assert_eq!(game.phase(), GamePhase::Paused);
        assert_eq!(game.state.player.pos.x, initial_x);

        game.update(
            &GameInput {
                right: true,
                pause: true,
                ..GameInput::default()
            },
            0.1,
        );
        assert_eq!(game.phase(), GamePhase::Playing);
        assert_eq!(game.state.player.pos.x, initial_x);

        game.update(
            &GameInput {
                right: true,
                ..GameInput::default()
            },
            0.1,
        );
        assert!(game.state.player.pos.x > initial_x);
    }

    #[test]
    fn barns_destroy_bottom_piece_for_player_fire() {
        let mut barn = build_barns().remove(0);
        let bottom_piece = BARN_COLS * (BARN_ROWS - 1) + 2;
        let top_piece = 2;
        let hit_rect = barn.piece_rect(bottom_piece);

        let destroyed = barn.destroy_piece_from_hit(hit_rect, true);

        assert_eq!(destroyed, Some(bottom_piece));
        assert!(!barn.pieces_alive[bottom_piece]);
        assert!(barn.pieces_alive[top_piece]);
    }

    #[test]
    fn barns_destroy_top_piece_for_alien_fire() {
        let mut barn = build_barns().remove(0);
        let top_piece = 3;
        let bottom_piece = BARN_COLS * (BARN_ROWS - 1) + 3;
        let hit_rect = barn.piece_rect(top_piece);

        let destroyed = barn.destroy_piece_from_hit(hit_rect, false);

        assert_eq!(destroyed, Some(top_piece));
        assert!(!barn.pieces_alive[top_piece]);
        assert!(barn.pieces_alive[bottom_piece]);
    }

    #[test]
    fn high_score_persists_across_game_reset() {
        let mut game = SpaceInvadersGame::new(SpaceInvadersConfig::default());
        game.add_score(400);
        assert_eq!(game.score(), 400);
        assert_eq!(game.high_score(), 400);

        game.reset();

        assert_eq!(game.score(), 0);
        assert_eq!(game.high_score(), 400);
    }

    #[test]
    fn placeholder_png_assets_decode() {
        for bytes in [
            include_bytes!("../assets/png/player_idle.png").as_slice(),
            include_bytes!("../assets/png/player_move_left.png").as_slice(),
            include_bytes!("../assets/png/player_move_right.png").as_slice(),
            include_bytes!("../assets/png/player_shoot.png").as_slice(),
            include_bytes!("../assets/png/player_explosion_1.png").as_slice(),
            include_bytes!("../assets/png/player_explosion_2.png").as_slice(),
            include_bytes!("../assets/png/alien_squid_1.png").as_slice(),
            include_bytes!("../assets/png/alien_squid_2.png").as_slice(),
            include_bytes!("../assets/png/alien_crab_1.png").as_slice(),
            include_bytes!("../assets/png/alien_crab_2.png").as_slice(),
            include_bytes!("../assets/png/alien_octo_1.png").as_slice(),
            include_bytes!("../assets/png/alien_octo_2.png").as_slice(),
            include_bytes!("../assets/png/alien_explosion.png").as_slice(),
            include_bytes!("../assets/png/player_bullet.png").as_slice(),
            include_bytes!("../assets/png/alien_bullet_1.png").as_slice(),
            include_bytes!("../assets/png/alien_bullet_2.png").as_slice(),
            include_bytes!("../assets/png/spark_1.png").as_slice(),
            include_bytes!("../assets/png/spark_2.png").as_slice(),
            include_bytes!("../assets/png/explosion_small_1.png").as_slice(),
            include_bytes!("../assets/png/explosion_small_2.png").as_slice(),
            include_bytes!("../assets/png/explosion_small_3.png").as_slice(),
            include_bytes!("../assets/png/explosion_small_4.png").as_slice(),
            include_bytes!("../assets/png/explosion_small_5.png").as_slice(),
            include_bytes!("../assets/png/explosion_small_6.png").as_slice(),
            include_bytes!("../assets/png/barrier_full.png").as_slice(),
            include_bytes!("../assets/png/barrier_damage_1.png").as_slice(),
            include_bytes!("../assets/png/barrier_damage_2.png").as_slice(),
            include_bytes!("../assets/png/barrier_damage_3.png").as_slice(),
            include_bytes!("../assets/png/barrier_chunk.png").as_slice(),
            include_bytes!("../assets/png/ufo_idle.png").as_slice(),
            include_bytes!("../assets/png/ufo_flash.png").as_slice(),
            include_bytes!("../assets/png/ufo_explosion.png").as_slice(),
            include_bytes!("../assets/png/life_icon.png").as_slice(),
            include_bytes!("../assets/png/score_icon.png").as_slice(),
            include_bytes!("../assets/png/wave_icon.png").as_slice(),
            include_bytes!("../assets/png/ready_icon.png").as_slice(),
            include_bytes!("../assets/png/title_01.png").as_slice(),
            include_bytes!("../assets/png/title_02.png").as_slice(),
            include_bytes!("../assets/png/title_03.png").as_slice(),
            include_bytes!("../assets/png/title_04.png").as_slice(),
            include_bytes!("../assets/png/title_05.png").as_slice(),
            include_bytes!("../assets/png/title_06.png").as_slice(),
            include_bytes!("../assets/png/title_07.png").as_slice(),
            include_bytes!("../assets/png/title_08.png").as_slice(),
            include_bytes!("../assets/png/title_09.png").as_slice(),
            include_bytes!("../assets/png/title_10.png").as_slice(),
            include_bytes!("../assets/png/title_11.png").as_slice(),
            include_bytes!("../assets/png/title_12.png").as_slice(),
            include_bytes!("../assets/png/title_13.png").as_slice(),
            include_bytes!("../assets/png/title_14.png").as_slice(),
            include_bytes!("../assets/png/title_15.png").as_slice(),
            include_bytes!("../assets/png/title_16.png").as_slice(),
            include_bytes!("../assets/png/title_17.png").as_slice(),
            include_bytes!("../assets/png/title_18.png").as_slice(),
            include_bytes!("../assets/png/title_19.png").as_slice(),
            include_bytes!("../assets/png/title_20.png").as_slice(),
        ] {
            image::load_from_memory(bytes).expect("placeholder png should decode");
        }
    }

    #[test]
    fn hosted_input_maps_expected_keys() {
        let input = input_from_hosted_events(&[
            HostedInputEvent::Key {
                key: "arrow-left".to_string(),
                pressed: true,
            },
            HostedInputEvent::Key {
                key: "space".to_string(),
                pressed: true,
            },
            HostedInputEvent::Key {
                key: "enter".to_string(),
                pressed: true,
            },
        ]);

        assert!(input.left);
        assert!(input.fire);
        assert!(input.start);
        assert!(!input.right);
        assert!(!input.pause);
    }

    #[test]
    fn hosted_frame_uses_bundle_asset_paths() {
        let game = SpaceInvadersGame::new(SpaceInvadersConfig::default());
        let frame = game.hosted_frame();

        assert_eq!(frame.size.width, WORLD_W);
        assert_eq!(frame.size.height, WORLD_H);
        assert!(frame.commands.iter().any(|command| matches!(
            command,
            HostedDrawCommand::Image { asset_path, .. } if asset_path.starts_with("assets/png/")
        )));
    }
}

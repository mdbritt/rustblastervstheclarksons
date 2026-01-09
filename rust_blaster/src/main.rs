//! RUST BLASTER - A 3D First-Person Shooter
//! Built with macroquad (pure Rust, no native deps!)
//!
//! Controls: WASD to move, Mouse to look, Left Click to shoot, 1-4 for weapons, ESC to pause
//! Xbox Controller: Left stick move, Right stick look, RT shoot, LT aim, LB/RB weapons, Start pause

use macroquad::prelude::*;
use macroquad::models::{Mesh, Vertex, draw_mesh};
use std::f32::consts::PI;
use gilrs::{Gilrs, Button, Axis, Event};

// ============================================================================
// GAMEPAD STATE
// ============================================================================

#[derive(Default)]
struct GamepadState {
    // Axes (-1.0 to 1.0)
    left_stick_x: f32,
    left_stick_y: f32,
    right_stick_x: f32,
    right_stick_y: f32,
    left_trigger: f32,
    right_trigger: f32,
    // Trigger buttons (for controllers that report triggers as buttons)
    lt_button: bool,
    rt_button: bool,
    // Buttons (currently pressed)
    a_pressed: bool,
    b_pressed: bool,
    x_pressed: bool,
    y_pressed: bool,
    lb_pressed: bool,
    rb_pressed: bool,
    start_pressed: bool,
    dpad_up: bool,
    dpad_down: bool,
    dpad_left: bool,
    dpad_right: bool,
    left_thumb: bool,
    // Button "just pressed" this frame
    a_just_pressed: bool,
    b_just_pressed: bool,
    lb_just_pressed: bool,
    rb_just_pressed: bool,
    start_just_pressed: bool,
    dpad_up_just: bool,
    dpad_down_just: bool,
    dpad_left_just: bool,
    dpad_right_just: bool,
}

impl GamepadState {
    fn clear_just_pressed(&mut self) {
        self.a_just_pressed = false;
        self.b_just_pressed = false;
        self.lb_just_pressed = false;
        self.rb_just_pressed = false;
        self.start_just_pressed = false;
        self.dpad_up_just = false;
        self.dpad_down_just = false;
        self.dpad_left_just = false;
        self.dpad_right_just = false;
    }
}

// ============================================================================
// CONSTANTS
// ============================================================================

const PLAYER_SPEED: f32 = 8.0;
const PLAYER_SPRINT: f32 = 1.6;
const MOUSE_SENS: f32 = 1.0;
const PLAYER_HEIGHT: f32 = 1.7;
const PLAYER_RADIUS: f32 = 0.3;
const MAX_HEALTH: f32 = 100.0;
const CELL_SIZE: f32 = 4.0;
const WALL_HEIGHT: f32 = 4.0;

// ============================================================================
// GAME STRUCTS
// ============================================================================

#[derive(Clone, Copy, PartialEq)]
enum GameState {
    Menu,
    Playing,
    Paused,
    Dead,
    Victory,
}

#[derive(Clone, Copy, PartialEq)]
enum WeaponType {
    Pistol,
    Shotgun,
    MachineGun,
    Rocket,
}

struct Weapon {
    wtype: WeaponType,
    damage: f32,
    fire_rate: f32,
    spread: f32,
    ammo: i32,
    max_ammo: i32,
    pellets: i32,
    explosive: bool,
    last_shot: f64,
}

impl Weapon {
    fn pistol() -> Self {
        Self { wtype: WeaponType::Pistol, damage: 25.0, fire_rate: 3.0, spread: 0.02,
               ammo: -1, max_ammo: -1, pellets: 1, explosive: false, last_shot: 0.0 }
    }
    fn shotgun() -> Self {
        Self { wtype: WeaponType::Shotgun, damage: 12.0, fire_rate: 1.2, spread: 0.12,
               ammo: 24, max_ammo: 24, pellets: 8, explosive: false, last_shot: 0.0 }
    }
    fn machinegun() -> Self {
        Self { wtype: WeaponType::MachineGun, damage: 10.0, fire_rate: 12.0, spread: 0.06,
               ammo: 200, max_ammo: 200, pellets: 1, explosive: false, last_shot: 0.0 }
    }
    fn rocket() -> Self {
        Self { wtype: WeaponType::Rocket, damage: 250.0, fire_rate: 0.5, spread: 0.0,
               ammo: 20, max_ammo: 20, pellets: 1, explosive: true, last_shot: 0.0 }
    }
    fn can_fire(&self, time: f64) -> bool {
        time - self.last_shot >= 1.0 / self.fire_rate as f64 && (self.ammo > 0 || self.ammo < 0)
    }
    fn fire(&mut self, time: f64) {
        self.last_shot = time;
        if self.ammo > 0 { self.ammo -= 1; }
    }
    fn name(&self) -> &str {
        match self.wtype {
            WeaponType::Pistol => "PISTOL",
            WeaponType::Shotgun => "SHOTGUN",
            WeaponType::MachineGun => "MACHINE GUN",
            WeaponType::Rocket => "ROCKET",
        }
    }
}

struct Player {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
    health: f32,
    armor: f32,
    weapons: Vec<Weapon>,
    current_weapon: usize,
    score: i32,
    damage_flash: f32,
    speed_boost: f32,      // Timer for speed boost
    damage_boost: f32,     // Timer for damage boost
    kills: i32,
    pickup_msg: String,
    pickup_msg_time: f32,
    is_aiming: bool,       // Aiming down sights
    aim_transition: f32,   // 0.0 = hip, 1.0 = ADS
}

impl Player {
    fn new(x: f32, z: f32) -> Self {
        Self {
            pos: vec3(x, PLAYER_HEIGHT, z),
            yaw: 0.0, pitch: 0.0, health: MAX_HEALTH, armor: 0.0,
            weapons: vec![Weapon::pistol(), Weapon::shotgun(), Weapon::machinegun(), Weapon::rocket()],
            current_weapon: 0, score: 0, damage_flash: 0.0,
            speed_boost: 0.0, damage_boost: 0.0, kills: 0,
            pickup_msg: String::new(), pickup_msg_time: 0.0,
            is_aiming: false, aim_transition: 0.0,
        }
    }
    fn forward(&self) -> Vec3 {
        vec3(self.yaw.cos() * self.pitch.cos(), self.pitch.sin(), self.yaw.sin() * self.pitch.cos())
    }
    fn right(&self) -> Vec3 {
        vec3((self.yaw + PI/2.0).cos(), 0.0, (self.yaw + PI/2.0).sin())
    }
}

#[derive(Clone, Copy, PartialEq)]
enum EnemyType { Grunt, Heavy, Demon }

struct Enemy {
    pos: Vec3,
    health: f32,
    max_health: f32,
    etype: EnemyType,
    speed: f32,
    damage: f32,
    attack_cd: f32,
    last_attack: f64,
    dead: bool,
    death_time: f64,
}

impl Enemy {
    fn grunt(x: f32, z: f32) -> Self {
        Self { pos: vec3(x, 1.0, z), health: 50.0, max_health: 50.0, etype: EnemyType::Grunt,
               speed: 3.5, damage: 10.0, attack_cd: 1.0, last_attack: 0.0, dead: false, death_time: 0.0 }
    }
    fn heavy(x: f32, z: f32) -> Self {
        Self { pos: vec3(x, 1.5, z), health: 150.0, max_health: 150.0, etype: EnemyType::Heavy,
               speed: 1.8, damage: 25.0, attack_cd: 1.5, last_attack: 0.0, dead: false, death_time: 0.0 }
    }
    fn demon(x: f32, z: f32) -> Self {
        Self { pos: vec3(x, 1.2, z), health: 70.0, max_health: 70.0, etype: EnemyType::Demon,
               speed: 6.0, damage: 15.0, attack_cd: 0.6, last_attack: 0.0, dead: false, death_time: 0.0 }
    }
    fn color(&self) -> Color {
        match self.etype {
            EnemyType::Grunt => Color::new(0.85, 0.2, 0.2, 1.0),
            EnemyType::Heavy => Color::new(0.5, 0.2, 0.7, 1.0),
            EnemyType::Demon => Color::new(1.0, 0.4, 0.0, 1.0),
        }
    }
    fn size(&self) -> f32 {
        match self.etype { EnemyType::Grunt => 0.8, EnemyType::Heavy => 1.2, EnemyType::Demon => 0.9 }
    }
    fn points(&self) -> i32 {
        match self.etype { EnemyType::Grunt => 100, EnemyType::Heavy => 300, EnemyType::Demon => 200 }
    }
}

struct Projectile {
    pos: Vec3,
    vel: Vec3,
    damage: f32,
    explosive: bool,
}

struct Particle {
    pos: Vec3,
    vel: Vec3,
    color: Color,
    life: f32,
    max_life: f32,
    size: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum PickupType {
    Health,
    Ammo,
    SpeedBoost,
    DamageBoost,
    Armor,
}

struct Pickup {
    pos: Vec3,
    pickup_type: PickupType,
    bob_offset: f32,
    collected: bool,
}

impl Pickup {
    fn new(x: f32, z: f32, pickup_type: PickupType) -> Self {
        Self {
            pos: vec3(x, 0.5, z),
            pickup_type,
            bob_offset: rand::gen_range(0.0, PI * 2.0),
            collected: false,
        }
    }

    fn color(&self) -> Color {
        match self.pickup_type {
            PickupType::Health => GREEN,
            PickupType::Ammo => YELLOW,
            PickupType::SpeedBoost => SKYBLUE,
            PickupType::DamageBoost => RED,
            PickupType::Armor => BLUE,
        }
    }

    fn name(&self) -> &str {
        match self.pickup_type {
            PickupType::Health => "HEALTH",
            PickupType::Ammo => "AMMO",
            PickupType::SpeedBoost => "SPEED BOOST",
            PickupType::DamageBoost => "DAMAGE BOOST",
            PickupType::Armor => "ARMOR",
        }
    }
}

struct Level {
    width: usize,
    height: usize,
    grid: Vec<Vec<char>>,
    floor_heights: Vec<Vec<f32>>,  // Height of each cell's floor
    name: String,
}

impl Level {
    // Create uniform floor heights for a level
    fn uniform_heights(width: usize, height: usize, h: f32) -> Vec<Vec<f32>> {
        vec![vec![h; width]; height]
    }

    fn level_1() -> Self {
        let grid: Vec<Vec<char>> = vec![
            "####################",
            "#.G..........+.....#",
            "#..................#",
            "#......G.....G..A..#",
            "#..................#",
            "#...#......#.......#",
            "#...#..+...#...G...#",
            "#...#......#.......#",
            "#..........A.......#",
            "#......H...........#",
            "#..................#",
            "#.........G....+...#",
            "#...#......#.......#",
            "#...#......#.......#",
            "#...#..D...#.......#",
            "#..................#",
            "#..A...............#",
            "#.........X........#",
            "#P.................#",
            "####################",
        ].iter().map(|s| s.chars().collect()).collect();
        let floor_heights = Self::uniform_heights(20, 20, 0.0);
        Self { width: 20, height: 20, grid, floor_heights, name: "THE BEGINNING".into() }
    }

    fn level_2() -> Self {
        let grid: Vec<Vec<char>> = vec![
            "##########################",
            "#.........+..............#",
            "#..G..G..G..G..G..G..G...#",
            "#........................#",
            "#..A.................A...#",
            "#.....H.......H.......D..#",
            "#........................#",
            "#...........S............#",
            "#........................#",
            "#..+.................+...#",
            "#..D..D..D..D..D..D......#",
            "#........................#",
            "#..A.................A...#",
            "#.....H.......H.......H..#",
            "#........................#",
            "#...........B............#",
            "#........................#",
            "#.......+.....X....+.....#",
            "#P.......................#",
            "##########################",
        ].iter().map(|s| s.chars().collect()).collect();
        let floor_heights = Self::uniform_heights(26, 20, 0.0);
        Self { width: 26, height: 20, grid, floor_heights, name: "DEMON'S LAIR".into() }
    }

    fn level_3() -> Self {
        let grid: Vec<Vec<char>> = vec![
            "##############################",
            "#..+.........R..........+....#",
            "#..H..H..H..H..H..H..H..H....#",
            "#............................#",
            "#..A....................A....#",
            "#............................#",
            "#..G.G.G.G.G.G.G.G.G.G.G.G...#",
            "#............................#",
            "#..+........S...........+....#",
            "#............................#",
            "#..D..D..D..D..D..D..D..D....#",
            "#............................#",
            "#..A........B...........A....#",
            "#............................#",
            "#..H.H.H.H.H.H.H.H.H.H.H.H...#",
            "#............................#",
            "#..+....................+....#",
            "#............................#",
            "#..............X.............#",
            "#P...........................#",
            "##############################",
        ].iter().map(|s: &&str| s.chars().collect()).collect();
        let floor_heights = Self::uniform_heights(30, 21, 0.0);
        Self { width: 30, height: 21, grid, floor_heights, name: "THE GAUNTLET".into() }
    }

    fn level_4() -> Self {
        // Multi-floor level! Has raised platforms and ramps
        let grid: Vec<Vec<char>> = vec![
            "####################################",
            "#..................................#",
            "#..G..G..+..G..G..A..G..G..+..G.G..#",
            "#..................................#",
            "#..###..###..###..###..###..###....#",
            "#..#.+..#.A..#.+..#.A..#.+..#......#",
            "#..#....#....#....#....#....#......#",
            "#..###..###..###..###..###..###....#",
            "#..................................#",
            "#..H..H..H..H..H..H..H..H..S..B....#",
            "#..................................#",
            "#..###..###..###..###..###..###....#",
            "#..#....#....#....#....#....#......#",
            "#..#.R..#.+..#.A..#.+..#.A..#......#",
            "#..###..###..###..###..###..###....#",
            "#..................................#",
            "#..D..D..D..D..D..D..D..D..+..A....#",
            "#..................................#",
            "#................X.................#",
            "#P.................................#",
            "####################################",
        ].iter().map(|s: &&str| s.chars().collect()).collect();

        // Create height map with raised platforms
        let mut floor_heights = Self::uniform_heights(36, 21, 0.0);
        // Raised central area (rows 4-7, 11-14)
        for y in 4..=7 {
            for x in 3..=8 { floor_heights[y][x] = 1.5; }
            for x in 10..=15 { floor_heights[y][x] = 1.5; }
            for x in 17..=22 { floor_heights[y][x] = 1.5; }
            for x in 24..=29 { floor_heights[y][x] = 1.5; }
        }
        for y in 11..=14 {
            for x in 3..=8 { floor_heights[y][x] = 1.5; }
            for x in 10..=15 { floor_heights[y][x] = 1.5; }
            for x in 17..=22 { floor_heights[y][x] = 1.5; }
            for x in 24..=29 { floor_heights[y][x] = 1.5; }
        }
        // Ramps leading up to platforms (gradual height increase)
        for y in 4..=7 {
            floor_heights[y][9] = 0.75; // ramp
        }
        for y in 11..=14 {
            floor_heights[y][9] = 0.75; // ramp
        }

        Self { width: 36, height: 21, grid, floor_heights, name: "THE MAZE OF CLARKSON".into() }
    }

    fn level_5() -> Self {
        // Epic multi-floor final level!
        let grid: Vec<Vec<char>> = vec![
            "########################################",
            "#......................................#",
            "#..H..H..H..H..H..H..H..H..H..H..H..H..#",
            "#......................................#",
            "#..+..A..+..A..+..A..+..A..+..A..+..A..#",
            "#......................................#",
            "#..D..D..D..D..D..D..D..D..D..D..D..D..#",
            "#......................................#",
            "#..R..S..B..R..S..B..R..S..B..R..S..B..#",
            "#......................................#",
            "#..G..G..G..G..G..G..G..G..G..G..G..G..#",
            "#......................................#",
            "#..+..A..+..A..+..A..+..A..+..A..+..A..#",
            "#......................................#",
            "#..H..H..H..H..H..H..H..H..H..H..H..H..#",
            "#......................................#",
            "#..D..D..D..D..D..D..D..D..D..D..D..D..#",
            "#......................................#",
            "#..................X...................#",
            "#P.....................................#",
            "########################################",
        ].iter().map(|s: &&str| s.chars().collect()).collect();

        // Create epic multi-tier height map
        let mut floor_heights = Self::uniform_heights(40, 21, 0.0);
        // Create terraced arena - each row section is higher
        for y in 2..=3 { for x in 1..39 { floor_heights[y][x] = 3.0; } }
        for y in 4..=5 { for x in 1..39 { floor_heights[y][x] = 2.5; } }
        for y in 6..=7 { for x in 1..39 { floor_heights[y][x] = 2.0; } }
        for y in 8..=9 { for x in 1..39 { floor_heights[y][x] = 1.5; } }
        for y in 10..=11 { for x in 1..39 { floor_heights[y][x] = 1.0; } }
        for y in 12..=13 { for x in 1..39 { floor_heights[y][x] = 0.5; } }
        // Lower levels stay at 0

        Self { width: 40, height: 21, grid, floor_heights, name: "CLARKSON'S FINAL STAND".into() }
    }

    fn is_wall(&self, x: i32, y: i32) -> bool {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 { return true; }
        self.grid[y as usize][x as usize] == '#'
    }

    fn get_floor_height(&self, x: f32, z: f32) -> f32 {
        let gx = (x / CELL_SIZE) as usize;
        let gz = (z / CELL_SIZE) as usize;
        if gz < self.height && gx < self.width {
            self.floor_heights[gz][gx]
        } else {
            0.0
        }
    }

    fn check_collision(&self, x: f32, z: f32, radius: f32) -> bool {
        let gx = (x / CELL_SIZE) as i32;
        let gz = (z / CELL_SIZE) as i32;
        for dy in -1..=1 {
            for dx in -1..=1 {
                if self.is_wall(gx + dx, gz + dy) {
                    let wx = (gx + dx) as f32 * CELL_SIZE + CELL_SIZE / 2.0;
                    let wz = (gz + dy) as f32 * CELL_SIZE + CELL_SIZE / 2.0;
                    let half = CELL_SIZE / 2.0;
                    let cx = x.max(wx - half).min(wx + half);
                    let cz = z.max(wz - half).min(wz + half);
                    let dist_sq = (x - cx).powi(2) + (z - cz).powi(2);
                    if dist_sq < radius * radius { return true; }
                }
            }
        }
        false
    }
}

// ============================================================================
// GAME WORLD
// ============================================================================

struct World {
    player: Player,
    enemies: Vec<Enemy>,
    projectiles: Vec<Projectile>,
    particles: Vec<Particle>,
    pickups: Vec<Pickup>,
    level: Level,
    current_level: usize,
    state: GameState,
    screen_shake: f32,
    muzzle_flash: f32,
    hit_marker: f32,
    enemy_texture: Option<Texture2D>,
    combo: i32,
    combo_timer: f32,
    total_kills: i32,
}

impl World {
    fn new() -> Self {
        let level = Level::level_1();
        let (px, pz) = Self::find_char(&level, 'P');
        let mut world = Self {
            player: Player::new(px, pz),
            enemies: Vec::new(),
            projectiles: Vec::new(),
            particles: Vec::new(),
            pickups: Vec::new(),
            level,
            current_level: 1,
            state: GameState::Menu,
            screen_shake: 0.0,
            muzzle_flash: 0.0,
            hit_marker: 0.0,
            enemy_texture: None,
            combo: 0,
            combo_timer: 0.0,
            total_kills: 0,
        };
        world.spawn_enemies();
        world.spawn_pickups();
        world
    }

    fn find_char(level: &Level, c: char) -> (f32, f32) {
        for (y, row) in level.grid.iter().enumerate() {
            for (x, &cell) in row.iter().enumerate() {
                if cell == c {
                    return (x as f32 * CELL_SIZE + CELL_SIZE/2.0, y as f32 * CELL_SIZE + CELL_SIZE/2.0);
                }
            }
        }
        (CELL_SIZE * 2.0, CELL_SIZE * 2.0)
    }

    fn spawn_enemies(&mut self) {
        self.enemies.clear();
        for (y, row) in self.level.grid.iter().enumerate() {
            for (x, &cell) in row.iter().enumerate() {
                let wx = x as f32 * CELL_SIZE + CELL_SIZE / 2.0;
                let wz = y as f32 * CELL_SIZE + CELL_SIZE / 2.0;
                match cell {
                    'G' => self.enemies.push(Enemy::grunt(wx, wz)),
                    'H' => self.enemies.push(Enemy::heavy(wx, wz)),
                    'D' => self.enemies.push(Enemy::demon(wx, wz)),
                    _ => {}
                }
            }
        }
    }

    fn spawn_pickups(&mut self) {
        self.pickups.clear();
        for (y, row) in self.level.grid.iter().enumerate() {
            for (x, &cell) in row.iter().enumerate() {
                let wx = x as f32 * CELL_SIZE + CELL_SIZE / 2.0;
                let wz = y as f32 * CELL_SIZE + CELL_SIZE / 2.0;
                match cell {
                    '+' => self.pickups.push(Pickup::new(wx, wz, PickupType::Health)),
                    'A' => self.pickups.push(Pickup::new(wx, wz, PickupType::Ammo)),
                    'S' => self.pickups.push(Pickup::new(wx, wz, PickupType::SpeedBoost)),
                    'B' => self.pickups.push(Pickup::new(wx, wz, PickupType::DamageBoost)),
                    'R' => self.pickups.push(Pickup::new(wx, wz, PickupType::Armor)),
                    _ => {}
                }
            }
        }
    }

    fn load_level(&mut self, num: usize) {
        self.level = match num {
            1 => Level::level_1(),
            2 => Level::level_2(),
            3 => Level::level_3(),
            4 => Level::level_4(),
            5 => Level::level_5(),
            _ => Level::level_1(),
        };
        self.current_level = num;
        let (px, pz) = Self::find_char(&self.level, 'P');
        self.player.pos = vec3(px, PLAYER_HEIGHT, pz);
        self.player.yaw = 0.0;
        self.player.pitch = 0.0;
        self.spawn_enemies();
        self.spawn_pickups();
        self.projectiles.clear();
        self.particles.clear();
    }

    fn restart(&mut self) {
        self.player.health = MAX_HEALTH;
        self.player.armor = 0.0;
        self.player.score = 0;
        self.player.kills = 0;
        self.player.speed_boost = 0.0;
        self.player.damage_boost = 0.0;
        self.player.weapons = vec![Weapon::pistol(), Weapon::shotgun(), Weapon::machinegun(), Weapon::rocket()];
        self.player.current_weapon = 0;
        self.combo = 0;
        self.combo_timer = 0.0;
        self.total_kills = 0;
        self.load_level(1);
        self.state = GameState::Playing;
    }

    fn alive_enemies(&self) -> usize {
        self.enemies.iter().filter(|e| !e.dead).count()
    }
}

// ============================================================================
// UPDATE
// ============================================================================

fn update(world: &mut World, dt: f32, gamepad: &GamepadState) {
    let time = get_time();

    // Decay effects
    world.screen_shake = (world.screen_shake - dt * 5.0).max(0.0);
    world.muzzle_flash = (world.muzzle_flash - dt * 10.0).max(0.0);
    world.hit_marker = (world.hit_marker - dt * 5.0).max(0.0);
    world.player.damage_flash = (world.player.damage_flash - dt * 2.0).max(0.0);

    match world.state {
        GameState::Menu => {
            // Start game with Enter, Space, or gamepad A/Start
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space)
                || gamepad.a_just_pressed || gamepad.start_just_pressed {
                world.restart();
                set_cursor_grab(true);
                show_mouse(false);
            }
        }
        GameState::Playing => {
            update_player(world, dt, time, gamepad);
            update_enemies(world, dt, time);
            update_projectiles(world, dt);
            update_particles(world, dt);
            update_pickups(world, dt);
            check_victory(world);

            // Combo timer decay
            world.combo_timer -= dt;
            if world.combo_timer <= 0.0 {
                world.combo = 0;
            }

            // Pause with ESC or Start button
            if is_key_pressed(KeyCode::Escape) || gamepad.start_just_pressed {
                world.state = GameState::Paused;
                set_cursor_grab(false);
                show_mouse(true);
            }
            if world.player.health <= 0.0 {
                world.state = GameState::Dead;
                set_cursor_grab(false);
                show_mouse(true);
            }
        }
        GameState::Paused => {
            // Unpause with ESC or Start/B button
            if is_key_pressed(KeyCode::Escape) || gamepad.start_just_pressed || gamepad.b_just_pressed {
                world.state = GameState::Playing;
                set_cursor_grab(true);
                show_mouse(false);
            }
        }
        GameState::Dead | GameState::Victory => {
            // Continue with Enter, Space, or gamepad A/Start
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space)
                || gamepad.a_just_pressed || gamepad.start_just_pressed {
                if world.state == GameState::Victory && world.current_level < 5 {
                    world.player.kills = 0; // Reset kills for new level
                    world.load_level(world.current_level + 1);
                    world.state = GameState::Playing;
                    set_cursor_grab(true);
                    show_mouse(false);
                } else {
                    world.state = GameState::Menu;
                }
            }
        }
    }
}

fn update_player(world: &mut World, dt: f32, time: f64, gamepad: &GamepadState) {
    // === GAMEPAD INPUT ===
    // Deadzone for sticks
    let deadzone = 0.15;
    let apply_deadzone = |v: f32| if v.abs() < deadzone { 0.0 } else { v };

    let gp_move_x = apply_deadzone(gamepad.left_stick_x);
    let gp_move_y = apply_deadzone(gamepad.left_stick_y);
    let gp_look_x = apply_deadzone(gamepad.right_stick_x);
    let gp_look_y = apply_deadzone(gamepad.right_stick_y);

    // Aiming down sights (right mouse button OR left trigger)
    let trigger_aim = gamepad.left_trigger > 0.3 || gamepad.lt_button;
    world.player.is_aiming = is_mouse_button_down(MouseButton::Right) || trigger_aim;
    let aim_speed = 6.0;
    if world.player.is_aiming {
        world.player.aim_transition = (world.player.aim_transition + dt * aim_speed).min(1.0);
    } else {
        world.player.aim_transition = (world.player.aim_transition - dt * aim_speed).max(0.0);
    }
    // Safety clamp
    world.player.aim_transition = world.player.aim_transition.clamp(0.0, 1.0);

    // Mouse look + right stick look
    // Sensitivity reduced to 0.8x when ADS
    let aim = world.player.aim_transition;
    let ads_sens_mult = 1.0 - aim * 0.2; // 1.0 when not aiming, 0.8 when fully aimed
    let delta = mouse_delta_position();
    let gamepad_look_sens = 0.03 * ads_sens_mult;
    let mouse_sens = MOUSE_SENS * ads_sens_mult;
    world.player.yaw -= delta.x * mouse_sens;
    world.player.yaw += gp_look_x * gamepad_look_sens * dt * 60.0;
    world.player.pitch = (world.player.pitch + delta.y * mouse_sens + gp_look_y * gamepad_look_sens * dt * 60.0)
        .clamp(-PI/2.0 + 0.1, PI/2.0 - 0.1);

    // Movement (keyboard + left stick)
    let mut move_dir = Vec3::ZERO;
    let forward = vec3(world.player.yaw.cos(), 0.0, world.player.yaw.sin());
    let right = world.player.right();

    // Keyboard movement
    if is_key_down(KeyCode::W) { move_dir += forward; }
    if is_key_down(KeyCode::S) { move_dir -= forward; }
    if is_key_down(KeyCode::A) { move_dir -= right; }
    if is_key_down(KeyCode::D) { move_dir += right; }

    // Gamepad left stick movement
    move_dir += forward * gp_move_y;  // Forward/back
    move_dir += right * gp_move_x;    // Left/right

    // Sprint with left shift OR left stick click (L3) OR B button
    let gamepad_sprint = gamepad.left_thumb || gamepad.b_pressed;

    if move_dir.length() > 0.0 {
        move_dir = move_dir.normalize();
        let mut speed = PLAYER_SPEED;
        if is_key_down(KeyCode::LeftShift) || gamepad_sprint { speed *= PLAYER_SPRINT; }
        if world.player.speed_boost > 0.0 { speed *= 1.5; } // Speed powerup!

        let new_x = world.player.pos.x + move_dir.x * speed * dt;
        let new_z = world.player.pos.z + move_dir.z * speed * dt;

        if !world.level.check_collision(new_x, world.player.pos.z, PLAYER_RADIUS) {
            world.player.pos.x = new_x;
        }
        if !world.level.check_collision(world.player.pos.x, new_z, PLAYER_RADIUS) {
            world.player.pos.z = new_z;
        }
    }

    // Update player Y based on floor height (smooth transition)
    let target_height = world.level.get_floor_height(world.player.pos.x, world.player.pos.z) + PLAYER_HEIGHT;
    let height_lerp_speed = 10.0;
    world.player.pos.y += (target_height - world.player.pos.y) * height_lerp_speed * dt;

    // Weapon switching (keyboard, mouse wheel, or gamepad bumpers/d-pad)
    if is_key_pressed(KeyCode::Key1) { world.player.current_weapon = 0; }
    if is_key_pressed(KeyCode::Key2) { world.player.current_weapon = 1; }
    if is_key_pressed(KeyCode::Key3) { world.player.current_weapon = 2; }
    if is_key_pressed(KeyCode::Key4) { world.player.current_weapon = 3; }

    let wheel = mouse_wheel().1;
    if wheel > 0.0 { world.player.current_weapon = (world.player.current_weapon + 1) % 4; }
    if wheel < 0.0 { world.player.current_weapon = (world.player.current_weapon + 3) % 4; }

    // Gamepad weapon switching: RB = next, LB = previous, D-pad for direct select
    if gamepad.rb_just_pressed {
        world.player.current_weapon = (world.player.current_weapon + 1) % 4;
    }
    if gamepad.lb_just_pressed {
        world.player.current_weapon = (world.player.current_weapon + 3) % 4;
    }
    // D-pad for direct weapon select
    if gamepad.dpad_up_just { world.player.current_weapon = 0; }
    if gamepad.dpad_right_just { world.player.current_weapon = 1; }
    if gamepad.dpad_down_just { world.player.current_weapon = 2; }
    if gamepad.dpad_left_just { world.player.current_weapon = 3; }

    // Shooting (left click OR right trigger OR A button)
    let gamepad_shoot = gamepad.right_trigger > 0.3 || gamepad.rt_button || gamepad.a_pressed;
    if is_mouse_button_down(MouseButton::Left) || gamepad_shoot {
        try_shoot(world, time);
    }
}

fn try_shoot(world: &mut World, time: f64) {
    let weapon = &mut world.player.weapons[world.player.current_weapon];
    if !weapon.can_fire(time) { return; }

    weapon.fire(time);
    world.muzzle_flash = 1.0;
    world.screen_shake = 0.1;

    let mut damage = weapon.damage;
    if world.player.damage_boost > 0.0 { damage *= 2.0; } // Damage powerup!
    // Reduced spread when aiming (50% tighter)
    let spread = weapon.spread * (1.0 - world.player.aim_transition * 0.5);
    let pellets = weapon.pellets;
    let explosive = weapon.explosive;

    for _ in 0..pellets {
        let sx = rand::gen_range(-spread, spread);
        let sy = rand::gen_range(-spread, spread);

        let forward = world.player.forward();
        let right = world.player.right();
        let up = vec3(0.0, 1.0, 0.0);

        let direction = (forward + right * sx + up * sy).normalize();

        if explosive {
            world.projectiles.push(Projectile {
                pos: world.player.pos,
                vel: direction * 25.0,
                damage,
                explosive: true,
            });
        } else {
            raycast_shot(world, direction, damage);
        }
    }
}

fn raycast_shot(world: &mut World, dir: Vec3, damage: f32) {
    let start = world.player.pos;
    let step = 0.3;
    let max_dist = 100.0;
    let mut dist = 0.0;

    // Track what particles to spawn after we're done with borrows
    let mut particle_spawns: Vec<(Vec3, Color, i32, f32, f32)> = Vec::new();
    let mut hit_enemy_idx: Option<usize> = None;
    let mut hit_pos = Vec3::ZERO;

    while dist < max_dist {
        let pos = start + dir * dist;

        if world.level.check_collision(pos.x, pos.z, 0.1) {
            particle_spawns.push((pos, GRAY, 8, 3.0, 0.1));
            break;
        }

        for (i, enemy) in world.enemies.iter().enumerate() {
            if enemy.dead { continue; }
            let d = pos - enemy.pos;
            let dist_sq = d.length_squared();
            let hit_r = enemy.size() * 0.8;

            if dist_sq < hit_r * hit_r {
                hit_enemy_idx = Some(i);
                hit_pos = pos;
                break;
            }
        }

        if hit_enemy_idx.is_some() { break; }
        dist += step;
    }

    // Now apply damage and spawn particles
    if let Some(idx) = hit_enemy_idx {
        let enemy = &mut world.enemies[idx];
        let color = enemy.color();
        let enemy_pos = enemy.pos;
        let points = enemy.points();

        enemy.health -= damage;
        world.hit_marker = 1.0;
        particle_spawns.push((hit_pos, color, 10, 4.0, 0.15));

        if enemy.health <= 0.0 {
            enemy.dead = true;
            enemy.death_time = get_time();

            // Combo system!
            world.combo += 1;
            world.combo_timer = 2.0; // 2 second combo window
            let combo_bonus = world.combo * 50;
            world.player.score += points + combo_bonus;
            world.player.kills += 1;
            world.total_kills += 1;

            particle_spawns.push((enemy_pos, color, 25, 6.0, 0.2));
        }
    }

    // Spawn all particles
    for (pos, color, count, speed, size) in particle_spawns {
        spawn_particles(world, pos, color, count, speed, size);
    }
}

fn update_enemies(world: &mut World, dt: f32, time: f64) {
    let player_pos = vec3(world.player.pos.x, 0.0, world.player.pos.z);

    for i in 0..world.enemies.len() {
        if world.enemies[i].dead { continue; }

        let enemy_pos = vec3(world.enemies[i].pos.x, 0.0, world.enemies[i].pos.z);
        let to_player = player_pos - enemy_pos;
        let dist = to_player.length();

        if dist > 1.5 {
            let dir = to_player.normalize();
            let speed = world.enemies[i].speed;
            let new_x = world.enemies[i].pos.x + dir.x * speed * dt;
            let new_z = world.enemies[i].pos.z + dir.z * speed * dt;

            if !world.level.check_collision(new_x, world.enemies[i].pos.z, 0.5) {
                world.enemies[i].pos.x = new_x;
            }
            if !world.level.check_collision(world.enemies[i].pos.x, new_z, 0.5) {
                world.enemies[i].pos.z = new_z;
            }

            // Update enemy Y based on floor height
            let floor_height = world.level.get_floor_height(world.enemies[i].pos.x, world.enemies[i].pos.z);
            world.enemies[i].pos.y = floor_height + world.enemies[i].size();
        }

        // Attack
        let attack_cd = world.enemies[i].attack_cd;
        let last_attack = world.enemies[i].last_attack;
        let damage = world.enemies[i].damage;

        if dist < 2.0 && time - last_attack > attack_cd as f64 {
            world.enemies[i].last_attack = time;

            // Armor absorbs damage first
            let mut actual_damage = damage;
            if world.player.armor > 0.0 {
                let armor_absorb = actual_damage.min(world.player.armor);
                world.player.armor -= armor_absorb;
                actual_damage -= armor_absorb * 0.7; // Armor is 70% effective
            }

            world.player.health = (world.player.health - actual_damage).max(0.0);
            world.player.damage_flash = 0.5;
            world.screen_shake = 0.25;
        }
    }
}

fn update_projectiles(world: &mut World, dt: f32) {
    let mut explosions: Vec<(Vec3, f32)> = Vec::new();

    world.projectiles.retain_mut(|proj| {
        proj.pos += proj.vel * dt;

        if world.level.check_collision(proj.pos.x, proj.pos.z, 0.2) {
            if proj.explosive { explosions.push((proj.pos, proj.damage)); }
            return false;
        }

        for enemy in &mut world.enemies {
            if enemy.dead { continue; }
            if (proj.pos - enemy.pos).length() < 1.0 {
                if proj.explosive { explosions.push((proj.pos, proj.damage)); }
                else { enemy.health -= proj.damage; }
                return false;
            }
        }
        true
    });

    for (pos, damage) in explosions {
        explode(world, pos, damage);
    }
}

fn explode(world: &mut World, pos: Vec3, damage: f32) {
    let radius = 10.0; // Massive blast radius
    world.screen_shake = 0.8; // Big screen shake

    // MASSIVE explosion particles - fireballs
    for _ in 0..120 {
        let vel = vec3(
            rand::gen_range(-15.0, 15.0),
            rand::gen_range(3.0, 20.0),
            rand::gen_range(-15.0, 15.0),
        );
        let colors = [ORANGE, YELLOW, RED, Color::new(1.0, 0.5, 0.0, 1.0)];
        let color = colors[rand::gen_range(0, 4)];
        world.particles.push(Particle {
            pos, vel, color, life: rand::gen_range(0.5, 1.8), max_life: 1.8, size: rand::gen_range(0.3, 0.8)
        });
    }

    // Smoke particles (slower, darker, longer lasting)
    for _ in 0..40 {
        let vel = vec3(
            rand::gen_range(-5.0, 5.0),
            rand::gen_range(1.0, 8.0),
            rand::gen_range(-5.0, 5.0),
        );
        world.particles.push(Particle {
            pos, vel, color: Color::new(0.3, 0.3, 0.3, 0.8), life: rand::gen_range(1.0, 2.5), max_life: 2.5, size: rand::gen_range(0.4, 1.0)
        });
    }

    // Bright white-hot core flash particles
    for _ in 0..20 {
        let vel = vec3(
            rand::gen_range(-20.0, 20.0),
            rand::gen_range(5.0, 25.0),
            rand::gen_range(-20.0, 20.0),
        );
        world.particles.push(Particle {
            pos, vel, color: WHITE, life: rand::gen_range(0.1, 0.4), max_life: 0.4, size: rand::gen_range(0.5, 1.2)
        });
    }

    // Damage enemies
    for enemy in &mut world.enemies {
        if enemy.dead { continue; }
        let dist = (pos - enemy.pos).length();
        if dist < radius {
            let falloff = 1.0 - dist / radius;
            enemy.health -= damage * falloff;
            if enemy.health <= 0.0 {
                enemy.dead = true;
                enemy.death_time = get_time();
                world.player.score += enemy.points();
            }
        }
    }

    // Self damage
    let player_dist = (pos - world.player.pos).length();
    if player_dist < radius {
        let falloff = 1.0 - player_dist / radius;
        world.player.health = (world.player.health - damage * falloff * 0.3).max(0.0);
        world.player.damage_flash = 0.3;
    }
}

fn update_particles(world: &mut World, dt: f32) {
    for p in &mut world.particles {
        p.pos += p.vel * dt;
        p.vel.y -= 15.0 * dt;
        p.life -= dt;
    }
    world.particles.retain(|p| p.life > 0.0);
}

fn spawn_particles(world: &mut World, pos: Vec3, color: Color, count: i32, speed: f32, size: f32) {
    for _ in 0..count {
        let vel = vec3(
            rand::gen_range(-speed, speed),
            rand::gen_range(0.0, speed * 1.5),
            rand::gen_range(-speed, speed),
        );
        world.particles.push(Particle {
            pos, vel, color, life: rand::gen_range(0.2, 0.5), max_life: 0.5, size
        });
    }
}

fn update_pickups(world: &mut World, dt: f32) {
    // Decay pickup message
    world.player.pickup_msg_time -= dt;

    // Decay powerups
    world.player.speed_boost = (world.player.speed_boost - dt).max(0.0);
    world.player.damage_boost = (world.player.damage_boost - dt).max(0.0);

    // Collect particle spawns to avoid borrow issues
    let mut particle_spawns: Vec<(Vec3, Color)> = Vec::new();

    // Check pickup collection
    for pickup in &mut world.pickups {
        if pickup.collected { continue; }

        let dx = world.player.pos.x - pickup.pos.x;
        let dz = world.player.pos.z - pickup.pos.z;
        let dist = (dx * dx + dz * dz).sqrt();

        if dist < 1.5 {
            pickup.collected = true;
            world.player.pickup_msg = format!("+{}", pickup.name());
            world.player.pickup_msg_time = 2.0;

            // Store particle data for later
            particle_spawns.push((pickup.pos, pickup.color()));

            match pickup.pickup_type {
                PickupType::Health => {
                    world.player.health = (world.player.health + 25.0).min(MAX_HEALTH);
                }
                PickupType::Ammo => {
                    for weapon in &mut world.player.weapons {
                        if weapon.max_ammo > 0 {  // Only refill weapons with limited ammo
                            weapon.ammo = (weapon.ammo + weapon.max_ammo / 2).min(weapon.max_ammo);
                        }
                    }
                }
                PickupType::SpeedBoost => {
                    world.player.speed_boost = 10.0; // 10 second speed boost
                }
                PickupType::DamageBoost => {
                    world.player.damage_boost = 10.0; // 10 second damage boost
                }
                PickupType::Armor => {
                    world.player.armor = (world.player.armor + 50.0).min(100.0);
                }
            }

            world.player.score += 50;
        }
    }

    // Spawn particles after the loop
    for (pos, color) in particle_spawns {
        spawn_particles(world, pos, color, 15, 5.0, 0.15);
    }
}

fn check_victory(world: &mut World) {
    if world.alive_enemies() == 0 {
        let (ex, ez) = World::find_char(&world.level, 'X');
        let dist = ((world.player.pos.x - ex).powi(2) + (world.player.pos.z - ez).powi(2)).sqrt();
        if dist < 2.0 {
            world.state = GameState::Victory;
            set_cursor_grab(false);
            show_mouse(true);
        }
    }
}

// ============================================================================
// RENDERING
// ============================================================================

fn draw_billboard_sprite(texture: &Texture2D, pos: Vec3, size: f32, player: &Player, tint: Color) {
    // Calculate billboard orientation (always face the player)
    let to_player = vec3(player.pos.x - pos.x, 0.0, player.pos.z - pos.z).normalize();
    let right = vec3(-to_player.z, 0.0, to_player.x);
    let up = vec3(0.0, 1.0, 0.0);

    let half_size = size / 2.0;
    let color_bytes: [u8; 4] = [
        (tint.r * 255.0) as u8,
        (tint.g * 255.0) as u8,
        (tint.b * 255.0) as u8,
        (tint.a * 255.0) as u8,
    ];

    // Four corners of the billboard
    let v1 = pos + right * (-half_size) + up * size;  // top-left
    let v2 = pos + right * half_size + up * size;     // top-right
    let v3 = pos + right * half_size;                  // bottom-right
    let v4 = pos + right * (-half_size);               // bottom-left

    let normal = vec4(0.0, 0.0, 1.0, 0.0);

    // Draw textured quad using mesh for proper 3D billboard
    let vertices: [Vertex; 4] = [
        Vertex { position: v1, uv: vec2(0.0, 0.0), color: color_bytes, normal },
        Vertex { position: v2, uv: vec2(1.0, 0.0), color: color_bytes, normal },
        Vertex { position: v3, uv: vec2(1.0, 1.0), color: color_bytes, normal },
        Vertex { position: v4, uv: vec2(0.0, 1.0), color: color_bytes, normal },
    ];
    let indices: [u16; 6] = [0, 1, 2, 0, 2, 3];

    draw_mesh(&Mesh {
        vertices: vertices.to_vec(),
        indices: indices.to_vec(),
        texture: Some(texture.clone()),
    });
}

// Draw a REAL 3D Clarkson model using geometric primitives (humanoid figure)
fn draw_3d_clarkson(_texture: &Texture2D, pos: Vec3, size: f32, tint: Color, player: &Player, time: f32) {
    // Face the player (rotate towards them)
    let to_player = vec3(player.pos.x - pos.x, 0.0, player.pos.z - pos.z);
    let angle = to_player.z.atan2(to_player.x);

    // Walking animation
    let walk_cycle = (time * 8.0).sin() * 0.15;
    let arm_swing = (time * 8.0).sin() * 0.2;

    // Scale - size is the full height we want
    let height = size;
    let s = height / 4.5;  // Scale factor for body parts

    // Base position is at the enemy's feet (pos is center, so go down by half height)
    let base = vec3(pos.x, pos.y - height * 0.4, pos.z);

    // Colors based on tint
    let skin_color = Color::new(0.85, 0.7, 0.55, 1.0);
    let hair_color = Color::new(0.25, 0.2, 0.15, 1.0);
    let shirt_color = Color::new(
        0.2 + tint.r * 0.4,
        0.2 + tint.g * 0.4,
        0.4 + tint.b * 0.3,
        1.0
    );
    let pants_color = Color::new(0.15, 0.15, 0.2, 1.0);
    let shoe_color = Color::new(0.1, 0.08, 0.05, 1.0);

    // Calculate rotated offsets
    let cos_a = angle.cos();
    let sin_a = angle.sin();

    // Helper to rotate a point around Y axis
    let rotate = |offset: Vec3| -> Vec3 {
        vec3(
            offset.x * cos_a - offset.z * sin_a,
            offset.y,
            offset.x * sin_a + offset.z * cos_a
        )
    };

    // FEET (at ground level) - Z is left/right, X is forward/back for walking
    let foot_y = base.y + s * 0.1;
    let left_foot_offset = rotate(vec3(walk_cycle * s, 0.0, -s * 0.2));
    let right_foot_offset = rotate(vec3(-walk_cycle * s, 0.0, s * 0.2));
    draw_cube(base + left_foot_offset + vec3(0.0, foot_y - base.y, 0.0), vec3(s * 0.35, s * 0.15, s * 0.25), None, shoe_color);
    draw_cube(base + right_foot_offset + vec3(0.0, foot_y - base.y, 0.0), vec3(s * 0.35, s * 0.15, s * 0.25), None, shoe_color);

    // LEGS
    let leg_y = base.y + s * 0.6;
    draw_cube(base + left_foot_offset + vec3(0.0, leg_y - base.y, 0.0), vec3(s * 0.22, s * 0.8, s * 0.22), None, pants_color);
    draw_cube(base + right_foot_offset + vec3(0.0, leg_y - base.y, 0.0), vec3(s * 0.22, s * 0.8, s * 0.22), None, pants_color);

    // TORSO (wider on Z = left/right, thinner on X = front/back)
    let torso_y = base.y + s * 1.6;
    draw_cube(base + vec3(0.0, torso_y - base.y, 0.0), vec3(s * 0.35, s * 1.0, s * 0.6), None, shirt_color);

    // NECK
    let neck_y = base.y + s * 2.2;
    draw_cube(base + vec3(0.0, neck_y - base.y, 0.0), vec3(s * 0.2, s * 0.15, s * 0.2), None, skin_color);

    // HEAD
    let head_y = base.y + s * 2.6;
    let head_pos = base + vec3(0.0, head_y - base.y, 0.0);
    draw_sphere(head_pos, s * 0.35, None, skin_color);

    // Hair on top
    draw_cube(head_pos + vec3(0.0, s * 0.2, 0.0), vec3(s * 0.4, s * 0.2, s * 0.4), None, hair_color);

    // Eyes (facing player) - use X axis as forward since atan2 gives angle from +X
    let eye_forward = rotate(vec3(s * 0.3, 0.0, 0.0));
    let eye_left = rotate(vec3(0.0, 0.0, -s * 0.12));
    let eye_right = rotate(vec3(0.0, 0.0, s * 0.12));
    draw_sphere(head_pos + eye_forward + eye_left + vec3(0.0, s * 0.05, 0.0), s * 0.06, None, WHITE);
    draw_sphere(head_pos + eye_forward + eye_right + vec3(0.0, s * 0.05, 0.0), s * 0.06, None, WHITE);

    // Pupils (slightly more forward than whites)
    let pupil_forward = rotate(vec3(s * 0.34, 0.0, 0.0));
    draw_sphere(head_pos + pupil_forward + eye_left + vec3(0.0, s * 0.05, 0.0), s * 0.03, None, BLACK);
    draw_sphere(head_pos + pupil_forward + eye_right + vec3(0.0, s * 0.05, 0.0), s * 0.03, None, BLACK);

    // ARMS (Z axis is left/right when X is forward)
    let arm_y = base.y + s * 1.8;
    let left_arm_offset = rotate(vec3(arm_swing * s * 0.5, 0.0, -s * 0.45));
    let right_arm_offset = rotate(vec3(-arm_swing * s * 0.5, 0.0, s * 0.45));
    draw_cube(base + left_arm_offset + vec3(0.0, arm_y - base.y, 0.0), vec3(s * 0.18, s * 0.7, s * 0.18), None, shirt_color);
    draw_cube(base + right_arm_offset + vec3(0.0, arm_y - base.y, 0.0), vec3(s * 0.18, s * 0.7, s * 0.18), None, shirt_color);

    // Hands
    let hand_y = base.y + s * 1.35;
    draw_sphere(base + left_arm_offset + vec3(0.0, hand_y - base.y, 0.0), s * 0.1, None, skin_color);
    draw_sphere(base + right_arm_offset + vec3(0.0, hand_y - base.y, 0.0), s * 0.1, None, skin_color);
}

// Generate a procedural "Jeremy Clarkson" style face texture
fn generate_clarkson_texture() -> Texture2D {
    let size = 64usize;
    let mut pixels = vec![0u8; size * size * 4];

    for y in 0..size {
        for x in 0..size {
            let idx = (y * size + x) * 4;
            let fx = x as f32 / size as f32;
            let fy = y as f32 / size as f32;

            // Default: transparent
            let mut r = 0u8;
            let mut g = 0u8;
            let mut b = 0u8;
            let mut a = 0u8;

            // Head shape (oval)
            let head_cx = 0.5;
            let head_cy = 0.45;
            let head_rx = 0.35;
            let head_ry = 0.42;
            let head_dist = ((fx - head_cx) / head_rx).powi(2) + ((fy - head_cy) / head_ry).powi(2);

            if head_dist < 1.0 {
                // Skin color
                r = 220; g = 180; b = 150; a = 255;

                // Hair (top of head, dark curly mess)
                if fy < 0.25 && head_dist < 0.8 {
                    let hair_noise = ((fx * 47.0).sin() * (fy * 31.0).cos()).abs();
                    if hair_noise > 0.3 || fy < 0.15 {
                        r = 60; g = 45; b = 35; // Dark brown hair
                    }
                }

                // Eyes
                let eye_y = 0.4;
                let eye_left_x = 0.35;
                let eye_right_x = 0.65;
                let eye_r = 0.06;

                let left_eye_dist = ((fx - eye_left_x).powi(2) + (fy - eye_y).powi(2)).sqrt();
                let right_eye_dist = ((fx - eye_right_x).powi(2) + (fy - eye_y).powi(2)).sqrt();

                if left_eye_dist < eye_r || right_eye_dist < eye_r {
                    r = 255; g = 255; b = 255; // White of eye
                }
                if left_eye_dist < eye_r * 0.5 || right_eye_dist < eye_r * 0.5 {
                    r = 50; g = 80; b = 120; // Blue iris
                }
                if left_eye_dist < eye_r * 0.25 || right_eye_dist < eye_r * 0.25 {
                    r = 0; g = 0; b = 0; // Pupil
                }

                // Eyebrows (thick, slightly furrowed - very Clarkson)
                let brow_y = 0.32;
                if fy > brow_y - 0.03 && fy < brow_y + 0.02 {
                    if (fx > 0.25 && fx < 0.45) || (fx > 0.55 && fx < 0.75) {
                        r = 50; g = 40; b = 30;
                    }
                }

                // Nose
                let nose_x = 0.5;
                if fx > nose_x - 0.04 && fx < nose_x + 0.04 && fy > 0.42 && fy < 0.58 {
                    r = 200; g = 160; b = 130; // Slight shadow
                }

                // Mouth (smirking)
                let mouth_y = 0.68;
                if fy > mouth_y - 0.02 && fy < mouth_y + 0.02 {
                    if fx > 0.35 && fx < 0.65 {
                        let smirk = (fx - 0.5) * 0.1;
                        if fy < mouth_y + smirk {
                            r = 150; g = 80; b = 80; // Lips
                        }
                    }
                }

                // Jeans jacket collar (bottom)
                if fy > 0.82 {
                    r = 50; g = 60; b = 120; // Denim blue
                }
            }

            pixels[idx] = r;
            pixels[idx + 1] = g;
            pixels[idx + 2] = b;
            pixels[idx + 3] = a;
        }
    }

    let texture = Texture2D::from_rgba8(size as u16, size as u16, &pixels);
    texture.set_filter(FilterMode::Nearest);
    texture
}

fn render_3d(world: &World) {
    // Camera with shake
    let shake = if world.screen_shake > 0.0 {
        vec3(
            rand::gen_range(-world.screen_shake, world.screen_shake) * 0.3,
            rand::gen_range(-world.screen_shake, world.screen_shake) * 0.3,
            0.0
        )
    } else { Vec3::ZERO };

    let cam_pos = world.player.pos + shake;
    let cam_target = cam_pos + world.player.forward() * 10.0;

    // Fixed FOV - dynamic FOV causes camera inversion issues in macroquad
    set_camera(&Camera3D {
        position: cam_pos,
        target: cam_target,
        up: Vec3::Y,
        fovy: 70.0,
        projection: Projection::Perspective,
        ..Default::default()
    });

    // Draw floor tiles with varying heights
    for y in 0..world.level.height {
        for x in 0..world.level.width {
            let floor_h = world.level.floor_heights[y][x];
            let wx = x as f32 * CELL_SIZE + CELL_SIZE / 2.0;
            let wz = y as f32 * CELL_SIZE + CELL_SIZE / 2.0;

            // Skip walls
            if world.level.grid[y][x] == '#' { continue; }

            // Floor color varies with height
            let height_tint = floor_h * 0.05;
            let floor_color = Color::new(0.15 + height_tint, 0.15 + height_tint, 0.2 + height_tint, 1.0);

            if floor_h > 0.01 {
                // Elevated floor - draw as a raised platform
                draw_cube(
                    vec3(wx, floor_h / 2.0, wz),
                    vec3(CELL_SIZE, floor_h, CELL_SIZE),
                    None,
                    floor_color
                );
                // Top surface
                draw_cube(
                    vec3(wx, floor_h + 0.05, wz),
                    vec3(CELL_SIZE, 0.1, CELL_SIZE),
                    None,
                    Color::new(0.2 + height_tint, 0.2 + height_tint, 0.25 + height_tint, 1.0)
                );
            } else {
                // Ground level floor
                draw_cube(
                    vec3(wx, -0.05, wz),
                    vec3(CELL_SIZE, 0.1, CELL_SIZE),
                    None,
                    floor_color
                );
            }
        }
    }

    // Ceiling
    draw_plane(
        vec3(world.level.width as f32 * CELL_SIZE / 2.0, WALL_HEIGHT + 3.0, world.level.height as f32 * CELL_SIZE / 2.0),
        vec2(world.level.width as f32 * CELL_SIZE, world.level.height as f32 * CELL_SIZE),
        None, Color::new(0.1, 0.1, 0.15, 1.0)
    );

    // Walls and exit
    for (y, row) in world.level.grid.iter().enumerate() {
        for (x, &cell) in row.iter().enumerate() {
            let wx = x as f32 * CELL_SIZE + CELL_SIZE / 2.0;
            let wz = y as f32 * CELL_SIZE + CELL_SIZE / 2.0;

            if cell == '#' {
                let v = ((x + y) % 3) as f32 * 0.03;
                let color = Color::new(0.3 + v, 0.3 + v, 0.35 + v, 1.0);
                draw_cube(vec3(wx, WALL_HEIGHT / 2.0, wz), vec3(CELL_SIZE, WALL_HEIGHT, CELL_SIZE), None, color);
                draw_cube_wires(vec3(wx, WALL_HEIGHT / 2.0, wz), vec3(CELL_SIZE, WALL_HEIGHT, CELL_SIZE), Color::new(0.2, 0.2, 0.25, 1.0));
            } else if cell == 'X' {
                let pulse = (get_time() as f32 * 3.0).sin() * 0.3 + 0.7;
                draw_cube(vec3(wx, 0.15, wz), vec3(2.5, 0.3, 2.5), None, Color::new(0.0, pulse, 0.0, 1.0));
            }
        }
    }

    // Enemies
    let time = get_time();
    for enemy in &world.enemies {
        if enemy.dead {
            let progress = ((time - enemy.death_time) as f32).min(1.0);
            if progress < 1.0 {
                let scale = 1.0 - progress;
                let y_off = progress * enemy.size();
                if let Some(tex) = &world.enemy_texture {
                    draw_billboard_sprite(tex, enemy.pos - vec3(0.0, y_off, 0.0), enemy.size() * 2.0 * scale, &world.player, Color::new(1.0, 1.0, 1.0, 1.0 - progress));
                } else {
                    draw_cube(
                        vec3(enemy.pos.x, enemy.pos.y - y_off, enemy.pos.z),
                        vec3(enemy.size() * scale, enemy.size() * 2.0 * scale, enemy.size() * scale),
                        None, enemy.color()
                    );
                }
            }
            continue;
        }

        // Draw enemy as real 3D humanoid model
        if let Some(tex) = &world.enemy_texture {
            let tint = match enemy.etype {
                EnemyType::Grunt => WHITE,
                EnemyType::Heavy => Color::new(0.7, 0.5, 1.0, 1.0),
                EnemyType::Demon => Color::new(1.0, 0.6, 0.3, 1.0),
            };
            // Draw real 3D Clarkson model with walking animation
            draw_3d_clarkson(tex, enemy.pos, enemy.size() * 2.5, tint, &world.player, time as f32);
        } else {
            // Fallback cube rendering
            draw_cube(enemy.pos, vec3(enemy.size(), enemy.size() * 2.0, enemy.size()), None, enemy.color());
            let to_player = (world.player.pos - enemy.pos).normalize();
            let eye_pos = enemy.pos + vec3(to_player.x * enemy.size() * 0.5, enemy.size() * 0.5, to_player.z * enemy.size() * 0.5);
            draw_sphere(eye_pos, 0.15, None, YELLOW);
        }

        // Health bar
        let hp_pct = enemy.health / enemy.max_health;
        let bar_w = enemy.size() * 1.5;
        let bar_pos = enemy.pos + vec3(0.0, enemy.size() * 1.8, 0.0);
        draw_cube(bar_pos, vec3(bar_w, 0.1, 0.1), None, DARKGRAY);
        draw_cube(bar_pos + vec3((hp_pct - 1.0) * bar_w * 0.5, 0.0, 0.0), vec3(bar_w * hp_pct, 0.12, 0.12), None, RED);
    }

    // Projectiles (rockets with trail)
    for proj in &world.projectiles {
        if proj.explosive {
            // Rocket body
            draw_sphere(proj.pos, 0.4, None, Color::new(0.3, 0.35, 0.3, 1.0));
            // Glowing tip
            draw_sphere(proj.pos + proj.vel.normalize() * 0.3, 0.25, None, ORANGE);
            // Fiery trail
            for i in 1..6 {
                let trail_pos = proj.pos - proj.vel.normalize() * (i as f32 * 0.3);
                let alpha = 1.0 - (i as f32 / 6.0);
                let size = 0.35 - (i as f32 * 0.04);
                draw_sphere(trail_pos, size, None, Color::new(1.0, 0.5 * alpha, 0.0, alpha));
            }
        }
    }

    // Particles
    for p in &world.particles {
        let alpha = p.life / p.max_life;
        let mut c = p.color;
        c.a = alpha;
        draw_sphere(p.pos, p.size, None, c);
    }

    // Pickups (bobbing and spinning!)
    let time = get_time() as f32;
    for pickup in &world.pickups {
        if pickup.collected { continue; }

        let bob = (time * 3.0 + pickup.bob_offset).sin() * 0.2;
        let pos = vec3(pickup.pos.x, pickup.pos.y + bob + 0.5, pickup.pos.z);

        // Draw pickup as glowing cube
        let color = pickup.color();
        let pulse = ((time * 5.0).sin() * 0.3 + 0.7) as f32;
        let glow_color = Color::new(color.r * pulse, color.g * pulse, color.b * pulse, 1.0);

        draw_cube(pos, vec3(0.5, 0.5, 0.5), None, glow_color);
        draw_cube_wires(pos, vec3(0.6, 0.6, 0.6), color);

        // Inner glow
        draw_sphere(pos, 0.3, None, Color::new(color.r, color.g, color.b, 0.5));
    }

    // Render first-person 3D weapon (before switching to 2D)
    render_weapon_3d(world);

    set_default_camera();
}

// Draw a solid oriented box using the player's coordinate system
fn draw_oriented_box(center: Vec3, half_extents: Vec3, right: Vec3, up: Vec3, forward: Vec3, color: Color) {
    // half_extents: (right_size, up_size, forward_size) in local space
    let r = right * half_extents.x;
    let u = up * half_extents.y;
    let f = forward * half_extents.z;

    // 8 corners of the box
    let c = [
        center - r - u - f, // 0: back-bottom-left
        center + r - u - f, // 1: back-bottom-right
        center + r + u - f, // 2: back-top-right
        center - r + u - f, // 3: back-top-left
        center - r - u + f, // 4: front-bottom-left
        center + r - u + f, // 5: front-bottom-right
        center + r + u + f, // 6: front-top-right
        center - r + u + f, // 7: front-top-left
    ];

    // Face normals (Vec4 with w=0)
    let n_back = vec4(-forward.x, -forward.y, -forward.z, 0.0);
    let n_front = vec4(forward.x, forward.y, forward.z, 0.0);
    let n_left = vec4(-right.x, -right.y, -right.z, 0.0);
    let n_right = vec4(right.x, right.y, right.z, 0.0);
    let n_top = vec4(up.x, up.y, up.z, 0.0);
    let n_bottom = vec4(-up.x, -up.y, -up.z, 0.0);

    // Build mesh vertices (24 vertices for 6 faces, each face has 4 vertices)
    let vertices = vec![
        // Back face
        Vertex { position: c[0], uv: vec2(0.0, 0.0), color: color.into(), normal: n_back },
        Vertex { position: c[1], uv: vec2(1.0, 0.0), color: color.into(), normal: n_back },
        Vertex { position: c[2], uv: vec2(1.0, 1.0), color: color.into(), normal: n_back },
        Vertex { position: c[3], uv: vec2(0.0, 1.0), color: color.into(), normal: n_back },
        // Front face
        Vertex { position: c[5], uv: vec2(0.0, 0.0), color: color.into(), normal: n_front },
        Vertex { position: c[4], uv: vec2(1.0, 0.0), color: color.into(), normal: n_front },
        Vertex { position: c[7], uv: vec2(1.0, 1.0), color: color.into(), normal: n_front },
        Vertex { position: c[6], uv: vec2(0.0, 1.0), color: color.into(), normal: n_front },
        // Left face
        Vertex { position: c[4], uv: vec2(0.0, 0.0), color: color.into(), normal: n_left },
        Vertex { position: c[0], uv: vec2(1.0, 0.0), color: color.into(), normal: n_left },
        Vertex { position: c[3], uv: vec2(1.0, 1.0), color: color.into(), normal: n_left },
        Vertex { position: c[7], uv: vec2(0.0, 1.0), color: color.into(), normal: n_left },
        // Right face
        Vertex { position: c[1], uv: vec2(0.0, 0.0), color: color.into(), normal: n_right },
        Vertex { position: c[5], uv: vec2(1.0, 0.0), color: color.into(), normal: n_right },
        Vertex { position: c[6], uv: vec2(1.0, 1.0), color: color.into(), normal: n_right },
        Vertex { position: c[2], uv: vec2(0.0, 1.0), color: color.into(), normal: n_right },
        // Top face
        Vertex { position: c[3], uv: vec2(0.0, 0.0), color: color.into(), normal: n_top },
        Vertex { position: c[2], uv: vec2(1.0, 0.0), color: color.into(), normal: n_top },
        Vertex { position: c[6], uv: vec2(1.0, 1.0), color: color.into(), normal: n_top },
        Vertex { position: c[7], uv: vec2(0.0, 1.0), color: color.into(), normal: n_top },
        // Bottom face
        Vertex { position: c[4], uv: vec2(0.0, 0.0), color: color.into(), normal: n_bottom },
        Vertex { position: c[5], uv: vec2(1.0, 0.0), color: color.into(), normal: n_bottom },
        Vertex { position: c[1], uv: vec2(1.0, 1.0), color: color.into(), normal: n_bottom },
        Vertex { position: c[0], uv: vec2(0.0, 1.0), color: color.into(), normal: n_bottom },
    ];

    // Indices for 6 faces (2 triangles each)
    let indices: Vec<u16> = vec![
        0, 1, 2, 0, 2, 3,       // back
        4, 5, 6, 4, 6, 7,       // front
        8, 9, 10, 8, 10, 11,    // left
        12, 13, 14, 12, 14, 15, // right
        16, 17, 18, 16, 18, 19, // top
        20, 21, 22, 20, 22, 23, // bottom
    ];

    let mesh = Mesh {
        vertices,
        indices,
        texture: None,
    };

    draw_mesh(&mesh);
}

fn render_weapon_3d(world: &World) {
    let time = get_time() as f32;
    let aim = world.player.aim_transition;

    // Get player orientation vectors
    let forward = world.player.forward();
    let right = world.player.right();
    let up = vec3(0.0, 1.0, 0.0);

    // Weapon sway (reduced when aiming)
    let sway_amount = 1.0 - aim * 0.8;
    let sway_x = (time * 1.5).sin() * 0.02 * sway_amount;
    let sway_y = (time * 2.0).cos() * 0.01 * sway_amount;

    // Muzzle kick
    let kick = world.muzzle_flash * 0.05;

    // Base position: in front of camera, offset right and down
    // When ADS: move to center
    let base_forward = 0.4;
    let base_right = 0.25;
    let base_down = 0.2;

    let ads_forward = 0.35;
    let ads_right = 0.0;
    let ads_down = 0.08;

    let fwd_offset = base_forward + (ads_forward - base_forward) * aim;
    let right_offset = base_right + (ads_right - base_right) * aim + sway_x;
    let down_offset = base_down + (ads_down - base_down) * aim + sway_y + kick;

    let weapon_pos = world.player.pos
        + forward * fwd_offset
        + right * right_offset
        - up * down_offset;

    let weapon = &world.player.weapons[world.player.current_weapon];

    // Colors
    let metal_dark = Color::new(0.15, 0.15, 0.18, 1.0);
    let metal_mid = Color::new(0.25, 0.25, 0.3, 1.0);
    let metal_light = Color::new(0.35, 0.35, 0.4, 1.0);
    let wood = Color::new(0.4, 0.25, 0.1, 1.0);
    let olive = Color::new(0.2, 0.28, 0.2, 1.0);

    // Helper to draw oriented box with full size (converts to half extents)
    let draw_box = |center: Vec3, size: Vec3, color: Color| {
        draw_oriented_box(center, size * 0.5, right, up, forward, color);
    };

    match weapon.wtype {
        WeaponType::Pistol => {
            // Grip
            let grip_pos = weapon_pos - up * 0.05;
            draw_box(grip_pos, vec3(0.03, 0.07, 0.04), metal_dark);
            // Slide/body
            let body_pos = weapon_pos + forward * 0.02;
            draw_box(body_pos, vec3(0.035, 0.04, 0.08), metal_mid);
            // Barrel
            let barrel_pos = weapon_pos + forward * 0.08;
            draw_box(barrel_pos, vec3(0.015, 0.015, 0.06), metal_dark);
            // Front sight
            let sight_pos = weapon_pos + forward * 0.1 + up * 0.025;
            draw_box(sight_pos, vec3(0.008, 0.015, 0.008), metal_dark);
            // Rear sight
            let rear_sight = weapon_pos - forward * 0.01 + up * 0.025;
            draw_box(rear_sight, vec3(0.025, 0.012, 0.008), metal_dark);
        }
        WeaponType::Shotgun => {
            // Stock
            let stock_pos = weapon_pos - forward * 0.1 - up * 0.02;
            draw_box(stock_pos, vec3(0.04, 0.06, 0.15), wood);
            // Receiver
            let receiver_pos = weapon_pos + forward * 0.02;
            draw_box(receiver_pos, vec3(0.04, 0.05, 0.1), metal_mid);
            // Double barrels
            let barrel1_pos = weapon_pos + forward * 0.15 + right * 0.012;
            let barrel2_pos = weapon_pos + forward * 0.15 - right * 0.012;
            draw_box(barrel1_pos, vec3(0.018, 0.018, 0.2), metal_dark);
            draw_box(barrel2_pos, vec3(0.018, 0.018, 0.2), metal_dark);
            // Pump grip
            let pump_pos = weapon_pos + forward * 0.08 - up * 0.03;
            draw_box(pump_pos, vec3(0.035, 0.03, 0.06), wood);
        }
        WeaponType::MachineGun => {
            // Stock
            let stock_pos = weapon_pos - forward * 0.12;
            draw_box(stock_pos, vec3(0.03, 0.05, 0.12), metal_dark);
            // Receiver
            let receiver_pos = weapon_pos + forward * 0.02;
            draw_box(receiver_pos, vec3(0.05, 0.06, 0.14), metal_mid);
            // Barrel
            let barrel_pos = weapon_pos + forward * 0.18;
            draw_box(barrel_pos, vec3(0.02, 0.02, 0.2), metal_dark);
            // Magazine
            let mag_pos = weapon_pos - up * 0.08;
            draw_box(mag_pos, vec3(0.025, 0.08, 0.04), metal_dark);
            // Top rail / red dot housing
            let rail_pos = weapon_pos + up * 0.04 + forward * 0.02;
            draw_box(rail_pos, vec3(0.03, 0.02, 0.06), metal_light);
            // Foregrip
            let foregrip_pos = weapon_pos + forward * 0.1 - up * 0.04;
            draw_box(foregrip_pos, vec3(0.025, 0.04, 0.04), metal_dark);
        }
        WeaponType::Rocket => {
            // Main tube
            let tube_pos = weapon_pos + forward * 0.05;
            draw_box(tube_pos, vec3(0.08, 0.08, 0.35), olive);
            // Rear exhaust
            let exhaust_pos = weapon_pos - forward * 0.15;
            draw_box(exhaust_pos, vec3(0.07, 0.07, 0.05), metal_dark);
            // Front opening
            let front_pos = weapon_pos + forward * 0.22;
            draw_box(front_pos, vec3(0.06, 0.06, 0.02), metal_dark);
            // Grip
            let grip_pos = weapon_pos - up * 0.08 - forward * 0.02;
            draw_box(grip_pos, vec3(0.03, 0.06, 0.05), metal_dark);
            // Sight
            let sight_pos = weapon_pos + up * 0.06 + forward * 0.05;
            draw_box(sight_pos, vec3(0.04, 0.03, 0.08), olive);
        }
    }

    // Muzzle flash (3D sphere at barrel end)
    if world.muzzle_flash > 0.3 {
        let flash_dist = match weapon.wtype {
            WeaponType::Pistol => 0.12,
            WeaponType::Shotgun => 0.26,
            WeaponType::MachineGun => 0.28,
            WeaponType::Rocket => 0.24,
        };
        let flash_pos = weapon_pos + forward * flash_dist;
        let flash_size = world.muzzle_flash * 0.08;
        draw_sphere(flash_pos, flash_size, None, Color::new(1.0, 0.9, 0.4, world.muzzle_flash));
        draw_sphere(flash_pos, flash_size * 0.5, None, Color::new(1.0, 1.0, 0.8, world.muzzle_flash));
    }
}

fn render_hud(world: &World) {
    let sw = screen_width();
    let sh = screen_height();
    let cx = sw / 2.0;
    let cy = sh / 2.0;

    // Crosshair
    let cross_color = if world.hit_marker > 0.0 { RED } else { WHITE };
    draw_line(cx - 15.0, cy, cx - 5.0, cy, 2.0, cross_color);
    draw_line(cx + 5.0, cy, cx + 15.0, cy, 2.0, cross_color);
    draw_line(cx, cy - 15.0, cx, cy - 5.0, 2.0, cross_color);
    draw_line(cx, cy + 5.0, cx, cy + 15.0, 2.0, cross_color);

    // Hit marker
    if world.hit_marker > 0.0 {
        let s = 12.0 * world.hit_marker;
        draw_line(cx - s, cy - s, cx - 4.0, cy - 4.0, 2.0, RED);
        draw_line(cx + s, cy - s, cx + 4.0, cy - 4.0, 2.0, RED);
        draw_line(cx - s, cy + s, cx - 4.0, cy + 4.0, 2.0, RED);
        draw_line(cx + s, cy + s, cx + 4.0, cy + 4.0, 2.0, RED);
    }

    // Muzzle flash
    if world.muzzle_flash > 0.0 {
        draw_rectangle(cx - 40.0, cy + 80.0, 80.0, 40.0, Color::new(1.0, 0.8, 0.4, world.muzzle_flash * 0.5));
    }

    // Damage flash
    if world.player.damage_flash > 0.0 {
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(1.0, 0.0, 0.0, world.player.damage_flash * 0.3));
    }

    // Speed boost screen tint
    if world.player.speed_boost > 0.0 {
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.5, 1.0, 0.1));
    }

    // Damage boost screen tint
    if world.player.damage_boost > 0.0 {
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(1.0, 0.3, 0.0, 0.1));
    }

    // Health bar
    let hp_pct = world.player.health / MAX_HEALTH;
    draw_rectangle(20.0, sh - 60.0, 200.0, 30.0, DARKGRAY);
    draw_rectangle(22.0, sh - 58.0, 196.0 * hp_pct, 26.0, Color::new(1.0 - hp_pct, hp_pct, 0.2, 1.0));
    draw_text(&format!("HEALTH: {:.0}", world.player.health), 25.0, sh - 40.0, 20.0, WHITE);

    // Armor bar
    if world.player.armor > 0.0 {
        let armor_pct = world.player.armor / 100.0;
        draw_rectangle(20.0, sh - 95.0, 200.0, 25.0, DARKGRAY);
        draw_rectangle(22.0, sh - 93.0, 196.0 * armor_pct, 21.0, BLUE);
        draw_text(&format!("ARMOR: {:.0}", world.player.armor), 25.0, sh - 78.0, 16.0, WHITE);
    }

    // Powerup indicators
    let mut powerup_y = sh - 130.0;
    if world.player.speed_boost > 0.0 {
        draw_rectangle(20.0, powerup_y, 150.0, 20.0, Color::new(0.0, 0.0, 0.0, 0.6));
        draw_text(&format!("SPEED: {:.1}s", world.player.speed_boost), 25.0, powerup_y + 15.0, 16.0, SKYBLUE);
        powerup_y -= 25.0;
    }
    if world.player.damage_boost > 0.0 {
        draw_rectangle(20.0, powerup_y, 150.0, 20.0, Color::new(0.0, 0.0, 0.0, 0.6));
        draw_text(&format!("DAMAGE x2: {:.1}s", world.player.damage_boost), 25.0, powerup_y + 15.0, 16.0, ORANGE);
    }

    // Weapon info
    let weapon = &world.player.weapons[world.player.current_weapon];
    draw_rectangle(sw - 220.0, sh - 100.0, 200.0, 80.0, Color::new(0.0, 0.0, 0.0, 0.6));
    draw_text(weapon.name(), sw - 210.0, sh - 75.0, 24.0, YELLOW);
    let ammo_str = if weapon.ammo < 0 { "INF".into() } else { format!("{}/{}", weapon.ammo, weapon.max_ammo) };
    draw_text(&ammo_str, sw - 210.0, sh - 45.0, 28.0, WHITE);

    // Weapon selector
    for i in 0..4 {
        let x = sw - 220.0 + i as f32 * 50.0;
        let color = if i == world.player.current_weapon { YELLOW } else { GRAY };
        draw_rectangle(x, sh - 130.0, 40.0, 25.0, Color::new(0.0, 0.0, 0.0, 0.6));
        draw_text(&format!("{}", i + 1), x + 15.0, sh - 110.0, 18.0, color);
    }

    // Score, kills and level
    draw_text(&format!("SCORE: {}", world.player.score), 20.0, 35.0, 28.0, YELLOW);
    draw_text(&format!("KILLS: {}", world.player.kills), 200.0, 35.0, 20.0, RED);
    draw_text(&format!("LEVEL {}/5: {}", world.current_level, world.level.name), 20.0, 60.0, 20.0, WHITE);

    // Combo display
    if world.combo > 1 {
        let combo_scale = 1.0 + world.combo as f32 * 0.1;
        draw_text(&format!("COMBO x{}!", world.combo), cx - 60.0, 140.0, (28.0 * combo_scale) as u16 as f32, ORANGE);
    }

    let alive = world.alive_enemies();
    let enemy_color = if alive > 0 { RED } else { GREEN };
    draw_text(&format!("ENEMIES: {}", alive), 20.0, 85.0, 20.0, enemy_color);

    if alive == 0 {
        draw_text("ALL CLARKSONS DEFEATED! FIND THE EXIT!", cx - 200.0, 100.0, 24.0, GREEN);
    }

    // Pickup message
    if world.player.pickup_msg_time > 0.0 {
        let alpha = world.player.pickup_msg_time.min(1.0);
        draw_text(&world.player.pickup_msg, cx - 50.0, cy + 50.0, 24.0, Color::new(0.0, 1.0, 0.0, alpha));
    }

    // Minimap
    render_minimap(world);
}

fn render_minimap(world: &World) {
    let sw = screen_width();
    let cell = 5.0;
    let map_x = sw - 160.0;
    let map_y = 20.0;
    let max_cells = 25;

    let w = world.level.width.min(max_cells);
    let h = world.level.height.min(max_cells);

    draw_rectangle(map_x - 5.0, map_y - 5.0, w as f32 * cell + 10.0, h as f32 * cell + 10.0, Color::new(0.0, 0.0, 0.0, 0.7));

    for y in 0..h {
        for x in 0..w {
            let c = world.level.grid[y][x];
            let color = match c {
                '#' => Color::new(0.3, 0.3, 0.35, 1.0),
                'X' => GREEN,
                _ => Color::new(0.1, 0.1, 0.12, 1.0),
            };
            draw_rectangle(map_x + x as f32 * cell, map_y + y as f32 * cell, cell - 1.0, cell - 1.0, color);
        }
    }

    // Player
    let px = (world.player.pos.x / CELL_SIZE) * cell;
    let pz = (world.player.pos.z / CELL_SIZE) * cell;
    draw_rectangle(map_x + px - cell/2.0, map_y + pz - cell/2.0, cell, cell, BLUE);

    // Direction
    let dir_len = 8.0;
    draw_line(
        map_x + px, map_y + pz,
        map_x + px + world.player.yaw.cos() * dir_len,
        map_y + pz + world.player.yaw.sin() * dir_len,
        2.0, SKYBLUE
    );

    // Enemies
    for enemy in &world.enemies {
        if enemy.dead { continue; }
        let ex = (enemy.pos.x / CELL_SIZE) * cell;
        let ez = (enemy.pos.z / CELL_SIZE) * cell;
        draw_rectangle(map_x + ex - cell/2.0, map_y + ez - cell/2.0, cell - 1.0, cell - 1.0, RED);
    }
}

fn render_menu() {
    let sw = screen_width();
    let sh = screen_height();
    let time = get_time() as f32;

    clear_background(Color::new(0.04, 0.04, 0.08, 1.0));

    // Animated title
    let title_pulse = (time * 2.0).sin() * 0.1 + 0.9;
    draw_text("RUST BLASTER", sw/2.0 - 180.0, sh/4.0, (56.0 * title_pulse) as u16 as f32, RED);
    draw_text("RUST BLASTER", sw/2.0 - 178.0, sh/4.0 - 2.0, (56.0 * title_pulse) as u16 as f32, ORANGE);

    draw_text("vs THE CLARKSONS", sw/2.0 - 100.0, sh/4.0 + 50.0, 26.0, YELLOW);
    draw_text("A 3D First-Person Shooter", sw/2.0 - 140.0, sh/4.0 + 80.0, 22.0, GRAY);

    // Blinking start text
    if (time * 2.0).sin() > 0.0 {
        draw_text("PRESS ENTER OR SPACE TO START", sw/2.0 - 175.0, sh/2.0, 22.0, WHITE);
    }

    let controls = [
        "WASD - Move",
        "Mouse - Look",
        "Left Click - Shoot",
        "1-4 / Scroll - Weapons",
        "Shift - Sprint",
        "ESC - Pause"
    ];
    for (i, c) in controls.iter().enumerate() {
        draw_text(c, sw/2.0 - 90.0, sh/2.0 + 60.0 + i as f32 * 25.0, 18.0, LIGHTGRAY);
    }

    // Features list
    draw_text("5 LEVELS - POWERUPS - COMBOS - ARMOR", sw/2.0 - 180.0, sh - 80.0, 18.0, SKYBLUE);
    draw_text("Built with Rust + macroquad", sw/2.0 - 110.0, sh - 40.0, 16.0, DARKGRAY);
}

fn render_pause() {
    let sw = screen_width();
    let sh = screen_height();

    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.6));
    draw_text("PAUSED", sw/2.0 - 70.0, sh/2.0 - 20.0, 48.0, WHITE);
    draw_text("Press ESC to Resume", sw/2.0 - 100.0, sh/2.0 + 30.0, 20.0, GRAY);
}

fn render_death(world: &World) {
    let sw = screen_width();
    let sh = screen_height();

    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.2, 0.0, 0.0, 0.8));
    draw_text("YOU DIED", sw/2.0 - 100.0, sh/2.0 - 40.0, 56.0, RED);
    draw_text(&format!("Final Score: {}", world.player.score), sw/2.0 - 90.0, sh/2.0 + 20.0, 24.0, WHITE);
    draw_text("Press ENTER to Restart", sw/2.0 - 120.0, sh/2.0 + 60.0, 20.0, GRAY);
}

fn render_victory(world: &World) {
    let sw = screen_width();
    let sh = screen_height();

    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.2, 0.0, 0.8));

    if world.current_level >= 5 {
        draw_text("VICTORY!", sw/2.0 - 90.0, sh/2.0 - 100.0, 56.0, GREEN);
        draw_text("You defeated all the Clarksons!", sw/2.0 - 160.0, sh/2.0 - 40.0, 22.0, WHITE);
        draw_text(&format!("Final Score: {}", world.player.score), sw/2.0 - 80.0, sh/2.0, 24.0, YELLOW);
        draw_text(&format!("Total Kills: {}", world.total_kills), sw/2.0 - 70.0, sh/2.0 + 35.0, 20.0, RED);
        draw_text(&format!("Best Combo: x{}", world.combo), sw/2.0 - 60.0, sh/2.0 + 60.0, 20.0, ORANGE);
        draw_text("Press ENTER for Menu", sw/2.0 - 100.0, sh/2.0 + 100.0, 20.0, GRAY);
    } else {
        draw_text("LEVEL COMPLETE!", sw/2.0 - 140.0, sh/2.0 - 60.0, 48.0, GREEN);
        draw_text(&format!("Score: {}", world.player.score), sw/2.0 - 50.0, sh/2.0, 24.0, WHITE);
        draw_text(&format!("Kills this level: {}", world.player.kills), sw/2.0 - 80.0, sh/2.0 + 30.0, 18.0, RED);
        draw_text(&format!("Next: Level {}/5", world.current_level + 1), sw/2.0 - 60.0, sh/2.0 + 60.0, 18.0, SKYBLUE);
        draw_text("Press ENTER for Next Level", sw/2.0 - 130.0, sh/2.0 + 95.0, 20.0, GRAY);
    }
}

// ============================================================================
// MAIN
// ============================================================================

fn window_conf() -> Conf {
    Conf {
        window_title: "RUST BLASTER".to_owned(),
        window_width: 1280,
        window_height: 720,
        fullscreen: false,
        platform: miniquad::conf::Platform {
            swap_interval: Some(0), // Disable vsync - uncapped FPS
            ..Default::default()
        },
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut world = World::new();
    let mut gamepad = GamepadState::default();

    // Initialize gilrs for gamepad support
    let mut gilrs = Gilrs::new().unwrap_or_else(|e| {
        println!("Failed to initialize gamepad support: {}", e);
        panic!("Gilrs initialization failed");
    });

    // Load Jeremy Clarkson texture
    if let Ok(tex) = load_texture("assets/enemy.png").await {
        tex.set_filter(FilterMode::Linear);
        world.enemy_texture = Some(tex);
        println!("Loaded the REAL Jeremy Clarkson!");
    } else {
        // Fallback to procedural
        world.enemy_texture = Some(generate_clarkson_texture());
        println!("Using procedural Clarkson");
    }

    println!("Xbox Controller: Left stick=move, Right stick=look, RT=shoot, LT=aim, LB/RB=weapons, Start=pause");

    loop {
        let dt = get_frame_time();

        // Poll gamepad events
        gamepad.clear_just_pressed();
        while let Some(Event { id: _, event, time: _ }) = gilrs.next_event() {
            match event {
                gilrs::EventType::ButtonPressed(button, _) => {
                    match button {
                        Button::South => { gamepad.a_pressed = true; gamepad.a_just_pressed = true; }
                        Button::East => { gamepad.b_pressed = true; gamepad.b_just_pressed = true; }
                        Button::West => { gamepad.x_pressed = true; }
                        Button::North => { gamepad.y_pressed = true; }
                        Button::LeftTrigger => { gamepad.lb_pressed = true; gamepad.lb_just_pressed = true; }
                        Button::RightTrigger => { gamepad.rb_pressed = true; gamepad.rb_just_pressed = true; }
                        Button::LeftTrigger2 => { gamepad.lt_button = true; }  // Analog trigger as button
                        Button::RightTrigger2 => { gamepad.rt_button = true; } // Analog trigger as button
                        Button::Start => { gamepad.start_pressed = true; gamepad.start_just_pressed = true; }
                        Button::DPadUp => { gamepad.dpad_up = true; gamepad.dpad_up_just = true; }
                        Button::DPadDown => { gamepad.dpad_down = true; gamepad.dpad_down_just = true; }
                        Button::DPadLeft => { gamepad.dpad_left = true; gamepad.dpad_left_just = true; }
                        Button::DPadRight => { gamepad.dpad_right = true; gamepad.dpad_right_just = true; }
                        Button::LeftThumb => { gamepad.left_thumb = true; }
                        _ => {}
                    }
                }
                gilrs::EventType::ButtonReleased(button, _) => {
                    match button {
                        Button::South => { gamepad.a_pressed = false; }
                        Button::East => { gamepad.b_pressed = false; }
                        Button::West => { gamepad.x_pressed = false; }
                        Button::North => { gamepad.y_pressed = false; }
                        Button::LeftTrigger => { gamepad.lb_pressed = false; }
                        Button::RightTrigger => { gamepad.rb_pressed = false; }
                        Button::LeftTrigger2 => { gamepad.lt_button = false; }
                        Button::RightTrigger2 => { gamepad.rt_button = false; }
                        Button::Start => { gamepad.start_pressed = false; }
                        Button::DPadUp => { gamepad.dpad_up = false; }
                        Button::DPadDown => { gamepad.dpad_down = false; }
                        Button::DPadLeft => { gamepad.dpad_left = false; }
                        Button::DPadRight => { gamepad.dpad_right = false; }
                        Button::LeftThumb => { gamepad.left_thumb = false; }
                        _ => {}
                    }
                }
                gilrs::EventType::AxisChanged(axis, value, _) => {
                    match axis {
                        Axis::LeftStickX => gamepad.left_stick_x = value,
                        Axis::LeftStickY => gamepad.left_stick_y = value,
                        Axis::RightStickX => gamepad.right_stick_x = value,
                        Axis::RightStickY => gamepad.right_stick_y = value,
                        Axis::LeftZ => gamepad.left_trigger = value,  // LT
                        Axis::RightZ => gamepad.right_trigger = value, // RT
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        update(&mut world, dt, &gamepad);

        match world.state {
            GameState::Menu => {
                render_menu();
            }
            GameState::Playing | GameState::Paused => {
                clear_background(Color::new(0.08, 0.08, 0.12, 1.0));
                render_3d(&world);
                render_hud(&world);
                if world.state == GameState::Paused {
                    render_pause();
                }
            }
            GameState::Dead => {
                clear_background(Color::new(0.08, 0.08, 0.12, 1.0));
                render_3d(&world);
                render_death(&world);
            }
            GameState::Victory => {
                clear_background(Color::new(0.08, 0.08, 0.12, 1.0));
                render_3d(&world);
                render_victory(&world);
            }
        }

        draw_text(&format!("FPS: {}", get_fps()), screen_width() - 80.0, 180.0, 16.0, WHITE);

        next_frame().await;
    }
}

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use rand::{
    RngCore, SeedableRng,
    rngs::{OsRng, StdRng},
    seq::SliceRandom,
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::{collections::VecDeque, io};

use crate::{models::ClassType, theme::Theme};

pub const MAP_WIDTH: usize = 41;
pub const MAP_HEIGHT: usize = 17;
const MIN_TERMINAL_WIDTH: u16 = MAP_WIDTH as u16 + 27;
const MIN_TERMINAL_HEIGHT: u16 = MAP_HEIGHT as u16 + 9;
const EXTRA_PASSAGES: usize = 7;
const GUARDIAN_TURN_INTERVAL: u32 = 3;
const GUARDIAN_AWARENESS: usize = 10;
const FRAGMENTS_TO_ESCAPE: u8 = 3;
const MAX_RESOLVE: u8 = 5;
const MAX_CHARGES: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tile {
    Wall,
    Floor,
    Fragment,
    Hazard,
    Guardian,
    Sigil,
    Exit,
    Purified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveStatus {
    Running,
    Won,
    Lost,
}

#[derive(Clone)]
struct Snapshot {
    tiles: Vec<Tile>,
    revealed: Vec<bool>,
    player: (usize, usize),
    resolve: u8,
    fragments: u8,
    sigils: u8,
    turns: u32,
    paladin_shield: bool,
    time_warning: bool,
}

pub struct ArchiveGame {
    class: ClassType,
    seed: [u8; 32],
    tiles: Vec<Tile>,
    revealed: Vec<bool>,
    player: (usize, usize),
    facing: (isize, isize),
    resolve: u8,
    fragments: u8,
    sigils: u8,
    charges: u8,
    turns: u32,
    status: ArchiveStatus,
    message: String,
    history: Vec<Snapshot>,
    paladin_shield: bool,
    time_warning: bool,
    show_help: bool,
}

impl ArchiveGame {
    pub fn new(class: ClassType, seed: [u8; 32]) -> Self {
        let mut game = Self {
            class,
            seed,
            tiles: vec![Tile::Wall; MAP_WIDTH * MAP_HEIGHT],
            revealed: vec![false; MAP_WIDTH * MAP_HEIGHT],
            player: (1, 1),
            facing: (1, 0),
            resolve: MAX_RESOLVE,
            fragments: 0,
            sigils: 0,
            charges: MAX_CHARGES,
            turns: 0,
            status: ArchiveStatus::Running,
            message: format!(
                "The Archive recognizes a {}. Recover three fragments and find E.",
                class.name()
            ),
            history: Vec::new(),
            paladin_shield: class == ClassType::TaskPaladin,
            time_warning: class == ClassType::TimeChronomancer,
            show_help: false,
        };
        game.generate_map();
        game.reveal_near_player();
        game
    }

    pub fn status(&self) -> ArchiveStatus {
        self.status
    }

    pub fn class(&self) -> ClassType {
        self.class
    }

    pub fn power_name(&self) -> &'static str {
        match self.class {
            ClassType::CodeWarlock => "Debug Familiar",
            ClassType::TaskPaladin => "Purifying Strike",
            ClassType::MindSage => "Archive Recall",
            ClassType::SystemsArchitect => "Restructure",
            ClassType::TimeChronomancer => "Rewind",
            ClassType::ArchAccountant => "Balance the Books",
        }
    }

    pub fn passive_name(&self) -> &'static str {
        match self.class {
            ClassType::CodeWarlock => "Debug Vision",
            ClassType::TaskPaladin => "Oath Shield",
            ClassType::MindSage => "Pattern Recognition",
            ClassType::SystemsArchitect => "Structural Analysis",
            ClassType::TimeChronomancer => "Temporal Awareness",
            ClassType::ArchAccountant => "Perfect Ledger",
        }
    }

    fn index(x: usize, y: usize) -> usize {
        y * MAP_WIDTH + x
    }

    fn tile(&self, x: usize, y: usize) -> Tile {
        self.tiles[Self::index(x, y)]
    }

    fn set_tile(&mut self, x: usize, y: usize, tile: Tile) {
        let index = Self::index(x, y);
        self.tiles[index] = tile;
    }

    fn generate_map(&mut self) {
        let mut rng = StdRng::from_seed(self.seed);
        let mut stack = vec![(1usize, 1usize)];
        self.set_tile(1, 1, Tile::Floor);
        while let Some(&(x, y)) = stack.last() {
            let mut directions = [(2isize, 0isize), (-2, 0), (0, 2), (0, -2)];
            directions.shuffle(&mut rng);
            let next = directions.into_iter().find_map(|(dx, dy)| {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx <= 0
                    || ny <= 0
                    || nx >= (MAP_WIDTH - 1) as isize
                    || ny >= (MAP_HEIGHT - 1) as isize
                {
                    return None;
                }
                let (nx, ny) = (nx as usize, ny as usize);
                (self.tile(nx, ny) == Tile::Wall).then_some((nx, ny, dx, dy))
            });
            if let Some((nx, ny, dx, dy)) = next {
                self.set_tile(
                    (x as isize + dx / 2) as usize,
                    (y as isize + dy / 2) as usize,
                    Tile::Floor,
                );
                self.set_tile(nx, ny, Tile::Floor);
                stack.push((nx, ny));
            } else {
                stack.pop();
            }
        }

        let mut loop_walls = (1..MAP_HEIGHT - 1)
            .flat_map(|y| (1..MAP_WIDTH - 1).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                if self.tile(x, y) != Tile::Wall {
                    return false;
                }
                let joins_horizontal = x % 2 == 0
                    && y % 2 == 1
                    && self.tile(x - 1, y) == Tile::Floor
                    && self.tile(x + 1, y) == Tile::Floor;
                let joins_vertical = x % 2 == 1
                    && y % 2 == 0
                    && self.tile(x, y - 1) == Tile::Floor
                    && self.tile(x, y + 1) == Tile::Floor;
                joins_horizontal || joins_vertical
            })
            .collect::<Vec<_>>();
        loop_walls.shuffle(&mut rng);
        for (x, y) in loop_walls.into_iter().take(EXTRA_PASSAGES) {
            self.set_tile(x, y, Tile::Floor);
        }

        let distances = self.distances_from((1, 1));
        let mut candidates = distances
            .iter()
            .enumerate()
            .filter_map(|(index, distance)| {
                let x = index % MAP_WIDTH;
                let y = index / MAP_WIDTH;
                distance
                    .filter(|distance| *distance >= 6 && self.tile(x, y) == Tile::Floor)
                    .map(|distance| (index, distance))
            })
            .collect::<Vec<_>>();
        candidates.shuffle(&mut rng);
        let Some(exit_position) = candidates
            .iter()
            .enumerate()
            .max_by_key(|(_, (_, distance))| *distance)
            .map(|(position, _)| position)
        else {
            return;
        };
        let (exit, _) = candidates.swap_remove(exit_position);
        self.tiles[exit] = Tile::Exit;

        let mut occupied = vec![(1usize, 1usize), (exit % MAP_WIDTH, exit / MAP_WIDTH)];
        for tile in [
            Tile::Fragment,
            Tile::Guardian,
            Tile::Sigil,
            Tile::Hazard,
            Tile::Fragment,
            Tile::Sigil,
            Tile::Guardian,
            Tile::Sigil,
            Tile::Hazard,
            Tile::Fragment,
            Tile::Guardian,
            Tile::Sigil,
            Tile::Sigil,
            Tile::Hazard,
        ] {
            let Some(position) = candidates
                .iter()
                .enumerate()
                .max_by_key(|(_, (index, distance))| {
                    let point = (*index % MAP_WIDTH, *index / MAP_WIDTH);
                    let separation = occupied
                        .iter()
                        .map(|anchor| anchor.0.abs_diff(point.0) + anchor.1.abs_diff(point.1))
                        .min()
                        .unwrap_or(0);
                    (separation, *distance)
                })
                .map(|(position, _)| position)
            else {
                break;
            };
            let (index, _) = candidates.swap_remove(position);
            self.tiles[index] = tile;
            occupied.push((index % MAP_WIDTH, index / MAP_WIDTH));
        }
    }

    fn distances_from(&self, start: (usize, usize)) -> Vec<Option<usize>> {
        let mut distances = vec![None; self.tiles.len()];
        let mut queue = VecDeque::from([start]);
        distances[Self::index(start.0, start.1)] = Some(0);
        while let Some((x, y)) = queue.pop_front() {
            let distance = distances[Self::index(x, y)].unwrap_or(0);
            for (dx, dy) in [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)] {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx < 0 || ny < 0 || nx >= MAP_WIDTH as isize || ny >= MAP_HEIGHT as isize {
                    continue;
                }
                let (nx, ny) = (nx as usize, ny as usize);
                let index = Self::index(nx, ny);
                if distances[index].is_none() && self.tiles[index] != Tile::Wall {
                    distances[index] = Some(distance + 1);
                    queue.push_back((nx, ny));
                }
            }
        }
        distances
    }

    fn snapshot(&self) -> Snapshot {
        Snapshot {
            tiles: self.tiles.clone(),
            revealed: self.revealed.clone(),
            player: self.player,
            resolve: self.resolve,
            fragments: self.fragments,
            sigils: self.sigils,
            turns: self.turns,
            paladin_shield: self.paladin_shield,
            time_warning: self.time_warning,
        }
    }

    fn push_history(&mut self) {
        self.history.push(self.snapshot());
        if self.history.len() > 32 {
            self.history.remove(0);
        }
    }

    fn reveal_near_player(&mut self) {
        let radius = if self.class == ClassType::MindSage {
            2
        } else {
            1
        };
        self.reveal_radius(radius);
    }

    fn reveal_radius(&mut self, radius: isize) {
        for dy in -radius..=radius {
            for dx in -radius..=radius {
                let x = self.player.0 as isize + dx;
                let y = self.player.1 as isize + dy;
                if x >= 0 && y >= 0 && x < MAP_WIDTH as isize && y < MAP_HEIGHT as isize {
                    self.revealed[Self::index(x as usize, y as usize)] = true;
                }
            }
        }
    }

    pub fn move_player(&mut self, dx: isize, dy: isize) {
        if self.status != ArchiveStatus::Running {
            return;
        }
        self.facing = (dx, dy);
        let nx = self.player.0 as isize + dx;
        let ny = self.player.1 as isize + dy;
        if nx < 0 || ny < 0 || nx >= MAP_WIDTH as isize || ny >= MAP_HEIGHT as isize {
            return;
        }
        let (nx, ny) = (nx as usize, ny as usize);
        let tile = self.tile(nx, ny);
        if tile == Tile::Wall {
            self.message = if self.class == ClassType::SystemsArchitect {
                "Structural Analysis: this wall can be altered with Restructure.".to_string()
            } else {
                "A shelf of sealed records blocks the way.".to_string()
            };
            return;
        }
        if tile == Tile::Hazard && self.class == ClassType::TimeChronomancer && self.time_warning {
            self.time_warning = false;
            self.revealed[Self::index(nx, ny)] = true;
            self.message =
                "Temporal Awareness shows the injury one second before it happens.".to_string();
            return;
        }

        self.push_history();
        self.turns = self.turns.saturating_add(1);
        self.player = (nx, ny);
        match tile {
            Tile::Fragment => {
                self.fragments = self.fragments.saturating_add(1).min(FRAGMENTS_TO_ESCAPE);
                self.charges = self.charges.saturating_add(1).min(MAX_CHARGES);
                self.set_tile(nx, ny, Tile::Floor);
                let hint = self.exit_hint();
                self.message = if self.fragments == FRAGMENTS_TO_ESCAPE {
                    format!(
                        "The final fragment breaks the seal. The exit awakens {hint}. A class charge returns."
                    )
                } else {
                    format!(
                        "Fragment {}/{} recovered. The sealed way whispers from {hint}. A class charge returns.",
                        self.fragments, FRAGMENTS_TO_ESCAPE
                    )
                };
            }
            Tile::Sigil => {
                let gained = if self.class == ClassType::ArchAccountant {
                    2
                } else {
                    1
                };
                self.sigils = self.sigils.saturating_add(gained);
                self.set_tile(nx, ny, Tile::Floor);
                self.message = if gained == 2 {
                    "Perfect Ledger identifies a second sigil hidden in the accounting.".to_string()
                } else {
                    "You recover an Archive sigil.".to_string()
                };
            }
            Tile::Guardian => {
                self.set_tile(nx, ny, Tile::Floor);
                if self.sigils > 0 {
                    self.sigils -= 1;
                    self.message = "A sigil satisfies the guardian's ancient demand.".to_string();
                } else {
                    self.take_damage("The guardian extracts one Resolve.");
                }
            }
            Tile::Hazard => {
                self.set_tile(nx, ny, Tile::Floor);
                self.take_damage("Entropy tears one Resolve from you.");
            }
            Tile::Exit if self.fragments >= FRAGMENTS_TO_ESCAPE => {
                self.status = ArchiveStatus::Won;
                self.message = format!(
                    "The Archive opens for the {}. You escape in {} turns.",
                    self.class.name(),
                    self.turns
                );
            }
            Tile::Exit => {
                self.message = "The shelves shift behind you.".to_string();
            }
            Tile::Purified => {
                self.message = "This room remains protected from Entropy.".to_string()
            }
            Tile::Floor | Tile::Wall => self.message = "The shelves shift behind you.".to_string(),
        }
        self.reveal_near_player();
        self.advance_guardians();
    }

    fn exit_hint(&self) -> String {
        let Some(index) = self.tiles.iter().position(|tile| *tile == Tile::Exit) else {
            return "somewhere beyond the mapped stacks".to_string();
        };
        let exit = (index % MAP_WIDTH, index / MAP_WIDTH);
        let vertical = match exit.1.cmp(&self.player.1) {
            std::cmp::Ordering::Less => "north",
            std::cmp::Ordering::Greater => "south",
            std::cmp::Ordering::Equal => "",
        };
        let horizontal = match exit.0.cmp(&self.player.0) {
            std::cmp::Ordering::Less => "west",
            std::cmp::Ordering::Greater => "east",
            std::cmp::Ordering::Equal => "",
        };
        match (vertical.is_empty(), horizontal.is_empty()) {
            (false, false) => format!("{vertical}-{horizontal}"),
            (false, true) => vertical.to_string(),
            (true, false) => horizontal.to_string(),
            (true, true) => "beneath your feet".to_string(),
        }
    }

    fn take_damage(&mut self, message: &str) {
        if self.class == ClassType::TaskPaladin && self.paladin_shield {
            self.paladin_shield = false;
            self.message = "Oath Shield turns the first wound into harmless light.".to_string();
            return;
        }
        self.resolve = self.resolve.saturating_sub(1);
        self.message = message.to_string();
        if self.resolve == 0 {
            self.status = ArchiveStatus::Lost;
            self.message =
                "Your Resolve is gone. The Archive files you among the forgotten.".to_string();
        }
    }

    pub fn use_power(&mut self) {
        if self.status != ArchiveStatus::Running {
            return;
        }
        if self.charges == 0 {
            self.message = format!("{} has no charges remaining.", self.power_name());
            return;
        }
        let previous_turn = self.turns;
        match self.class {
            ClassType::CodeWarlock => self.debug_familiar(),
            ClassType::TaskPaladin => self.purifying_strike(),
            ClassType::MindSage => self.archive_recall(),
            ClassType::SystemsArchitect => self.restructure(),
            ClassType::TimeChronomancer => self.rewind(),
            ClassType::ArchAccountant => self.balance_books(),
        }
        if self.turns != previous_turn {
            self.advance_guardians();
        }
    }

    fn advance_guardians(&mut self) {
        if self.status != ArchiveStatus::Running
            || self.turns == 0
            || self.turns % GUARDIAN_TURN_INTERVAL != 0
        {
            return;
        }

        let distances = self.distances_from(self.player);
        let guardians = self
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(index, tile)| {
                (*tile == Tile::Guardian).then_some((index % MAP_WIDTH, index / MAP_WIDTH))
            })
            .collect::<Vec<_>>();
        let mut moved = 0usize;
        let mut attacked = false;

        for (x, y) in guardians {
            if self.status != ArchiveStatus::Running || self.tile(x, y) != Tile::Guardian {
                continue;
            }
            let awareness = x.abs_diff(self.player.0) + y.abs_diff(self.player.1);
            if awareness > GUARDIAN_AWARENESS || !self.revealed[Self::index(x, y)] {
                continue;
            }
            if awareness == 1 {
                if !attacked {
                    self.take_damage("A roaming guardian strikes and extracts one Resolve.");
                    attacked = true;
                }
                continue;
            }

            let current_distance = distances[Self::index(x, y)].unwrap_or(usize::MAX);
            let destination = [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)]
                .into_iter()
                .filter_map(|(dx, dy)| {
                    let nx = x as isize + dx;
                    let ny = y as isize + dy;
                    if nx <= 0
                        || ny <= 0
                        || nx >= (MAP_WIDTH - 1) as isize
                        || ny >= (MAP_HEIGHT - 1) as isize
                    {
                        return None;
                    }
                    let point = (nx as usize, ny as usize);
                    (self.tile(point.0, point.1) == Tile::Floor)
                        .then_some((point, distances[Self::index(point.0, point.1)]?))
                })
                .filter(|(_, distance)| *distance < current_distance)
                .min_by_key(|(_, distance)| *distance)
                .map(|(point, _)| point);

            if let Some((nx, ny)) = destination {
                self.set_tile(x, y, Tile::Floor);
                self.set_tile(nx, ny, Tile::Guardian);
                self.revealed[Self::index(nx, ny)] = true;
                moved += 1;
            }
        }

        if moved > 0 && !attacked {
            self.message.push_str(if moved == 1 {
                " A guardian moves through the stacks."
            } else {
                " Guardians move through the stacks."
            });
        }
    }

    fn spend_power(&mut self) {
        self.push_history();
        self.charges = self.charges.saturating_sub(1);
        self.turns = self.turns.saturating_add(1);
    }

    fn debug_familiar(&mut self) {
        let mut targets = Vec::new();
        for y in 0..MAP_HEIGHT {
            for x in 0..MAP_WIDTH {
                let distance = self.player.0.abs_diff(x) + self.player.1.abs_diff(y);
                if distance <= 4 && matches!(self.tile(x, y), Tile::Hazard | Tile::Guardian) {
                    targets.push((distance, x, y));
                }
            }
        }
        targets.sort_unstable();
        let Some((_, x, y)) = targets.first().copied() else {
            self.message = "Debug Familiar finds no hostile process nearby.".to_string();
            return;
        };
        self.spend_power();
        self.set_tile(x, y, Tile::Floor);
        self.reveal_radius(3);
        self.message = "Debug Familiar terminates the nearest hostile process.".to_string();
    }

    fn purifying_strike(&mut self) {
        let mut targets = Vec::new();
        for dy in -1isize..=1 {
            for dx in -1isize..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let x = self.player.0 as isize + dx;
                let y = self.player.1 as isize + dy;
                if x >= 0
                    && y >= 0
                    && x < MAP_WIDTH as isize
                    && y < MAP_HEIGHT as isize
                    && matches!(
                        self.tile(x as usize, y as usize),
                        Tile::Hazard | Tile::Guardian
                    )
                {
                    targets.push((x as usize, y as usize));
                }
            }
        }
        if targets.is_empty() && self.resolve == MAX_RESOLVE {
            self.message = "Purifying Strike finds no corruption to cleanse.".to_string();
            return;
        }
        self.spend_power();
        for (x, y) in targets {
            self.set_tile(x, y, Tile::Purified);
        }
        self.resolve = self.resolve.saturating_add(1).min(MAX_RESOLVE);
        self.message = "Purifying Strike cleanses the chamber and restores Resolve.".to_string();
    }

    fn archive_recall(&mut self) {
        self.spend_power();
        self.reveal_radius(6);
        let nearest = (0..MAP_HEIGHT)
            .flat_map(|y| (0..MAP_WIDTH).map(move |x| (x, y)))
            .filter(|&(x, y)| self.tile(x, y) == Tile::Fragment)
            .min_by_key(|&(x, y)| self.player.0.abs_diff(x) + self.player.1.abs_diff(y));
        self.message = match nearest {
            Some((x, y)) => format!(
                "Archive Recall: the nearest fragment lies {}{}.",
                if y < self.player.1 {
                    "north"
                } else if y > self.player.1 {
                    "south"
                } else {
                    ""
                },
                if x < self.player.0 {
                    "west"
                } else if x > self.player.0 {
                    "east"
                } else {
                    ""
                }
            ),
            None => "Archive Recall finds no unrecovered fragments.".to_string(),
        };
    }

    fn restructure(&mut self) {
        let x = self.player.0 as isize + self.facing.0;
        let y = self.player.1 as isize + self.facing.1;
        if x <= 0 || y <= 0 || x >= (MAP_WIDTH - 1) as isize || y >= (MAP_HEIGHT - 1) as isize {
            self.message = "The Archive's outer foundation cannot be restructured.".to_string();
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if self.tile(x, y) != Tile::Wall {
            self.message = "Restructure requires an adjacent wall.".to_string();
            return;
        }
        self.spend_power();
        self.set_tile(x, y, Tile::Floor);
        self.revealed[Self::index(x, y)] = true;
        self.message = "You redraw the blueprint. A passage replaces the wall.".to_string();
    }

    fn rewind(&mut self) {
        if self.history.is_empty() {
            self.message = "Rewind finds no previous moment to restore.".to_string();
            return;
        }
        let current_charges = self.charges;
        let steps = self.history.len().min(3);
        let target = self.history[self.history.len() - steps].clone();
        self.history.truncate(self.history.len() - steps);
        self.tiles = target.tiles;
        self.revealed = target.revealed;
        self.player = target.player;
        self.resolve = target.resolve;
        self.fragments = target.fragments;
        self.sigils = target.sigils;
        self.charges = current_charges.saturating_sub(1);
        self.turns = target.turns.saturating_add(1);
        self.paladin_shield = target.paladin_shield;
        self.time_warning = target.time_warning;
        self.status = ArchiveStatus::Running;
        self.message = format!(
            "Rewind erases {steps} turn{}.",
            if steps == 1 { "" } else { "s" }
        );
        self.reveal_near_player();
    }

    fn balance_books(&mut self) {
        if self.sigils == 0 {
            self.message = "Balance the Books requires one sigil in the asset column.".to_string();
            return;
        }
        if self.resolve == MAX_RESOLVE {
            self.message = "The Resolve ledger is already balanced.".to_string();
            return;
        }
        self.spend_power();
        self.sigils -= 1;
        self.resolve = self.resolve.saturating_add(2).min(MAX_RESOLVE);
        self.message = "One sigil is reconciled into two Resolve.".to_string();
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        if self.show_help {
            match key.code {
                KeyCode::Char('?') | KeyCode::Esc | KeyCode::Enter => self.show_help = false,
                KeyCode::Char('q') => return true,
                _ => {}
            }
            return false;
        }
        if self.status != ArchiveStatus::Running {
            match key.code {
                KeyCode::Char('r') => *self = Self::new(self.class, random_seed()),
                KeyCode::Char('q') | KeyCode::Esc | KeyCode::Enter => return true,
                KeyCode::Char('?') => self.show_help = true,
                _ => {}
            }
            return false;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_player(0, -1),
            KeyCode::Down | KeyCode::Char('j') => self.move_player(0, 1),
            KeyCode::Left | KeyCode::Char('h') => self.move_player(-1, 0),
            KeyCode::Right | KeyCode::Char('l') => self.move_player(1, 0),
            KeyCode::Char('p') => self.use_power(),
            KeyCode::Char('?') => self.show_help = true,
            KeyCode::Char('q') | KeyCode::Esc => return true,
            _ => {}
        }
        false
    }

    fn passively_visible(&self, x: usize, y: usize) -> bool {
        if self.revealed[Self::index(x, y)] {
            return true;
        }
        let distance = self.player.0.abs_diff(x) + self.player.1.abs_diff(y);
        match self.class {
            ClassType::CodeWarlock => {
                distance <= 3 && matches!(self.tile(x, y), Tile::Hazard | Tile::Guardian)
            }
            ClassType::SystemsArchitect if self.tile(x, y) == Tile::Wall => {
                [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)]
                    .into_iter()
                    .any(|(dx, dy)| {
                        let nx = x as isize + dx;
                        let ny = y as isize + dy;
                        nx >= 0
                            && ny >= 0
                            && nx < MAP_WIDTH as isize
                            && ny < MAP_HEIGHT as isize
                            && self.revealed[Self::index(nx as usize, ny as usize)]
                    })
            }
            _ => false,
        }
    }
}

fn tile_span(tile: Tile, architect_wall: bool, exit_unlocked: bool) -> Span<'static> {
    match tile {
        Tile::Wall => Span::styled(
            if architect_wall { "+" } else { "#" },
            Style::default().fg(if architect_wall {
                Color::Rgb(96, 165, 250)
            } else {
                Color::Rgb(55, 65, 81)
            }),
        ),
        Tile::Floor => Span::styled("·", Style::default().fg(Color::Rgb(71, 78, 96))),
        Tile::Fragment => Span::styled(
            "F",
            Style::default()
                .fg(Color::Rgb(34, 211, 238))
                .add_modifier(Modifier::BOLD),
        ),
        Tile::Hazard => Span::styled(
            "!",
            Style::default()
                .fg(Color::Rgb(248, 113, 113))
                .add_modifier(Modifier::BOLD),
        ),
        Tile::Guardian => Span::styled(
            "G",
            Style::default()
                .fg(Color::Rgb(232, 121, 249))
                .add_modifier(Modifier::BOLD),
        ),
        Tile::Sigil => Span::styled(
            "S",
            Style::default()
                .fg(Color::Rgb(250, 204, 21))
                .add_modifier(Modifier::BOLD),
        ),
        Tile::Exit => Span::styled(
            if exit_unlocked { "E" } else { "·" },
            if exit_unlocked {
                Style::default()
                    .fg(Color::Rgb(74, 222, 128))
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::Rgb(71, 78, 96))
            },
        ),
        Tile::Purified => Span::styled(
            "*",
            Style::default()
                .fg(Color::Rgb(134, 239, 172))
                .add_modifier(Modifier::BOLD),
        ),
    }
}

fn chronicle_color(game: &ArchiveGame) -> Color {
    if game.status == ArchiveStatus::Won {
        return Color::Rgb(74, 222, 128);
    }
    if game.status == ArchiveStatus::Lost {
        return Color::Rgb(248, 113, 113);
    }
    let message = game.message.to_ascii_lowercase();
    if message.contains("fragment") {
        Color::Rgb(34, 211, 238)
    } else if message.contains("sigil") || message.contains("ledger") {
        Color::Rgb(250, 204, 21)
    } else if message.contains("guardian") {
        Color::Rgb(232, 121, 249)
    } else if message.contains("entropy") || message.contains("injury") || message.contains("wound")
    {
        Color::Rgb(248, 113, 113)
    } else if message.contains("exit remains sealed") {
        Color::Rgb(251, 191, 36)
    } else if message.contains("exit") || message.contains("escape") {
        Color::Rgb(74, 222, 128)
    } else if message.contains("recall")
        || message.contains("restructure")
        || message.contains("rewind")
        || message.contains("familiar")
        || message.contains("shield")
        || message.contains("strike")
    {
        Theme::for_class(game.class).primary
    } else {
        Color::Rgb(203, 213, 225)
    }
}

fn draw_archive(frame: &mut Frame, game: &ArchiveGame, username: &str, level: i32) {
    let area = frame.size();
    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(9, 10, 17))),
        area,
    );
    if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(Span::styled(
                    "THE FORGOTTEN ARCHIVE",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(format!(
                    "The Archive requires at least {MIN_TERMINAL_WIDTH} columns × {MIN_TERMINAL_HEIGHT} rows."
                )),
                Line::from("Resize the terminal, or press q / Esc to leave."),
            ])
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL)),
            area,
        );
        return;
    }

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(MAP_HEIGHT as u16 + 2),
            Constraint::Length(4),
        ])
        .split(area);
    let resolve = "♥".repeat(game.resolve as usize);
    let display_name = if username.chars().count() > 16 {
        format!("{}…", username.chars().take(15).collect::<String>())
    } else {
        username.to_string()
    };
    let class_theme = Theme::for_class(game.class);
    let header = Paragraph::new(Line::from(vec![
        Span::raw(" "),
        Span::styled(
            display_name,
            Style::default()
                .fg(class_theme.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(format!(" · Lv {level} ")),
        Span::styled(
            game.class.name(),
            Style::default().fg(class_theme.secondary),
        ),
        Span::raw("  R:"),
        Span::styled(resolve, Style::default().fg(Color::Rgb(248, 113, 113))),
        Span::raw("  "),
        Span::styled("F:", Style::default().fg(Color::Rgb(34, 211, 238))),
        Span::styled(
            format!("{}/{}", game.fragments, FRAGMENTS_TO_ESCAPE),
            Style::default().fg(Color::Rgb(165, 243, 252)),
        ),
        Span::raw(" "),
        Span::styled("S:", Style::default().fg(Color::Rgb(250, 204, 21))),
        Span::styled(
            game.sigils.to_string(),
            Style::default().fg(Color::Rgb(254, 240, 138)),
        ),
        Span::raw(" "),
        Span::styled("P:", Style::default().fg(class_theme.primary)),
        Span::styled(
            game.charges.to_string(),
            Style::default().fg(class_theme.secondary),
        ),
        Span::styled(
            format!(" T:{}", game.turns),
            Style::default().fg(Color::Gray),
        ),
    ]))
    .block(
        Block::default().borders(Borders::ALL).title(Span::styled(
            " THE FORGOTTEN ARCHIVE ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
    );
    frame.render_widget(header, rows[0]);

    let centered = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(MAP_WIDTH as u16 + 2),
            Constraint::Length(23),
            Constraint::Min(1),
        ])
        .split(rows[1]);
    let mut map_lines = Vec::with_capacity(MAP_HEIGHT);
    for y in 0..MAP_HEIGHT {
        let mut line = Vec::with_capacity(MAP_WIDTH);
        for x in 0..MAP_WIDTH {
            if game.player == (x, y) {
                line.push(Span::styled(
                    "@",
                    Style::default()
                        .fg(Theme::for_class(game.class).primary)
                        .add_modifier(Modifier::BOLD),
                ));
            } else if game.passively_visible(x, y) {
                line.push(tile_span(
                    game.tile(x, y),
                    game.class == ClassType::SystemsArchitect
                        && !game.revealed[ArchiveGame::index(x, y)],
                    game.fragments >= FRAGMENTS_TO_ESCAPE,
                ));
            } else {
                line.push(Span::raw(" "));
            }
        }
        map_lines.push(Line::from(line));
    }
    frame.render_widget(
        Paragraph::new(map_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Rgb(71, 85, 105)))
                .title(Span::styled(
                    " Shifting Stacks ",
                    Style::default().fg(class_theme.primary),
                )),
        ),
        centered[1],
    );

    let side = vec![
        Line::from(Span::styled(
            "Class gifts",
            Style::default()
                .fg(class_theme.primary)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::raw("Passive: "),
            Span::styled(
                game.passive_name(),
                Style::default().fg(class_theme.secondary),
            ),
        ]),
        Line::from(vec![
            Span::styled("[p] ", Style::default().fg(class_theme.primary)),
            Span::styled(
                game.power_name(),
                Style::default()
                    .fg(class_theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        legend_line("F", " fragment", Color::Rgb(34, 211, 238)),
        legend_line("S", " sigil", Color::Rgb(250, 204, 21)),
        legend_line("G", " guardian · moves/3", Color::Rgb(232, 121, 249)),
        legend_line("!", " entropy", Color::Rgb(248, 113, 113)),
        legend_line(
            "E",
            if game.fragments >= FRAGMENTS_TO_ESCAPE {
                " exit READY"
            } else {
                " hidden until fragments"
            },
            if game.fragments >= FRAGMENTS_TO_ESCAPE {
                Color::Rgb(74, 222, 128)
            } else {
                Color::Rgb(148, 163, 184)
            },
        ),
        Line::from(""),
        Line::from("Move: arrows/hjkl"),
        Line::from("Help: ?  Leave: q"),
    ];
    frame.render_widget(
        Paragraph::new(side).wrap(Wrap { trim: true }).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Field Notes "),
        ),
        centered[2],
    );

    frame.render_widget(
        Paragraph::new(Span::styled(
            game.message.as_str(),
            Style::default()
                .fg(chronicle_color(game))
                .add_modifier(Modifier::BOLD),
        ))
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(chronicle_color(game)))
                .title(" Chronicle Whisper "),
        ),
        rows[2],
    );

    if game.show_help {
        draw_help(frame, area, game);
    } else if game.status != ArchiveStatus::Running {
        draw_result(frame, area, game);
    }
}

fn legend_line(symbol: &'static str, label: &'static str, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            symbol,
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(label),
    ])
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(area.height.saturating_sub(height) / 2),
            Constraint::Length(height.min(area.height)),
            Constraint::Min(0),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(area.width.saturating_sub(width) / 2),
            Constraint::Length(width.min(area.width)),
            Constraint::Min(0),
        ])
        .split(vertical[1])[1]
}

fn draw_help(frame: &mut Frame, area: Rect, game: &ArchiveGame) {
    let popup = centered_rect(58, 16, area);
    frame.render_widget(Clear, popup);
    let text = vec![
        Line::from(Span::styled(
            "HOW THE ARCHIVE DREAMS",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Recover three F fragments to reveal E."),
        Line::from("Guardians accept one S sigil or take one Resolve."),
        Line::from("Nearby guardians move every third turn."),
        Line::from("Entropy ! takes one Resolve when entered."),
        Line::from("Fragments restore one class-power charge."),
        Line::from(""),
        Line::from(format!("Passive — {}", game.passive_name())),
        Line::from(format!("Power [p] — {}", game.power_name())),
        Line::from(""),
        Line::from("Arrows/hjkl move · p power · q leave"),
        Line::from("Press ?, Esc, or Enter to close this record."),
    ];
    frame.render_widget(
        Paragraph::new(text)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Unsealed Record "),
            ),
        popup,
    );
}

fn draw_result(frame: &mut Frame, area: Rect, game: &ArchiveGame) {
    let popup = centered_rect(54, 11, area);
    frame.render_widget(Clear, popup);
    let (title, color) = if game.status == ArchiveStatus::Won {
        (" THE RECORD SURVIVES ", Color::Green)
    } else {
        (" THE ARCHIVE CLOSES ", Color::Red)
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                if game.status == ArchiveStatus::Won {
                    "You carry the recovered words into daylight."
                } else {
                    "The shelves close around the unfinished record."
                },
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!(
                "{} · {} fragments · {} turns",
                game.class.name(),
                game.fragments,
                game.turns
            )),
            Line::from(""),
            Line::from("[r] enter a newly shifted Archive"),
            Line::from("[Enter / q / Esc] return to the terminal"),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(title)),
        popup,
    );
}

pub fn random_seed() -> [u8; 32] {
    let mut seed = [0; 32];
    OsRng.fill_bytes(&mut seed);
    seed
}

/// Runs an isolated alternate-screen game. It receives character display data
/// only and has no database or sync handle, so Archive actions cannot mutate
/// productivity state or award XP.
pub fn run(class: ClassType, username: &str, level: i32, seed: [u8; 32]) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    if let Err(error) = execute!(stdout, EnterAlternateScreen) {
        let _ = disable_raw_mode();
        return Err(error.into());
    }
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = match Terminal::new(backend) {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen);
            return Err(error.into());
        }
    };

    let mut game = ArchiveGame::new(class, seed);
    let result = (|| -> Result<()> {
        loop {
            terminal.draw(|frame| draw_archive(frame, &game, username, level))?;
            match event::read()? {
                Event::Key(key) if game.handle_key(key) => break,
                _ => {}
            }
        }
        Ok(())
    })();

    let raw_result = disable_raw_mode();
    let screen_result = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let cursor_result = terminal.show_cursor();
    result?;
    raw_result?;
    screen_result?;
    cursor_result?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    const CLASSES: [ClassType; 6] = [
        ClassType::CodeWarlock,
        ClassType::TaskPaladin,
        ClassType::MindSage,
        ClassType::SystemsArchitect,
        ClassType::TimeChronomancer,
        ClassType::ArchAccountant,
    ];

    fn seed() -> [u8; 32] {
        [7; 32]
    }

    fn path_between(
        game: &ArchiveGame,
        start: (usize, usize),
        goal: (usize, usize),
    ) -> Vec<(isize, isize)> {
        let mut previous = vec![None; game.tiles.len()];
        let mut queue = VecDeque::from([start]);
        previous[ArchiveGame::index(start.0, start.1)] = Some(start);
        while let Some((x, y)) = queue.pop_front() {
            if (x, y) == goal {
                break;
            }
            for (dx, dy) in [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)] {
                let nx = x as isize + dx;
                let ny = y as isize + dy;
                if nx < 0 || ny < 0 || nx >= MAP_WIDTH as isize || ny >= MAP_HEIGHT as isize {
                    continue;
                }
                let next = (nx as usize, ny as usize);
                let index = ArchiveGame::index(next.0, next.1);
                if previous[index].is_none() && game.tile(next.0, next.1) != Tile::Wall {
                    previous[index] = Some((x, y));
                    queue.push_back(next);
                }
            }
        }
        let mut positions = vec![goal];
        while *positions.last().unwrap() != start {
            let current = *positions.last().unwrap();
            positions.push(previous[ArchiveGame::index(current.0, current.1)].unwrap());
        }
        positions.reverse();
        positions
            .windows(2)
            .map(|pair| {
                (
                    pair[1].0 as isize - pair[0].0 as isize,
                    pair[1].1 as isize - pair[0].1 as isize,
                )
            })
            .collect()
    }

    #[test]
    fn generated_archive_is_reachable_and_contains_the_required_objects() {
        let game = ArchiveGame::new(ClassType::MindSage, seed());
        for (tile, expected) in [
            (Tile::Fragment, FRAGMENTS_TO_ESCAPE as usize),
            (Tile::Sigil, 5),
            (Tile::Guardian, 3),
            (Tile::Hazard, 3),
            (Tile::Exit, 1),
        ] {
            assert_eq!(
                game.tiles
                    .iter()
                    .filter(|candidate| **candidate == tile)
                    .count(),
                expected
            );
        }
        let distances = game.distances_from((1, 1));
        for (index, tile) in game.tiles.iter().enumerate() {
            if matches!(tile, Tile::Fragment | Tile::Exit) {
                assert!(distances[index].is_some());
            }
        }

        let discoveries = game
            .tiles
            .iter()
            .enumerate()
            .filter_map(|(index, tile)| {
                matches!(
                    tile,
                    Tile::Fragment | Tile::Sigil | Tile::Guardian | Tile::Hazard | Tile::Exit
                )
                .then_some((index % MAP_WIDTH, index / MAP_WIDTH))
            })
            .collect::<Vec<_>>();
        for (position, first) in discoveries.iter().enumerate() {
            for second in &discoveries[position + 1..] {
                assert!(
                    first.0.abs_diff(second.0) + first.1.abs_diff(second.1) >= 4,
                    "Archive discoveries should be distributed throughout the maze"
                );
            }
        }

        let exit = game
            .tiles
            .iter()
            .position(|tile| *tile == Tile::Exit)
            .unwrap();
        assert_eq!(
            distances[exit],
            distances.iter().copied().flatten().max(),
            "the exit should remain at the deepest reachable point"
        );

        let walkable = game
            .tiles
            .iter()
            .filter(|tile| **tile != Tile::Wall)
            .count();
        let connections = (0..MAP_HEIGHT)
            .flat_map(|y| (0..MAP_WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| {
                if game.tile(x, y) == Tile::Wall {
                    return 0;
                }
                usize::from(x + 1 < MAP_WIDTH && game.tile(x + 1, y) != Tile::Wall)
                    + usize::from(y + 1 < MAP_HEIGHT && game.tile(x, y + 1) != Tile::Wall)
            })
            .sum::<usize>();
        assert!(
            connections >= walkable,
            "the Archive should contain loops and alternate routes"
        );
    }

    #[test]
    fn every_class_has_a_distinct_power_and_passive() {
        let powers = CLASSES
            .map(|class| ArchiveGame::new(class, seed()).power_name())
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        let passives = CLASSES
            .map(|class| ArchiveGame::new(class, seed()).passive_name())
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(powers.len(), CLASSES.len());
        assert_eq!(passives.len(), CLASSES.len());
    }

    #[test]
    fn every_class_can_complete_the_same_generated_archive() {
        for class in CLASSES {
            let mut game = ArchiveGame::new(class, seed());
            let mut targets = game
                .tiles
                .iter()
                .enumerate()
                .filter_map(|(index, tile)| {
                    (*tile == Tile::Fragment).then_some((index % MAP_WIDTH, index / MAP_WIDTH))
                })
                .collect::<Vec<_>>();
            let exit = game
                .tiles
                .iter()
                .position(|tile| *tile == Tile::Exit)
                .map(|index| (index % MAP_WIDTH, index / MAP_WIDTH))
                .unwrap();
            targets.push(exit);
            for target in targets {
                for (dx, dy) in path_between(&game, game.player, target) {
                    let before = game.player;
                    game.move_player(dx, dy);
                    if game.player == before && game.status == ArchiveStatus::Running {
                        game.move_player(dx, dy);
                    }
                }
            }
            assert_eq!(game.status(), ArchiveStatus::Won, "{}", class.name());
        }
    }

    #[test]
    fn class_active_powers_apply_their_distinct_effects() {
        let mut warlock = ArchiveGame::new(ClassType::CodeWarlock, seed());
        warlock.set_tile(3, 1, Tile::Hazard);
        warlock.use_power();
        assert_eq!(warlock.tile(3, 1), Tile::Floor);

        let mut paladin = ArchiveGame::new(ClassType::TaskPaladin, seed());
        paladin.set_tile(2, 1, Tile::Guardian);
        paladin.use_power();
        assert_eq!(paladin.tile(2, 1), Tile::Purified);

        let mut sage = ArchiveGame::new(ClassType::MindSage, seed());
        let revealed_before = sage.revealed.iter().filter(|value| **value).count();
        sage.use_power();
        assert!(sage.revealed.iter().filter(|value| **value).count() > revealed_before);

        let mut architect = ArchiveGame::new(ClassType::SystemsArchitect, seed());
        architect.set_tile(2, 1, Tile::Wall);
        architect.use_power();
        assert_eq!(architect.tile(2, 1), Tile::Floor);

        let mut accountant = ArchiveGame::new(ClassType::ArchAccountant, seed());
        accountant.sigils = 1;
        accountant.resolve = 2;
        accountant.use_power();
        assert_eq!(accountant.resolve, 4);
        assert_eq!(accountant.sigils, 0);
    }

    #[test]
    fn paladin_shield_and_chronomancer_warning_prevent_their_first_damage() {
        let mut paladin = ArchiveGame::new(ClassType::TaskPaladin, seed());
        let target = (paladin.player.0 + 1, paladin.player.1);
        paladin.set_tile(target.0, target.1, Tile::Hazard);
        paladin.move_player(1, 0);
        assert_eq!(paladin.resolve, MAX_RESOLVE);
        assert!(!paladin.paladin_shield);

        let mut chronomancer = ArchiveGame::new(ClassType::TimeChronomancer, seed());
        let target = (chronomancer.player.0 + 1, chronomancer.player.1);
        chronomancer.set_tile(target.0, target.1, Tile::Hazard);
        chronomancer.move_player(1, 0);
        assert_eq!(chronomancer.player, (1, 1));
        assert_eq!(chronomancer.resolve, MAX_RESOLVE);
        assert!(!chronomancer.time_warning);
    }

    #[test]
    fn chronomancer_rewinds_prior_actions_and_spends_one_charge() {
        let mut game = ArchiveGame::new(ClassType::TimeChronomancer, seed());
        game.set_tile(2, 1, Tile::Floor);
        game.set_tile(3, 1, Tile::Floor);
        game.move_player(1, 0);
        game.move_player(1, 0);
        assert_eq!(game.player, (3, 1));
        game.use_power();
        assert_eq!(game.player, (1, 1));
        assert_eq!(game.charges, MAX_CHARGES - 1);
    }

    #[test]
    fn collecting_fragments_unlocks_the_exit_without_xp_or_database_state() {
        let mut game = ArchiveGame::new(ClassType::CodeWarlock, seed());
        for tile in &mut game.tiles {
            if *tile == Tile::Exit {
                *tile = Tile::Floor;
            }
        }
        for x in 2..=5 {
            game.set_tile(x, 1, Tile::Floor);
        }
        game.set_tile(5, 1, Tile::Exit);
        assert_eq!(tile_span(Tile::Exit, false, false).content, "·");
        for x in 2..=4 {
            game.set_tile(x, 1, Tile::Fragment);
            game.move_player(1, 0);
            if x == 2 {
                assert!(game.message.contains("east"));
            }
        }
        assert!(game.message.contains("exit awakens east"));
        assert_eq!(tile_span(Tile::Exit, false, true).content, "E");
        game.move_player(1, 0);
        assert_eq!(game.status(), ArchiveStatus::Won);
        assert_eq!(game.fragments, FRAGMENTS_TO_ESCAPE);
    }

    #[test]
    fn guardians_advance_every_third_turn_and_only_one_attacks_per_phase() {
        let mut game = ArchiveGame::new(ClassType::CodeWarlock, seed());
        for tile in &mut game.tiles {
            if *tile == Tile::Guardian {
                *tile = Tile::Floor;
            }
        }
        for x in 1..=4 {
            game.set_tile(x, 1, Tile::Floor);
            game.revealed[ArchiveGame::index(x, 1)] = true;
        }
        game.set_tile(4, 1, Tile::Guardian);

        game.turns = 2;
        game.advance_guardians();
        assert_eq!(game.tile(4, 1), Tile::Guardian);

        game.turns = 3;
        game.advance_guardians();
        assert_eq!(game.tile(3, 1), Tile::Guardian);
        assert_eq!(game.tile(4, 1), Tile::Floor);

        game.set_tile(3, 1, Tile::Floor);
        game.set_tile(2, 1, Tile::Guardian);
        game.set_tile(1, 2, Tile::Guardian);
        game.revealed[ArchiveGame::index(1, 2)] = true;
        game.turns = 6;
        game.advance_guardians();
        assert_eq!(game.resolve, MAX_RESOLVE - 1);
        assert!(game.message.contains("roaming guardian"));
    }

    #[test]
    fn archive_renders_in_normal_and_narrow_terminals() {
        let game = ArchiveGame::new(ClassType::SystemsArchitect, seed());
        for (width, height) in [(90, 28), (80, 24), (40, 12)] {
            let backend = TestBackend::new(width, height);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| draw_archive(frame, &game, "Archivist", 12))
                .unwrap();
        }
    }

    #[test]
    fn player_glyph_uses_the_active_class_color() {
        for class in [
            ClassType::CodeWarlock,
            ClassType::TaskPaladin,
            ClassType::MindSage,
            ClassType::SystemsArchitect,
            ClassType::TimeChronomancer,
            ClassType::ArchAccountant,
        ] {
            let game = ArchiveGame::new(class, seed());
            let backend = TestBackend::new(90, 28);
            let mut terminal = Terminal::new(backend).unwrap();
            terminal
                .draw(|frame| draw_archive(frame, &game, "Archivist", 12))
                .unwrap();

            let player_cell = terminal
                .backend()
                .buffer()
                .content
                .iter()
                .find(|cell| cell.symbol() == "@")
                .expect("rendered Archive should contain the player glyph");
            assert_eq!(player_cell.fg, Theme::for_class(class).primary);
            assert!(player_cell.modifier.contains(Modifier::BOLD));
        }
    }

    #[test]
    fn map_symbols_use_distinct_semantic_colors() {
        let fragment = tile_span(Tile::Fragment, false, false).style.fg;
        let sigil = tile_span(Tile::Sigil, false, false).style.fg;
        let guardian = tile_span(Tile::Guardian, false, false).style.fg;
        let hazard = tile_span(Tile::Hazard, false, false).style.fg;
        let locked_exit = tile_span(Tile::Exit, false, false).style.fg;
        let open_exit = tile_span(Tile::Exit, false, true).style.fg;

        let colors = [fragment, sigil, guardian, hazard, locked_exit, open_exit];
        for (index, color) in colors.iter().enumerate() {
            assert!(
                !colors[..index].contains(color),
                "Archive map symbols should have distinct colors"
            );
        }
    }

    #[test]
    fn different_seeds_generate_different_archives() {
        let first = ArchiveGame::new(ClassType::CodeWarlock, [7; 32]);
        let second = ArchiveGame::new(ClassType::CodeWarlock, [8; 32]);
        assert_ne!(first.tiles, second.tiles);
    }
}

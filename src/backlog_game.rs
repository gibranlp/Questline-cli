// ─────────────────────────────────────────────────────────────────────────────
// backlog_game.rs — THE BACKLOG. El segundo easter egg: un escape room de
// terminal con parser de comandos. Vive aparte del TUI principal, igual que
// archive_game.rs: recibe datos de display y nunca una Database.
// ─────────────────────────────────────────────────────────────────────────────

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use std::{collections::HashSet, io};

use crate::{
    backlog_rooms::{Action, Lock, ROOM_COUNT, ROOMS, Room, Thing, class_index},
    models::ClassType,
    theme::Theme,
};

const MIN_TERMINAL_WIDTH: u16 = 72;
const MIN_TERMINAL_HEIGHT: u16 = 22;
/// Attention at which the Backlog stops observing and starts filing.
const MAX_ATTENTION: u8 = 6;
const BASE_ATTUNE_CHARGES: u8 = 3;
const TRANSCRIPT_LIMIT: usize = 400;
const INPUT_LIMIT: usize = 120;

const EXIT_WORDS: &[&str] = &[
    "door",
    "doors",
    "exit",
    "out",
    "way out",
    "dials",
    "dial",
    "reader",
    "card reader",
    "service door",
    "doorway",
    "forward",
    "on",
    "onward",
    "ahead",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BacklogStatus {
    Running,
    /// Reached daylight.
    Escaped,
    /// Walked out through the terminal instead.
    Left,
}

/// How a transcript entry is coloured. Kept separate from the text so the whole
/// log can be restyled per class without re-writing any prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tone {
    /// The player's own typed line, echoed back.
    Echo,
    /// Room titles and section breaks.
    Heading,
    /// Ordinary description.
    Prose,
    /// Ambience and asides.
    Muted,
    /// The Backlog noticing you.
    Alarm,
    /// A class-specific reading of the room.
    Insight,
    /// A way out opening.
    Passage,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verb {
    Look,
    Examine,
    Take,
    Use,
    Read,
    Open,
    Listen,
    Say,
    Inventory,
    Wait,
    Attune,
    Help,
    Leave,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    verb: Verb,
    /// The direct object, articles stripped.
    object: String,
    /// The indirect object of `use X on Y`.
    target: String,
    /// Everything after the verb, normalized but otherwise untouched — this is
    /// what a phrase lock is matched against.
    payload: String,
}

pub struct BacklogGame {
    class: ClassType,
    room: usize,
    inventory: Vec<&'static str>,
    flags: HashSet<&'static str>,
    /// Flags raised inside the current room, so a filing can take them back.
    room_flags: Vec<&'static str>,
    attention: u8,
    /// Chronomancers only register every second provocation.
    chrono_skip: bool,
    filings: u32,
    paladin_grace: bool,
    charges: u8,
    turns: u32,
    transcript: Vec<(Tone, String)>,
    input: String,
    input_history: Vec<String>,
    history_cursor: Option<usize>,
    /// Normalized text of the last command that failed, for repeat detection.
    last_failed: Option<String>,
    scroll: u16,
    follow_tail: bool,
    show_help: bool,
    leave_armed: bool,
    escaped_before: bool,
    status: BacklogStatus,
}

impl BacklogGame {
    pub fn new(class: ClassType, escaped_before: bool) -> Self {
        let mut game = Self {
            class,
            room: 0,
            inventory: Vec::new(),
            flags: HashSet::new(),
            room_flags: Vec::new(),
            attention: 0,
            chrono_skip: false,
            filings: 0,
            paladin_grace: class == ClassType::TaskPaladin,
            charges: if class == ClassType::ArchAccountant {
                BASE_ATTUNE_CHARGES + 1
            } else {
                BASE_ATTUNE_CHARGES
            },
            turns: 0,
            transcript: Vec::new(),
            input: String::new(),
            input_history: Vec::new(),
            history_cursor: None,
            last_failed: None,
            scroll: 0,
            follow_tail: true,
            show_help: false,
            leave_armed: false,
            escaped_before,
            status: BacklogStatus::Running,
        };
        game.push(
            Tone::Muted,
            "The Great Backlog does not have an entrance. It has an arrival.".to_string(),
        );
        game.push(
            Tone::Muted,
            "Type LOOK to begin. Type HELP if the dark gets procedural.".to_string(),
        );
        game.describe_room(true);
        game
    }

    pub fn status(&self) -> BacklogStatus {
        self.status
    }

    pub fn class(&self) -> ClassType {
        self.class
    }

    pub fn attention(&self) -> u8 {
        self.attention
    }

    pub fn room_index(&self) -> usize {
        self.room
    }

    pub fn filings(&self) -> u32 {
        self.filings
    }

    fn current(&self) -> &'static Room {
        &ROOMS[self.room]
    }

    /// 0 settled, 1 uneasy, 2 wrong.
    fn tier(&self) -> usize {
        match self.attention {
            0..=1 => 0,
            2..=4 => 1,
            _ => 2,
        }
    }

    fn push(&mut self, tone: Tone, text: String) {
        for chunk in text.split('\n') {
            self.transcript.push((tone, chunk.to_string()));
        }
        if self.transcript.len() > TRANSCRIPT_LIMIT {
            let overflow = self.transcript.len() - TRANSCRIPT_LIMIT;
            self.transcript.drain(0..overflow);
        }
        self.follow_tail = true;
    }

    fn blank(&mut self) {
        if self
            .transcript
            .last()
            .is_some_and(|(_, text)| text.is_empty())
        {
            return;
        }
        self.transcript.push((Tone::Prose, String::new()));
    }

    // ── visibility ──────────────────────────────────────────────────────────

    /// Mind Sages read past the gate. Everyone else has to earn the fixture.
    fn can_see(&self, thing: &Thing) -> bool {
        match thing.hidden_until {
            None => true,
            Some(flag) => self.class == ClassType::MindSage || self.flags.contains(flag),
        }
    }

    fn find_thing(&self, name: &str) -> Option<&'static Thing> {
        if name.is_empty() {
            return None;
        }
        let room = self.current();
        // Exact noun or alias first, then a forgiving substring pass so that
        // "sign-in sheet" and "the brass dials" both land.
        room.things
            .iter()
            .find(|thing| {
                self.can_see(thing) && (thing.noun == name || thing.aliases.contains(&name))
            })
            .or_else(|| {
                room.things.iter().find(|thing| {
                    self.can_see(thing)
                        && (thing.noun.contains(name)
                            || name.contains(thing.noun)
                            || thing
                                .aliases
                                .iter()
                                .any(|alias| alias.contains(name) || name.contains(*alias)))
                })
            })
    }

    // ── attention ───────────────────────────────────────────────────────────

    /// Raises attention. Examining and reading never call this — the writing is
    /// the reward and punishing curiosity would be the wrong game.
    fn notice(&mut self, amount: u8) {
        if self.status != BacklogStatus::Running {
            return;
        }
        let before = self.tier();
        for _ in 0..amount {
            if self.class == ClassType::TimeChronomancer {
                self.chrono_skip = !self.chrono_skip;
                if self.chrono_skip {
                    continue;
                }
            }
            self.attention = self.attention.saturating_add(1);
        }
        if self.attention >= MAX_ATTENTION {
            self.file_the_player();
            return;
        }
        if self.tier() > before {
            let line = self.current().ambience[self.tier()].to_string();
            self.push(Tone::Alarm, line);
        }
    }

    fn file_the_player(&mut self) {
        if self.paladin_grace {
            self.paladin_grace = false;
            self.attention = 0;
            self.blank();
            self.push(
                Tone::Passage,
                crate::backlog_rooms::PALADIN_GRACE_TEXT.to_string(),
            );
            return;
        }
        self.filings += 1;
        self.attention = MAX_ATTENTION / 2;
        for flag in std::mem::take(&mut self.room_flags) {
            self.flags.remove(flag);
        }
        self.blank();
        self.push(Tone::Alarm, crate::backlog_rooms::FILED_TEXT.to_string());
        self.describe_room(true);
    }

    // ── rooms ───────────────────────────────────────────────────────────────

    fn describe_room(&mut self, with_intro: bool) {
        let room = self.current();
        self.blank();
        self.push(Tone::Heading, format!("{} — {}", self.room + 1, room.title));
        if with_intro {
            self.push(Tone::Prose, room.intro.to_string());
        }
        let tier = self.tier();
        self.push(Tone::Muted, room.ambience[tier].to_string());

        let visible: Vec<&str> = room
            .things
            .iter()
            .filter(|thing| self.can_see(thing))
            .map(|thing| thing.glance)
            .collect();
        if !visible.is_empty() {
            self.push(
                Tone::Prose,
                format!("You can see {}.", join_clauses(&visible)),
            );
        }

        if self.class == ClassType::SystemsArchitect {
            let hidden = room
                .things
                .iter()
                .filter(|thing| thing.hidden_until.is_some())
                .count();
            self.push(
                Tone::Insight,
                format!(
                    "Structural read: {} fixtures, {} not yet load-bearing. \
The way out is {}-locked.",
                    room.things.len(),
                    hidden,
                    room.lock.shape()
                ),
            );
        }
    }

    fn advance_room(&mut self) {
        let exit_text = self.current().exit_text.to_string();
        self.blank();
        self.push(Tone::Passage, exit_text);
        self.room_flags.clear();
        self.last_failed = None;
        if self.room + 1 >= ROOM_COUNT {
            self.status = BacklogStatus::Escaped;
            let closing = if self.escaped_before {
                crate::backlog_rooms::ESCAPE_AGAIN
            } else {
                crate::backlog_rooms::ESCAPE_FIRST
            };
            self.blank();
            self.push(Tone::Passage, closing.to_string());
            return;
        }
        self.room += 1;
        self.attention = self.attention.saturating_sub(2);
        self.describe_room(true);
    }

    fn raise_flag(&mut self, flag: &'static str) -> bool {
        if self.flags.insert(flag) {
            self.room_flags.push(flag);
            true
        } else {
            false
        }
    }

    // ── command handling ────────────────────────────────────────────────────

    pub fn submit(&mut self, raw: &str) {
        if self.status != BacklogStatus::Running {
            return;
        }
        let normalized = normalize(raw);
        if normalized.is_empty() {
            return;
        }
        self.turns += 1;
        self.push(Tone::Echo, format!("> {normalized}"));
        let repeated_failure = self
            .last_failed
            .as_ref()
            .is_some_and(|previous| *previous == normalized);
        let command = parse(&normalized);
        let failed = self.resolve(command);
        if failed {
            if repeated_failure {
                self.push(
                    Tone::Alarm,
                    "You try it again, the same way, and the room registers the repetition."
                        .to_string(),
                );
                self.notice(1);
            }
            self.last_failed = Some(normalized);
        } else {
            self.last_failed = None;
        }
    }

    /// Returns true when the attempt failed, which is what repeat-detection and
    /// the attention meter care about.
    fn resolve(&mut self, command: Command) -> bool {
        match command.verb {
            Verb::Look => {
                self.describe_room(false);
                false
            }
            Verb::Help => {
                self.show_help = true;
                false
            }
            Verb::Inventory => {
                if self.inventory.is_empty() {
                    self.push(
                        Tone::Muted,
                        "You are carrying nothing, which is how everyone arrives.".to_string(),
                    );
                } else {
                    let items: Vec<&str> = self.inventory.clone();
                    self.push(
                        Tone::Prose,
                        format!("You are carrying {}.", join_clauses(&items)),
                    );
                }
                false
            }
            Verb::Wait => {
                self.push(
                    Tone::Muted,
                    "You wait. The Backlog is extremely good at this and has been \
practising longer."
                        .to_string(),
                );
                self.notice(1);
                false
            }
            Verb::Attune => {
                self.attune();
                false
            }
            Verb::Leave => {
                self.arm_leave();
                false
            }
            Verb::Examine => self.examine(&command.object),
            Verb::Listen => {
                let target = if command.object.is_empty() {
                    self.current().listen.unwrap_or_default().to_string()
                } else {
                    command.object.clone()
                };
                if target.is_empty() {
                    self.push(
                        Tone::Muted,
                        "You listen. There is nothing in this room doing anything worth \
overhearing."
                            .to_string(),
                    );
                    return false;
                }
                self.interact(&target)
            }
            Verb::Take | Verb::Read | Verb::Open | Verb::Use => self.physical(command),
            Verb::Say => {
                if command.payload.is_empty() {
                    self.push(
                        Tone::Muted,
                        "You say nothing, at length. The room takes the point.".to_string(),
                    );
                    return false;
                }
                self.try_exit(Some(&command.payload))
            }
            Verb::Unknown => {
                self.push(
                    Tone::Muted,
                    "The Backlog does not recognise that. It recognises very little, \
and it is not embarrassed about it. Try HELP."
                        .to_string(),
                );
                false
            }
        }
    }

    fn physical(&mut self, command: Command) -> bool {
        // `use X on Y` always resolves against Y — that is the thing being acted on.
        if command.verb == Verb::Use && !command.target.is_empty() {
            return self.use_item_on(&command.object, &command.target);
        }

        // `use badge` with nothing to use it on, where the way out wants exactly
        // that. Saves the player from guessing the noun on the far side.
        if command.verb == Verb::Use
            && !command.object.is_empty()
            && self.find_thing(&command.object).is_none()
            && self
                .inventory
                .iter()
                .any(|held| held.contains(command.object.as_str()))
        {
            return self.try_exit(None);
        }

        let object = if command.object.is_empty() {
            match command.verb {
                Verb::Open => EXIT_WORDS[0].to_string(),
                _ => String::new(),
            }
        } else {
            command.object.clone()
        };

        if object.is_empty() {
            self.push(
                Tone::Muted,
                "On what? The room is patient but it is not a mind reader; \
it gave that up along with the windows."
                    .to_string(),
            );
            return false;
        }

        // Opening or entering the way out never resolves to a fixture — the exit
        // is authoritative, so `open door` works even where a door is also a thing.
        if matches!(command.verb, Verb::Open) && is_exit_word(&object) {
            return self.try_exit(None);
        }

        self.interact(&object)
    }

    fn examine(&mut self, object: &str) -> bool {
        if object.is_empty() {
            self.describe_room(false);
            return false;
        }
        match self.find_thing(object) {
            Some(thing) => self.push(Tone::Prose, thing.examine.to_string()),
            None => self.no_such_thing(object),
        }
        // Looking at things is always free. That is the whole contract with the
        // player: the writing is the reward, so curiosity is never taxed.
        false
    }

    fn interact(&mut self, object: &str) -> bool {
        let Some(thing) = self.find_thing(object) else {
            self.no_such_thing(object);
            return false;
        };
        match &thing.action {
            Action::Inert(text) => {
                self.push(Tone::Prose, text.to_string());
                false
            }
            Action::Grants { item, text, repeat } => {
                if self.inventory.contains(item) {
                    self.push(Tone::Muted, repeat.to_string());
                } else {
                    self.inventory.push(*item);
                    self.push(Tone::Prose, text.to_string());
                }
                false
            }
            Action::Reveals { flag, text, repeat } => {
                if self.raise_flag(flag) {
                    self.push(Tone::Prose, text.to_string());
                } else {
                    self.push(Tone::Muted, repeat.to_string());
                }
                false
            }
            Action::NeedsItem {
                item,
                flag,
                text,
                refusal,
            } => {
                if self.inventory.contains(item) {
                    self.raise_flag(flag);
                    self.push(Tone::Prose, text.to_string());
                    false
                } else {
                    self.push(Tone::Muted, refusal.to_string());
                    true
                }
            }
        }
    }

    fn use_item_on(&mut self, item: &str, target: &str) -> bool {
        let Some(thing) = self.find_thing(target) else {
            // `use badge on door` where the door is only ever the exit.
            if is_exit_word(target) {
                return self.try_exit(None);
            }
            self.no_such_thing(target);
            return false;
        };
        if let Action::NeedsItem {
            item: wanted,
            flag,
            text,
            refusal,
        } = &thing.action
        {
            let carried = self
                .inventory
                .iter()
                .any(|held| held.contains(item) || item.contains(*held));
            if carried && (wanted.contains(item) || item.contains(*wanted)) {
                self.raise_flag(flag);
                self.push(Tone::Prose, text.to_string());
                return false;
            }
            if !carried {
                self.push(
                    Tone::Muted,
                    format!("You are not carrying anything answering to \"{item}\"."),
                );
                return true;
            }
            self.push(Tone::Muted, refusal.to_string());
            return true;
        }
        if is_exit_word(target) {
            return self.try_exit(None);
        }
        self.push(
            Tone::Muted,
            "Nothing happens, in the complete and unhurried way that things do not happen here."
                .to_string(),
        );
        true
    }

    fn no_such_thing(&mut self, object: &str) {
        self.push(
            Tone::Muted,
            format!(
                "There is no {object} here. There is no {object} anywhere, probably, \
but the room only speaks for itself."
            ),
        );
    }

    fn attune(&mut self) {
        if self.charges == 0 {
            self.push(
                Tone::Muted,
                "You reach for the Order's training and find it has already told you \
everything it knows about this floor."
                    .to_string(),
            );
            return;
        }
        self.charges -= 1;
        let insight = self.current().insights[class_index(self.class)];
        self.blank();
        self.push(Tone::Insight, insight.to_string());
    }

    fn arm_leave(&mut self) {
        if self.leave_armed {
            self.status = BacklogStatus::Left;
            return;
        }
        self.leave_armed = true;
        self.push(
            Tone::Alarm,
            "There is no back door. There is the way you came, which is now carpet, \
and the way on. Ask again and the terminal will let you out, \
which is not the same as the Backlog letting you out."
                .to_string(),
        );
    }

    /// The single authority on whether the way out opens.
    fn try_exit(&mut self, spoken: Option<&str>) -> bool {
        let room = self.current();
        let opened = match &room.lock {
            Lock::Item(item) => self.inventory.contains(item),
            Lock::Flag(flag) => self.flags.contains(flag),
            Lock::Phrase { accepts, rejects } => {
                let Some(said) = spoken else {
                    self.push(
                        Tone::Muted,
                        "The way out is not waiting on a handle. It is waiting on a sentence. \
Use SAY."
                            .to_string(),
                    );
                    self.notice(1);
                    return true;
                };
                if accepts.iter().any(|accept| said.contains(accept)) {
                    true
                } else if let Some((_, response)) =
                    rejects.iter().find(|(reject, _)| said.contains(reject))
                {
                    self.push(Tone::Alarm, (*response).to_string());
                    self.notice(1);
                    return true;
                } else {
                    false
                }
            }
            Lock::AnyAnswer { rejects } => {
                let Some(said) = spoken else {
                    self.push(
                        Tone::Muted,
                        "The door is waiting to be told one true thing. Use SAY.".to_string(),
                    );
                    self.notice(1);
                    return true;
                };
                // Compared both ways: "tomorrow" and "I'll do it tomorrow" are the
                // same refusal, but "I don't know" must not be shortened into
                // "don't know" before the door has had a chance to recognise it.
                let bare = strip_intent(said);
                if let Some((_, response)) = rejects
                    .iter()
                    .find(|(reject, _)| bare == *reject || said == *reject)
                {
                    self.push(Tone::Alarm, (*response).to_string());
                    self.notice(1);
                    return true;
                }
                !bare.is_empty()
            }
        };

        if opened {
            self.advance_room();
            false
        } else {
            let locked = room.locked_text.to_string();
            self.push(Tone::Alarm, locked);
            if spoken.is_none() && room.lock.wants_words() {
                self.push(Tone::Muted, "Use SAY.".to_string());
            }
            self.notice(1);
            true
        }
    }

    // ── input ───────────────────────────────────────────────────────────────

    /// Returns true when the game is over and the caller should tear down.
    pub fn handle_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press {
            return false;
        }
        if self.show_help {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.show_help = false,
                _ => {}
            }
            return false;
        }
        if self.status != BacklogStatus::Running {
            return matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::Char(_));
        }
        match key.code {
            KeyCode::Enter => {
                let line = std::mem::take(&mut self.input);
                self.history_cursor = None;
                if !line.trim().is_empty() {
                    self.input_history.push(line.trim().to_string());
                }
                self.submit(&line);
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Esc => {
                if self.input.is_empty() {
                    self.arm_leave();
                    if self.status == BacklogStatus::Left {
                        return true;
                    }
                } else {
                    self.input.clear();
                }
            }
            KeyCode::Up => self.recall(-1),
            KeyCode::Down => self.recall(1),
            KeyCode::PageUp => {
                self.follow_tail = false;
                self.scroll = self.scroll.saturating_sub(6);
            }
            KeyCode::PageDown => {
                self.scroll = self.scroll.saturating_add(6);
            }
            KeyCode::Char(character) if self.input.chars().count() < INPUT_LIMIT => {
                self.input.push(character)
            }
            _ => {}
        }
        false
    }

    fn recall(&mut self, direction: isize) {
        if self.input_history.is_empty() {
            return;
        }
        let last = self.input_history.len() - 1;
        self.history_cursor = match (self.history_cursor, direction) {
            (None, -1) => Some(last),
            (None, _) => None,
            (Some(0), -1) => Some(0),
            (Some(index), -1) => Some(index - 1),
            (Some(index), _) if index >= last => None,
            (Some(index), _) => Some(index + 1),
        };
        self.input = match self.history_cursor {
            Some(index) => self.input_history[index].clone(),
            None => String::new(),
        };
    }
}

// ── parsing ─────────────────────────────────────────────────────────────────

/// Lowercases, drops punctuation the parser has no use for, and collapses
/// whitespace. Digits survive because one lock is a three-digit code.
pub fn normalize(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for character in raw.chars() {
        if character.is_alphanumeric() {
            out.extend(character.to_lowercase());
        } else if character != '\'' {
            // Apostrophes vanish so "don't" and "dont" are the same answer;
            // everything else becomes a separator.
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_articles(text: &str) -> String {
    let mut words: Vec<&str> = text.split_whitespace().collect();
    while matches!(
        words.first(),
        Some(&"the") | Some(&"a") | Some(&"an") | Some(&"my")
    ) {
        words.remove(0);
    }
    words.join(" ")
}

/// Removes the "I will" scaffolding so a promise is compared on its content.
fn strip_intent(text: &str) -> String {
    let mut current = text.to_string();
    for prefix in [
        "i am going to ",
        "im going to ",
        "i will ",
        "ill ",
        "i want to ",
        "i intend to ",
        "i promise to ",
        "i am ",
        "im ",
        "i ",
    ] {
        if let Some(rest) = current.strip_prefix(prefix) {
            current = rest.to_string();
            break;
        }
    }
    strip_articles(current.trim())
}

fn is_exit_word(word: &str) -> bool {
    EXIT_WORDS.contains(&word)
}

fn verb_for(word: &str) -> Verb {
    match word {
        "look" | "l" | "survey" => Verb::Look,
        "examine" | "x" | "inspect" | "study" | "consider" => Verb::Examine,
        "take" | "get" | "g" | "grab" | "pick" | "pocket" => Verb::Take,
        "use" | "apply" | "swipe" | "put" | "insert" => Verb::Use,
        "read" => Verb::Read,
        // `search` and `check` act on a thing rather than describing it —
        // "search the drawer" should open the drawer, not narrate it.
        "open" | "enter" | "go" | "push" | "unlock" | "proceed" | "descend" | "search"
        | "check" | "turn" => Verb::Open,
        "listen" | "hear" => Verb::Listen,
        "say" | "speak" | "answer" | "tell" | "state" | "declare" | "type" => Verb::Say,
        "inventory" | "i" | "inv" | "satchel" | "carrying" => Verb::Inventory,
        "wait" | "z" | "rest" | "stay" => Verb::Wait,
        "attune" | "insight" | "focus" | "meditate" | "recall" => Verb::Attune,
        "help" | "commands" | "verbs" => Verb::Help,
        "leave" | "quit" | "exit" | "q" | "abandon" => Verb::Leave,
        _ => Verb::Unknown,
    }
}

/// Splits an already-normalized line into a command. Pure — the whole parser is
/// testable without a terminal.
pub fn parse(normalized: &str) -> Command {
    let mut words = normalized.split_whitespace();
    let head = words.next().unwrap_or_default();
    let rest: String = words.collect::<Vec<_>>().join(" ");
    let mut verb = verb_for(head);

    // A bare noun is a look-at, which is what people type when they forget the verb.
    if verb == Verb::Unknown && rest.is_empty() && !head.is_empty() {
        verb = Verb::Examine;
        return Command {
            verb,
            object: strip_articles(head),
            target: String::new(),
            payload: head.to_string(),
        };
    }

    let payload = rest.clone();

    // `use X on Y` / `use X with Y`. Also tolerates `put X in Y`.
    let (object, target) = if matches!(verb, Verb::Use) {
        let mut split = None;
        for joiner in [" on ", " with ", " in ", " into ", " to ", " against "] {
            if let Some(position) = rest.find(joiner) {
                split = Some((position, joiner.len()));
                break;
            }
        }
        match split {
            Some((position, length)) => (
                strip_articles(&rest[..position]),
                strip_articles(&rest[position + length..]),
            ),
            None => (strip_articles(&rest), String::new()),
        }
    } else {
        let trimmed = rest
            .strip_prefix("at ")
            .or_else(|| rest.strip_prefix("to "))
            .unwrap_or(&rest);
        (strip_articles(trimmed), String::new())
    };

    Command {
        verb,
        object,
        target,
        payload,
    }
}

fn join_clauses(items: &[&str]) -> String {
    match items {
        [] => String::new(),
        [only] => (*only).to_string(),
        [head @ .., last] => format!("{}, and {last}", head.join(", ")),
    }
}

// ── rendering ───────────────────────────────────────────────────────────────

fn tone_style(tone: Tone, theme: &Theme) -> Style {
    match tone {
        Tone::Echo => Style::default()
            .fg(theme.primary)
            .add_modifier(Modifier::BOLD),
        Tone::Heading => Style::default()
            .fg(theme.secondary)
            .add_modifier(Modifier::BOLD),
        Tone::Prose => Style::default().fg(Color::Rgb(203, 213, 225)),
        Tone::Muted => Style::default().fg(Color::Rgb(113, 122, 140)),
        Tone::Alarm => Style::default().fg(Color::Rgb(248, 113, 113)),
        Tone::Insight => Style::default().fg(Color::Rgb(250, 204, 21)),
        Tone::Passage => Style::default()
            .fg(Color::Rgb(134, 239, 172))
            .add_modifier(Modifier::BOLD),
    }
}

fn attention_bar(attention: u8) -> String {
    let filled = attention.min(MAX_ATTENTION) as usize;
    let empty = MAX_ATTENTION as usize - filled;
    format!("{}{}", "▓".repeat(filled), "░".repeat(empty))
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

pub fn draw_backlog(frame: &mut Frame, game: &BacklogGame, username: &str, level: i32) {
    let area = frame.size();
    let theme = Theme::for_class(game.class);

    if area.width < MIN_TERMINAL_WIDTH || area.height < MIN_TERMINAL_HEIGHT {
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from("The Backlog needs more room than this."),
                Line::from(format!(
                    "Resize to at least {MIN_TERMINAL_WIDTH}x{MIN_TERMINAL_HEIGHT}."
                )),
                Line::from(""),
                Line::from("It has plenty of room. It is being generous."),
            ])
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }

    frame.render_widget(
        Block::default().style(Style::default().bg(Color::Rgb(12, 12, 14))),
        area,
    );

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(6),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);

    // Header.
    let room = &ROOMS[game.room];
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {} ", room.title),
                Style::default()
                    .fg(theme.secondary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!("· floor {} of {ROOM_COUNT}", game.room + 1),
                Style::default().fg(Color::Rgb(113, 122, 140)),
            ),
        ]))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" THE GREAT BACKLOG "),
        ),
        rows[0],
    );

    // Transcript.
    let lines: Vec<Line> = game
        .transcript
        .iter()
        .map(|(tone, text)| Line::from(Span::styled(text.clone(), tone_style(*tone, &theme))))
        .collect();
    let transcript = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .block(Block::default().borders(Borders::ALL).title(" Descent "));
    let inner_width = rows[1].width.saturating_sub(2);
    let inner_height = rows[1].height.saturating_sub(2);
    let total = transcript.line_count(inner_width) as u16;
    let max_scroll = total.saturating_sub(inner_height);
    let offset = if game.follow_tail {
        max_scroll
    } else {
        game.scroll.min(max_scroll)
    };
    frame.render_widget(transcript.scroll((offset, 0)), rows[1]);

    // Prompt.
    let prompt = if game.status == BacklogStatus::Running {
        Line::from(vec![
            Span::styled(
                " > ",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                game.input.clone(),
                Style::default().fg(Color::Rgb(226, 232, 240)),
            ),
            Span::styled(
                "▌",
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ])
    } else {
        Line::from(Span::styled(
            " The floor is quiet. [Enter] returns you to the terminal.",
            Style::default().fg(Color::Rgb(113, 122, 140)),
        ))
    };
    frame.render_widget(
        Paragraph::new(prompt).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" What do you do "),
        ),
        rows[2],
    );

    // Status strip.
    let mut status = vec![
        Span::styled(format!(" {username} "), Style::default().fg(theme.primary)),
        Span::styled(
            format!("Lv{level} · {} ", game.class.name()),
            Style::default().fg(Color::Rgb(113, 122, 140)),
        ),
        Span::styled("Attention ", Style::default().fg(Color::Rgb(113, 122, 140))),
        Span::styled(
            attention_bar(game.attention),
            Style::default().fg(if game.tier() == 2 {
                Color::Rgb(248, 113, 113)
            } else if game.tier() == 1 {
                Color::Rgb(250, 204, 21)
            } else {
                Color::Rgb(74, 222, 128)
            }),
        ),
        Span::styled(
            format!(
                "  attune {}  satchel {}",
                game.charges,
                game.inventory.len()
            ),
            Style::default().fg(Color::Rgb(113, 122, 140)),
        ),
    ];
    if game.class == ClassType::CodeWarlock {
        status.push(Span::styled(
            format!("  // exit.lock == {}", room.lock.shape().to_uppercase()),
            Style::default().fg(theme.secondary),
        ));
    }
    frame.render_widget(Paragraph::new(Line::from(status)), rows[3]);

    if game.show_help {
        draw_help(frame, area, game);
    } else if game.status != BacklogStatus::Running {
        draw_result(frame, area, game);
    }
}

/// Width of the verb column inside the help overlay. The longest entry is
/// "USE <item> ON <thing>" at 21 columns.
const HELP_VERB_WIDTH: usize = 21;
const HELP_INDENT: &str = "  ";

const HELP_VERBS: &[(&str, &str)] = &[
    ("LOOK", "the room again"),
    ("EXAMINE <thing>", "read it — always free"),
    ("X <thing>", "short for EXAMINE"),
    ("TAKE / READ / OPEN", "act on a thing"),
    ("USE <item> ON <thing>", "the classic escape-room move"),
    ("LISTEN", "some floors are heard, not seen"),
    ("SAY <words>", "for the ways out that want a sentence"),
    ("INVENTORY  ·  I", "what you are carrying"),
    ("ATTUNE", "your Order's reading of this floor"),
    ("WAIT", "not recommended"),
];

fn draw_help(frame: &mut Frame, area: Rect, game: &BacklogGame) {
    let theme = Theme::for_class(game.class);
    // 18 authored lines plus the two border rows — sized to fit exactly, and
    // still inside the 72x22 minimum the game refuses to draw below.
    let popup = centered_rect(66, 20, area);
    frame.render_widget(Clear, popup);

    // Centred heading and footers, but the verb table is left-aligned so the
    // two columns actually line up. Per-line alignment beats the paragraph's.
    let centered =
        |text: String, style: Style| Line::styled(text, style).alignment(Alignment::Center);
    let muted = Style::default().fg(Color::Rgb(148, 163, 184));

    let mut text = vec![
        centered(
            "WHAT THE BACKLOG UNDERSTANDS".to_string(),
            Style::default()
                .fg(Color::Rgb(250, 204, 21))
                .add_modifier(Modifier::BOLD),
        ),
        Line::from(""),
    ];
    text.extend(HELP_VERBS.iter().map(|(verb, gloss)| {
        Line::from(vec![
            Span::raw(HELP_INDENT),
            Span::styled(
                format!("{verb:<HELP_VERB_WIDTH$}"),
                Style::default()
                    .fg(theme.primary)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled((*gloss).to_string(), muted),
        ])
    }));
    text.push(Line::from(""));
    text.push(centered(
        "Examining never raises Attention. Failing does.".to_string(),
        Style::default().fg(Color::Rgb(203, 213, 225)),
    ));
    text.push(centered(
        "↑ ↓ recall · PgUp/PgDn scroll · Esc twice leaves".to_string(),
        muted,
    ));
    text.push(Line::from(""));
    text.push(centered(
        format!(
            "{} · {} attune charges left",
            game.class.name(),
            game.charges
        ),
        Style::default().fg(theme.secondary),
    ));
    text.push(centered("[Esc / Enter] close".to_string(), muted));

    // No Wrap: trimming would eat the column padding, and every line is
    // authored to fit the popup's interior width.
    frame.render_widget(
        Paragraph::new(text).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Orientation "),
        ),
        popup,
    );
}

fn draw_result(frame: &mut Frame, area: Rect, game: &BacklogGame) {
    let popup = centered_rect(58, 11, area);
    frame.render_widget(Clear, popup);
    let escaped = game.status == BacklogStatus::Escaped;
    let (title, color, headline) = if escaped {
        (
            " DAYLIGHT ",
            Color::Rgb(134, 239, 172),
            "You left under your own power.",
        )
    } else {
        (
            " STILL DOWN THERE ",
            Color::Rgb(248, 113, 113),
            "You close the terminal. The floor does not close.",
        )
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                headline,
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!(
                "{} · floor {} of {ROOM_COUNT} · {} turns",
                game.class.name(),
                game.room + 1,
                game.turns
            )),
            Line::from(format!(
                "{} filed · {} attune charges unspent",
                game.filings, game.charges
            )),
            Line::from(""),
            Line::from("[Enter] return to the terminal"),
        ])
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL).title(title)),
        popup,
    );
}

/// Runs an isolated alternate-screen escape room. Like the Forgotten Archive it
/// receives character display data only — no database, no sync handle — so
/// nothing that happens down here can touch productivity state or award XP.
pub fn run(
    class: ClassType,
    username: &str,
    level: i32,
    escaped_before: bool,
) -> Result<BacklogStatus> {
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

    let mut game = BacklogGame::new(class, escaped_before);
    let result = (|| -> Result<()> {
        loop {
            terminal.draw(|frame| draw_backlog(frame, &game, username, level))?;
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
    Ok(game.status())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backlog_rooms::Lock;
    use ratatui::backend::TestBackend;

    const CLASSES: [ClassType; 6] = [
        ClassType::CodeWarlock,
        ClassType::TaskPaladin,
        ClassType::MindSage,
        ClassType::SystemsArchitect,
        ClassType::TimeChronomancer,
        ClassType::ArchAccountant,
    ];

    /// One clean run through all seven floors. This is the regression test that
    /// matters: every edit to the room content has to keep this walkable.
    const WALKTHROUGH: &[&str] = &[
        // 1 — Reception That Never Opened
        "read sheet",
        "open drawer",
        "open door",
        // 2 — The Corridor of Almost
        "listen",
        "open door",
        // 3 — The Ledger That Would Not Close
        "read ledger",
        "say 741",
        // 4 — The Meeting That Has Not Ended
        "read agenda",
        "say this could have been an email",
        // 5 — The Room With Your Handwriting
        "read note",
        "read second note",
        "read third note",
        "open door",
        // 6 — Level 0: The Open Tabs
        "search tabs",
        "search spinner",
        "use badge on reader",
        "open door",
        // 7 — The Door Marked TOMORROW
        "say i will finish the failing test",
    ];

    fn play(class: ClassType, script: &[&str]) -> BacklogGame {
        let mut game = BacklogGame::new(class, false);
        for line in script {
            game.submit(line);
        }
        game
    }

    #[test]
    fn the_backlog_can_be_escaped_from_start_to_daylight() {
        let game = play(ClassType::CodeWarlock, WALKTHROUGH);
        assert_eq!(game.status(), BacklogStatus::Escaped);
        assert_eq!(game.room_index(), ROOM_COUNT - 1);
        // A clean run never trips the Backlog into filing anyone.
        assert_eq!(game.filings(), 0);
    }

    #[test]
    fn every_class_can_escape_the_same_backlog() {
        for class in CLASSES {
            let game = play(class, WALKTHROUGH);
            assert_eq!(
                game.status(),
                BacklogStatus::Escaped,
                "{} could not escape",
                class.name()
            );
        }
    }

    #[test]
    fn every_room_reads_differently_for_every_class() {
        let insights: HashSet<&str> = ROOMS
            .iter()
            .flat_map(|room| room.insights.iter().copied())
            .collect();
        assert_eq!(insights.len(), ROOM_COUNT * 6);
    }

    #[test]
    fn parser_accepts_abbreviations_and_articles() {
        let short = parse(&normalize("x ledger"));
        let long = parse(&normalize("examine the ledger"));
        assert_eq!((short.verb, short.object), (long.verb, long.object));

        let used = parse(&normalize("use the visitor badge on the card reader"));
        assert_eq!(used.verb, Verb::Use);
        assert_eq!(used.object, "visitor badge");
        assert_eq!(used.target, "card reader");

        // A bare noun is treated as a look-at, which is what people type when
        // they forget there is a verb.
        assert_eq!(parse(&normalize("ledger")).verb, Verb::Examine);
        assert_eq!(parse(&normalize("i")).verb, Verb::Inventory);
    }

    #[test]
    fn normalize_folds_punctuation_and_apostrophes() {
        assert_eq!(normalize("  I DON'T know!!  "), "i dont know");
        assert_eq!(normalize("7-4-1"), "7 4 1");
    }

    #[test]
    fn unknown_nouns_are_refused_in_voice_and_never_panic() {
        let mut game = BacklogGame::new(ClassType::MindSage, false);
        let before = game.transcript.len();
        game.submit("examine the wizard");
        assert!(game.transcript.len() > before);
        assert!(
            game.transcript
                .iter()
                .any(|(_, text)| text.contains("There is no wizard here"))
        );
        assert_eq!(game.attention(), 0);
    }

    #[test]
    fn looking_at_things_never_raises_attention() {
        let mut game = BacklogGame::new(ClassType::CodeWarlock, false);
        for _ in 0..12 {
            game.submit("examine the desk");
            game.submit("read the sign-in sheet");
            game.submit("look");
        }
        assert_eq!(game.attention(), 0);
        assert_eq!(game.filings(), 0);
    }

    #[test]
    fn failing_the_way_out_raises_attention() {
        let mut game = BacklogGame::new(ClassType::CodeWarlock, false);
        game.submit("open door");
        assert_eq!(game.attention(), 1);
        assert_eq!(
            game.room_index(),
            0,
            "the reader has no reason to let you by"
        );
    }

    #[test]
    fn chronomancers_are_noticed_at_half_rate() {
        let mut fast = BacklogGame::new(ClassType::CodeWarlock, false);
        let mut slow = BacklogGame::new(ClassType::TimeChronomancer, false);
        for _ in 0..4 {
            fast.notice(1);
            slow.notice(1);
        }
        assert_eq!(fast.attention(), 4);
        assert_eq!(slow.attention(), 2);
    }

    #[test]
    fn the_paladins_oath_absorbs_the_first_filing() {
        let mut paladin = BacklogGame::new(ClassType::TaskPaladin, false);
        for _ in 0..MAX_ATTENTION {
            paladin.notice(1);
        }
        assert_eq!(paladin.filings(), 0, "the oath eats the first one");
        assert_eq!(paladin.attention(), 0);
        for _ in 0..MAX_ATTENTION {
            paladin.notice(1);
        }
        assert_eq!(paladin.filings(), 1, "and only the first one");
    }

    #[test]
    fn filing_takes_back_what_this_room_taught_you_but_never_the_satchel() {
        let mut game = BacklogGame::new(ClassType::CodeWarlock, false);
        game.submit("read sheet");
        game.submit("open drawer");
        assert!(game.inventory.contains(&"visitor badge"));
        assert!(game.flags.contains("sheet_read"));

        for _ in 0..MAX_ATTENTION {
            game.notice(1);
        }
        assert_eq!(game.filings(), 1);
        assert!(
            !game.flags.contains("sheet_read"),
            "a filing takes back the room's discoveries"
        );
        assert!(
            game.inventory.contains(&"visitor badge"),
            "but never what you are carrying"
        );
        // Still winnable afterwards: the badge is enough on its own.
        game.submit("open door");
        assert_eq!(game.room_index(), 1);
    }

    #[test]
    fn hidden_fixtures_stay_hidden_until_earned_except_for_mind_sages() {
        let mut warlock = BacklogGame::new(ClassType::CodeWarlock, false);
        assert!(warlock.find_thing("drawer").is_none());
        warlock.submit("read sheet");
        assert!(warlock.find_thing("drawer").is_some());

        let sage = BacklogGame::new(ClassType::MindSage, false);
        assert!(
            sage.find_thing("drawer").is_some(),
            "Pattern Recognition reads past the gate"
        );
    }

    #[test]
    fn the_meeting_only_ends_for_a_statement() {
        let prefix: Vec<&str> = WALKTHROUGH[..8].to_vec();
        let mut game = play(ClassType::TaskPaladin, &prefix);
        assert_eq!(game.room_index(), 3, "should be standing in the meeting");

        game.submit("say any other questions");
        assert_eq!(game.room_index(), 3, "a question is how it eats");
        assert!(game.attention() > 0);

        game.submit("say i have a hard stop");
        assert_eq!(game.room_index(), 4);
    }

    #[test]
    fn the_last_door_refuses_every_postponement() {
        let game = play(ClassType::ArchAccountant, WALKTHROUGH);
        assert_eq!(game.status(), BacklogStatus::Escaped);

        for excuse in ["tomorrow", "later", "soon", "someday", "I don't know"] {
            let mut attempt = play(
                ClassType::ArchAccountant,
                &WALKTHROUGH[..WALKTHROUGH.len() - 1],
            );
            assert_eq!(attempt.room_index(), ROOM_COUNT - 1);
            attempt.submit(&format!("say {excuse}"));
            assert_eq!(
                attempt.status(),
                BacklogStatus::Running,
                "\"{excuse}\" is what the door is named after"
            );
        }
    }

    #[test]
    fn attune_spends_a_charge_and_accountants_carry_a_spare() {
        let mut warlock = BacklogGame::new(ClassType::CodeWarlock, false);
        assert_eq!(warlock.charges, BASE_ATTUNE_CHARGES);
        warlock.submit("attune");
        assert_eq!(warlock.charges, BASE_ATTUNE_CHARGES - 1);
        assert!(
            warlock
                .transcript
                .iter()
                .any(|(tone, _)| *tone == Tone::Insight)
        );

        let accountant = BacklogGame::new(ClassType::ArchAccountant, false);
        assert_eq!(accountant.charges, BASE_ATTUNE_CHARGES + 1);
    }

    #[test]
    fn every_gate_flag_is_reachable_within_its_own_room() {
        for room in ROOMS.iter() {
            let raised: HashSet<&str> = room
                .things
                .iter()
                .filter_map(|thing| match &thing.action {
                    Action::Reveals { flag, .. } => Some(*flag),
                    Action::NeedsItem { flag, .. } => Some(*flag),
                    _ => None,
                })
                .collect();
            for thing in room.things {
                if let Some(gate) = thing.hidden_until {
                    assert!(
                        raised.contains(gate),
                        "{}: nothing in the room raises \"{gate}\"",
                        room.id
                    );
                }
            }
            if let Lock::Flag(flag) = &room.lock {
                assert!(
                    raised.contains(flag),
                    "{}: the way out wants \"{flag}\" and nothing raises it",
                    room.id
                );
            }
        }
    }

    #[test]
    fn leaving_takes_two_asks() {
        let mut game = BacklogGame::new(ClassType::MindSage, false);
        game.submit("leave");
        assert_eq!(game.status(), BacklogStatus::Running);
        game.submit("leave");
        assert_eq!(game.status(), BacklogStatus::Left);
    }

    #[test]
    fn input_history_recalls_previous_lines() {
        let mut game = BacklogGame::new(ClassType::MindSage, false);
        for line in ["look", "examine desk"] {
            game.input = line.to_string();
            game.handle_key(KeyEvent::from(KeyCode::Enter));
        }
        game.handle_key(KeyEvent::from(KeyCode::Up));
        assert_eq!(game.input, "examine desk");
        game.handle_key(KeyEvent::from(KeyCode::Up));
        assert_eq!(game.input, "look");
        game.handle_key(KeyEvent::from(KeyCode::Down));
        assert_eq!(game.input, "examine desk");
    }

    fn render(class: ClassType, width: u16, height: u16, script: &[&str]) -> Terminal<TestBackend> {
        let game = play(class, script);
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| draw_backlog(frame, &game, "Vaelan", 12))
            .unwrap();
        terminal
    }

    #[test]
    fn the_backlog_renders_in_normal_and_narrow_terminals() {
        for (width, height) in [(120u16, 40u16), (90, 28), (80, 24), (72, 22), (40, 12)] {
            render(ClassType::SystemsArchitect, width, height, &["look"]);
        }
    }

    #[test]
    fn the_ending_overlay_renders_after_escaping() {
        let terminal = render(ClassType::MindSage, 90, 28, WALKTHROUGH);
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("DAYLIGHT"));
    }

    #[test]
    fn the_prompt_caret_uses_the_active_class_color() {
        for class in CLASSES {
            let terminal = render(class, 90, 28, &["look"]);
            let buffer = terminal.backend().buffer();
            let caret = buffer
                .content
                .iter()
                .find(|cell| cell.symbol() == "▌")
                .unwrap_or_else(|| panic!("no prompt caret drawn for {}", class.name()));
            assert_eq!(caret.fg, Theme::for_class(class).primary);
        }
    }

    #[test]
    fn the_warlock_sees_a_debug_line_and_other_orders_do_not() {
        let warlock = render(ClassType::CodeWarlock, 120, 40, &["look"]);
        let sage = render(ClassType::MindSage, 120, 40, &["look"]);
        let text = |terminal: &Terminal<TestBackend>| -> String {
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .map(|cell| cell.symbol())
                .collect()
        };
        assert!(text(&warlock).contains("exit.lock == OBJECT"));
        assert!(!text(&sage).contains("exit.lock"));
    }

    #[test]
    fn the_help_overlay_draws_over_the_transcript() {
        let mut game = play(ClassType::TimeChronomancer, &["look"]);
        game.submit("help");
        assert!(game.show_help);
        let mut terminal = Terminal::new(TestBackend::new(90, 28)).unwrap();
        terminal
            .draw(|frame| draw_backlog(frame, &game, "Vaelan", 12))
            .unwrap();
        let rendered: String = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect();
        assert!(rendered.contains("WHAT THE BACKLOG UNDERSTANDS"));

        // Esc closes it and hands the room back.
        game.handle_key(KeyEvent::from(KeyCode::Esc));
        assert!(!game.show_help);
    }
}

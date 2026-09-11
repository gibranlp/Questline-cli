// ─────────────────────────────────────────────────────────────────────────────
// backlog_rooms.rs — puro contenido. Aquí vive toda la escritura del easter egg
// del Backlog. Cero lógica: el motor (backlog_game.rs) interpreta estos datos.
// ─────────────────────────────────────────────────────────────────────────────

use crate::models::ClassType;

/// How a fixture answers when the player does something to it beyond looking.
pub enum Action {
    /// Nothing changes. The text is the answer.
    Inert(&'static str),
    /// Puts an item in the satchel. Idempotent — taking it twice is not an error.
    Grants {
        item: &'static str,
        text: &'static str,
        repeat: &'static str,
    },
    /// Raises a flag, which may unveil fixtures gated on it.
    Reveals {
        flag: &'static str,
        text: &'static str,
        repeat: &'static str,
    },
    /// Wants a carried item. `use <item> on <this>` raises the flag.
    NeedsItem {
        item: &'static str,
        flag: &'static str,
        text: &'static str,
        refusal: &'static str,
    },
}

/// A fixture in a room. Nouns are matched lowercase against `noun` and `aliases`.
pub struct Thing {
    pub noun: &'static str,
    pub aliases: &'static [&'static str],
    /// One clause appended to the room description while the fixture is visible.
    pub glance: &'static str,
    pub examine: &'static str,
    /// The fixture stays unaddressable until this flag is raised. Mind Sages
    /// ignore this gate entirely — that is their whole gift.
    pub hidden_until: Option<&'static str>,
    pub action: Action,
}

/// What the way out wants.
pub enum Lock {
    /// A spoken answer. `accepts` are normalized substring matches; `rejects`
    /// are the wrong-but-anticipated answers that earn their own reply.
    Phrase {
        accepts: &'static [&'static str],
        rejects: &'static [(&'static str, &'static str)],
    },
    /// Anything in the satchel.
    Item(&'static str),
    /// Something you had to notice first.
    Flag(&'static str),
    /// Any honest answer that is not a postponement.
    AnyAnswer {
        rejects: &'static [(&'static str, &'static str)],
    },
}

impl Lock {
    /// Coarse shape of the lock, surfaced by the Systems Architect passive and
    /// by the Code Warlock's debug line.
    pub fn shape(&self) -> &'static str {
        match self {
            Lock::Phrase { .. } => "phrase",
            Lock::Item(_) => "object",
            Lock::Flag(_) => "knowledge",
            Lock::AnyAnswer { .. } => "confession",
        }
    }

    /// True when the way out is opened by speaking rather than by carrying.
    pub fn wants_words(&self) -> bool {
        matches!(self, Lock::Phrase { .. } | Lock::AnyAnswer { .. })
    }
}

pub struct Room {
    pub id: &'static str,
    pub title: &'static str,
    /// Indexed by attention tier: settled, uneasy, wrong.
    pub ambience: [&'static str; 3],
    pub intro: &'static str,
    pub things: &'static [Thing],
    /// The noun that a bare `listen` resolves to, when the room has one.
    pub listen: Option<&'static str>,
    pub lock: Lock,
    /// What the way out says when it refuses.
    pub locked_text: &'static str,
    /// What happens when it stops refusing.
    pub exit_text: &'static str,
    /// One line per class, in `ClassType` declaration order.
    pub insights: [&'static str; 6],
}

/// Index into the six-entry class tables. Declaration order in `ClassType`.
pub fn class_index(class: ClassType) -> usize {
    match class {
        ClassType::CodeWarlock => 0,
        ClassType::TaskPaladin => 1,
        ClassType::MindSage => 2,
        ClassType::SystemsArchitect => 3,
        ClassType::TimeChronomancer => 4,
        ClassType::ArchAccountant => 5,
    }
}

pub const ROOM_COUNT: usize = 7;

// ─────────────────────────────────────────────────────────────────────────────
// 1 — Reception That Never Opened
// ─────────────────────────────────────────────────────────────────────────────

const RECEPTION: Room = Room {
    id: "reception",
    title: "Reception That Never Opened",
    ambience: [
        "The fluorescent light holds one steady note.",
        "The light has begun to consider flickering.",
        "The light is not flickering. It is deciding about you.",
    ],
    intro: "You are behind the front desk of somewhere that never had a front. \
The carpet is the colour of a decision nobody made. There is no entrance. \
There is a way further in.",
    things: &[
        Thing {
            noun: "desk",
            aliases: &["counter", "reception"],
            glance: "a reception desk",
            examine: "Laminate, worn through in one small oval where a wrist rested for years. \
Nobody was ever employed here. The wrist is well documented.",
            hidden_until: None,
            action: Action::Inert(
                "The desk has no opinion. It has been receiving nobody for a long time and has \
grown comfortable in the work.",
            ),
        },
        Thing {
            noun: "sheet",
            aliases: &[
                "sign-in",
                "signin",
                "sign in sheet",
                "log",
                "register",
                "clipboard",
            ],
            glance: "a sign-in sheet",
            examine: "Four hundred entries. All the same handwriting. All of them yours. \
The times are plausible. The dates are not. The last line is blank and dated today, and the \
pen beside it is warm.",
            hidden_until: None,
            action: Action::Reveals {
                flag: "sheet_read",
                text: "You turn the sheet over. Taped to the back is a note in the same hand: \
\"They keep the badges in the drawer. There is always a drawer.\"",
                repeat: "You have already read both sides. Neither has improved.",
            },
        },
        Thing {
            noun: "drawer",
            aliases: &["desk drawer"],
            glance: "a drawer under the desk, slightly open",
            examine: "It was always slightly open. You are fairly sure it was always slightly open.",
            hidden_until: Some("sheet_read"),
            action: Action::Grants {
                item: "visitor badge",
                text: "Inside: one visitor badge on a lanyard. The name field is filled in. \
It is your name. It was printed at some point, by someone, for a visit you have not made yet.",
                repeat: "The drawer holds nothing else, and has stopped pretending it might.",
            },
        },
        Thing {
            noun: "bell",
            aliases: &["service bell"],
            glance: "a service bell",
            examine: "Brass. Polished by use. Struck often, and recently.",
            hidden_until: None,
            action: Action::Inert(
                "You ring it. Nothing comes. Something acknowledges. Those are not the same, \
and the difference is the whole reason this floor exists.",
            ),
        },
        Thing {
            noun: "door",
            aliases: &["exit", "reader", "card reader", "way", "way in"],
            glance: "a door with a card reader",
            examine: "A grey door with a grey reader beside it, and a small light that is neither \
red nor green but the colour of a form still being processed.",
            hidden_until: None,
            action: Action::Inert(
                "The reader waits. It has waited through every renovation this floor never had.",
            ),
        },
    ],
    listen: None,
    lock: Lock::Item("visitor badge"),
    locked_text: "The reader considers you and finds no record. The light stays the colour of \
a form still being processed.",
    exit_text: "The badge passes the reader. The light does not turn green. It turns off, which \
is somehow worse, and the door opens onto a corridor.",
    insights: [
        "Code Warlock: every system with a reader has a drawer of spare credentials, and every \
drawer is unlocked, because the alternative was a ticket nobody wanted to file.",
        "Task Paladin: you are standing in a room with an unchecked box in it. The sheet has a \
blank line. Blank lines are not restful.",
        "Mind Sage: four hundred entries in one hand is not a log. It is a loop, and loops are \
read from the back.",
        "Systems Architect: the desk was built before the door. Whatever opens the door was \
stored in the desk, because that is where the budget put it.",
        "Time Chronomancer: the pen is warm. Someone signed in recently and it was not recently. \
Check what the last entry was waiting for.",
        "Arch Accountant: four hundred arrivals, zero departures. The ledger does not balance, \
and the missing column is on the reverse of the page.",
    ],
};

// ─────────────────────────────────────────────────────────────────────────────
// 2 — The Corridor of Almost
// ─────────────────────────────────────────────────────────────────────────────

const CORRIDOR: Room = Room {
    id: "corridor",
    title: "The Corridor of Almost",
    ambience: [
        "The corridor continues in both directions, politely.",
        "The corridor is longer behind you than it was in front of you.",
        "The corridor has stopped bothering with the pretence of length.",
    ],
    intro: "No puzzle here. No lock, no code, no clever thing to carry. Just a corridor \
the colour of a waiting room, lit from a source you cannot find, going on for a distance \
that keeps declining to be measured. This is what most of the Backlog is. \
The rooms with puzzles in them are the exception, and they are the mercy.",
    things: &[
        Thing {
            noun: "carpet",
            aliases: &["floor", "ground"],
            glance: "carpet, damp, patterned in a way that stops when you look directly at it",
            examine: "Commercial grade. Damp without any water. The pattern repeats every eleven \
paces, except for one tile near the middle where it repeats every ten, and you will notice this \
for the rest of your life.",
            hidden_until: None,
            action: Action::Inert("It gives slightly. It has given slightly for years."),
        },
        Thing {
            noun: "doors",
            aliases: &["door", "doorways", "frames"],
            glance: "doors along both walls",
            examine: "Every eight paces, a door. None of them open. None of them are locked \
either. They simply are not the kind of thing that opens. They were installed by someone \
who understood that a corridor needs doors to be a corridor, and stopped there.",
            hidden_until: None,
            action: Action::Inert(
                "The handle turns all the way and accomplishes nothing, which is the most \
honest thing this floor has done so far.",
            ),
        },
        Thing {
            noun: "lights",
            aliases: &["light", "ceiling", "fluorescents", "lamp"],
            glance: "light with no fixture",
            examine: "There is no fixture. There is no window. There is light. Three facts, \
arranged in the order you would like least.",
            hidden_until: None,
            action: Action::Inert("There is nothing to switch. It is not that kind of light."),
        },
        Thing {
            noun: "hum",
            aliases: &["sound", "noise", "buzz", "silence"],
            glance: "a hum you stopped hearing a while ago",
            examine: "You stopped hearing it about a minute in. It has not stopped. \
You have simply been carrying it.",
            hidden_until: None,
            action: Action::Reveals {
                flag: "listened",
                text: "You stop, and listen properly, for the first time since you got here.\n\n\
It is not machinery. It is the sound of a great many things being deferred at once, \
very quietly, in a very large room.\n\nUnderneath it, further along, one of the deferrals \
is louder than the others. You have a direction now. It is not a good one, but it is a direction.",
                repeat: "You listen again. The same enormous patience. It has not moved.",
            },
        },
    ],
    listen: Some("hum"),
    lock: Lock::Flag("listened"),
    locked_text: "You walk. The corridor accepts the walking and returns you nothing. \
Distance is not the currency here.",
    exit_text: "You follow the loud deferral until the carpet gives way to tile, \
and the corridor, having made its point, stops.",
    insights: [
        "Code Warlock: you are in a loop with no exit condition. The condition is not a place. \
Stop stepping and start sampling.",
        "Task Paladin: there is nothing here to complete. That is the trial. \
Some corridors are crossed by paying attention rather than by finishing something.",
        "Mind Sage: you have been discarding a signal since you arrived because it was constant. \
Constant is not the same as empty.",
        "Systems Architect: no fixture, no source, no end. A corridor with no load-bearing purpose \
is not architecture. It is a delay, and delays are made of sound.",
        "Time Chronomancer: you have spent longer here than you noticed, which means something \
has been running the whole time. Find what has been running.",
        "Arch Accountant: nothing in this corridor is an asset. Audit the one thing you have been \
receiving continuously and recording nowhere.",
    ],
};

// ─────────────────────────────────────────────────────────────────────────────
// 3 — The Ledger That Would Not Close
// ─────────────────────────────────────────────────────────────────────────────

const LEDGER: Room = Room {
    id: "ledger",
    title: "The Ledger That Would Not Close",
    ambience: [
        "A lamp throws a clean circle onto an open book.",
        "The lamp has narrowed. The circle is smaller than it was.",
        "The lamp lights the book and nothing else. There is nothing else.",
    ],
    intro: "A small office, if an office can be small and have no walls you can reach. \
A desk under one lamp. On the desk, a ledger, open, weighted flat by a brass rule. \
On the far side, a door with three dials.",
    things: &[
        Thing {
            noun: "ledger",
            aliases: &["book", "accounts", "entries"],
            glance: "a ledger, open",
            examine: "Columns in a careful hand. Everything balances. Everything balances \
beautifully, for eight hundred pages, except for three lines that were never closed:\n\n\
  Line seven      — one hour, borrowed from a Tuesday, not returned.\n\
  Line four       — one apology, drafted, never sent.\n\
  Line one        — one promise, made to yourself, in private, with feeling.\n\n\
Someone has written in the margin, pressing hard enough to tear: \
\"THE DIALS WANT THEM IN THE ORDER THEY REFUSED TO CLOSE.\"",
            hidden_until: None,
            action: Action::Reveals {
                flag: "ledger_read",
                text: "You lift the brass rule. Under it, a receipt, folded twice.",
                repeat: "The three open lines remain open. They are very patient about it.",
            },
        },
        Thing {
            noun: "receipt",
            aliases: &["slip", "paper", "note"],
            glance: "a receipt, folded twice",
            examine: "A receipt for a single coffee, paid for in a currency the Realm stopped \
minting, at 11:40 on a Tuesday. On the back, in the same careful hand:\n\n\
\"I balanced everything except the three that were mine. Speak them and the door opens. \
It does not care whether you meant them. It cares that you say them in order.\"",
            hidden_until: Some("ledger_read"),
            action: Action::Inert(
                "You put the receipt in your pocket. It will not help. It was never going to help. \
You keep it anyway, which the ledger notes without comment.",
            ),
        },
        Thing {
            noun: "dials",
            aliases: &["dial", "door", "lock", "brass dials", "exit"],
            glance: "a door with three brass dials",
            examine: "Three dials, each numbered zero through nine, each currently on zero. \
No handle. Below them, engraved small: SPEAK THEM.",
            hidden_until: None,
            action: Action::Inert("The dials do not turn by hand. They turn when they are told."),
        },
        Thing {
            noun: "lamp",
            aliases: &["light"],
            glance: "one lamp",
            examine: "A green glass banker's lamp. Warm. On a circuit that goes somewhere \
you would rather not follow.",
            hidden_until: None,
            action: Action::Inert(
                "You reach for the switch and reconsider. The circle of light is the room. \
Ending the circle would be a statement about the room.",
            ),
        },
        Thing {
            noun: "chair",
            aliases: &["seat"],
            glance: "a chair pushed back from the desk",
            examine: "Pushed back at the angle of someone who stood up quickly, meaning to \
return in a minute. The cushion has recovered its shape completely, which tells you \
how long the minute has been.",
            hidden_until: None,
            action: Action::Inert("You do not sit down. Good."),
        },
    ],
    listen: None,
    lock: Lock::Phrase {
        accepts: &["741", "7 4 1", "seven four one"],
        rejects: &[
            (
                "147",
                "The dials turn, consider, and return to zero. Ascending order is tidy. \
The ledger did not fail tidily. It failed in the order it failed.",
            ),
            (
                "000",
                "Zero, zero, zero. The dials do not move. Nothing is not a number here; \
nothing is the state they are already in.",
            ),
            (
                "balanced",
                "The word lands on the desk and sits there. Eight hundred pages balanced. \
That was never the problem.",
            ),
        ],
    },
    locked_text: "The dials hold at zero. They are not being difficult. They are being specific.",
    exit_text: "Seven. Four. One. Each dial turns itself, unhurried, like something being \
finally allowed to. The book closes on its own. The door does not open so much as admit \
that it was a door.",
    insights: [
        "Code Warlock: the margin note is the spec. Three values, ordered by failure time, \
not by magnitude. Read the spec before writing the input.",
        "Task Paladin: three things were left unchecked. The door wants them named, \
in the order they went unchecked, out loud, which is the least anyone has asked of you today.",
        "Mind Sage: eight hundred balanced pages are noise. Three unclosed lines are the signal. \
Their line numbers are the code.",
        "Systems Architect: three dials, three unclosed entries. The mapping is not clever. \
Preserve the order the source gave you and do not sort it.",
        "Time Chronomancer: an hour borrowed from a Tuesday and never returned. \
That debt is line seven, and line seven goes first, because it went wrong first.",
        "Arch Accountant: you know this book. You have kept this book. The unclosed lines are \
seven, four, and one, and you already knew that before you asked.",
    ],
};

// ─────────────────────────────────────────────────────────────────────────────
// 4 — The Meeting That Has Not Ended
// ─────────────────────────────────────────────────────────────────────────────

const MEETING: Room = Room {
    id: "meeting",
    title: "The Meeting That Has Not Ended",
    ambience: [
        "Someone is sharing their screen. It has been someone's screen for some time.",
        "The meeting has expanded to fill the time available, and then some more.",
        "Two new attendees have joined. Nobody invited them. Nobody will mention it.",
    ],
    intro: "A conference room. Eleven people around a table built for eight. \
They turn to you with the warm, patient, entirely sincere faces of Meeting Mimics, \
and one of them gestures at a free chair with real kindness. \
On the whiteboard, an agenda. At the end of the agenda, one item, circled.",
    things: &[
        Thing {
            noun: "attendees",
            aliases: &[
                "mimics",
                "people",
                "meeting mimics",
                "everyone",
                "them",
                "attendee",
            ],
            glance: "eleven attendees",
            examine: "They are delighted you could make it. They have been delighted for a \
duration that does not appear on any calendar. When you look directly at one, it nods \
along with something nobody said, because a Meeting Mimic's only defence is agreement, \
and its only appetite is your afternoon.",
            hidden_until: None,
            action: Action::Inert(
                "You cannot fight them. Nobody has ever fought a meeting. \
People have only ever left one.",
            ),
        },
        Thing {
            noun: "agenda",
            aliases: &["whiteboard", "board", "list", "items"],
            glance: "an agenda on the whiteboard",
            examine: "  1. Quick sync\n  2. Align on alignment\n  3. Circle back re: item 2\n  \
4. Parking lot\n  5. (circled, twice) Any other questions?\n\n\
Item five is circled twice because item five is not an item. It is a mouth.",
            hidden_until: None,
            action: Action::Reveals {
                flag: "agenda_read",
                text: "Under the marker tray, someone has written very small, at the height \
of a person who was crouching and did not want to be seen crouching:\n\n\
\"It ends for exactly one sentence. Not a question. Never a question. \
A question is how it eats.\"",
                repeat: "Item five is still circled. It has been circled harder.",
            },
        },
        Thing {
            noun: "clock",
            aliases: &["wall clock", "time"],
            glance: "a wall clock",
            examine: "It reads four minutes past the hour. It has read four minutes past the \
hour since you sat down, which you did not do. It will read four minutes past the hour \
for as long as anyone is still adding to the parking lot.",
            hidden_until: None,
            action: Action::Inert(
                "The hands do not move when you watch them. This is not supernatural. \
This is every meeting.",
            ),
        },
        Thing {
            noun: "chair",
            aliases: &["free chair", "empty chair", "seat"],
            glance: "one free chair, pulled out for you",
            examine: "Pulled out at a welcoming angle. Warm, already. There is a notebook \
in front of it, open to a fresh page, and a pen laid across the page at a diagonal, \
and if you sit down you will be here for the rest of the Realm.",
            hidden_until: None,
            action: Action::Inert(
                "You do not sit. Somewhere in the room, very slightly, the temperature drops.",
            ),
        },
        Thing {
            noun: "door",
            aliases: &["exit", "way out"],
            glance: "a door behind the projector screen",
            examine: "There is a door. Everyone knows there is a door. \
Nobody is going to be the one who mentions the door.",
            hidden_until: None,
            action: Action::Inert(
                "You put a hand on the handle. Eleven faces turn, still warm, still patient, \
still waiting for you to have one more thought before you go.",
            ),
        },
    ],
    listen: None,
    lock: Lock::Phrase {
        accepts: &[
            "this could have been an email",
            "this should have been an email",
            "could have been an email",
            "should have been an email",
            "i have a hard stop",
            "i have a hard stop at the hour",
            "hard stop",
            "lets take this offline",
            "let us take this offline",
            "take this offline",
        ],
        rejects: &[
            (
                "any other questions",
                "You say it. Eleven hands go up. The clock does not move. \
Item five was never an item.",
            ),
            (
                "one more thing",
                "The room brightens. Someone opens a laptop. \
You have just extended the meeting by an amount nobody will measure.",
            ),
            (
                "lets circle back",
                "Delighted agreement, all round. A new meeting is created. \
You are in both of them now.",
            ),
            (
                "let us circle back",
                "Delighted agreement, all round. A new meeting is created. \
You are in both of them now.",
            ),
            (
                "thank you",
                "They thank you back, warmly, at length, one at a time, going clockwise.",
            ),
            (
                "goodbye",
                "\"Before you go —\" says the nearest one, and means it kindly, \
and the clock stays at four minutes past.",
            ),
        ],
    },
    locked_text: "The handle turns and the door does not. Eleven people wait, generously, \
for you to say something that has an end on it.",
    exit_text: "You say it. Flat, unapologetic, load-bearing.\n\n\
The Mimics do not object. They cannot. It is not a question, and a thing that lives on \
questions has no purchase on a statement. Two of them nod, and mean it, and look relieved.\n\n\
The meeting ends. Somewhere above you, a calendar frees forty minutes it had no right to.",
    insights: [
        "Code Warlock: every reply that ends in a question mark recurses. Return a value, \
not another call.",
        "Task Paladin: you end a meeting the way you end anything. You state that it is done \
and you do not soften it into a question.",
        "Mind Sage: item five is circled twice, and the note under the marker tray says why. \
Read the tray.",
        "Systems Architect: this room has no termination condition. Supply one. \
A declarative sentence terminates. An interrogative does not.",
        "Time Chronomancer: the clock stopped at four past. Meetings do not end at a time; \
they end when someone declares a boundary. Declare yours out loud.",
        "Arch Accountant: this meeting has cost eleven salaries times an unbounded duration. \
Say the true sentence about its format and the cost stops accruing.",
    ],
};

// ─────────────────────────────────────────────────────────────────────────────
// 5 — The Room With Your Handwriting
// ─────────────────────────────────────────────────────────────────────────────

const HANDWRITING: Room = Room {
    id: "handwriting",
    title: "The Room With Your Handwriting",
    ambience: [
        "A small room. Paper on every surface. All of it yours.",
        "There is more paper than there was. You have not seen any of it arrive.",
        "The handwriting on the newest page is steadier than yours has ever been.",
    ],
    intro: "Domestic, almost. A desk, a chair that fits you, a lamp at the angle you like. \
Every flat surface is covered in notes, and every note is in your handwriting, \
and none of them are notes you remember writing. \
Three of them are face up.",
    things: &[
        Thing {
            noun: "note",
            aliases: &["first note", "one", "first"],
            glance: "a note, face up",
            examine: "The paper is old enough to have softened at the folds.",
            hidden_until: None,
            action: Action::Reveals {
                flag: "note_two",
                text: "\"I can absolutely handle that tomorrow.\"\n\n\
                       — Past You, and the date is a long time ago\n\n\
You set it down. Underneath it, a second note you are certain was not there.",
                repeat: "Past You has nothing to add. Past You never does.",
            },
        },
        Thing {
            noun: "second note",
            aliases: &["second", "two", "next note"],
            glance: "a second note, underneath the first",
            examine: "Newer paper. The same hand, pressed harder.",
            hidden_until: Some("note_two"),
            action: Action::Reveals {
                flag: "note_three",
                text: "\"I inherited this disaster.\"\n\n\
                       — Future You, and the date has not happened\n\n\
There is a third note. It is blank. There is a pen beside it.",
                repeat: "Future You has said their piece and is not interested in repeating it.",
            },
        },
        Thing {
            noun: "third note",
            aliases: &["third", "three", "blank note", "blank", "pen"],
            glance: "a third note, blank, with a pen beside it",
            examine: "Blank. Waiting. The pen is the one you actually like, \
which is a detail the Backlog did not have to get right and got right anyway.",
            hidden_until: Some("note_three"),
            action: Action::Reveals {
                flag: "note_read",
                text: "You pick up the pen to write something to whoever comes next, \
and find you have already written it, in the middle of the page, \
in handwriting that is yours today:\n\n\
\"Neither of them is coming. It has always been this one.\"\n\n\
                       — Present You\n\n\
The room, having been understood, loses interest in you. \
The door at the back, which you would swear was a wall, is a door.",
                repeat: "Present You is standing right here and does not need to be read.",
            },
        },
        Thing {
            noun: "notes",
            aliases: &["paper", "papers", "pile", "surfaces", "the rest"],
            glance: "hundreds of other notes",
            examine: "Reminders about reminders. Lists of lists. A page that is just the word \
\"Monday\" written forty times with increasing confidence and then, at the bottom, \
once, very small, without any.",
            hidden_until: None,
            action: Action::Inert(
                "You could read them all. That is precisely the offer being made, \
and it is the offer this room has always made, and the pile is why the room is full.",
            ),
        },
        Thing {
            noun: "mirror",
            aliases: &["glass", "reflection"],
            glance: "a mirror over the desk",
            examine: "You, at the desk, reading. In the reflection the pile is smaller. \
Not empty. Smaller. Someone in there has been working on it.",
            hidden_until: None,
            action: Action::Inert(
                "You reach out. So does the reflection, a fraction early, \
like someone who has done this before and knows how it goes.",
            ),
        },
    ],
    listen: None,
    lock: Lock::Flag("note_read"),
    locked_text: "The wall at the back is a wall. It will keep being a wall until \
this room has been read to the end.",
    exit_text: "You leave the notes where they are. All of them. \
It is the first thing you have put down since you got here.",
    insights: [
        "Code Warlock: three references, and only one of them is live. \
Past and Future are both null pointers. Dereference the third.",
        "Task Paladin: two of these notes are excuses with dates on them. \
The third is blank, and blank is the only one you can actually check off.",
        "Mind Sage: the notes are ordered. Each one uncovers the next. \
Read them in sequence and stop trying to read the pile.",
        "Systems Architect: the pile is unbounded and the sequence is length three. \
Work the sequence. The pile is not a work item, it is a load test.",
        "Time Chronomancer: you are being introduced to two people who do not exist. \
The only tense with any leverage in it is the one you are standing in.",
        "Arch Accountant: Past You opened the account. Future You inherits the balance. \
Present You is the only signatory with authority, and always was.",
    ],
};

// ─────────────────────────────────────────────────────────────────────────────
// 6 — Level 0: The Open Tabs
// ─────────────────────────────────────────────────────────────────────────────

const OPEN_TABS: Room = Room {
    id: "open-tabs",
    title: "Level 0: The Open Tabs",
    ambience: [
        "The room is very large and very bright and makes no sound at all.",
        "The brightness has become a texture. You can feel it on your arms.",
        "Somewhere in here, something finished loading, and you did not see what.",
    ],
    intro: "The floor plate says LEVEL 0. This is the oldest part — the Age of Open Tabs, \
preserved exactly. It goes further than the light does. \
Ranks and ranks of them, standing open, every single one fully loaded, \
every single one finished, every single one still here.",
    things: &[
        Thing {
            noun: "tabs",
            aliases: &["tab", "ranks", "rows", "them", "windows"],
            glance: "ranks of open tabs, all fully loaded",
            examine: "Articles you were going to read. Documentation you were going to need. \
A recipe. Four versions of the same recipe. A thing someone linked you in 2019 that you \
have kept alive across two machines and one house move.\n\n\
They are all finished loading. That is what makes this Level 0. \
Nothing here is waiting on the network. Everything here is waiting on you.",
            hidden_until: None,
            action: Action::Reveals {
                flag: "counted",
                text: "You start counting and stop, because counting them is the trap, \
and because while you were counting you noticed something the stillness was hiding:\n\n\
one of them, far off, is still spinning.",
                repeat: "You are not going to count them. Good.",
            },
        },
        Thing {
            noun: "spinner",
            aliases: &[
                "spinning tab",
                "loading tab",
                "the one still loading",
                "still loading",
            ],
            glance: "one tab, far off, still spinning",
            examine: "It has been loading for eleven years. Not stalled — loading. \
The little wheel turns with total sincerity. Whatever it is fetching, it has never once \
considered giving up, and that is either the bravest thing on this floor or the saddest, \
and Level 0 has never been asked to decide.",
            hidden_until: Some("counted"),
            action: Action::Reveals {
                flag: "reader_found",
                text: "You walk to it. It takes a while. Behind it, set into the far wall \
where no wall was, is a service door with a card reader beside it — \
the same grey reader, the same processing-coloured light.",
                repeat: "It is still loading. It will be loading after you leave.",
            },
        },
        Thing {
            noun: "reader",
            aliases: &["card reader", "service door", "door", "exit", "panel"],
            glance: "a service door with a card reader",
            examine: "Identical to the one at reception. Same model, same grey, \
same small light the colour of a form still being processed. \
Whatever built the front of this place also built the back, and did not change suppliers.",
            hidden_until: Some("reader_found"),
            action: Action::NeedsItem {
                item: "visitor badge",
                flag: "level0_open",
                text: "You hold the badge to the reader. It reads the name — \
your name, printed for a visit you had not made yet, which you have now unambiguously made.\n\n\
The light goes out. The door releases.",
                refusal: "The reader wants credentials. You are, formally speaking, \
not a visitor here yet, and it is very strict about the paperwork it invented.",
            },
        },
        Thing {
            noun: "plate",
            aliases: &["floor plate", "sign", "level 0", "floor"],
            glance: "a floor plate reading LEVEL 0",
            examine: "LEVEL 0. Stamped, not printed. Below it, smaller: \
\"NOTHING HERE IS UNFINISHED. THAT IS NOT THE SAME AS DONE.\"",
            hidden_until: None,
            action: Action::Inert("The plate is fixed. The plate has always been fixed."),
        },
    ],
    listen: Some("tabs"),
    lock: Lock::Flag("level0_open"),
    locked_text: "There is no way out along the walls, and the walls are further away \
each time you look for them.",
    exit_text: "You step through, and behind you Level 0 stays exactly as it is, \
bright and enormous and entirely finished, with one small wheel still turning in it.",
    insights: [
        "Code Warlock: every one of these completed successfully and nothing consumed the result. \
Find the one request still in flight; it is the only one with a handler attached.",
        "Task Paladin: none of these are unfinished. Every one of them is undone. \
Look for the single one that is still trying.",
        "Mind Sage: the stillness is uniform, which makes any motion in it enormous. \
Stop cataloguing and look for movement.",
        "Systems Architect: this floor has no exit along its perimeter, so the exit is a service \
door, and service doors take the same credential as the front of house. \
You were issued one at reception.",
        "Time Chronomancer: eleven years of loading is not a stall, it is a commitment. \
Follow the only thing here that is still spending time.",
        "Arch Accountant: every tab here is a sunk cost held open in case it becomes an asset. \
Exactly one is still accruing. Go to it.",
    ],
};

// ─────────────────────────────────────────────────────────────────────────────
// 7 — The Door Marked TOMORROW
// ─────────────────────────────────────────────────────────────────────────────

const TOMORROW: Room = Room {
    id: "tomorrow",
    title: "The Door Marked TOMORROW",
    ambience: [
        "One room. One door. The word TOMORROW, stencilled, at eye height.",
        "The stencil has been repainted many times. You can feel the layers.",
        "The door is not hostile. It is the last polite thing in the building.",
    ],
    intro: "A small square room at the bottom of everything, and it is not frightening, \
which is the most frightening thing about it. One door. Stencilled across it at eye height, \
in letters that have been repainted many times: TOMORROW.\n\n\
Set into the door at waist height, a brass slot. Beside the slot, a card. \
Beside the card, a pen.\n\n\
And above all of it, engraved, small, and clearly added later by someone who had \
been through this door themselves:\n\n\
  NAME ONE THING YOU WILL ACTUALLY FINISH.\n\
  THE DOOR HAS HEARD THEM ALL. IT IS NOT KEEPING SCORE.\n\
  IT ONLY WANTS ONE THAT IS TRUE.",
    things: &[
        Thing {
            noun: "door",
            aliases: &["tomorrow", "stencil", "exit", "way out"],
            glance: "a door marked TOMORROW",
            examine: "Under the fresh stencil, the ghosts of older ones. \
TOMORROW, over TOMORROW, over TOMORROW, going back further than the paint should allow. \
Nobody has ever renamed this door. Several people have clearly considered it.",
            hidden_until: None,
            action: Action::Inert("No handle on this side. There was never going to be a handle."),
        },
        Thing {
            noun: "slot",
            aliases: &["brass slot", "letterbox"],
            glance: "a brass slot",
            examine: "Worn smooth at the lip by a great many cards going in. \
There is no basket on the other side. Whatever goes in is not filed. \
It is simply heard, once, and then it is over.",
            hidden_until: None,
            action: Action::Inert(
                "You put your hand near it. There is a draught, coming the wrong way, \
smelling faintly of a morning.",
            ),
        },
        Thing {
            noun: "card",
            aliases: &["blank card", "paper", "pen"],
            glance: "a blank card and a pen",
            examine: "One card. One line on it. Room for one thing. \
The card is not large and this is deliberate; the Backlog has watched people try to \
buy their way out with a list, and the door does not take lists.",
            hidden_until: None,
            action: Action::Inert(
                "You do not need to write it down. The slot is a formality. \
The door has been listening since you arrived.",
            ),
        },
        Thing {
            noun: "engraving",
            aliases: &["inscription", "instructions", "text", "words"],
            glance: "an engraving above the slot",
            examine: "NAME ONE THING YOU WILL ACTUALLY FINISH.\n\
THE DOOR HAS HEARD THEM ALL. IT IS NOT KEEPING SCORE.\n\
IT ONLY WANTS ONE THAT IS TRUE.\n\n\
Added later, in a different hand, at the very bottom, almost worn away:\n\
\"it does not check. that is not the point. i checked.\"",
            hidden_until: None,
            action: Action::Inert(
                "You read it again. It says the same thing. It was always going to.",
            ),
        },
    ],
    listen: None,
    lock: Lock::AnyAnswer {
        rejects: &[
            (
                "tomorrow",
                "The door does not move. It is named after that answer. \
It has heard that answer more than any door in the Realm, and it is the only one \
it cannot accept.",
            ),
            (
                "later",
                "The door waits. \"Later\" is just tomorrow with the confidence removed.",
            ),
            (
                "soon",
                "The door waits. \"Soon\" is a duration nobody has ever been able to produce.",
            ),
            (
                "someday",
                "The draught from the slot stops for a moment, which is the closest \
this door comes to disappointment.",
            ),
            (
                "eventually",
                "The door waits. That word has been in the paint since the beginning.",
            ),
            (
                "next week",
                "The door waits. It has an excellent memory for next weeks.",
            ),
            (
                "everything",
                "Nothing happens. The card has one line on it, and \"everything\" \
is how people end up down here.",
            ),
            (
                "nothing",
                "The door waits. It did not ask for honesty about the void. \
It asked for one thing.",
            ),
            (
                "i dont know",
                "The door waits, without judgement, for as long as this takes.",
            ),
            (
                "i don't know",
                "The door waits, without judgement, for as long as this takes.",
            ),
        ],
    },
    locked_text: "The door has no handle and does not need one. It is waiting to be told \
one true thing, and it is prepared to wait longer than you are.",
    exit_text: "You say it out loud. One thing. A small one, probably — \
the doors that open are almost always opened by small ones.\n\n\
The slot takes nothing, because there was never a card to take. \
The door opens onto a stairwell with daylight at the top of it, \
which this building has no business having.\n\n\
Behind you, the Great Backlog does not object, does not follow, and does not close. \
It never closes. It simply notes, in a hand very like your own, \
that you left under your own power and said what you were leaving to do.",
    insights: [
        "Code Warlock: there is no validator on the other side of this slot. \
The assertion is local. It always was.",
        "Task Paladin: one box. Name a box you will actually check. \
Not the important one. The one you will actually check.",
        "Mind Sage: the door is not testing your knowledge. \
It is asking for a single node you intend to reach, and it will take a small one.",
        "Systems Architect: scope it down until it is boring. \
Boring is the only scope that has ever made it through this door.",
        "Time Chronomancer: the door is named after the tense you have been hiding in. \
Answer in any tense but that one.",
        "Arch Accountant: one line item. Real, small, and payable. \
The door has never once accepted a projection.",
    ],
};

// `static`, not `const` — the engine hands out `&'static Room` and `&'static Thing`
// references, which only a static can actually back.
pub static ROOMS: [Room; ROOM_COUNT] = [
    RECEPTION,
    CORRIDOR,
    LEDGER,
    MEETING,
    HANDWRITING,
    OPEN_TABS,
    TOMORROW,
];

/// Shown when the Backlog files you.
pub const FILED_TEXT: &str = "Attention resolves on you all at once, the way a room \
notices a sound that has been going for a while.\n\n\
You are not attacked. You are processed. Hands you do not see take your details, \
your intentions, and your afternoon, and set them somewhere orderly, \
and you come back to yourself at the start of this room with the distinct sense \
of having been added to something.";

/// Shown when a Task Paladin's oath eats the first filing.
pub const PALADIN_GRACE_TEXT: &str = "Attention resolves on you all at once — \
and finds a checklist where a person should be, every line of it closed, \
nothing outstanding, nothing to file.\n\n\
The Backlog has no procedure for a completed thing. It lets go, once, \
with what an unkind observer would call reluctance.";

/// The last words on a first escape.
pub const ESCAPE_FIRST: &str = "You climb into the daylight. \
The Chronicle records the ascent and, being the Chronicle, declines to make anything of it.";

/// The last words on every escape after the first.
pub const ESCAPE_AGAIN: &str = "You climb into the daylight. \
The Backlog recognised you on the way in this time, and did not say anything, \
and let you go a little sooner. It has your handwriting on file now. \
It considers that an arrangement.";

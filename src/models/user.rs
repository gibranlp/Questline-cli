// ─────────────────────────────────────────────────────────────────────────────
// models/user.rs — el modelo del héroe y las clases disponibles con sus poderes
// ─────────────────────────────────────────────────────────────────────────────

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// Las seis clases del héroe — cada una con su rollo y sus poderes únicos
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClassType {
    CodeWarlock,
    TaskPaladin,
    MindSage,
    SystemsArchitect,
    TimeChronomancer,
    ArchAccountant,
}

impl ClassType {
    pub fn name(&self) -> &'static str {
        match self {
            ClassType::CodeWarlock => "Code Warlock",
            ClassType::TaskPaladin => "Task Paladin",
            ClassType::MindSage => "Mind Sage",
            ClassType::SystemsArchitect => "Systems Architect",
            ClassType::TimeChronomancer => "Time Chronomancer",
            ClassType::ArchAccountant => "Arch Accountant",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ClassType::CodeWarlock => "Summoner of scripts, breaker of production.",
            ClassType::TaskPaladin => "Holy warrior against procrastination.",
            ClassType::MindSage => "Cartographer of thoughts and master of knowledge trees.",
            ClassType::SystemsArchitect => "Builder of order from chaos.",
            ClassType::TimeChronomancer => "Manipulator of hours, minutes, and deadlines.",
            ClassType::ArchAccountant => "Master of ledgers, destroyer of bad financial decisions.",
        }
    }

    pub fn flavor(&self) -> &'static str {
        match self {
            ClassType::CodeWarlock => "Caffeine in. Code out. The loop must continue.",
            ClassType::TaskPaladin => "The to-do list shall be purified.",
            ClassType::MindSage => "Every idea is a node. Every node is power.",
            ClassType::SystemsArchitect => {
                "Give me enough folders and I will organize the universe."
            }
            ClassType::TimeChronomancer => "Time is not money. Time is everything.",
            ClassType::ArchAccountant => {
                "Numbers do not lie. But accountants can make them confess."
            }
        }
    }

    // Pues aquí parseamos el string a la clase correspondiente — acepta tanto el nombre bonito como el slug
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "Code Warlock" | "code-warlock" => Some(ClassType::CodeWarlock),
            "Task Paladin" | "task-paladin" => Some(ClassType::TaskPaladin),
            "Mind Sage" | "mind-sage" => Some(ClassType::MindSage),
            "Systems Architect" | "systems-architect" => Some(ClassType::SystemsArchitect),
            "Time Chronomancer" | "time-chronomancer" => Some(ClassType::TimeChronomancer),
            "Arch Accountant" | "arch-accountant" => Some(ClassType::ArchAccountant),
            _ => None,
        }
    }

    // Aquí están todos los poderes que se desbloquean por nivel — uno cada 5 niveles hasta el 100.
    // Qué rollo escribir esto, pero queda épico cuando el héroe sube de nivel.
    pub fn powers(&self) -> Vec<(i32, &'static str, &'static str)> {
        match self {
            ClassType::ArchAccountant => vec![
                (1, "Receipt Awareness", "You begin noticing where every coin goes. Nothing escapes your first ledger."),
                (5, "Spreadsheet Initiate", "Bend basic formulas to your will. SUM() shall never betray you again."),
                (10, "Expense Sense", "You smell wasteful spending before the transaction even clears."),
                (15, "Budget Barrier", "An invisible shield that deflects impulse purchases and lifestyle creep."),
                (20, "Ledger Vision", "See every transaction's true nature and intention at a glance."),
                (25, "Audit Whisperer", "Your presence alone causes expense reports to quietly self-correct."),
                (30, "Decimal Precision", "Rounding errors fear you. Fractions bow. Decimals align on command."),
                (35, "Financial Divination", "Predict budget shortfalls 30 days before anyone else senses them."),
                (40, "Compound Interest Ritual", "Turn idle gold into growing gold through ancient compounding rites."),
                (45, "Budget Necromancy", "Resurrect dead budgets and breathe life back into failed spending plans."),
                (50, "The Golden Balance", "Assets and liabilities align themselves perfectly in your presence."),
                (55, "Ledger Mastery", "No spreadsheet is too large, no formula too tangled to unravel."),
                (60, "Audit Immunity", "Your records are so immaculate they cannot be questioned. Ever."),
                (65, "Expense Telepathy", "You know how every team member spent money before they tell you."),
                (70, "Financial Time Travel", "Read past spending patterns and see the financial crises still forming."),
                (75, "Portfolio Alchemy", "Turn underperforming assets into gold through sheer accounting will."),
                (80, "Accountant's Clairvoyance", "The next fiscal quarter's numbers reveal themselves to you in visions."),
                (85, "Infinite Spreadsheet", "Your ledgers extend beyond the limits of mortal memory and storage."),
                (90, "Economic Singularity", "All financial systems within your domain converge toward perfect efficiency."),
                (95, "The Final Audit", "One glance from you and every discrepancy in existence resolves itself."),
                (100, "Omniscient Ledger Lord", "You see all money, everywhere, at all times. Nothing escapes the ledger."),
            ],
            ClassType::CodeWarlock => vec![
                (1, "Terminal Spark", "The first command is cast. A blinking cursor becomes a gateway to infinite possibilities."),
                (5, "Hello World Ritual", "The ancient incantation. Print one line of text and the power begins."),
                (10, "Debug Familiar", "A spectral creature that sniffs out bugs and barks at undefined variables."),
                (15, "Stack Overflow Telepathy", "Sense the exact search query needed before you even open a browser tab."),
                (20, "Infinite Tabs Curse", "You have accepted your fate. 47 tabs open and somehow still productive."),
                (25, "Regex Conjuration", "Summon cryptic pattern-matching sigils that only you can later decipher."),
                (30, "Git Resurrection", "Raise deleted branches from the dead using arcane commit archaeology."),
                (35, "Production Panic Resistance", "Heart rate stays level when the deployment alert fires at 3am."),
                (40, "API Whispering", "Third-party endpoints reveal their undocumented behaviors to you alone."),
                (45, "Code Refactoring Storm", "Transform tangled legacy code into clean architecture in a single session."),
                (50, "Dark Automation Magic", "Scripts run at night while you sleep, doing your bidding without complaint."),
                (55, "Infinite CLI Power", "The terminal obeys your every keystroke without delay or question."),
                (60, "Daemon Summoning", "Summon background processes that serve you faithfully and silently forever."),
                (65, "Deployment Clairvoyance", "See exactly which line of code will break production before it happens."),
                (70, "Segmentation Fault Immunity", "Memory errors and null pointers bounce off you without leaving a mark."),
                (75, "Distributed System Sorcery", "Microservices align themselves at your command across all distributed nodes."),
                (80, "Recursive Enlightenment", "You understand recursion because you understand recursion."),
                (85, "Kernel Manipulation", "You speak directly to the operating system in its most primitive language."),
                (90, "Reality Compiles Successfully", "Your code runs on the first try. Every time. Without exception."),
                (95, "The Source Code of Existence", "You have glimpsed the underlying patterns that make all computation run."),
                (100, "Architect of Simulations", "You no longer use tools. You build the tools that build the tools."),
            ],
            ClassType::MindSage => vec![
                (1, "First Node", "The first node of thought is anchored, establishing your repository of mind."),
                (5, "Thought Mapping", "Lay your thoughts in spatial form and watch hidden connections emerge."),
                (10, "Idea Chain Lightning", "One idea instantly spawns three more. Your mind never stops branching."),
                (15, "Concept Linking", "Discover relationships between ideas that no one else has yet noticed."),
                (20, "Cognitive Compression", "Compress complex topics into elegant mental models you recall instantly."),
                (25, "Mental Indexing", "Every thought you have ever had is catalogued, searchable, retrievable."),
                (30, "Memory Sharding", "Distribute knowledge across multiple systems so nothing is ever truly lost."),
                (35, "Insight Flash", "Sudden clarity strikes at unexpected moments, usually during a walk."),
                (40, "Pattern Detection", "Detect recurring patterns in data, behavior, and thought before others see them."),
                (45, "Idea Teleportation", "Jump instantly between distant concepts and return carrying new insights."),
                (50, "Knowledge Trees", "Your notes form living, growing trees of interconnected, searchable wisdom."),
                (55, "Parallel Thinking", "Hold multiple conflicting ideas in mind simultaneously without confusion."),
                (60, "Synapse Overclock", "Think faster than others and retain everything you process with full fidelity."),
                (65, "Concept Synthesis", "Fuse disparate ideas into powerful new frameworks no one has seen before."),
                (70, "Mental Search Engine", "Your mind retrieves any memory in milliseconds, tagged and ranked by relevance."),
                (75, "Brain Cache", "Frequently used knowledge sits at the forefront of mind, always ready to deploy."),
                (80, "Wisdom Replication", "Share your mental models with others and watch their thinking transform."),
                (85, "Neural Graph Mastery", "Your entire knowledge base forms a navigable, living, interconnected network."),
                (90, "Hyper Intelligence Mode", "Brief windows of total clarity where all complex problems suddenly simplify."),
                (95, "Universal Idea Map", "You can represent any concept, in any domain, on a single mental canvas."),
                (100, "Omniscient Mind Architect", "You think thoughts that have never been thought before, and never forget them."),
            ],
            ClassType::TaskPaladin => vec![
                (1, "Oath of Completion", "You swear to finish what you start. Even the smallest task now carries sacred purpose."),
                (5, "Checklist Strike", "Every completed task delivers a satisfying strike against the chaos of delay."),
                (10, "Focus Aura", "An invisible field of concentration that holds distractions at a safe distance."),
                (15, "Procrastination Shield", "The urge to delay bounces harmlessly off your sacred armor of intent."),
                (20, "Time Blocking", "Carve your hours into sacred blocks that no meeting or message may invade."),
                (25, "Deadline Vision", "Deadlines glow at the edge of your awareness, always urging you forward."),
                (30, "Priority Judgment", "The most important task reveals itself the moment you open your list."),
                (35, "Productivity Burst", "Channel all energy into a sprint that moves mountains in 25 minutes flat."),
                (40, "Distraction Immunity", "Notifications, noise, and temptation cannot reach you once you enter flow."),
                (45, "Task Chain Combo", "Complete one task and the momentum carries you directly into the next."),
                (50, "Holy Pomodoro", "Time itself bends to support your focused, rhythmic work sessions."),
                (55, "Multitasking Resistance", "You have learned the truth: serial focus destroys parallel dabbling every time."),
                (60, "Schedule Bending", "Reshape your calendar around your peak energy, not other people's requests."),
                (65, "Momentum Drive", "Starting is effortless now. The first task falls, and all the rest follow."),
                (70, "Execution Mastery", "You do not just plan. You execute. The gap between intent and action is zero."),
                (75, "Task Storm", "Enter a mode where tasks fall before you like wheat before a force of nature."),
                (80, "Time Expansion", "Your focused hours contain more actual work than others' entire days."),
                (85, "Project Conquest", "Entire projects collapse before your sustained, organized, relentless assault."),
                (90, "Absolute Discipline", "You no longer need motivation. Discipline has permanently replaced it."),
                (95, "Productivity Singularity", "Your output exceeds what any single individual should be capable of producing."),
                (100, "Avatar of Completion", "The uncompleted list is your only enemy. And you always, always win."),
            ],
            ClassType::SystemsArchitect => vec![
                (1, "Empty Framework", "You see that every great system begins with structure. The first folder is created."),
                (5, "Folder Creation", "Bring structure to chaos by naming and organizing what was once scattered."),
                (10, "Workflow Vision", "See the invisible paths tasks travel and draw them into visible clarity."),
                (15, "Chaos Containment", "Establish boundaries around disorder before it spreads and consumes everything."),
                (20, "System Blueprint", "Sketch entire systems completely before touching a single tool or file."),
                (25, "Automation Trigger", "Identify the repetitive action in any workflow and eliminate it with one rule."),
                (30, "Dependency Control", "Understand which pieces connect to what and never accidentally break the chain."),
                (35, "Project Structuring", "Transform vague goals into clean folders, clear phases, and measurable milestones."),
                (40, "Complexity Reduction", "Simplify the complicated until what remains is only the essential."),
                (45, "Order Aura", "Things around you naturally organize themselves to match your established systems."),
                (50, "Framework Summoning", "Call up the right methodology for any situation from your mental library."),
                (55, "Strategic Foresight", "See three steps ahead and build deliberately for the future you already predicted."),
                (60, "Macro Planning", "Operate at the level of months and years, not hours and days."),
                (65, "Infinite Organization", "No system is too large, too tangled, or too far gone for you to structure."),
                (70, "Systems Thinking", "You see organizations, projects, and habits as interconnected living systems."),
                (75, "Process Optimization", "Every workflow you touch becomes faster, cleaner, and more reliable."),
                (80, "Fractal Organization", "Your structure scales perfectly from a single task to an entire organization."),
                (85, "Mega Infrastructure", "Build systems so robust they outlive the very projects that created them."),
                (90, "Master Architecture", "Design the architecture that other architects will someday study and teach."),
                (95, "Reality Refactoring", "Restructure entire domains of life and work as if editing source code."),
                (100, "Cosmic System Designer", "You see the universe itself as a system. And you know exactly how to improve it."),
            ],
            ClassType::TimeChronomancer => vec![
                (1, "Tick Awareness", "For the first time, you notice the passing of every minute and the cost of wasting one."),
                (5, "Minute Awareness", "You finally know where your minutes actually go. The answer is humbling."),
                (10, "Calendar Reading", "Your calendar speaks to you in patterns. You understand its hidden language."),
                (15, "Focus Window", "Identify the two hours in your day when everything flows without friction."),
                (20, "Time Compression", "Make one focused hour feel like three when you enter the right state."),
                (25, "Schedule Teleportation", "Instantly see what needs to happen, when, and in exactly what sequence."),
                (30, "Distraction Freeze", "Stop time-thieves in their tracks before they steal your most precious hours."),
                (35, "Time Budgeting", "Allocate hours to priorities like currency, and never overspend your budget."),
                (40, "Productivity Loop", "Enter a rhythm where each completed block feeds naturally into the next."),
                (45, "Deadline Shield", "Deadlines pass over you without panic. You were finished days ago."),
                (50, "Chrono Planning", "Plan entire weeks with the clarity most people reserve for single days."),
                (55, "Hour Duplication", "Batch similar tasks together to create the illusion of hours you do not have."),
                (60, "Time Threading", "Run multiple workstreams in parallel, each advancing without interfering."),
                (65, "Future Forecast", "Predict with eerie accuracy how long every task will actually take to finish."),
                (70, "Temporal Optimization", "Remove everything from your schedule that cannot justify the time it costs."),
                (75, "Time Dilation", "Deep work sessions feel like minutes. Your output suggests otherwise."),
                (80, "Calendar Mastery", "Your schedule is a work of art. Every block earns its place with purpose."),
                (85, "Timeline Editing", "Rewrite the plan at will, reordering all priorities without losing momentum."),
                (90, "Time Loop Escape", "Break free of inefficiencies that consumed your time for years."),
                (95, "Eternal Deadline Control", "You set the deadlines now. They no longer have any authority over you."),
                (100, "Master of Time", "There is no schedule that can bind you. Time flows entirely by your will."),
            ],
        }
    }

    pub fn passive_description(&self) -> &'static str {
        match self {
            ClassType::TaskPaladin =>
                "+5 XP per task  |  +10 XP high priority  |  +15 XP full daily chain",
            ClassType::CodeWarlock =>
                "+5 XP per note  |  +15 XP new project  |  +10 XP on sync",
            ClassType::MindSage =>
                "+10 XP long notes  |  +5 XP journals  |  +10% fragment chance  |  +5 XP per fragment",
            ClassType::SystemsArchitect =>
                "+10 XP new project  |  +15 XP archive  |  +5 XP restore",
            ClassType::TimeChronomancer =>
                "+10 XP focus sessions  |  +25 XP pomodoros  |  +5 XP daily adventures",
            ClassType::ArchAccountant =>
                "+2 XP all rewards  |  +5% XP all sources  |  +10 XP full daily chain",
        }
    }

    pub fn order(&self) -> &'static str {
        match self {
            ClassType::ArchAccountant => "The Order of the Ledger",
            ClassType::SystemsArchitect => "The Builders of Order",
            ClassType::TimeChronomancer => "The Keepers of Hours",
            ClassType::TaskPaladin => "The Sacred Checklist",
            ClassType::MindSage => "The Silent Archive",
            ClassType::CodeWarlock => "The Terminal Covenant",
        }
    }

    pub fn lore(&self) -> &'static str {
        match self {
            ClassType::ArchAccountant => "The Arch Accountants were among the first followers of the Questline. Where others sought glory, they sought balance. Where others asked \"Can we afford this?\" — they already had the answer.",
            ClassType::SystemsArchitect => "Systems Architects see patterns where others see chaos. Processes where others see confusion. Many are capable of producing folder hierarchies before understanding the project itself.",
            ClassType::TimeChronomancer => "The Time Chronomancers study the most precious resource in existence: Time. Unlike gold, it cannot be earned. Unlike knowledge, it cannot be stored. Most discoveries are deeply unsettling.",
            ClassType::TaskPaladin => "The Task Paladins are the defenders of execution. While others debate. While others plan. While others research. Task Paladins complete things. And checking a box feels incredible.",
            ClassType::MindSage => "The Mind Sages dedicate themselves to preserving knowledge. Nothing is too small to record. Their archives contain billions of interconnected ideas. Many visitors become permanently lost.",
            ClassType::CodeWarlock => "No one knows exactly how the Code Warlocks began. Their own records are incomplete. Mostly because they forgot to back them up. History records that this was a mistake.",
        }
    }

    pub fn specializations(&self) -> Vec<(&'static str, &'static str)> {
        match self {
            ClassType::CodeWarlock => vec![
                ("Automation Mage", "+10% XP from Note Creation"),
                ("System Weaver", "+10% XP from Project Completion"),
                ("Bug Hunter", "+10% XP from Task Completion"),
            ],
            ClassType::TaskPaladin => vec![
                ("Execution Knight", "+10% XP from Task Completion"),
                ("Guardian of Order", "+10% XP from Project Completion"),
                ("Momentum Crusader", "+10% XP from Note Creation"),
            ],
            ClassType::MindSage => vec![
                ("Knowledge Keeper", "+10% XP from Note Creation"),
                ("Cognitive Cartographer", "+10% XP from Project Completion"),
                ("Insight Seeker", "+10% XP from Task Completion"),
            ],
            ClassType::SystemsArchitect => vec![
                ("Infrastructure Builder", "+10% XP from Project Completion"),
                ("Process Optimizer", "+10% XP from Task Completion"),
                ("Modular Designer", "+10% XP from Note Creation"),
            ],
            ClassType::TimeChronomancer => vec![
                ("Temporal Ward", "+10% XP from Task Completion"),
                ("History Weaver", "+10% XP from Note Creation"),
                ("Timeline Editor", "+10% XP from Project Completion"),
            ],
            ClassType::ArchAccountant => vec![
                ("Ledger Overseer", "+10% XP from Note Creation"),
                ("Audit Judge", "+10% XP from Task Completion"),
                ("Asset Growth Specialist", "+10% XP from Project Completion"),
            ],
        }
    }
}

// El modelo del usuario — mapea directo a la tabla `users` en SQLite
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub class: ClassType,
    pub level: i32,
    pub xp: i32,
    pub created_at: DateTime<Utc>,
    pub specialization: Option<String>,
}

impl User {
    // La fórmula de XP para subir de nivel — cuadrática, así que al rato se pone bien cañón
    pub fn xp_for_next_level(level: i32) -> i32 {
        if level >= 100 {
            0
        } else {
            200 + (level * level * 12)
        }
    }

    // El título cambia según nivel y clase — 5 rangos antes del título final del nivel 100
    pub fn title(&self) -> &'static str {
        match self.class {
            ClassType::CodeWarlock => match self.level {
                1..=9 => "Novice Coder",
                10..=24 => "Script Adept",
                25..=49 => "Terminal Magus",
                50..=74 => "Daemon Lord",
                75..=99 => "Master of Automation",
                _ => "Architect of Simulations",
            },
            ClassType::TaskPaladin => match self.level {
                1..=9 => "Squire of Order",
                10..=24 => "Keeper of Tasks",
                25..=49 => "Knight of Completion",
                50..=74 => "Champion of Discipline",
                75..=99 => "Guardian of Momentum",
                _ => "The Unfinished Finisher",
            },
            ClassType::MindSage => match self.level {
                1..=9 => "Apprentice Thinker",
                10..=24 => "Mapmaker of Nodes",
                25..=49 => "Mind Explorer",
                50..=74 => "Keeper of Knowledge",
                75..=99 => "Sage of Connections",
                _ => "Omniscient Mind Architect",
            },
            ClassType::SystemsArchitect => match self.level {
                1..=9 => "Framework Apprentice",
                10..=24 => "Blueprint Drafter",
                25..=49 => "Builder of Structure",
                50..=74 => "Order Designer",
                75..=99 => "Architect of Flow",
                _ => "Cosmic System Designer",
            },
            ClassType::TimeChronomancer => match self.level {
                1..=9 => "Watcher of Seconds",
                10..=24 => "Minute Weaver",
                25..=49 => "Hour Shaper",
                50..=74 => "Deadline Shield",
                75..=99 => "Master of Schedules",
                _ => "Master of Time",
            },
            ClassType::ArchAccountant => match self.level {
                1..=9 => "Ledger Apprentice",
                10..=24 => "Formula Initiate",
                25..=49 => "Expense Judge",
                50..=74 => "Golden Balancer",
                75..=99 => "Portfolio Alchemist",
                _ => "Omniscient Ledger Lord",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_xp_formula() {
        assert_eq!(User::xp_for_next_level(1), 212);
        assert_eq!(User::xp_for_next_level(2), 248);
        assert_eq!(User::xp_for_next_level(10), 1400);
        assert_eq!(User::xp_for_next_level(100), 0);
    }

    #[test]
    fn test_user_titles() {
        let mut u = User {
            id: Uuid::new_v4(),
            username: "Test".to_string(),
            class: ClassType::CodeWarlock,
            level: 1,
            xp: 0,
            created_at: Utc::now(),
            specialization: None,
        };
        assert_eq!(u.title(), "Novice Coder");
        u.level = 15;
        assert_eq!(u.title(), "Script Adept");
        u.level = 50;
        assert_eq!(u.title(), "Daemon Lord");
        u.level = 100;
        assert_eq!(u.title(), "Architect of Simulations");
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// theme/mod.rs — definiciones de colores y temas del TUI
// ─────────────────────────────────────────────────────────────────────────────
use crate::models::ClassType;
use ratatui::style::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ThemeChoice {
    ClassDefault,
    Forest,
    AncientLibrary,
    MountainFortress,
    ArcaneWorkshop,
    OceanTemple,
    LightMode,
    DarkMode,
    HighContrast,
    ColorblindFriendly,
    Nord,
    Dracula,
    GruvboxDark,
    CatppuccinMocha,
    TokyoNight,
    SolarizedDark,
    SolarizedLight,
    TerminalNative,
    Pywal,
}

// Structural representation of color schemes for terminal UI elements.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub primary: Color,
    pub secondary: Color,
    pub background: Color,
    pub surface: Color,
    pub panel: Color,
    pub border: Color,
    pub selection: Color,
    pub text: Color,
    pub muted: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub xp_bar: Color,
    pub focus_timer: Color,
    pub disabled: Color,
}

const SUCCESS: Color = Color::Rgb(34, 197, 94);
const WARNING: Color = Color::Rgb(245, 158, 11);
const DANGER: Color = Color::Rgb(239, 68, 68);
const XP_BAR: Color = Color::Rgb(132, 204, 22);
const FOCUS_TIMER: Color = Color::Rgb(20, 184, 166);
const DISABLED: Color = Color::Rgb(107, 114, 128);

impl Theme {
    pub fn all_choices() -> &'static [ThemeChoice] {
        &[
            ThemeChoice::ClassDefault,
            ThemeChoice::Nord,
            ThemeChoice::Dracula,
            ThemeChoice::GruvboxDark,
            ThemeChoice::CatppuccinMocha,
            ThemeChoice::TokyoNight,
            ThemeChoice::SolarizedDark,
            ThemeChoice::SolarizedLight,
            ThemeChoice::TerminalNative,
            ThemeChoice::Forest,
            ThemeChoice::AncientLibrary,
            ThemeChoice::MountainFortress,
            ThemeChoice::ArcaneWorkshop,
            ThemeChoice::OceanTemple,
            ThemeChoice::LightMode,
            ThemeChoice::DarkMode,
            ThemeChoice::HighContrast,
            ThemeChoice::ColorblindFriendly,
            ThemeChoice::Pywal,
        ]
    }

    pub fn theme_key(choice: ThemeChoice) -> &'static str {
        match choice {
            ThemeChoice::ClassDefault => "ClassDefault",
            ThemeChoice::Forest => "Forest",
            ThemeChoice::AncientLibrary => "AncientLibrary",
            ThemeChoice::MountainFortress => "MountainFortress",
            ThemeChoice::ArcaneWorkshop => "ArcaneWorkshop",
            ThemeChoice::OceanTemple => "OceanTemple",
            ThemeChoice::LightMode => "LightMode",
            ThemeChoice::DarkMode => "DarkMode",
            ThemeChoice::HighContrast => "HighContrast",
            ThemeChoice::ColorblindFriendly => "ColorblindFriendly",
            ThemeChoice::Nord => "Nord",
            ThemeChoice::Dracula => "Dracula",
            ThemeChoice::GruvboxDark => "GruvboxDark",
            ThemeChoice::CatppuccinMocha => "CatppuccinMocha",
            ThemeChoice::TokyoNight => "TokyoNight",
            ThemeChoice::SolarizedDark => "SolarizedDark",
            ThemeChoice::SolarizedLight => "SolarizedLight",
            ThemeChoice::TerminalNative => "TerminalNative",
            ThemeChoice::Pywal => "Pywal",
        }
    }

    pub fn theme_label(choice: ThemeChoice) -> &'static str {
        match choice {
            ThemeChoice::ClassDefault => "Order Regalia",
            ThemeChoice::Forest => "Evergrowth Grove",
            ThemeChoice::AncientLibrary => "Archive Lantern",
            ThemeChoice::MountainFortress => "Fortress Slate",
            ThemeChoice::ArcaneWorkshop => "Warlock Crucible",
            ThemeChoice::OceanTemple => "Tidebound Temple",
            ThemeChoice::LightMode => "Dawn Ledger",
            ThemeChoice::DarkMode => "Void Ledger",
            ThemeChoice::HighContrast => "First Cursor",
            ThemeChoice::ColorblindFriendly => "Clear Sigil",
            ThemeChoice::Nord => "Frostbound Archive",
            ThemeChoice::Dracula => "Swarm Regent",
            ThemeChoice::GruvboxDark => "Ember Backlog",
            ThemeChoice::CatppuccinMocha => "Moonlit Campfire",
            ThemeChoice::TokyoNight => "Cursorfall Night",
            ThemeChoice::SolarizedDark => "Chronicle Dusk",
            ThemeChoice::SolarizedLight => "Chronicle Dawn",
            ThemeChoice::TerminalNative => "Terminal Sigil",
            ThemeChoice::Pywal => "Wallpaper Relic",
        }
    }

    pub fn choice_from_key(key: &str) -> ThemeChoice {
        match key {
            "Forest" => ThemeChoice::Forest,
            "AncientLibrary" => ThemeChoice::AncientLibrary,
            "MountainFortress" => ThemeChoice::MountainFortress,
            "ArcaneWorkshop" => ThemeChoice::ArcaneWorkshop,
            "OceanTemple" => ThemeChoice::OceanTemple,
            "LightMode" => ThemeChoice::LightMode,
            "DarkMode" => ThemeChoice::DarkMode,
            "HighContrast" => ThemeChoice::HighContrast,
            "ColorblindFriendly" => ThemeChoice::ColorblindFriendly,
            "Nord" => ThemeChoice::Nord,
            "Dracula" => ThemeChoice::Dracula,
            "GruvboxDark" => ThemeChoice::GruvboxDark,
            "CatppuccinMocha" => ThemeChoice::CatppuccinMocha,
            "TokyoNight" => ThemeChoice::TokyoNight,
            "SolarizedDark" => ThemeChoice::SolarizedDark,
            "SolarizedLight" => ThemeChoice::SolarizedLight,
            "TerminalNative" => ThemeChoice::TerminalNative,
            "Pywal" => ThemeChoice::Pywal,
            _ => ThemeChoice::ClassDefault,
        }
    }

    pub fn for_choice(choice: ThemeChoice, class: ClassType) -> Self {
        match choice {
            ThemeChoice::ClassDefault => Self::for_class(class),
            ThemeChoice::Forest => Self::neutral(Color::Rgb(34, 197, 94)),
            ThemeChoice::AncientLibrary => Self::neutral(Color::Rgb(217, 119, 6)),
            ThemeChoice::MountainFortress => Self::neutral(Color::Rgb(100, 116, 139)),
            ThemeChoice::ArcaneWorkshop => Self::neutral(Color::Rgb(219, 39, 119)),
            ThemeChoice::OceanTemple => Self::neutral(Color::Rgb(14, 165, 233)),
            ThemeChoice::LightMode => Self {
                primary: Color::Blue,
                secondary: Color::Rgb(96, 165, 250),
                background: Color::White,
                surface: Color::Rgb(240, 242, 245),
                panel: Color::Rgb(220, 224, 230),
                border: Color::Rgb(100, 116, 139),
                selection: Color::Blue,
                text: Color::Black,
                muted: Color::Rgb(100, 116, 139),
                success: SUCCESS,
                warning: WARNING,
                danger: DANGER,
                xp_bar: XP_BAR,
                focus_timer: FOCUS_TIMER,
                disabled: DISABLED,
            },
            ThemeChoice::DarkMode => Self::neutral(Color::Rgb(168, 85, 247)),
            ThemeChoice::HighContrast => Self {
                primary: Color::White,
                secondary: Color::White,
                background: Color::Black,
                surface: Color::Black,
                panel: Color::Black,
                border: Color::White,
                selection: Color::White,
                text: Color::White,
                muted: Color::Gray,
                success: SUCCESS,
                warning: WARNING,
                danger: DANGER,
                xp_bar: XP_BAR,
                focus_timer: FOCUS_TIMER,
                disabled: DISABLED,
            },
            ThemeChoice::ColorblindFriendly => Self::neutral(Color::Rgb(0, 114, 178)),
            ThemeChoice::Nord => Self::palette(
                Color::Rgb(136, 192, 208),
                Color::Rgb(129, 161, 193),
                Color::Rgb(46, 52, 64),
                Color::Rgb(59, 66, 82),
                Color::Rgb(67, 76, 94),
                Color::Rgb(76, 86, 106),
                Color::Rgb(94, 129, 172),
                Color::Rgb(236, 239, 244),
                Color::Rgb(216, 222, 233),
            ),
            ThemeChoice::Dracula => Self::palette(
                Color::Rgb(189, 147, 249),
                Color::Rgb(255, 121, 198),
                Color::Rgb(40, 42, 54),
                Color::Rgb(48, 50, 65),
                Color::Rgb(68, 71, 90),
                Color::Rgb(98, 114, 164),
                Color::Rgb(68, 71, 90),
                Color::Rgb(248, 248, 242),
                Color::Rgb(189, 147, 249),
            ),
            ThemeChoice::GruvboxDark => Self::palette(
                Color::Rgb(250, 189, 47),
                Color::Rgb(131, 165, 152),
                Color::Rgb(40, 40, 40),
                Color::Rgb(50, 48, 47),
                Color::Rgb(60, 56, 54),
                Color::Rgb(102, 92, 84),
                Color::Rgb(69, 133, 136),
                Color::Rgb(235, 219, 178),
                Color::Rgb(168, 153, 132),
            ),
            ThemeChoice::CatppuccinMocha => Self::palette(
                Color::Rgb(203, 166, 247),
                Color::Rgb(137, 180, 250),
                Color::Rgb(30, 30, 46),
                Color::Rgb(24, 24, 37),
                Color::Rgb(49, 50, 68),
                Color::Rgb(88, 91, 112),
                Color::Rgb(137, 180, 250),
                Color::Rgb(205, 214, 244),
                Color::Rgb(166, 173, 200),
            ),
            ThemeChoice::TokyoNight => Self::palette(
                Color::Rgb(122, 162, 247),
                Color::Rgb(187, 154, 247),
                Color::Rgb(26, 27, 38),
                Color::Rgb(31, 35, 53),
                Color::Rgb(41, 46, 66),
                Color::Rgb(86, 95, 137),
                Color::Rgb(65, 72, 104),
                Color::Rgb(192, 202, 245),
                Color::Rgb(169, 177, 214),
            ),
            ThemeChoice::SolarizedDark => Self::palette(
                Color::Rgb(38, 139, 210),
                Color::Rgb(42, 161, 152),
                Color::Rgb(0, 43, 54),
                Color::Rgb(7, 54, 66),
                Color::Rgb(0, 52, 65),
                Color::Rgb(88, 110, 117),
                Color::Rgb(42, 161, 152),
                Color::Rgb(238, 232, 213),
                Color::Rgb(147, 161, 161),
            ),
            ThemeChoice::SolarizedLight => Self {
                primary: Color::Rgb(38, 139, 210),
                secondary: Color::Rgb(42, 161, 152),
                background: Color::Rgb(253, 246, 227),
                surface: Color::Rgb(238, 232, 213),
                panel: Color::Rgb(230, 223, 202),
                border: Color::Rgb(147, 161, 161),
                selection: Color::Rgb(42, 161, 152),
                text: Color::Rgb(0, 43, 54),
                muted: Color::Rgb(101, 123, 131),
                success: SUCCESS,
                warning: WARNING,
                danger: DANGER,
                xp_bar: XP_BAR,
                focus_timer: FOCUS_TIMER,
                disabled: DISABLED,
            },
            ThemeChoice::TerminalNative => Self::terminal_native(),
            ThemeChoice::Pywal => Self::from_pywal()
                .or_else(Self::xresources)
                .unwrap_or_else(|| Self::for_class(class)),
        }
    }

    // Generates a theme configuration mapped to the user class type.
    pub fn for_class(class: ClassType) -> Self {
        match class {
            ClassType::CodeWarlock => Self {
                primary: Color::Rgb(168, 85, 247),
                secondary: Color::Rgb(192, 132, 252),
                background: Color::Rgb(15, 10, 25),
                surface: Color::Rgb(24, 17, 36),
                panel: Color::Rgb(34, 26, 51),
                border: Color::Rgb(76, 29, 149),
                selection: Color::Rgb(109, 40, 217),
                text: Color::Rgb(245, 243, 255),
                muted: Color::Rgb(167, 139, 250),
                success: SUCCESS,
                warning: WARNING,
                danger: DANGER,
                xp_bar: XP_BAR,
                focus_timer: FOCUS_TIMER,
                disabled: DISABLED,
            },
            ClassType::TaskPaladin => Self {
                primary: Color::Rgb(255, 105, 180),
                secondary: Color::Rgb(249, 168, 212),
                background: Color::Rgb(25, 11, 20),
                surface: Color::Rgb(38, 16, 29),
                panel: Color::Rgb(51, 22, 37),
                border: Color::Rgb(190, 24, 93),
                selection: Color::Rgb(219, 39, 119),
                text: Color::Rgb(255, 241, 247),
                muted: Color::Rgb(249, 168, 212),
                success: SUCCESS,
                warning: WARNING,
                danger: DANGER,
                xp_bar: XP_BAR,
                focus_timer: FOCUS_TIMER,
                disabled: DISABLED,
            },
            ClassType::MindSage => Self {
                primary: Color::Rgb(6, 182, 212),
                secondary: Color::Rgb(103, 232, 249),
                background: Color::Rgb(7, 22, 26),
                surface: Color::Rgb(12, 34, 40),
                panel: Color::Rgb(18, 50, 59),
                border: Color::Rgb(8, 145, 178),
                selection: Color::Rgb(14, 165, 233),
                text: Color::Rgb(236, 254, 255),
                muted: Color::Rgb(103, 232, 249),
                success: SUCCESS,
                warning: WARNING,
                danger: DANGER,
                xp_bar: XP_BAR,
                focus_timer: FOCUS_TIMER,
                disabled: DISABLED,
            },
            ClassType::SystemsArchitect => Self {
                primary: Color::Rgb(59, 130, 246),
                secondary: Color::Rgb(147, 197, 253),
                background: Color::Rgb(8, 17, 31),
                surface: Color::Rgb(16, 32, 58),
                panel: Color::Rgb(21, 42, 74),
                border: Color::Rgb(37, 99, 235),
                selection: Color::Rgb(29, 78, 216),
                text: Color::Rgb(239, 246, 255),
                muted: Color::Rgb(147, 197, 253),
                success: SUCCESS,
                warning: WARNING,
                danger: DANGER,
                xp_bar: XP_BAR,
                focus_timer: FOCUS_TIMER,
                disabled: DISABLED,
            },
            ClassType::TimeChronomancer => Self {
                primary: Color::Rgb(249, 115, 22),
                secondary: Color::Rgb(253, 186, 116),
                background: Color::Rgb(26, 15, 8),
                surface: Color::Rgb(41, 24, 14),
                panel: Color::Rgb(56, 33, 19),
                border: Color::Rgb(234, 88, 12),
                selection: Color::Rgb(194, 65, 12),
                text: Color::Rgb(255, 247, 237),
                muted: Color::Rgb(253, 186, 116),
                success: SUCCESS,
                warning: WARNING,
                danger: DANGER,
                xp_bar: XP_BAR,
                focus_timer: FOCUS_TIMER,
                disabled: DISABLED,
            },
            ClassType::ArchAccountant => Self {
                primary: Color::Rgb(245, 158, 11),
                secondary: Color::Rgb(252, 211, 77),
                background: Color::Rgb(25, 19, 5),
                surface: Color::Rgb(38, 29, 8),
                panel: Color::Rgb(56, 43, 11),
                border: Color::Rgb(217, 119, 6),
                selection: Color::Rgb(180, 83, 9),
                text: Color::Rgb(255, 251, 235),
                muted: Color::Rgb(252, 211, 77),
                success: SUCCESS,
                warning: WARNING,
                danger: DANGER,
                xp_bar: XP_BAR,
                focus_timer: FOCUS_TIMER,
                disabled: DISABLED,
            },
        }
    }

    // Default theme used during onboarding before a class is chosen.
    pub fn default_theme() -> Self {
        Self::neutral(Color::Gray)
    }

    // Neutral theme for non-class choices: primary accent + shared dark palette.
    fn neutral(primary: Color) -> Self {
        Self::palette(
            primary,
            Color::Rgb(148, 163, 184),
            Color::Rgb(15, 17, 23),
            Color::Rgb(21, 25, 34),
            Color::Rgb(29, 36, 51),
            Color::Rgb(51, 65, 85),
            primary,
            Color::Rgb(229, 231, 235),
            Color::Rgb(148, 163, 184),
        )
    }

    fn palette(
        primary: Color,
        secondary: Color,
        background: Color,
        surface: Color,
        panel: Color,
        border: Color,
        selection: Color,
        text: Color,
        muted: Color,
    ) -> Self {
        Self {
            primary,
            secondary,
            background,
            surface,
            panel,
            border,
            selection,
            text,
            muted,
            success: SUCCESS,
            warning: WARNING,
            danger: DANGER,
            xp_bar: XP_BAR,
            focus_timer: FOCUS_TIMER,
            disabled: DISABLED,
        }
    }

    pub fn from_pywal() -> Option<Self> {
        let home = std::env::var("HOME").ok()?;
        let path = std::path::Path::new(&home).join(".cache/wal/colors.json");
        let data = std::fs::read_to_string(path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&data).ok()?;
        let special = json.get("special")?;
        let colors = json.get("colors")?;
        let color = |section: &serde_json::Value, key: &str| -> Option<Color> {
            hex_color(section.get(key)?.as_str()?)
        };

        Some(Self::palette(
            color(colors, "color4").or_else(|| color(colors, "color5"))?,
            color(colors, "color6").or_else(|| color(colors, "color2"))?,
            color(special, "background")?,
            color(colors, "color0").or_else(|| color(special, "background"))?,
            color(colors, "color8").or_else(|| color(colors, "color0"))?,
            color(colors, "color8").or_else(|| color(colors, "color7"))?,
            color(colors, "color4").or_else(|| color(colors, "color5"))?,
            color(special, "foreground")?,
            color(colors, "color7").or_else(|| color(special, "foreground"))?,
        ))
    }

    fn terminal_native() -> Self {
        Self::xresources().unwrap_or(Self {
            primary: Color::Blue,
            secondary: Color::Cyan,
            background: Color::Reset,
            surface: Color::Reset,
            panel: Color::Reset,
            border: Color::Gray,
            selection: Color::Blue,
            text: Color::Reset,
            muted: Color::DarkGray,
            success: Color::Green,
            warning: Color::Yellow,
            danger: Color::Red,
            xp_bar: Color::Green,
            focus_timer: Color::Cyan,
            disabled: Color::DarkGray,
        })
    }

    fn xresources() -> Option<Self> {
        let home = std::env::var("HOME").ok()?;
        let candidates = [
            std::path::Path::new(&home).join(".Xresources"),
            std::path::Path::new(&home).join(".Xdefaults"),
        ];
        let data = candidates
            .iter()
            .find_map(|path| std::fs::read_to_string(path).ok())?;
        let lookup = |name: &str| xresource_color(&data, name);

        Some(Self {
            primary: lookup("color4").or_else(|| lookup("color12"))?,
            secondary: lookup("color6")
                .or_else(|| lookup("color14"))
                .unwrap_or(Color::Cyan),
            background: lookup("background").unwrap_or(Color::Reset),
            surface: lookup("color0")
                .or_else(|| lookup("background"))
                .unwrap_or(Color::Reset),
            panel: lookup("color8")
                .or_else(|| lookup("color0"))
                .unwrap_or(Color::Reset),
            border: lookup("color8")
                .or_else(|| lookup("color7"))
                .unwrap_or(Color::Gray),
            selection: lookup("color4")
                .or_else(|| lookup("color12"))
                .unwrap_or(Color::Blue),
            text: lookup("foreground").unwrap_or(Color::Reset),
            muted: lookup("color7")
                .or_else(|| lookup("foreground"))
                .unwrap_or(Color::Gray),
            success: lookup("color2")
                .or_else(|| lookup("color10"))
                .unwrap_or(Color::Green),
            warning: lookup("color3")
                .or_else(|| lookup("color11"))
                .unwrap_or(Color::Yellow),
            danger: lookup("color1")
                .or_else(|| lookup("color9"))
                .unwrap_or(Color::Red),
            xp_bar: lookup("color2")
                .or_else(|| lookup("color10"))
                .unwrap_or(Color::Green),
            focus_timer: lookup("color6")
                .or_else(|| lookup("color14"))
                .unwrap_or(Color::Cyan),
            disabled: lookup("color8").unwrap_or(Color::DarkGray),
        })
    }
}

fn xresource_color(data: &str, name: &str) -> Option<Color> {
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('!') || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let key_name = key
            .trim()
            .rsplit(|c| c == '*' || c == '.')
            .next()
            .unwrap_or("")
            .trim();
        if key_name.eq_ignore_ascii_case(name) {
            if let Some(color) = hex_color(value.trim()) {
                return Some(color);
            }
        }
    }
    None
}

fn hex_color(input: &str) -> Option<Color> {
    let hex = input.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color::Rgb(r, g, b))
}

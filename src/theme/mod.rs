// ─────────────────────────────────────────────────────────────────────────────
// theme/mod.rs — definiciones de colores y temas del TUI
// ─────────────────────────────────────────────────────────────────────────────
use crate::models::ClassType;
use ratatui::style::{Color, Modifier, Style};

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
    SpectrumOS,
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
    selection_text: Color,
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
    pub fn selected_fg(&self) -> Color {
        self.selection_text
    }

    pub fn selected_style(&self) -> Style {
        Style::default()
            .fg(self.selected_fg())
            .bg(self.selection)
            .add_modifier(Modifier::BOLD)
    }

    pub fn primary_selected_style(&self) -> Style {
        Style::default()
            .fg(self.selected_fg())
            .bg(self.primary)
            .add_modifier(Modifier::BOLD)
    }

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
            ThemeChoice::SpectrumOS,
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
            ThemeChoice::SpectrumOS => "SpectrumOS",
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
            ThemeChoice::SpectrumOS => "SpectrumOS",
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
            "SpectrumOS" => ThemeChoice::SpectrumOS,
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
                selection_text: Color::Black,
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
                selection_text: Color::Black,
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
                selection_text: Color::Black,
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
            ThemeChoice::SpectrumOS => {
                Self::from_spectrum_os().unwrap_or_else(Self::spectrum_fallback)
            }
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
                selection_text: Color::Black,
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
                selection_text: Color::Black,
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
                selection_text: Color::Black,
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
                selection_text: Color::Black,
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
                selection_text: Color::Black,
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
                selection_text: Color::Black,
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
            selection_text: Color::Black,
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

    pub fn spectrum_palette_path() -> Option<std::path::PathBuf> {
        let cache_root = std::env::var_os("XDG_CACHE_HOME")
            .filter(|value| !value.is_empty())
            .map(std::path::PathBuf::from)
            .or_else(|| {
                std::env::var_os("HOME")
                    .filter(|value| !value.is_empty())
                    .map(|home| std::path::PathBuf::from(home).join(".cache"))
            })?;
        Some(
            cache_root
                .join("spectrumos")
                .join("theme")
                .join("current")
                .join("palette.json"),
        )
    }

    pub fn spectrum_palette_modified() -> Option<std::time::SystemTime> {
        std::fs::metadata(Self::spectrum_palette_path()?)
            .ok()?
            .modified()
            .ok()
    }

    pub fn from_spectrum_os() -> Option<Self> {
        let data = std::fs::read_to_string(Self::spectrum_palette_path()?).ok()?;
        Self::from_spectrum_json(&data)
    }

    fn from_spectrum_json(data: &str) -> Option<Self> {
        let payload: serde_json::Value = serde_json::from_str(data).ok()?;
        if payload.get("schema_version")?.as_u64()? != 1 {
            return None;
        }
        let roles = payload.get("roles")?.as_object()?;
        let role = |name: &str| hex_color(roles.get(name)?.as_str()?);
        let selection = role("accent")?;
        let success = role("success")?;
        let accent_alt = role("accent_alt")?;

        Some(Self {
            primary: selection,
            secondary: accent_alt,
            background: role("background")?,
            surface: role("surface")?,
            panel: role("surface_alt")?,
            border: role("surface_alt")?,
            selection,
            selection_text: readable_text_on(selection),
            text: role("foreground")?,
            muted: role("muted")?,
            success,
            warning: role("warning")?,
            danger: role("error")?,
            xp_bar: success,
            focus_timer: accent_alt,
            disabled: role("muted")?,
        })
    }

    fn spectrum_fallback() -> Self {
        Self::from_spectrum_json(
            r##"{
                "schema_version": 1,
                "roles": {
                    "background": "#101218",
                    "surface": "#191c24",
                    "surface_alt": "#252a35",
                    "foreground": "#e9edf5",
                    "muted": "#9ca6b8",
                    "accent": "#76b7ff",
                    "accent_alt": "#c49aff",
                    "success": "#78dba9",
                    "warning": "#f2c66d",
                    "error": "#ff8c9a"
                }
            }"##,
        )
        .expect("the bundled SpectrumOS fallback palette must remain valid")
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
            selection_text: Color::Black,
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
            selection_text: Color::Black,
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

fn readable_text_on(color: Color) -> Color {
    let Color::Rgb(r, g, b) = color else {
        return Color::Black;
    };
    let channel = |value: u8| {
        let value = value as f64 / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance = 0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b);
    let black_contrast = (luminance + 0.05) / 0.05;
    let white_contrast = 1.05 / (luminance + 0.05);
    if black_contrast >= white_contrast {
        Color::Black
    } else {
        Color::White
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIGHT_SPECTRUM: &str = r##"{
        "schema_version": 1,
        "name": "Spectrum Light Test",
        "variant": "light",
        "roles": {
            "background": "#f7f8fb",
            "surface": "#eef1f6",
            "surface_alt": "#e1e6ee",
            "foreground": "#1c2430",
            "muted": "#566273",
            "accent": "#245f9e",
            "accent_alt": "#6a3d91",
            "success": "#176b46",
            "warning": "#7a5200",
            "error": "#a3293d"
        }
    }"##;

    #[test]
    fn spectrum_theme_maps_every_semantic_role() {
        let theme = Theme::from_spectrum_json(LIGHT_SPECTRUM).unwrap();
        assert_eq!(theme.background, Color::Rgb(247, 248, 251));
        assert_eq!(theme.surface, Color::Rgb(238, 241, 246));
        assert_eq!(theme.panel, Color::Rgb(225, 230, 238));
        assert_eq!(theme.text, Color::Rgb(28, 36, 48));
        assert_eq!(theme.muted, Color::Rgb(86, 98, 115));
        assert_eq!(theme.primary, Color::Rgb(36, 95, 158));
        assert_eq!(theme.secondary, Color::Rgb(106, 61, 145));
        assert_eq!(theme.success, Color::Rgb(23, 107, 70));
        assert_eq!(theme.warning, Color::Rgb(122, 82, 0));
        assert_eq!(theme.danger, Color::Rgb(163, 41, 61));
        assert_eq!(theme.selected_fg(), Color::White);
    }

    #[test]
    fn spectrum_theme_rejects_unknown_or_incomplete_palettes() {
        assert!(Theme::from_spectrum_json(r#"{"schema_version": 2, "roles": {}}"#).is_none());
        assert!(Theme::from_spectrum_json(r#"{"schema_version": 1, "roles": {}}"#).is_none());
        assert!(Theme::from_spectrum_json("not json").is_none());
    }

    #[test]
    fn spectrum_theme_is_selectable_and_has_a_stable_key() {
        assert!(Theme::all_choices().contains(&ThemeChoice::SpectrumOS));
        assert_eq!(Theme::theme_key(ThemeChoice::SpectrumOS), "SpectrumOS");
        assert_eq!(Theme::theme_label(ThemeChoice::SpectrumOS), "SpectrumOS");
        assert_eq!(Theme::choice_from_key("SpectrumOS"), ThemeChoice::SpectrumOS);
    }
}

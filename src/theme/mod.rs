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
        Self {
            primary,
            secondary: Color::Rgb(148, 163, 184),
            background: Color::Rgb(15, 17, 23),
            surface: Color::Rgb(21, 25, 34),
            panel: Color::Rgb(29, 36, 51),
            border: Color::Rgb(51, 65, 85),
            selection: primary,
            text: Color::Rgb(229, 231, 235),
            muted: Color::Rgb(148, 163, 184),
            success: SUCCESS,
            warning: WARNING,
            danger: DANGER,
            xp_bar: XP_BAR,
            focus_timer: FOCUS_TIMER,
            disabled: DISABLED,
        }
    }
}

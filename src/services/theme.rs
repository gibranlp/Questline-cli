// ─────────────────────────────────────────────────────────────────────────────
// services/theme.rs — los temas de color disponibles en el app
// ─────────────────────────────────────────────────────────────────────────────
use crate::models::ClassType;
use crate::theme::{Theme, ThemeChoice};

// Service coordinating user theme configuration dynamically based on active character class and choices.
pub struct ThemeService {
    current_theme: Theme,
    choice: ThemeChoice,
    class: Option<ClassType>,
}

impl Default for ThemeService {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemeService {
    // Initializer starting with the default theme.
    pub fn new() -> Self {
        Self {
            current_theme: Theme::default_theme(),
            choice: ThemeChoice::ClassDefault,
            class: None,
        }
    }

    // Dynamic switcher updates current theme layout rules.
    pub fn set_class(&mut self, class: ClassType) {
        self.class = Some(class);
        self.apply_choice();
    }

    // Update active theme choice
    pub fn set_theme_choice(&mut self, choice: ThemeChoice) {
        self.choice = choice;
        self.apply_choice();
    }

    pub fn choice(&self) -> ThemeChoice {
        self.choice
    }

    // Accessor returning active UI layout settings.
    pub fn theme(&self) -> Theme {
        self.current_theme
    }

    fn apply_choice(&mut self) {
        if self.choice == ThemeChoice::Pywal {
            if let Some(theme) = Theme::from_pywal() {
                self.current_theme = theme;
            }
            return;
        }

        if let Some(c) = self.class {
            self.current_theme = Theme::for_choice(self.choice, c);
        }
    }
}

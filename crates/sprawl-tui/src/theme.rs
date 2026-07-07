use ratatui::style::Color;

pub struct ThemeColors {
    pub destructive: Color,  // Red — irreversible/blocked
    pub needs_review: Color, // Yellow/Amber — ambiguous
    pub safe: Color,         // Green — verified/complete
    pub info: Color,         // Cyan — informational
    pub dormant: Color,      // Gray — snoozed/disabled
}

impl ThemeColors {
    pub fn from_terminal() -> Self {
        // Map semantic roles onto terminal ANSI palette
        // Adapts to light/dark terminal themes
        Self {
            destructive: Color::Red,
            needs_review: Color::Yellow,
            safe: Color::Green,
            info: Color::Cyan,
            dormant: Color::DarkGray,
        }
    }
}

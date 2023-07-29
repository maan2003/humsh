use crossterm::style::{Color, ContentStyle, StyledContent, Stylize};

pub struct Style {
    pub heading: ContentStyle,
    pub normal: ContentStyle,
    pub button: ContentStyle,
    pub status: ContentStyle,
    pub directory: ContentStyle,
    pub error: ContentStyle,
    pub command: ContentStyle,
    pub flag_off: ContentStyle,
    pub flag_on: ContentStyle,
    pub prompt_char: StyledContent<&'static str>,
}

pub fn builtin() -> Style {
    Style {
        heading: ContentStyle::new().with(Color::Blue),
        normal: ContentStyle::new(),
        button: ContentStyle::new().with(Color::Grey),
        status: ContentStyle::new().with(Color::Magenta),
        directory: ContentStyle::new().with(Color::Cyan),
        error: ContentStyle::new().with(Color::Red),
        command: ContentStyle::new().with(Color::DarkGreen),
        flag_off: ContentStyle::new().with(Color::DarkGrey),
        flag_on: ContentStyle::new().with(Color::Cyan),
        prompt_char: " λ ".with(Color::Yellow),
    }
}

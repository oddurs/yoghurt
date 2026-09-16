//! Colour, by role rather than by value.
//!
//! Six states distinguished by colour is a tool that lies to about one man in
//! twelve, and to anybody piping it, and to anybody whose terminal palette does
//! not match the assumption it was written under.
//!
//! So every colour here is named for what it is *for*, `auto` maps those roles
//! onto the terminal's own ANSI palette — yoghurt should look like the terminal
//! it runs in rather than like somebody else's screenshot — and `mono` drops
//! colour entirely and loses nothing, because every state already carries a
//! glyph.

use ratatui::style::Color;

/// Which palette is in force.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Theme {
    /// The terminal's own colours.
    #[default]
    Auto,
    /// No colour at all.
    Mono,
}

impl Theme {
    /// What the environment asks for.
    ///
    /// `NO_COLOR` is honoured without configuration, because somebody who has
    /// set it has already said what they want and should not have to say it
    /// again per program.
    #[must_use]
    pub fn from_environment() -> Self {
        if std::env::var_os("NO_COLOR").is_some_and(|v| !v.is_empty()) {
            return Self::Mono;
        }
        match std::env::var("YOGHURT_THEME").as_deref() {
            Ok("mono") => Self::Mono,
            _ => Self::Auto,
        }
    }

    /// The theme with this name, if there is one.
    #[must_use]
    pub fn from_label(name: &str) -> Option<Self> {
        match name {
            "auto" => Some(Self::Auto),
            "mono" => Some(Self::Mono),
            _ => None,
        }
    }

    /// One colour, by what it is for.
    ///
    /// Under `mono` every role resolves to the terminal's default, so nothing
    /// is emphasised by hue and everything still says what it is.
    #[must_use]
    pub fn colour(self, role: Role) -> Color {
        if self == Self::Mono {
            return Color::Reset;
        }
        match role {
            Role::Wanted => Color::Green,
            Role::PulledIn => Color::Blue,
            Role::Outdated => Color::Yellow,
            Role::Broken => Color::Red,
            Role::Unaccounted => Color::Magenta,
            Role::Heading => Color::White,
            Role::Muted => Color::DarkGray,
            Role::Accent => Color::Cyan,
        }
    }
}

/// What a colour is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// Something somebody asked for.
    Wanted,
    /// Something that came with something else.
    PulledIn,
    /// Something with a newer version published.
    Outdated,
    /// Something that is not there.
    Broken,
    /// Something nothing accounts for.
    Unaccounted,
    /// A group heading, or the name of a thing.
    Heading,
    /// Anything secondary: sizes, versions, the footer.
    Muted,
    /// Something to copy, or the identity in the header.
    Accent,
}

#[cfg(test)]
mod tests {
    use super::{Role, Theme};
    use ratatui::style::Color;

    const EVERY_ROLE: [Role; 8] = [
        Role::Wanted,
        Role::PulledIn,
        Role::Outdated,
        Role::Broken,
        Role::Unaccounted,
        Role::Heading,
        Role::Muted,
        Role::Accent,
    ];

    #[test]
    fn mono_resolves_every_role_to_the_terminals_default() {
        for role in EVERY_ROLE {
            assert_eq!(
                Theme::Mono.colour(role),
                Color::Reset,
                "under mono nothing may be emphasised by hue: {role:?}"
            );
        }
    }

    #[test]
    fn auto_gives_every_role_a_colour_from_the_terminals_own_palette() {
        for role in EVERY_ROLE {
            let colour = Theme::Auto.colour(role);
            assert_ne!(colour, Color::Reset, "{role:?}");
            assert!(
                !matches!(colour, Color::Rgb(..) | Color::Indexed(_)),
                "a fixed colour would look like somebody else's screenshot: {role:?} is {colour:?}"
            );
        }
    }

    #[test]
    fn the_states_are_told_apart_by_hue_when_there_is_hue_to_use() {
        let states = [Role::Wanted, Role::PulledIn, Role::Outdated, Role::Broken];
        let mut seen: Vec<Color> = Vec::new();
        for role in states {
            let colour = Theme::Auto.colour(role);
            assert!(
                !seen.contains(&colour),
                "{role:?} shares a colour with something else"
            );
            seen.push(colour);
        }
    }

    #[test]
    fn the_themes_are_reachable_by_name() {
        assert_eq!(Theme::from_label("auto"), Some(Theme::Auto));
        assert_eq!(Theme::from_label("mono"), Some(Theme::Mono));
        assert_eq!(Theme::from_label("gotham"), None);
    }
}

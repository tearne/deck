//! Contextual controls, laid out once across the screen's full width.
use crate::config::{Action, KeyBinding};
use crossterm::event::KeyCode;
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Context {
    Decks,
    Command,
    Search,
    Move,
    Naming,
    Playlist,
    Candidates,
    Tags,
    TagPreview,
    Messages,
    ConfirmLoad,
    ConfirmDismiss,
    ConfirmQuit,
}

pub(crate) struct State {
    pub context: Context,
    pub browser: bool,
    pub focusable: bool,
    pub command_chords: bool,
    pub insert: bool,
    pub workspace: bool,
    pub filtered: bool,
    pub rename_toggle: bool,
}

#[derive(Default)]
pub(crate) struct Hints {
    groups: Vec<(String, Vec<String>, Option<Color>)>,
    accent: Option<Color>,
}

#[derive(Clone, Copy)]
enum Bindings {
    Any,
    Alt,
    Views,
}

fn key_name(code: &KeyCode) -> String {
    match code {
        KeyCode::Char(' ') => "Space".into(),
        KeyCode::Char(c) => c.to_string(),
        KeyCode::Left => "←".into(),
        KeyCode::Right => "→".into(),
        KeyCode::Up => "↑".into(),
        KeyCode::Down => "↓".into(),
        other => format!("{other:?}"),
    }
}

// Sort aliases so the hint is stable across HashMap seeds. Filter bindings by
// the layer the input dispatcher actually accepts in this context.
fn action_key(
    map: &HashMap<KeyBinding, Action>,
    action: Action,
    scope: Bindings,
) -> Option<String> {
    let mut keys: Vec<_> = map
        .iter()
        .filter_map(|(binding, a)| {
            if *a != action {
                return None;
            }
            match binding {
                KeyBinding::AltChord(code) if matches!(scope, Bindings::Any | Bindings::Alt) => {
                    Some(format!("Alt+{}", key_name(code)))
                }
                KeyBinding::SpaceChord(code)
                    if matches!(scope, Bindings::Any | Bindings::Views) =>
                {
                    Some(format!("Space+{}", key_name(code)))
                }
                KeyBinding::Key(code)
                    if matches!(scope, Bindings::Any)
                        || (matches!(scope, Bindings::Views) && action == Action::Help) =>
                {
                    Some(key_name(code))
                }
                _ => None,
            }
        })
        .collect();
    keys.sort();
    keys.into_iter().next()
}

/// Pane navigation belongs in the destination headers, using live Alt bindings.
pub(crate) fn pane_keys(map: &HashMap<KeyBinding, Action>) -> [Option<String>; 2] {
    [Action::PaneLeft, Action::PaneRight].map(|a| action_key(map, a, Bindings::Alt))
}

impl Hints {
    fn group(&mut self, name: &str, hints: &[&str]) {
        self.groups.push((
            name.into(),
            hints.iter().map(|s| s.to_string()).collect(),
            self.accent,
        ));
    }
    fn action(
        &mut self,
        map: &HashMap<KeyBinding, Action>,
        a: Action,
        scope: Bindings,
        description: &str,
    ) {
        if let Some(key) = action_key(map, a, scope) {
            self.groups
                .last_mut()
                .unwrap()
                .1
                .push(format!("{key} {description}"));
        }
    }

    pub(crate) fn for_state(s: &State, map: &HashMap<KeyBinding, Action>) -> Self {
        use Context::*;
        let accent = match s.context {
            Command | Naming => Some(crate::browser::mode_accent(
                crate::browser::BrowserMode::Command,
            )),
            Search => Some(crate::browser::mode_accent(
                crate::browser::BrowserMode::Search,
            )),
            Move => Some(crate::browser::mode_accent(
                crate::browser::BrowserMode::Move,
            )),
            Playlist => Some(crate::render::PLAYLIST_FAMILY.0),
            Tags | TagPreview => Some(crate::render::TAGS_FAMILY.0),
            Candidates | ConfirmLoad | ConfirmDismiss | ConfirmQuit => {
                Some(Color::Rgb(230, 190, 100))
            }
            Decks | Messages => None,
        };
        let mut h = Self {
            accent,
            ..Self::default()
        };
        match s.context {
            ConfirmLoad | ConfirmDismiss | ConfirmQuit => {
                let title = match s.context {
                    ConfirmLoad => "Load playing deck?",
                    ConfirmDismiss => "Destroy deck?",
                    _ => "Quit?",
                };
                h.group(title, &["y confirm", "any other key cancel"]);
                return h;
            }
            Tags => {
                h.group("Actions", &["Enter save and exit", "Esc cancel"]);
                h.group("Edit", &["↑/↓ field"]);
                if s.rename_toggle {
                    h.groups
                        .last_mut()
                        .unwrap()
                        .1
                        .push("Space toggle rename".into());
                } else {
                    h.groups.last_mut().unwrap().1.extend(
                        [
                            "type text",
                            "←/→ cursor",
                            "Home/End start/end",
                            "Backspace/Delete erase",
                        ]
                        .map(String::from),
                    );
                }
                return h;
            }
            Naming => {
                h.group("Actions", &["Enter create", "Esc cancel"]);
                h.group("Edit", &["type name", "Backspace erase"]);
            }
            Move => {
                h.group("Actions", &["y move here", "Esc cancel"]);
                h.group(
                    "Navigate",
                    &["j/k select", "Enter/l into folder", "h/Backspace parent"],
                );
            }
            Search => {
                h.group("Edit", &["type to filter", "Backspace erase"]);
                h.groups.last_mut().unwrap().1.push(
                    if s.filtered {
                        "Esc clear filter"
                    } else {
                        "Esc command mode"
                    }
                    .into(),
                );
                h.group("Navigate", &["↑/↓ select", "Enter open/load"]);
            }
            Command => {
                h.group(
                    "Navigate",
                    &[
                        "j/k select",
                        "h/Backspace parent",
                        "l into folder/playlist",
                        "Enter open/load",
                    ],
                );
                h.group(
                    "Actions",
                    &[
                        "/ search",
                        "# listen",
                        "e edit tags",
                        "m move file",
                        "n new playlist",
                        "` loaded locations",
                        "T tag warnings",
                        "@ set workspace",
                    ],
                );
                if s.workspace {
                    h.groups
                        .last_mut()
                        .unwrap()
                        .1
                        .push("' clear workspace".into());
                }
                if s.insert {
                    h.groups
                        .last_mut()
                        .unwrap()
                        .1
                        .push("a/A insert after/before".into());
                }
                h.groups.last_mut().unwrap().1.push(
                    if s.filtered {
                        "Esc clear filter"
                    } else {
                        "Esc decks"
                    }
                    .into(),
                );
            }
            Playlist => {
                h.group(
                    "Navigate",
                    &["j/k select", "Enter load/relink", "Esc browser"],
                );
                h.group(
                    "Edit",
                    &["J/K reorder", "x/Delete remove", "a/A insert after/before"],
                );
            }
            Candidates => {
                h.group("Actions", &["Enter confirm", "Esc cancel"]);
                h.group("Navigate", &["j/k select"]);
            }
            TagPreview => h.group("Navigate", &["Esc browser"]),
            Messages => h.group("Navigate", &["j/k scroll", "Esc decks"]),
            Decks => h.group("Switch", &[]),
        }
        // Switching decks/focus and choosing views are shared controls, so
        // they retain a neutral colour instead of inheriting pane ownership.
        h.accent = None;
        if s.browser && s.context != Decks {
            h.group("Switch", &["Tab decks"]);
            for (a, desc) in [
                (Action::SelectPrevDeck, "previous deck"),
                (Action::SelectNextDeck, "next deck"),
                (Action::SelectDeck1, "deck 1"),
                (Action::SelectDeck2, "deck 2"),
                (Action::SelectDeck3, "deck 3"),
            ] {
                h.action(map, a, Bindings::Alt, desc);
            }
            if s.context != TagPreview {
                h.action(map, Action::PanelWiden, Bindings::Alt, "resize left");
                h.action(map, Action::PanelNarrow, Bindings::Alt, "resize right");
            }
        } else if s.focusable {
            if s.context != Decks {
                h.group("Switch", &[]);
            }
            h.groups.last_mut().unwrap().1.push(
                if s.context == Decks {
                    "Tab focus view"
                } else {
                    "Tab decks"
                }
                .into(),
            );
        }
        if matches!(s.context, Decks | Messages) || s.command_chords {
            let scope = if s.command_chords && s.context != Decks {
                Bindings::Views
            } else {
                Bindings::Any
            };
            h.group("Views", &[]);
            for (a, desc) in [
                (Action::OpenBrowser, "browser"),
                (Action::Help, "keyboard reference"),
                (Action::MessageHistory, "messages"),
                (Action::ArtView, "art"),
            ] {
                h.action(map, a, scope, desc);
            }
        }
        h
    }

    /// Wrap between hints, keeping each key with its description. Only a hint
    /// wider than the entire screen is split; terminal-cell widths include CJK.
    pub(crate) fn lines(&self, width: u16) -> Vec<Line<'static>> {
        let width = usize::from(width);
        if width == 0 {
            return vec![];
        }
        let mut result = Vec::new();
        for (label, hints, accent) in &self.groups {
            let text_style = Style::default().fg(accent.unwrap_or(Color::Rgb(200, 205, 220)));
            let label_style = text_style.add_modifier(Modifier::BOLD);
            if hints.is_empty() {
                continue;
            }
            let mut line = Line::default();
            for (index, text) in std::iter::once(format!("{label}: "))
                .chain(hints.iter().cloned())
                .enumerate()
            {
                let style = if index == 0 { label_style } else { text_style };
                let sep = if index > 1 && line.width() > 0 {
                    " · "
                } else {
                    ""
                };
                if line.width() + Line::from(sep).width() + Line::from(text.clone()).width() > width
                    && line.width() > 0
                {
                    result.push(line);
                    line = Line::default();
                } else if !sep.is_empty() {
                    line.spans.push(Span::styled(sep.to_string(), text_style));
                }
                // Split only oversized entries, without cutting a wide character.
                let mut part = String::new();
                for c in text.chars() {
                    let mut next = part.clone();
                    next.push(c);
                    if line.width() + Line::from(next).width() > width {
                        line.spans
                            .push(Span::styled(std::mem::take(&mut part), style));
                        result.push(line);
                        line = Line::default();
                    }
                    if Line::from(c.to_string()).width() <= width {
                        part.push(c);
                    }
                }
                line.spans.push(Span::styled(part, style));
            }
            if line.width() > 0 {
                result.push(line);
            }
        }
        result
    }
}

/// Leave room for active inputs and content on short screens. Earlier groups
/// carry confirmation and editing controls, so truncation retains those first.
pub(crate) fn split(area: Rect, rows: usize) -> (Rect, Rect) {
    let reserve = if area.height >= 12 {
        6
    } else {
        (area.height / 2).max(1)
    };
    let height = (rows.min(u16::MAX as usize) as u16).min(area.height.saturating_sub(reserve));
    (
        Rect {
            height: area.height - height,
            ..area
        },
        Rect {
            y: area.y + area.height - height,
            height,
            ..area
        },
    )
}

pub(crate) fn render(frame: &mut ratatui::Frame, area: Rect, mut lines: Vec<Line<'static>>) {
    if area.height == 0 {
        return;
    }
    if area.height > 2 && lines.len() > area.height as usize {
        // Signal the height limit instead of silently losing the lower groups.
        let last = area.height as usize - 1;
        lines[last] = Line::from("… more hints: enlarge terminal");
    }
    frame.render_widget(
        Paragraph::new(lines).style(Style::default().bg(Color::Rgb(25, 30, 45))),
        area,
    );
}

/// Cell-aware wrapping for status/input text. Preserve whitespace and explicit
/// line breaks; unlike word wrapping this keeps long paths and the input tail.
pub(crate) fn wrap_text(text: &str, width: u16) -> Vec<Line<'static>> {
    if width == 0 {
        return vec![];
    }
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let mut row = String::new();
        for c in paragraph.chars() {
            let mut next = row.clone();
            next.push(c);
            if Line::from(next).width() > width as usize && !row.is_empty() {
                lines.push(Line::from(std::mem::take(&mut row)));
            }
            if Line::from(c.to_string()).width() <= width as usize {
                row.push(c);
            }
        }
        lines.push(Line::from(row));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{Terminal, backend::TestBackend};

    fn keymap() -> HashMap<KeyBinding, Action> {
        let mut map = HashMap::new();
        let map = crate::config::parse_keymap(include_str!("../resources/config.toml"), &mut map);
        assert!(!map.is_empty());
        map
    }
    fn state(context: Context) -> State {
        State {
            context,
            browser: true,
            focusable: true,
            command_chords: false,
            insert: true,
            workspace: true,
            filtered: false,
            rename_toggle: false,
        }
    }
    fn text(h: &Hints, width: u16) -> String {
        h.lines(width)
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn typing_contexts_do_not_advertise_intercepted_controls() {
        let map = keymap();
        for context in [Context::Search, Context::Naming, Context::Tags] {
            let t = text(&Hints::for_state(&state(context), &map), 100);
            assert!(!t.contains("keyboard reference"), "{t}");
            assert!(!t.contains("Space+"), "{t}");
            if context == Context::Tags {
                assert!(!t.contains("Tab decks"));
                assert!(!t.contains("previous deck"));
                assert!(t.contains("Enter save"));
            }
        }
    }

    #[test]
    fn configured_bindings_preserve_case_and_match_dispatch_layers() {
        let mut map = HashMap::new();
        map.insert(KeyBinding::AltChord(KeyCode::Char('Z')), Action::PaneLeft);
        map.insert(KeyBinding::Key(KeyCode::Char('z')), Action::PaneRight);
        map.insert(KeyBinding::SpaceChord(KeyCode::Char('K')), Action::Help);
        let mut s = state(Context::Command);
        s.command_chords = true;
        let t = text(&Hints::for_state(&s, &map), 100);
        assert_eq!(pane_keys(&map), [Some("Alt+Z".into()), None]);
        assert!(!t.contains("pane left"));
        assert!(!t.contains("pane right")); // plain binding not handled inside browser
        assert!(t.contains("Space+K keyboard reference"));
    }

    #[test]
    fn confirmation_and_rename_toggle_replace_ordinary_hints() {
        let map = keymap();
        for c in [
            Context::ConfirmLoad,
            Context::ConfirmDismiss,
            Context::ConfirmQuit,
        ] {
            let t = text(&Hints::for_state(&state(c), &map), 80);
            assert!(t.contains("y confirm"));
            assert!(t.contains("any other key cancel"));
            assert!(!t.contains("pane left"));
        }
        let mut s = state(Context::Tags);
        s.rename_toggle = true;
        let t = text(&Hints::for_state(&s, &map), 80);
        assert!(t.contains("toggle rename"));
        assert!(!t.contains("type text"));
    }

    #[test]
    fn whole_hints_wrap_without_loss_at_pane_breakpoints() {
        let mut h = Hints::default();
        h.group(
            "Actions",
            &[
                "Enter open/load",
                "e edit tags",
                "m move file",
                "n new playlist",
                "↑/↓ select",
            ],
        );
        for width in [140, 80, 49, 40, 20] {
            let lines = h.lines(width);
            assert!(lines.iter().all(|l| l.width() <= width as usize));
            let t = text(&h, width);
            for item in [
                "Enter open/load",
                "e edit tags",
                "m move file",
                "n new playlist",
                "↑/↓ select",
            ] {
                assert!(t.contains(item), "lost/split {item} at {width}: {t}");
            }
        }
        assert!(wrap_text("音楽 search █", 5).iter().all(|l| l.width() <= 5));
    }

    #[test]
    fn all_contexts_render_at_short_and_narrow_sizes() {
        let map = keymap();
        for context in [
            Context::Decks,
            Context::Command,
            Context::Search,
            Context::Move,
            Context::Naming,
            Context::Playlist,
            Context::Candidates,
            Context::Tags,
            Context::TagPreview,
            Context::Messages,
            Context::ConfirmLoad,
            Context::ConfirmDismiss,
            Context::ConfirmQuit,
        ] {
            for (width, height) in [(140, 40), (80, 24), (40, 12), (20, 6), (1, 1), (0, 0)] {
                let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
                terminal
                    .draw(|f| {
                        let lines = Hints::for_state(&state(context), &map).lines(width);
                        let (content, hints) = split(f.area(), lines.len());
                        assert_eq!(content.height + hints.height, height);
                        assert!(height == 0 || content.height > 0);
                        render(f, hints, lines);
                    })
                    .unwrap();
            }
        }
    }
}

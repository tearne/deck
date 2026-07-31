use std::io;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEventKind};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};

pub(crate) const AUDIO_EXTENSIONS: &[&str] = &["flac", "mp3", "ogg", "wav", "aac", "opus", "m4a"];

pub(crate) fn is_audio(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| AUDIO_EXTENSIONS.contains(&e.to_lowercase().as_str()))
        .unwrap_or(false)
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum EntryKind {
    Dir,
    Audio,
    Other,
}

pub(crate) struct BrowserEntry {
    pub(crate) name: String,
    pub(crate) path: std::path::PathBuf,
    pub(crate) kind: EntryKind,
}

/// The browser is modal. Letters type into the filter only in `Search`; in every
/// other mode they are commands, which is what frees the keyspace. `Command` and
/// `Search` are the primary modes (`Tab` toggles, and the last one is restored on
/// reopen); `Move` is a transient sub-mode entered to pick a destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BrowserMode {
    Command,
    Search,
    Move,
}

pub(crate) struct BrowserState {
    pub(crate) cwd: std::path::PathBuf,
    pub(crate) entries: Vec<BrowserEntry>,
    pub(crate) cursor: usize,
    pub(crate) workspace: Option<std::path::PathBuf>,
    pub(crate) search_term: String,
    pub(crate) search_results: Option<Vec<std::path::PathBuf>>,
    /// Flat list of all audio files under the workspace; populated on first search keystroke.
    workspace_files: Option<Vec<std::path::PathBuf>>,
    pub(crate) mode: BrowserMode,
}

impl BrowserState {
    pub(crate) fn new(dir: std::path::PathBuf, workspace: Option<std::path::PathBuf>) -> io::Result<Self> {
        let dir = std::fs::canonicalize(&dir).unwrap_or(dir);
        let mut entries = Vec::new();

        if dir.parent().is_some() {
            entries.push(BrowserEntry {
                name: "..".to_string(),
                path: dir.parent().unwrap().to_path_buf(),
                kind: EntryKind::Dir,
            });
        }

        let mut raw: Vec<_> = std::fs::read_dir(&dir)?.filter_map(|e| e.ok()).collect();
        raw.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());

        let mut dirs  = Vec::new();
        let mut audio = Vec::new();
        let mut other = Vec::new();
        for entry in raw {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if path.is_dir() {
                dirs.push(BrowserEntry { name, path, kind: EntryKind::Dir });
            } else if is_audio(&path) {
                audio.push(BrowserEntry { name, path, kind: EntryKind::Audio });
            } else {
                other.push(BrowserEntry { name, path, kind: EntryKind::Other });
            }
        }
        entries.extend(dirs);
        entries.extend(audio);
        entries.extend(other);

        // Start on the first selectable entry that isn't `..` so Enter navigates
        // into content rather than immediately going back up. `..` is reachable via Up.
        let cursor = entries
            .iter()
            .position(|e| Self::is_selectable(&e.kind) && e.name != "..")
            .unwrap_or(0);

        Ok(Self { cwd: dir, entries, cursor, workspace, search_term: String::new(), search_results: None, workspace_files: None, mode: BrowserMode::Command })
    }

    pub(crate) fn is_selectable(kind: &EntryKind) -> bool {
        matches!(kind, EntryKind::Dir | EntryKind::Audio)
    }

    /// The primary mode to restore on reopen — `Move` is transient and collapses
    /// to `Command`.
    pub(crate) fn primary_mode(&self) -> BrowserMode {
        match self.mode {
            BrowserMode::Search => BrowserMode::Search,
            _ => BrowserMode::Command,
        }
    }

    /// Whether the cursor may rest on an entry. Picking a move destination stops
    /// only on folders, so tracks never take the highlight while choosing.
    fn is_cursor_stop(&self, kind: &EntryKind) -> bool {
        if self.mode == BrowserMode::Move {
            matches!(kind, EntryKind::Dir)
        } else {
            Self::is_selectable(kind)
        }
    }

    /// Place the cursor on the first navigable entry, skipping `..`.
    pub(crate) fn snap_to_cursor_stop(&mut self) {
        self.cursor = self.entries.iter()
            .position(|e| self.is_cursor_stop(&e.kind) && e.name != "..")
            .or_else(|| self.entries.iter().position(|e| self.is_cursor_stop(&e.kind)))
            .unwrap_or(0);
    }

    pub(crate) fn move_down(&mut self) {
        let next = (self.cursor + 1..self.entries.len())
            .find(|&i| self.is_cursor_stop(&self.entries[i].kind));
        if let Some(i) = next {
            self.cursor = i;
        }
    }

    pub(crate) fn move_up(&mut self) {
        let prev = (0..self.cursor)
            .rev()
            .find(|&i| self.is_cursor_stop(&self.entries[i].kind));
        if let Some(i) = prev {
            self.cursor = i;
        }
    }

    /// Returns the path of the currently highlighted audio file, or `None` if
    /// the cursor is on a directory or non-audio entry.
    pub(crate) fn highlighted_audio_path(&self) -> Option<std::path::PathBuf> {
        if let Some(ref results) = self.search_results {
            results.get(self.cursor).cloned()
        } else {
            self.entries.get(self.cursor).and_then(|e| {
                if e.kind == EntryKind::Audio { Some(e.path.clone()) } else { None }
            })
        }
    }

    /// Recompute the filtered result list from the current term. With a workspace
    /// set, search runs recursively beneath it; otherwise it filters the current
    /// directory's own listing. An empty term clears the filter.
    fn update_search(&mut self) {
        self.cursor = 0;
        if self.search_term.is_empty() {
            self.search_results = None;
            return;
        }
        if let Some(ws) = self.workspace.clone() {
            let files = self.workspace_files.get_or_insert_with(|| collect_workspace_files(&ws));
            self.search_results = Some(run_search(&self.search_term, files, &ws));
        } else {
            let matcher = SkimMatcherV2::default();
            let mut scored: Vec<(i64, std::path::PathBuf)> = self
                .entries
                .iter()
                .filter(|e| e.name != ".." && Self::is_selectable(&e.kind))
                .filter_map(|e| matcher.fuzzy_match(&e.name, &self.search_term).map(|s| (s, e.path.clone())))
                .collect();
            scored.sort_by(|a, b| b.0.cmp(&a.0));
            self.search_results = Some(scored.into_iter().map(|(_, p)| p).collect());
        }
    }

    /// Move the cursor down over whichever list is showing (results or entries).
    fn nav_down(&mut self) {
        if let Some(ref results) = self.search_results {
            if self.cursor + 1 < results.len() { self.cursor += 1; }
        } else {
            self.move_down();
        }
    }

    fn nav_up(&mut self) {
        if self.search_results.is_some() {
            if self.cursor > 0 { self.cursor -= 1; }
        } else {
            self.move_up();
        }
    }

    /// Rebuild the browser at `dir`, preserving the workspace and mode.
    fn navigate_to(&mut self, dir: std::path::PathBuf) -> io::Result<()> {
        let workspace = self.workspace.clone();
        let mode = self.mode;
        *self = BrowserState::new(dir, workspace)?;
        self.mode = mode;
        if mode == BrowserMode::Move { self.snap_to_cursor_stop(); }
        Ok(())
    }
}

/// Compute a human-readable relative path from `base` to `target`, using `./` and `../` notation.
fn relative_path(base: &std::path::Path, target: &std::path::Path) -> String {
    if let Ok(rel) = target.strip_prefix(base) {
        let s = rel.display().to_string();
        if s.is_empty() { "./".to_string() } else { format!("./{s}") }
    } else {
        let base_comps: Vec<_> = base.components().collect();
        let target_comps: Vec<_> = target.components().collect();
        let common = base_comps.iter().zip(target_comps.iter())
            .take_while(|(a, b)| a == b)
            .count();
        let up = base_comps.len() - common;
        let mut parts = vec!["..".to_string(); up];
        parts.extend(target_comps[common..].iter().map(|c| c.as_os_str().to_string_lossy().into_owned()));
        parts.join("/")
    }
}

fn collect_workspace_files(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(read) = std::fs::read_dir(&dir) else { continue };
        let mut children: Vec<_> = read.filter_map(|e| e.ok()).collect();
        children.sort_by_key(|e| e.file_name().to_string_lossy().to_lowercase());
        for entry in children {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if is_audio(&path) {
                files.push(path);
            }
        }
    }
    files
}

fn run_search(term: &str, files: &[std::path::PathBuf], workspace: &std::path::Path) -> Vec<std::path::PathBuf> {
    let matcher = SkimMatcherV2::default();
    let mut scored: Vec<(i64, &std::path::PathBuf)> = files
        .iter()
        .filter_map(|p| {
            let display = p.strip_prefix(workspace).unwrap_or(p).display().to_string();
            matcher.fuzzy_match(&display, term).map(|score| (score, p))
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    scored.into_iter().map(|(_, p)| p.clone()).collect()
}

pub(crate) enum BrowserResult {
    Selected(std::path::PathBuf),
    DirectoryChosen(std::path::PathBuf),
    WorkspaceSet(std::path::PathBuf),
    WorkspaceCleared,
    ReturnToPlayer,
}

/// The colour identity and key legend for a mode — the visual signal of which
/// mode the browser is in. Command amber, Search cyan, Move blue: distinct hues
/// so the mode reads at a glance.
struct ModeTheme {
    accent: Color,
    highlight_fg: Color,
    highlight_bg: Color,
    label: &'static str,
    legend: &'static str,
}

fn mode_theme(mode: BrowserMode) -> ModeTheme {
    match mode {
        BrowserMode::Command => ModeTheme {
            accent: Color::Rgb(210, 170, 80),
            highlight_fg: Color::Yellow,
            highlight_bg: Color::Rgb(60, 50, 0),
            label: "COMMAND",
            legend: "j/k move · Enter/l open · h up · / search · @ workspace · Esc exit",
        },
        BrowserMode::Search => ModeTheme {
            accent: Color::Rgb(100, 180, 220),
            highlight_fg: Color::Rgb(210, 235, 255),
            highlight_bg: Color::Rgb(20, 50, 70),
            label: "SEARCH",
            legend: "type to filter · ↑/↓ move · Enter open · Tab command · Esc exit",
        },
        BrowserMode::Move => ModeTheme {
            accent: Color::Rgb(150, 190, 250),
            highlight_fg: Color::Rgb(200, 220, 255),
            highlight_bg: Color::Rgb(30, 50, 90),
            label: "MOVE",
            legend: "j/k move · Enter/l into · h up · y move here · Esc cancel (exit)",
        },
    }
}

pub(crate) fn render_browser(
    frame: &mut ratatui::Frame,
    area: ratatui::layout::Rect,
    state: &BrowserState,
    deck_slot: usize,
) {
    let bg    = Color::Rgb(20, 20, 38);
    let theme = mode_theme(state.mode);
    let border_style = Style::default().fg(theme.accent).bg(bg);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    // Top bar: mode-dependent. Search shows the filter field; Move a destination
    // banner; Command the workspace status.
    match state.mode {
        BrowserMode::Search => {
            let mut spans = vec![
                ratatui::text::Span::styled(" search: ", Style::default().fg(theme.accent).bg(bg)),
            ];
            if state.search_term.is_empty() {
                spans.push(ratatui::text::Span::styled("type to filter", Style::default().fg(Color::Rgb(60, 60, 80)).bg(bg)));
            } else {
                spans.push(ratatui::text::Span::styled(state.search_term.clone(), Style::default().fg(Color::White).bg(bg)));
            }
            spans.push(ratatui::text::Span::styled("█", Style::default().fg(theme.accent).bg(bg)));
            frame.render_widget(Paragraph::new(Line::from(spans)).style(Style::default().bg(bg)), chunks[0]);
        }
        BrowserMode::Move => {
            let name = state.cwd.file_name().and_then(|n| n.to_str()).unwrap_or("/");
            frame.render_widget(
                Paragraph::new(format!(" Move here → {name}    y: confirm   Esc: cancel"))
                    .style(Style::default().fg(theme.accent).bg(bg)),
                chunks[0],
            );
        }
        BrowserMode::Command => {
            let msg = if state.workspace.is_some() {
                " workspace set · ' to clear · / to search"
            } else {
                " Press @ to set this directory as your search workspace"
            };
            frame.render_widget(
                Paragraph::new(msg).style(Style::default().fg(Color::Rgb(80, 100, 140)).bg(bg)),
                chunks[0],
            );
        }
    }

    // List: search results or directory entries.
    let items: Vec<ListItem> = if let Some(ref results) = state.search_results {
        let base = state.workspace.as_deref().unwrap_or(&state.cwd);
        results.iter().map(|p| {
            let label = p.strip_prefix(base).unwrap_or(p).display().to_string();
            ListItem::new(label).style(Style::default().fg(Color::Yellow))
        }).collect()
    } else {
        state.entries.iter().map(|e| {
            // In Move mode only folders are choosable, so they read in clear blue
            // while tracks and other files dim back to signal "not selectable".
            let (label, color) = if state.mode == BrowserMode::Move {
                match e.kind {
                    EntryKind::Dir   => (format!("{}/", e.name), Color::Rgb(120, 160, 230)),
                    EntryKind::Audio => (e.name.clone(), Color::Rgb(60, 80, 120)),
                    EntryKind::Other => (e.name.clone(), Color::Rgb(45, 55, 85)),
                }
            } else {
                match e.kind {
                    EntryKind::Dir   => (format!("{}/", e.name), Color::Rgb(80, 110, 180)),
                    EntryKind::Audio => (e.name.clone(), Color::Yellow),
                    EntryKind::Other => (e.name.clone(), Color::Rgb(60, 60, 80)),
                }
            };
            ListItem::new(label).style(Style::default().fg(color))
        }).collect()
    };

    // Title: workspace root (bright) + relative path from workspace to cwd (dim).
    // Falls back to full cwd when no workspace is set.
    let path_title = if let Some(ws) = &state.workspace {
        let rel_str = relative_path(ws, &state.cwd);
        Line::from(vec![
            ratatui::text::Span::styled(
                format!(" @: {} ", ws.display()),
                Style::default().fg(Color::Yellow).bg(bg),
            ),
            ratatui::text::Span::styled(
                format!("[{}] ", rel_str),
                Style::default().fg(Color::Rgb(80, 80, 60)).bg(bg),
            ),
        ])
    } else {
        Line::from(ratatui::text::Span::styled(
            format!(" {} ", state.cwd.display()),
            Style::default().fg(Color::Yellow).bg(bg),
        ))
    };

    let result_count_title = state.search_results.as_ref().map(|r| {
        Line::from(ratatui::text::Span::styled(
            format!(" {} results ", r.len()),
            Style::default().fg(Color::Rgb(100, 140, 220)).bg(bg),
        )).alignment(Alignment::Left)
    });

    let mut block = Block::default()
        .title(path_title.alignment(Alignment::Left))
        .title(Line::from(ratatui::text::Span::styled(format!(" deck {} ", deck_slot + 1), Style::default().fg(Color::Yellow).bg(bg))).alignment(Alignment::Right))
        .border_style(border_style)
        .style(Style::default().bg(bg))
        .borders(Borders::ALL);
    if let Some(t) = result_count_title {
        // Second left title sits below the path title — used during search to show match count.
        block = block.title_bottom(t);
    }

    let highlight_style = Style::default()
        .fg(theme.highlight_fg)
        .bg(theme.highlight_bg)
        .add_modifier(Modifier::BOLD);
    let list = List::new(items)
        .block(block)
        .highlight_style(highlight_style);

    let mut list_state = ListState::default().with_selected(Some(state.cursor));
    frame.render_stateful_widget(list, chunks[1], &mut list_state);

    // Status bar: the mode label (in its accent) followed by that mode's legend.
    let status = Line::from(vec![
        ratatui::text::Span::styled(
            format!(" {} ", theme.label),
            Style::default().fg(Color::Black).bg(theme.accent).add_modifier(Modifier::BOLD),
        ),
        ratatui::text::Span::styled(
            format!("  {}", theme.legend),
            Style::default().fg(Color::Rgb(120, 130, 160)).bg(bg),
        ),
    ]);
    frame.render_widget(Paragraph::new(status).style(Style::default().bg(bg)), chunks[2]);
}

pub(crate) fn handle_browser_key(
    state: &mut BrowserState,
    key: crossterm::event::KeyEvent,
) -> io::Result<Option<BrowserResult>> {
    // Repeat counts as a press: terminals without key-release reporting deliver
    // auto-repeat as presses anyway, so held-key behaviour stays uniform.
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) { return Ok(None); }
    match state.mode {
        BrowserMode::Command => command_key(state, key),
        BrowserMode::Search => search_key(state, key),
        BrowserMode::Move => move_key(state, key),
    }
}

/// Open the highlighted entry: descend into a directory, or select an audio file.
fn open_highlighted(state: &mut BrowserState) -> io::Result<Option<BrowserResult>> {
    if let Some(ref results) = state.search_results {
        return Ok(results.get(state.cursor).cloned().map(BrowserResult::Selected));
    }
    let Some(entry) = state.entries.get(state.cursor) else { return Ok(None) };
    match entry.kind {
        EntryKind::Dir => {
            state.navigate_to(entry.path.clone())?;
            Ok(None)
        }
        EntryKind::Audio => Ok(Some(BrowserResult::Selected(entry.path.clone()))),
        EntryKind::Other => Ok(None),
    }
}

/// Ascend to the parent directory.
fn go_up(state: &mut BrowserState) -> io::Result<()> {
    if let Some(parent) = state.cwd.parent().map(|p| p.to_path_buf()) {
        state.navigate_to(parent)?;
    }
    Ok(())
}

fn set_workspace(state: &mut BrowserState) -> Option<BrowserResult> {
    state.workspace = Some(state.cwd.clone());
    state.workspace_files = None;
    Some(BrowserResult::WorkspaceSet(state.cwd.clone()))
}

fn clear_workspace(state: &mut BrowserState) -> Option<BrowserResult> {
    state.workspace = None;
    state.workspace_files = None;
    state.search_term.clear();
    state.search_results = None;
    state.cursor = 0;
    Some(BrowserResult::WorkspaceCleared)
}

fn command_key(state: &mut BrowserState, key: crossterm::event::KeyEvent) -> io::Result<Option<BrowserResult>> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => { state.nav_up(); Ok(None) }
        KeyCode::Down | KeyCode::Char('j') => { state.nav_down(); Ok(None) }
        KeyCode::Enter | KeyCode::Char('l') => open_highlighted(state),
        KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => { go_up(state)?; Ok(None) }
        KeyCode::Tab | KeyCode::Char('/') => { state.mode = BrowserMode::Search; Ok(None) }
        KeyCode::Char('@') => Ok(set_workspace(state)),
        KeyCode::Char('\'') => Ok(clear_workspace(state)),
        KeyCode::Esc => Ok(Some(BrowserResult::ReturnToPlayer)),
        _ => Ok(None),
    }
}

fn search_key(state: &mut BrowserState, key: crossterm::event::KeyEvent) -> io::Result<Option<BrowserResult>> {
    match key.code {
        KeyCode::Up => { state.nav_up(); Ok(None) }
        KeyCode::Down => { state.nav_down(); Ok(None) }
        KeyCode::Enter => open_highlighted(state),
        KeyCode::Tab => { state.mode = BrowserMode::Command; Ok(None) }
        KeyCode::Esc => Ok(Some(BrowserResult::ReturnToPlayer)),
        KeyCode::Backspace => {
            state.search_term.pop();
            state.update_search();
            Ok(None)
        }
        KeyCode::Char(c) => {
            state.search_term.push(c);
            state.update_search();
            Ok(None)
        }
        _ => Ok(None),
    }
}

fn move_key(state: &mut BrowserState, key: crossterm::event::KeyEvent) -> io::Result<Option<BrowserResult>> {
    match key.code {
        KeyCode::Up | KeyCode::Char('k') => { state.nav_up(); Ok(None) }
        KeyCode::Down | KeyCode::Char('j') => { state.nav_down(); Ok(None) }
        KeyCode::Enter | KeyCode::Char('l') => {
            // Only directories are stops here, so this always descends.
            open_highlighted(state)
        }
        KeyCode::Backspace | KeyCode::Left | KeyCode::Char('h') => { go_up(state)?; Ok(None) }
        KeyCode::Char('y') => Ok(Some(BrowserResult::DirectoryChosen(state.cwd.clone()))),
        KeyCode::Esc => Ok(Some(BrowserResult::ReturnToPlayer)),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_state(kinds: &[EntryKind]) -> BrowserState {
        BrowserState {
            cwd: PathBuf::from("/test"),
            entries: kinds
                .iter()
                .enumerate()
                .map(|(i, k)| BrowserEntry {
                    name: format!("entry{i}"),
                    path: PathBuf::from(format!("/test/entry{i}")),
                    kind: k.clone(),
                })
                .collect(),
            cursor: 0,
            workspace: None,
            search_term: String::new(),
            search_results: None,
            workspace_files: None,
            mode: BrowserMode::Command,
        }
    }

    #[test]
    fn test_is_audio_known_extensions() {
        assert!(is_audio(&PathBuf::from("track.flac")));
        assert!(is_audio(&PathBuf::from("track.mp3")));
        assert!(is_audio(&PathBuf::from("track.ogg")));
        assert!(is_audio(&PathBuf::from("track.wav")));
    }

    #[test]
    fn test_is_audio_case_insensitive() {
        assert!(is_audio(&PathBuf::from("track.FLAC")));
        assert!(is_audio(&PathBuf::from("track.Mp3")));
    }

    #[test]
    fn test_is_audio_non_audio() {
        assert!(!is_audio(&PathBuf::from("readme.txt")));
        assert!(!is_audio(&PathBuf::from("noextension")));
        assert!(!is_audio(&PathBuf::from("image.png")));
    }

    #[test]
    fn test_dirs_and_audio_are_selectable() {
        assert!(BrowserState::is_selectable(&EntryKind::Dir));
        assert!(BrowserState::is_selectable(&EntryKind::Audio));
        assert!(!BrowserState::is_selectable(&EntryKind::Other));
    }

    #[test]
    fn test_cursor_down_skips_other() {
        // [Audio, Other, Audio] — down from 0 should land on 2
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Other, EntryKind::Audio]);
        state.cursor = 0;
        state.move_down();
        assert_eq!(state.cursor, 2);
    }

    #[test]
    fn test_cursor_up_skips_other() {
        // [Audio, Other, Audio] — up from 2 should land on 0
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Other, EntryKind::Audio]);
        state.cursor = 2;
        state.move_up();
        assert_eq!(state.cursor, 0);
    }

    #[test]
    fn test_cursor_down_does_not_pass_end() {
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Audio]);
        state.cursor = 1;
        state.move_down();
        assert_eq!(state.cursor, 1);
    }

    #[test]
    fn test_cursor_up_does_not_pass_start() {
        let mut state = make_state(&[EntryKind::Audio, EntryKind::Audio]);
        state.cursor = 0;
        state.move_up();
        assert_eq!(state.cursor, 0);
    }
}

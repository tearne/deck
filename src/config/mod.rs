use crossterm::event::KeyCode;

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Action {
    Quit, Help, VinylModeToggle,
    ZoomIn, ZoomOut, HeightIncrease, HeightDecrease,
    LatencyIncrease, LatencyDecrease,
    FpsIncrease, FpsDecrease,
    PaletteCycle,
    NudgeModeToggle,
    ArtCycle,
    // Selected-deck controls
    SelectDeck1, SelectDeck2, SelectDeck3,
    PlayPause, OpenBrowser,
    PitchUp, PitchDown, PitchReset,
    BpmTap, MetronomeToggle, DetectBpm,
    LoopTap, LoopExit,
    BpmIncrease, BpmDecrease,
    BaseBpmIncrease, BaseBpmDecrease,
    SpeedReset,
    NudgeForward, NudgeBackward,
    OffsetIncrease, OffsetDecrease,
    JumpForward1bt,  JumpBackward1bt,
    JumpForward1b,   JumpBackward1b,
    JumpForward4b,   JumpBackward4b,
    JumpForward8b,   JumpBackward8b,
    JumpForward16b,  JumpBackward16b,
    JumpForward32b,  JumpBackward32b,
    JumpForward64b,  JumpBackward64b,
    Cue, CuePlay,
    PflLevelUp, PflLevelDown, PflLevelReset, PflOnOff,
    // Mixer — per-deck (addressed directly regardless of selected deck)
    Deck1LevelUp, Deck1LevelDown, Deck1LevelMax, Deck1LevelMin,
    Deck1FilterIncrease, Deck1FilterDecrease, Deck1FilterReset,
    Deck1FilterSlopeIncrease, Deck1FilterSlopeDecrease,
    Deck1GainIncrease, Deck1GainDecrease,
    Deck2LevelUp, Deck2LevelDown, Deck2LevelMax, Deck2LevelMin,
    Deck2FilterIncrease, Deck2FilterDecrease, Deck2FilterReset,
    Deck2FilterSlopeIncrease, Deck2FilterSlopeDecrease,
    Deck2GainIncrease, Deck2GainDecrease,
    Deck3LevelUp, Deck3LevelDown, Deck3LevelMax, Deck3LevelMin,
    Deck3FilterIncrease, Deck3FilterDecrease, Deck3FilterReset,
    Deck3FilterSlopeIncrease, Deck3FilterSlopeDecrease,
    Deck3GainIncrease, Deck3GainDecrease,
    // Deck swap
    SwapDeck1Deck2, SwapDeck2Deck3,
}

pub(crate) static ACTION_NAMES: &[(&str, Action)] = &[
    ("quit",              Action::Quit),
    ("help",              Action::Help),

    ("vinyl_mode_toggle",   Action::VinylModeToggle),
    ("zoom_in",           Action::ZoomIn),
    ("zoom_out",          Action::ZoomOut),
    ("height_increase",   Action::HeightIncrease),
    ("height_decrease",   Action::HeightDecrease),
    ("latency_increase",  Action::LatencyIncrease),
    ("latency_decrease",  Action::LatencyDecrease),
    ("fps_increase",      Action::FpsIncrease),
    ("fps_decrease",      Action::FpsDecrease),
    ("palette_cycle",     Action::PaletteCycle),
    ("nudge_mode_toggle",  Action::NudgeModeToggle),
    ("art_cycle",          Action::ArtCycle),
    // Selected-deck controls
    ("select_deck1",        Action::SelectDeck1),
    ("select_deck2",        Action::SelectDeck2),
    ("select_deck3",        Action::SelectDeck3),
    ("play_pause",          Action::PlayPause),
    ("open_browser",        Action::OpenBrowser),
    ("pitch_up",            Action::PitchUp),
    ("pitch_down",          Action::PitchDown),
    ("pitch_reset",         Action::PitchReset),
    ("bpm_tap",             Action::BpmTap),
    ("metronome_toggle",    Action::MetronomeToggle),
    ("detect_bpm",          Action::DetectBpm),
    ("loop_tap",            Action::LoopTap),
    ("loop_exit",           Action::LoopExit),
    ("bpm_increase",        Action::BpmIncrease),
    ("bpm_decrease",        Action::BpmDecrease),
    ("base_bpm_increase",   Action::BaseBpmIncrease),
    ("base_bpm_decrease",   Action::BaseBpmDecrease),
    ("speed_reset",         Action::SpeedReset),
    ("nudge_forward",       Action::NudgeForward),
    ("nudge_backward",      Action::NudgeBackward),
    ("offset_increase",     Action::OffsetIncrease),
    ("offset_decrease",     Action::OffsetDecrease),
    ("jump_forward_1bt",    Action::JumpForward1bt),
    ("jump_backward_1bt",   Action::JumpBackward1bt),
    ("jump_forward_1b",     Action::JumpForward1b),
    ("jump_backward_1b",    Action::JumpBackward1b),
    ("jump_forward_4b",     Action::JumpForward4b),
    ("jump_backward_4b",    Action::JumpBackward4b),
    ("jump_forward_8b",     Action::JumpForward8b),
    ("jump_backward_8b",    Action::JumpBackward8b),
    ("jump_forward_16b",    Action::JumpForward16b),
    ("jump_backward_16b",   Action::JumpBackward16b),
    ("jump_forward_32b",    Action::JumpForward32b),
    ("jump_backward_32b",   Action::JumpBackward32b),
    ("jump_forward_64b",    Action::JumpForward64b),
    ("jump_backward_64b",   Action::JumpBackward64b),
    ("cue",                 Action::Cue),
    ("cue_play",            Action::CuePlay),
    ("pfl_up",              Action::PflLevelUp),
    ("pfl_down",            Action::PflLevelDown),
    ("pfl_reset",           Action::PflLevelReset),
    ("pfl_on_off",          Action::PflOnOff),
    // Mixer — per-deck
    ("deck1_level_up",          Action::Deck1LevelUp),
    ("deck1_level_down",        Action::Deck1LevelDown),
    ("deck1_level_max",         Action::Deck1LevelMax),
    ("deck1_level_min",         Action::Deck1LevelMin),
    ("deck1_filter_increase",        Action::Deck1FilterIncrease),
    ("deck1_filter_decrease",        Action::Deck1FilterDecrease),
    ("deck1_filter_reset",           Action::Deck1FilterReset),
    ("deck1_filter_slope_increase",  Action::Deck1FilterSlopeIncrease),
    ("deck1_filter_slope_decrease",  Action::Deck1FilterSlopeDecrease),
    ("deck1_gain_increase",     Action::Deck1GainIncrease),
    ("deck1_gain_decrease",     Action::Deck1GainDecrease),
    ("deck2_level_up",          Action::Deck2LevelUp),
    ("deck2_level_down",        Action::Deck2LevelDown),
    ("deck2_level_max",         Action::Deck2LevelMax),
    ("deck2_level_min",         Action::Deck2LevelMin),
    ("deck2_filter_increase",        Action::Deck2FilterIncrease),
    ("deck2_filter_decrease",        Action::Deck2FilterDecrease),
    ("deck2_filter_reset",           Action::Deck2FilterReset),
    ("deck2_filter_slope_increase",  Action::Deck2FilterSlopeIncrease),
    ("deck2_filter_slope_decrease",  Action::Deck2FilterSlopeDecrease),
    ("deck2_gain_increase",     Action::Deck2GainIncrease),
    ("deck2_gain_decrease",     Action::Deck2GainDecrease),
    ("deck3_level_up",          Action::Deck3LevelUp),
    ("deck3_level_down",        Action::Deck3LevelDown),
    ("deck3_level_max",         Action::Deck3LevelMax),
    ("deck3_level_min",         Action::Deck3LevelMin),
    ("deck3_filter_increase",        Action::Deck3FilterIncrease),
    ("deck3_filter_decrease",        Action::Deck3FilterDecrease),
    ("deck3_filter_reset",           Action::Deck3FilterReset),
    ("deck3_filter_slope_increase",  Action::Deck3FilterSlopeIncrease),
    ("deck3_filter_slope_decrease",  Action::Deck3FilterSlopeDecrease),
    ("deck3_gain_increase",     Action::Deck3GainIncrease),
    ("deck3_gain_decrease",     Action::Deck3GainDecrease),
    ("swap_deck1_deck2",        Action::SwapDeck1Deck2),
    ("swap_deck2_deck3",        Action::SwapDeck2Deck3),
];

#[derive(Hash, Eq, PartialEq)]
pub(crate) enum KeyBinding {
    Key(KeyCode),
    SpaceChord(KeyCode),
}

pub(crate) fn parse_key(s: &str) -> Option<KeyBinding> {
    if let Some(rest) = s.strip_prefix("space+") {
        return parse_bare_key(rest).map(KeyBinding::SpaceChord);
    }
    parse_bare_key(s).map(KeyBinding::Key)
}

pub(crate) fn parse_bare_key(s: &str) -> Option<KeyCode> {
    match s {
        "space"     => Some(KeyCode::Char(' ')),
        "left"      => Some(KeyCode::Left),
        "right"     => Some(KeyCode::Right),
        "up"        => Some(KeyCode::Up),
        "down"      => Some(KeyCode::Down),
        "enter"     => Some(KeyCode::Enter),
        "backspace" => Some(KeyCode::Backspace),
        "esc"       => Some(KeyCode::Esc),
        s if s.chars().count() == 1 => Some(KeyCode::Char(s.chars().next().unwrap())),
        other => {
            eprintln!("deck: unknown key {:?} in config — binding skipped", other);
            None
        }
    }
}

pub(crate) const DEFAULT_CONFIG: &str = include_str!("../../resources/config.toml");

pub(crate) const FPS_LEVELS: &[u32] = &[15, 20, 24, 30, 45, 60, 90, 120, 240];

pub(crate) struct DisplayConfig {
    pub(crate) playhead_position: u8,      // 0–100, clamped
    pub(crate) warning_threshold_secs: f32, // seconds before end to activate warning flash
    pub(crate) detail_height: usize,       // total rows per detail waveform (including 2-row tick area)
    pub(crate) target_fps: u32,
}

impl Default for DisplayConfig {
    fn default() -> Self { Self { playhead_position: 20, warning_threshold_secs: 30.0, detail_height: 5, target_fps: 60 } }
}

/// Finds or creates the config file and returns its text plus an optional notice.
pub(crate) fn resolve_config(use_local_config: bool) -> (String, Option<String>) {
    if use_local_config {
        let dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        return resolve_or_create(dir.join("config.toml"));
    }
    // Check next to the binary first, then ~/.config/tj/config.toml, then auto-create.
    let adjacent = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("config.toml")))
        .filter(|p| p.exists());
    if let Some(path) = adjacent {
        return (std::fs::read_to_string(&path).unwrap_or_default(), None);
    }
    let user_path = match home_dir() {
        Some(h) => h.join(".config/deck/config.toml"),
        None => return (DEFAULT_CONFIG.to_string(), None),
    };
    resolve_or_create(user_path)
}

/// Reads `path` if it exists, otherwise creates it from the embedded default.
fn resolve_or_create(path: std::path::PathBuf) -> (String, Option<String>) {
    if path.exists() {
        (std::fs::read_to_string(&path).unwrap_or_default(), None)
    } else {
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let notice = if std::fs::write(&path, DEFAULT_CONFIG).is_ok() {
            Some(format!("config created: {}", path.display()))
        } else {
            None
        };
        (DEFAULT_CONFIG.to_string(), notice)
    }
}

pub(crate) fn load_config(use_local_config: bool) -> (std::collections::HashMap<KeyBinding, Action>, DisplayConfig, Option<String>) {
    let (text, notice) = resolve_config(use_local_config);
    // Seed with defaults so any keys absent from the user config still work.
    let mut map = parse_keymap(DEFAULT_CONFIG, &mut std::collections::HashMap::new());
    let keymap = parse_keymap(&text, &mut map);
    let display = parse_display_config(&text);
    (keymap, display, notice)
}

pub(crate) fn snap_to_fps_level(fps: u32) -> u32 {
    *FPS_LEVELS.iter()
        .min_by_key(|&&level| (level as i64 - fps as i64).unsigned_abs())
        .unwrap_or(&120)
}

pub(crate) fn parse_display_config(text: &str) -> DisplayConfig {
    let parsed: toml::Value = match toml::from_str(text) {
        Ok(v) => v,
        Err(_) => return DisplayConfig::default(),
    };
    let display = parsed.get("display");
    let pos = display
        .and_then(|v| v.get("playhead_position"))
        .and_then(|v| v.as_integer())
        .unwrap_or(20)
        .clamp(0, 100) as u8;
    let warning_threshold_secs = display
        .and_then(|v| v.get("warning_threshold_secs"))
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        .unwrap_or(30.0)
        .max(0.0) as f32;
    let detail_height = display
        .and_then(|v| v.get("detail_height"))
        .and_then(|v| v.as_integer())
        .unwrap_or(5)
        .max(3) as usize;
    let target_fps = display
        .and_then(|v| v.get("target_fps"))
        .and_then(|v| v.as_integer())
        .map(|v| snap_to_fps_level(v.clamp(1, 500) as u32))
        .unwrap_or(120);
    DisplayConfig { playhead_position: pos, warning_threshold_secs, detail_height, target_fps }
}

pub(crate) fn parse_keymap(text: &str, map: &mut std::collections::HashMap<KeyBinding, Action>)
    -> std::collections::HashMap<KeyBinding, Action>
{
    let parsed: toml::Value = match toml::from_str(text) {
        Ok(v) => v,
        Err(e) => { eprintln!("deck: failed to parse config: {e}"); return std::mem::take(map); }
    };
    let keys = match parsed.get("keys").and_then(|v| v.as_table()) {
        Some(t) => t,
        None => return std::mem::take(map),
    };
    for (name, val) in keys {
        let action = match ACTION_NAMES.iter().find(|(n, _)| *n == name.as_str()) {
            Some((_, a)) => *a,
            None => { eprintln!("deck: unknown function {name:?} in config — skipped"); continue; }
        };
        let key_strs: Vec<&str> = if let Some(s) = val.as_str() {
            vec![s]
        } else if let Some(arr) = val.as_array() {
            arr.iter().filter_map(|v| v.as_str()).collect()
        } else {
            eprintln!("deck: key value for {name:?} must be a string or array of strings");
            continue;
        };
        for key_str in key_strs {
            if let Some(binding) = parse_key(key_str) {
                map.insert(binding, action);
            }
        }
    }
    std::mem::take(map)
}

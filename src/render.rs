use crate::state::{
    unix_now, unix_now_ms, Activity, ClickRegion, FlashMode, MenuAction, MenuClickRegion,
    NotifyMode, SessionInfo, SettingKey, State, ViewMode,
};
use std::fmt::Write;
use std::io::Write as IoWrite;
use zellij_tile::prelude::{InputMode, TabInfo};

struct Style {
    symbol: &'static str,
    r: u8,
    g: u8,
    b: u8,
}

fn activity_priority(activity: &Activity) -> u8 {
    match activity {
        Activity::Waiting => 8,
        Activity::Tool(_) => 7,
        Activity::Thinking => 6,
        Activity::Prompting => 5,
        Activity::Notification => 4,
        Activity::Init => 3,
        Activity::Done => 2,
        Activity::AgentDone => 1,
        Activity::Idle => 0,
    }
}

fn activity_style(activity: &Activity) -> Style {
    match activity {
        Activity::Init => Style { symbol: "◆", r: 122, g: 162, b: 247 },       // blue
        Activity::Thinking => Style { symbol: "●", r: 187, g: 154, b: 247 },    // purple
        Activity::Tool(name) => {
            let symbol = match name.as_str() {
                "Bash" => "⚡",
                "Read" | "Glob" | "Grep" => "◉",
                "Edit" | "Write" => "✎",
                "Task" => "⊜",
                "WebSearch" | "WebFetch" => "◈",
                _ => "⚙",
            };
            Style { symbol, r: 255, g: 158, b: 100 }  // orange
        }
        Activity::Prompting => Style { symbol: "▶", r: 158, g: 206, b: 106 },   // green
        Activity::Waiting => Style { symbol: "⚠", r: 247, g: 118, b: 142 },     // red
        Activity::Notification => Style { symbol: "◇", r: 224, g: 175, b: 104 }, // yellow
        Activity::Done => Style { symbol: "✓", r: 158, g: 206, b: 106 },         // green
        Activity::AgentDone => Style { symbol: "✓", r: 125, g: 207, b: 255 },    // cyan
        Activity::Idle => Style { symbol: "○", r: 86, g: 95, b: 137 },           // comment
    }
}

fn fg_c(c: Color) -> String {
    format!("\x1b[38;2;{};{};{}m", c.0, c.1, c.2)
}

fn bg_c(c: Color) -> String {
    format!("\x1b[48;2;{};{};{}m", c.0, c.1, c.2)
}

fn display_width(s: &str) -> usize {
    s.chars().count()
}

const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const ELAPSED_THRESHOLD: u64 = 30;

/// Powerline right-pointing solid arrow (thick separator between segments)
const ARROW: &str = "\u{e0b0}";
/// Powerline right-pointing thin arrow (thin separator within a segment)
const ARROW_THIN: &str = "\u{e0b1}";

type Color = (u8, u8, u8);

// -- Tokyo Night palette --
const BG: Color = (26, 27, 38);           // #1a1b26
const BG_DARK: Color = (22, 22, 30);      // #16161e
const BG_HIGHLIGHT: Color = (41, 46, 66); // #292e42
const FG: Color = (192, 202, 245);        // #c0caf5
const FG_DARK: Color = (169, 177, 214);   // #a9b1d6
const COMMENT: Color = (86, 95, 137);     // #565f89

const BLUE: Color = (122, 162, 247);      // #7aa2f7
const CYAN: Color = (125, 207, 255);      // #7dcfff
const GREEN: Color = (158, 206, 106);     // #9ece6a
const ORANGE: Color = (255, 158, 100);    // #ff9e64
const PURPLE: Color = (187, 154, 247);    // #bb9af7
const RED: Color = (247, 118, 142);       // #f7768e
const YELLOW: Color = (224, 175, 104);    // #e0af68

// Bar background — match terminal background
const BAR_BG: Color = BG;
// Flash: yellow tint
const FLASH_BG_BRIGHT: Color = (80, 70, 30);

/// Write a powerline solid arrow: fg=from, bg=to, then ARROW char.
fn arrow(buf: &mut String, col: &mut usize, from: Color, to: Color) {
    let _ = write!(buf, "{}{}{ARROW}", fg_c(from), bg_c(to));
    *col += 1;
}

/// Write a powerline thin arrow (within the same segment): fg=line_color, no bg change.
fn arrow_thin(buf: &mut String, col: &mut usize, line_color: Color) {
    let _ = write!(buf, "{}{ARROW_THIN}", fg_c(line_color));
    *col += 1;
}

fn format_elapsed(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

fn mode_style(mode: InputMode) -> (Color, &'static str) {
    match mode {
        InputMode::Normal => (GREEN, "NORMAL"),
        InputMode::Locked => (RED, "LOCKED"),
        InputMode::Pane => (CYAN, "PANE"),
        InputMode::Tab => (CYAN, "TAB"),
        InputMode::Resize => (YELLOW, "RESIZE"),
        InputMode::Move => (YELLOW, "MOVE"),
        InputMode::Scroll => (ORANGE, "SCROLL"),
        InputMode::EnterSearch => (ORANGE, "SEARCH"),
        InputMode::Search => (ORANGE, "SEARCH"),
        InputMode::RenameTab => (YELLOW, "RENAME"),
        InputMode::RenamePane => (YELLOW, "RENAME"),
        InputMode::Session => (PURPLE, "SESSION"),
        InputMode::Prompt => (PURPLE, "PROMPT"),
        InputMode::Tmux => (BLUE, "TMUX"),
    }
}

pub fn render_status_bar(state: &mut State, _rows: usize, cols: usize) {
    state.click_regions.clear();
    state.menu_click_regions.clear();

    let mut buf = String::with_capacity(cols * 4);
    let bar_bg = bg_c(BAR_BG);
    buf.push_str("\x1b[H\x1b[?7l\x1b[?25l");

    if cols < 5 {
        let _ = write!(buf, "{bar_bg}{:width$}{RESET}", "", width = cols);
        print!("{buf}");
        let _ = std::io::stdout().flush();
        return;
    }

    // === Left prefix: [session pill][arrow][space][arrow][mode pill][arrow] ===
    let (mode_color, mode_text) = mode_style(state.input_mode);
    let session_text = state
        .zellij_session_name
        .as_deref()
        .unwrap_or("zellij");

    let session_pill_text = format!(" {session_text} ");
    let session_pill_width = display_width(&session_pill_text);
    let mode_pill_text = format!(" {mode_text} ");
    let mode_pill_width = display_width(&mode_pill_text);

    // Total prefix width: session + arrow(1) + arrow(1) + mode + arrow(1)
    // (no leading arrow on session — flush left; no space between session and mode)
    let total_prefix_width = session_pill_width + 1 + 1 + mode_pill_width + 1;

    let mut col = 0usize;
    if total_prefix_width <= cols {
        // session text (no leading arrow) [sapphire → mode]
        let _ = write!(
            buf,
            "{}{}{BOLD}{session_pill_text}{RESET}",
            bg_c(BLUE), fg_c(BG_DARK),
        );
        col += session_pill_width;
        arrow(&mut buf, &mut col, BLUE, mode_color);

        // mode text [mode → bar_bg]
        let _ = write!(
            buf,
            "{}{}{BOLD}{mode_pill_text}{RESET}",
            bg_c(mode_color), fg_c(BG_DARK),
        );
        col += mode_pill_width;
        arrow(&mut buf, &mut col, mode_color, BAR_BG);
    } else if session_pill_width + 1 <= cols {
        let _ = write!(
            buf,
            "{}{}{BOLD}{session_pill_text}{RESET}",
            bg_c(BLUE), fg_c(BG_DARK),
        );
        col += session_pill_width;
        arrow(&mut buf, &mut col, BLUE, BAR_BG);
    }

    state.prefix_click_region = Some((0, col));
    let prefix_used = col;

    if col < cols {
        match state.view_mode {
            ViewMode::Normal => {
                render_tabs(state, &mut buf, &mut col, cols, prefix_used);
            }
            ViewMode::Settings => {
                let _ = write!(buf, "{bar_bg}");
                render_settings_menu(state, &mut buf, &mut col);
            }
        }
    }

    // Fill remaining with bar bg
    if col < cols {
        let remaining = cols - col;
        let _ = write!(buf, "{bar_bg}{:width$}", "", width = remaining);
    }
    let _ = write!(buf, "{RESET}");

    print!("{buf}");
    let _ = std::io::stdout().flush();
}

/// Render tabs in powerline style with Tokyo Night colors.
///
/// Each tab: `[bar→accent ARROW][accent: " index "][accent: THIN_ARROW][accent: " name "][accent→bar ARROW]`
///
/// Active tab: peach accent, inactive: surface0 accent.
/// Claude activity icon shown inline after name.
fn render_tabs(
    state: &mut State,
    buf: &mut String,
    col: &mut usize,
    cols: usize,
    _prefix_width: usize,
) {
    let now_s = unix_now();
    let now_ms = unix_now_ms();

    let mut tabs: Vec<&TabInfo> = state.tabs.iter().collect();
    tabs.sort_by_key(|t| t.position);

    let count = tabs.len();
    if count == 0 {
        return;
    }

    // For each tab, find the best Claude session
    let best_sessions: Vec<Option<&SessionInfo>> = tabs
        .iter()
        .map(|tab| {
            state
                .sessions
                .values()
                .filter(|s| s.tab_index == Some(tab.position))
                .max_by_key(|s| activity_priority(&s.activity))
        })
        .collect();

    // Pre-compute elapsed strings
    let elapsed_strs: Vec<Option<String>> = best_sessions
        .iter()
        .map(|session: &Option<&SessionInfo>| {
            if !state.settings.elapsed_time {
                return None;
            }
            session.and_then(|s| {
                let elapsed = now_s.saturating_sub(s.last_event_ts);
                if elapsed >= ELAPSED_THRESHOLD {
                    Some(format_elapsed(elapsed))
                } else {
                    None
                }
            })
        })
        .collect();

    // Compute max tab name length.
    // Each tab: leading_arrow(1) + " index "(~3-4) + thin_arrow(1) + " name " + trailing_arrow(1)
    // Claude tabs add " symbol"(2).
    let fixed_per_tab: usize = tabs.iter().map(|t| {
        let idx_str = format!("{}", t.position + 1);
        // leading_arrow + space + index + space + thin_arrow + trailing_space + trailing_arrow
        1 + 1 + idx_str.len() + 1 + 1 + 1 + 1
    }).sum();
    let claude_overhead: usize = best_sessions
        .iter()
        .map(|s| if s.is_some() { 2 } else { 0 })
        .sum();
    let elapsed_overhead: usize = elapsed_strs
        .iter()
        .map(|e| e.as_ref().map_or(0, |s| 1 + s.len()))
        .sum();

    let overhead = *col + fixed_per_tab + claude_overhead + elapsed_overhead + count; // +count for leading space in name area
    let max_name_len = if overhead < cols {
        ((cols - overhead) / count).min(20)
    } else {
        0
    };

    for (i, tab) in tabs.iter().enumerate() {
        if *col + 8 > cols {
            break;
        }

        let session = best_sessions[i];
        let is_claude = session.is_some();
        let is_active = tab.active;
        let tab_name = &tab.name;

        // Check flash
        let is_flash_bright = state
            .sessions
            .values()
            .filter(|s| s.tab_index == Some(tab.position))
            .any(|s| {
                state
                    .flash_deadlines
                    .get(&s.pane_id)
                    .map(|&deadline| now_ms < deadline && (now_ms / 250) % 2 == 0)
                    .unwrap_or(false)
            });

        // Tab background color: active=highlight, inactive=dark, flash=yellow
        let tab_bg = if is_flash_bright {
            FLASH_BG_BRIGHT
        } else if is_active {
            BG_HIGHLIGHT
        } else {
            BG_DARK
        };

        // Text color on the tab
        let tab_fg = if is_flash_bright {
            YELLOW
        } else if is_active {
            FG     // bright text on highlight bg
        } else {
            FG_DARK // dimmer text on dark bg
        };

        // Thin arrow line color (between index and name)
        let thin_color = if is_active { COMMENT } else { COMMENT };

        // Truncate name
        let char_count = tab_name.chars().count();
        let truncated = if max_name_len == 0 {
            String::new()
        } else if char_count > max_name_len {
            let s: String = tab_name.chars().take(max_name_len.saturating_sub(1)).collect();
            format!("{s}…")
        } else {
            tab_name.to_string()
        };

        let region_start = *col;

        // Leading arrow: [bar_bg → tab_bg]
        arrow(buf, col, BAR_BG, tab_bg);

        // Index part: " N "
        let idx_str = format!("{}", tab.position + 1);
        let _ = write!(
            buf,
            "{}{}{BOLD} {} {RESET}",
            bg_c(tab_bg), fg_c(tab_fg), idx_str,
        );
        *col += 1 + idx_str.len() + 1; // " N "

        // Thin arrow separator between index and name (stays on same bg)
        let _ = write!(buf, "{}", bg_c(tab_bg));
        arrow_thin(buf, col, thin_color);

        // Name part: " name "
        let _ = write!(
            buf,
            "{}{}{BOLD} ",
            bg_c(tab_bg), fg_c(tab_fg),
        );
        *col += 1;

        if !truncated.is_empty() {
            let _ = write!(buf, "{truncated}");
            *col += display_width(&truncated);
        }

        // Claude activity indicator
        if is_claude {
            let s = session.unwrap();
            let style = activity_style(&s.activity);

            if !matches!(s.activity, Activity::Idle) {
                let sym_fg = if is_flash_bright {
                    fg_c(YELLOW)
                } else {
                    format!("\x1b[38;2;{};{};{}m", style.r, style.g, style.b)
                };
                let _ = write!(buf, " {RESET}{}{sym_fg}{}", bg_c(tab_bg), style.symbol);
                *col += 1 + display_width(style.symbol);
            }

            if let Some(ref es) = elapsed_strs[i] {
                if *col + 1 + es.len() + 1 < cols {
                    let _ = write!(
                        buf,
                        " {RESET}{}{}{}",
                        bg_c(tab_bg), fg_c(COMMENT), es,
                    );
                    *col += 1 + es.len();
                }
            }
        }

        // Trailing space + closing arrow: [tab_bg → bar_bg]
        let _ = write!(buf, "{RESET}{}", bg_c(tab_bg));
        // trailing space before arrow
        let _ = write!(buf, " ");
        *col += 1;
        arrow(buf, col, tab_bg, BAR_BG);

        // Register click region
        if is_claude {
            let waiting_session = state
                .sessions
                .values()
                .filter(|s| s.tab_index == Some(tab.position))
                .find(|s| matches!(s.activity, Activity::Waiting));

            state.click_regions.push(ClickRegion {
                start_col: region_start,
                end_col: *col,
                tab_index: tab.position,
                pane_id: waiting_session.map_or(0, |s| s.pane_id),
                is_waiting: waiting_session.is_some(),
            });
        } else {
            state.click_regions.push(ClickRegion {
                start_col: region_start,
                end_col: *col,
                tab_index: tab.position,
                pane_id: 0,
                is_waiting: false,
            });
        }
    }
}

fn notify_mode_label(mode: NotifyMode) -> (&'static str, &'static str, String, String) {
    match mode {
        NotifyMode::Always => ("●", "Notify: always", fg_c(GREEN), fg_c(FG)),
        NotifyMode::Unfocused => ("◐", "Notify: unfocused", fg_c(YELLOW), fg_c(YELLOW)),
        NotifyMode::Never => ("○", "Notify: off", fg_c(COMMENT), fg_c(COMMENT)),
    }
}

fn flash_mode_label(mode: FlashMode) -> (&'static str, &'static str, String, String) {
    match mode {
        FlashMode::Persist => ("●", "Flash: persist", fg_c(GREEN), fg_c(FG)),
        FlashMode::Once => ("◐", "Flash: brief", fg_c(YELLOW), fg_c(YELLOW)),
        FlashMode::Off => ("○", "Flash: off", fg_c(COMMENT), fg_c(COMMENT)),
    }
}

/// Render a three-state toggle and register its click region.
fn render_tristate(
    buf: &mut String,
    col: &mut usize,
    state_regions: &mut Vec<MenuClickRegion>,
    key: SettingKey,
    symbol: &str,
    label: &str,
    sym_color: &str,
    label_color: &str,
) {
    let region_start = *col;
    let width = display_width(symbol) + 1 + label.len();
    *col += width;

    state_regions.push(MenuClickRegion {
        start_col: region_start,
        end_col: *col,
        action: MenuAction::ToggleSetting(key),
    });

    let _ = write!(buf, "{sym_color}{symbol} {label_color}{label}");
}

fn render_settings_menu(state: &mut State, buf: &mut String, col: &mut usize) {
    let _ = write!(buf, " ");
    *col += 1;

    // Notifications
    {
        let (symbol, label, sym_color, label_color) =
            notify_mode_label(state.settings.notifications);
        render_tristate(
            buf, col, &mut state.menu_click_regions,
            SettingKey::Notifications, symbol, label, &sym_color, &label_color,
        );
    }

    // Flash
    {
        let _ = write!(buf, "  ");
        *col += 2;
        let (symbol, label, sym_color, label_color) =
            flash_mode_label(state.settings.flash);
        render_tristate(
            buf, col, &mut state.menu_click_regions,
            SettingKey::Flash, symbol, label, &sym_color, &label_color,
        );
    }

    // Elapsed time
    {
        let _ = write!(buf, "  ");
        *col += 2;
        let enabled = state.settings.elapsed_time;
        let (symbol, sym_color, label_color) = if enabled {
            ("●", fg_c(GREEN), fg_c(FG))
        } else {
            ("○", fg_c(COMMENT), fg_c(COMMENT))
        };
        let label = if enabled { "Elapsed time: on" } else { "Elapsed time: off" };
        render_tristate(
            buf, col, &mut state.menu_click_regions,
            SettingKey::ElapsedTime, symbol, label, &sym_color, &label_color,
        );
    }

    // Close button
    let _ = write!(buf, "  ");
    *col += 2;
    let close_start = *col;
    let _ = write!(buf, "{}×", fg_c(RED));
    *col += 1;

    state.menu_click_regions.push(MenuClickRegion {
        start_col: close_start,
        end_col: *col,
        action: MenuAction::CloseMenu,
    });
}

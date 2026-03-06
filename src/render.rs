use crate::config::Color;
use crate::state::{unix_now, unix_now_ms, Activity, ClickRegion, SessionInfo, State};
use std::fmt::Write;
use std::io::Write as IoWrite;
use zellij_tile::prelude::TabInfo;

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

fn activity_symbol(activity: &Activity) -> &'static str {
    match activity {
        Activity::Init => "◆",
        Activity::Thinking => "●",
        Activity::Tool(name) => match name.as_str() {
            "Bash" => "⚡",
            "Read" | "Glob" | "Grep" => "◉",
            "Edit" | "Write" => "✎",
            "Task" => "⊜",
            "WebSearch" | "WebFetch" => "◈",
            _ => "⚙",
        },
        Activity::Prompting => "▶",
        Activity::Waiting => "⚠",
        Activity::Notification => "◇",
        Activity::Done | Activity::AgentDone => "✓",
        Activity::Idle => "○",
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

fn format_elapsed(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

pub fn render_status_bar(state: &mut State, _rows: usize, cols: usize) {
    state.click_regions.clear();

    let cfg = &state.config;
    let mut buf = String::with_capacity(cols * 4);
    let bar_bg = bg_c(cfg.bar_bg);
    buf.push_str("\x1b[H\x1b[?7l\x1b[?25l");

    if cols < 5 {
        let _ = write!(buf, "{bar_bg}{:width$}{RESET}", "", width = cols);
        print!("{buf}");
        let _ = std::io::stdout().flush();
        return;
    }

    // === Left prefix: [session pill][arrow][mode pill][arrow] ===
    let (mode_bg, mode_fg, mode_text) = cfg.mode_style(state.input_mode);
    let session_text = state
        .zellij_session_name
        .as_deref()
        .unwrap_or("zellij");

    let session_pill_text = format!(" {session_text} ");
    let session_pill_width = display_width(&session_pill_text);
    let mode_pill_text = format!(" {mode_text} ");
    let mode_pill_width = display_width(&mode_pill_text);

    let sep_left_width = display_width(&cfg.separator_left);
    let total_prefix_width = session_pill_width + sep_left_width + sep_left_width + mode_pill_width + sep_left_width;

    let mut col = 0usize;
    if total_prefix_width <= cols {
        // Session pill
        let _ = write!(
            buf,
            "{}{}{BOLD}{session_pill_text}{RESET}",
            bg_c(cfg.session_bg), fg_c(cfg.session_fg),
        );
        col += session_pill_width;
        // Arrow: session → mode
        let _ = write!(buf, "{}{}{}", fg_c(cfg.session_bg), bg_c(mode_bg), cfg.separator_left);
        col += sep_left_width;

        // Mode pill
        let _ = write!(
            buf,
            "{}{}{BOLD}{mode_pill_text}{RESET}",
            bg_c(mode_bg), fg_c(mode_fg),
        );
        col += mode_pill_width;
        // Arrow: mode → bar_bg
        let _ = write!(buf, "{}{}{}", fg_c(mode_bg), bar_bg, cfg.separator_left);
        col += sep_left_width;
    } else if session_pill_width + sep_left_width <= cols {
        let _ = write!(
            buf,
            "{}{}{BOLD}{session_pill_text}{RESET}",
            bg_c(cfg.session_bg), fg_c(cfg.session_fg),
        );
        col += session_pill_width;
        let _ = write!(buf, "{}{}{}", fg_c(cfg.session_bg), bar_bg, cfg.separator_left);
        col += sep_left_width;
    }

    if col < cols {
        render_tabs(state, &mut buf, &mut col, cols);
    }

    // Fill remaining with bar bg
    if col < cols {
        let remaining = cols - col;
        let _ = write!(buf, "{}{:width$}", bg_c(state.config.bar_bg), "", width = remaining);
    }
    let _ = write!(buf, "{RESET}");

    print!("{buf}");
    let _ = std::io::stdout().flush();
}

fn render_tabs(
    state: &mut State,
    buf: &mut String,
    col: &mut usize,
    cols: usize,
) {
    let cfg = &state.config;
    let now_s = unix_now();
    let now_ms = unix_now_ms();

    let mut tabs: Vec<&TabInfo> = state.tabs.iter().collect();
    tabs.sort_by_key(|t| t.position);

    let count = tabs.len();
    if count == 0 {
        return;
    }

    let sep_left_width = display_width(&cfg.separator_left);
    let sep_tab_width = display_width(&cfg.separator_tab);

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
            if !cfg.elapsed_time {
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

    // Compute max tab name length
    let fixed_per_tab: usize = tabs.iter().map(|t| {
        let idx_str = format!("{}", t.position + 1);
        // leading_sep + space + index + space + thin_sep + trailing_space + trailing_sep
        sep_left_width + 1 + idx_str.len() + 1 + sep_tab_width + 1 + sep_left_width
    }).sum();
    let claude_overhead: usize = best_sessions
        .iter()
        .map(|s| if s.is_some() { 2 } else { 0 })
        .sum();
    let elapsed_overhead: usize = elapsed_strs
        .iter()
        .map(|e| e.as_ref().map_or(0, |s| 1 + s.len()))
        .sum();

    let overhead = *col + fixed_per_tab + claude_overhead + elapsed_overhead + count;
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

        // Tab colors
        let tab_bg = if is_flash_bright {
            cfg.flash_bg
        } else if is_active {
            cfg.tab_active_bg
        } else {
            cfg.tab_inactive_bg
        };

        let tab_fg = if is_flash_bright {
            cfg.flash_fg
        } else if is_active {
            cfg.tab_active_fg
        } else {
            cfg.tab_inactive_fg
        };

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
        let _ = write!(buf, "{}{}{}", fg_c(cfg.bar_bg), bg_c(tab_bg), cfg.separator_left);
        *col += sep_left_width;

        // Index part: " N "
        let idx_str = format!("{}", tab.position + 1);
        let _ = write!(
            buf,
            "{}{}{BOLD} {} {RESET}",
            bg_c(tab_bg), fg_c(tab_fg), idx_str,
        );
        *col += 1 + idx_str.len() + 1;

        // Thin separator between index and name
        let _ = write!(buf, "{}{}{}", bg_c(tab_bg), fg_c(cfg.tab_separator_fg), cfg.separator_tab);
        *col += sep_tab_width;

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
            if !matches!(s.activity, Activity::Idle) {
                let symbol = activity_symbol(&s.activity);
                let icon_color = if is_flash_bright {
                    cfg.flash_fg
                } else {
                    cfg.activity_color(&s.activity)
                };
                let _ = write!(buf, " {RESET}{}{}{}", bg_c(tab_bg), fg_c(icon_color), symbol);
                *col += 1 + display_width(symbol);
            }

            if let Some(ref es) = elapsed_strs[i] {
                if *col + 1 + es.len() + 1 < cols {
                    let _ = write!(
                        buf,
                        " {RESET}{}{}{}",
                        bg_c(tab_bg), fg_c(cfg.elapsed_fg), es,
                    );
                    *col += 1 + es.len();
                }
            }
        }

        // Trailing space + closing arrow
        let _ = write!(buf, "{RESET}{} ", bg_c(tab_bg));
        *col += 1;
        let _ = write!(buf, "{}{}{}", fg_c(tab_bg), bg_c(cfg.bar_bg), cfg.separator_left);
        *col += sep_left_width;

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

use crossterm::event::{self, Event, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::{Screen, ScreenState};

pub struct ComposeFields {
    to: String,
    cc: String,
    bcc: String,
    body: String,
    body_cursor: (usize, usize), // (line, col)
    focused: usize,              // 0: to, 1: cc, 2: bcc, 3: body
}

impl Default for ComposeFields {
    fn default() -> Self {
        Self {
            to: Default::default(),
            cc: Default::default(),
            bcc: Default::default(),
            body: Default::default(),
            body_cursor: Default::default(),
            focused: Default::default(),
        }
    }
}

impl ComposeFields {
    pub fn render_compose(&self, f: &mut Frame) {
        let area = f.area();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(area);
        let fields = [
            ("To", &self.to, self.focused == 0),
            ("Cc", &self.cc, self.focused == 1),
            ("Bcc", &self.bcc, self.focused == 2),
        ];
        for (i, (label, value, focused)) in fields.iter().enumerate() {
            let block = Block::default().borders(Borders::ALL).title(*label);
            let style = if *focused {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default()
            };
            let para = Paragraph::new(value.as_str()).block(block).style(style);
            f.render_widget(para, chunks[i]);
        }
        // Body field with cursor
        let body_block = Block::default().borders(Borders::ALL).title("Body");
        let body_style = if self.focused == 3 {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        };
        let mut lines: Vec<String> = self.body.lines().map(|l| l.to_string()).collect();
        if lines.is_empty() {
            lines.push(String::new());
        }
        let (cur_line, cur_col) = self.body_cursor;
        if self.focused == 3 {
            if cur_line < lines.len() {
                let l = &mut lines[cur_line];
                if cur_col <= l.len() {
                    l.insert(cur_col, '▏'); // Unicode thin cursor
                } else {
                    l.push('▏');
                }
            } else {
                lines.push("▏".to_string());
            }
        }
        let para = Paragraph::new(lines.join("\n"))
            .block(body_block)
            .style(body_style);
        f.render_widget(para, chunks[3]);
        let help = Paragraph::new(
        "[Ctrl+S] Send | [Tab] Next | [Shift+Tab] Prev | [Esc] Cancel | [Arrows] Move | [Enter] Newline",
    )
    .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, chunks[4]);
    }
}

pub fn handle_compose(state: &mut ScreenState, event: Event) {
    if let Event::Key(KeyEvent {
        code, modifiers, ..
    }) = event
    {
        match (code, modifiers) {
            (crossterm::event::KeyCode::Char('s'), event::KeyModifiers::CONTROL) => {
                state.screen = Screen::Main
            }
            (crossterm::event::KeyCode::Tab, _) => {
                state.compose.focused = (state.compose.focused + 1) % 4
            }
            (crossterm::event::KeyCode::BackTab, _) => {
                state.compose.focused = (state.compose.focused + 3) % 4
            }
            (crossterm::event::KeyCode::Esc, _) => state.screen = Screen::Main,
            (crossterm::event::KeyCode::Char(c), _) => {
                match state.compose.focused {
                    0 => state.compose.to.push(c),
                    1 => state.compose.cc.push(c),
                    2 => state.compose.bcc.push(c),
                    3 => {
                        // Insert char at cursor
                        let (line, col) = state.compose.body_cursor;
                        let mut lines: Vec<String> =
                            state.compose.body.lines().map(|l| l.to_string()).collect();
                        if lines.is_empty() {
                            lines.push(String::new());
                        }
                        if line >= lines.len() {
                            lines.resize(line + 1, String::new());
                        }
                        lines[line].insert(col, c);
                        state.compose.body = lines.join("\n");
                        state.compose.body_cursor.1 += 1;
                    }
                    _ => {}
                }
            }
            (crossterm::event::KeyCode::Enter, _) => {
                if state.compose.focused == 3 {
                    let (line, col) = state.compose.body_cursor;
                    let mut lines: Vec<String> =
                        state.compose.body.lines().map(|l| l.to_string()).collect();
                    if lines.is_empty() {
                        lines.push(String::new());
                    }
                    // If cursor is at a new line at the end, add an empty line
                    if line >= lines.len() {
                        lines.push(String::new());
                    }
                    // Defensive: ensure line is in bounds
                    if line < lines.len() {
                        let rest = lines[line][col..].to_string();
                        lines[line].truncate(col);
                        lines.insert(line + 1, rest);
                        state.compose.body = lines.join("\n");
                        state.compose.body_cursor = (line + 1, 0);
                    } else {
                        // If line is still out of bounds, just add a new empty line
                        lines.push(String::new());
                        state.compose.body = lines.join("\n");
                        state.compose.body_cursor = (lines.len() - 1, 0);
                    }
                }
            }
            (crossterm::event::KeyCode::Backspace, _) => {
                match state.compose.focused {
                    0 => {
                        state.compose.to.pop();
                    }
                    1 => {
                        state.compose.cc.pop();
                    }
                    2 => {
                        state.compose.bcc.pop();
                    }
                    3 => {
                        let (line, col) = state.compose.body_cursor;
                        let mut lines: Vec<String> =
                            state.compose.body.lines().map(|l| l.to_string()).collect();
                        if lines.is_empty() {
                            lines.push(String::new());
                        }
                        if line < lines.len() {
                            if col > 0 && col <= lines[line].len() {
                                lines[line].remove(col - 1);
                                state.compose.body_cursor.1 -= 1;
                            } else if col == 0 && line > 0 {
                                // Merge with previous line
                                let prev_len = lines[line - 1].len();
                                let cur = lines.remove(line);
                                lines[line - 1].push_str(&cur);
                                state.compose.body_cursor = (line - 1, prev_len);
                            }
                        }
                        // Clamp cursor to valid position
                        let (mut line, mut col) = state.compose.body_cursor;
                        if line >= lines.len() {
                            line = lines.len().saturating_sub(1);
                        }
                        let line_len = lines.get(line).map(|l| l.len()).unwrap_or(0);
                        if col > line_len {
                            col = line_len;
                        }
                        state.compose.body_cursor = (line, col);
                        state.compose.body = lines.join("\n");
                    }
                    _ => {}
                }
            }
            (crossterm::event::KeyCode::Left, _) => {
                if state.compose.focused == 3 {
                    let (line, col) = state.compose.body_cursor;
                    if col > 0 {
                        state.compose.body_cursor.1 -= 1;
                    } else if line > 0 {
                        let prev_len = state
                            .compose
                            .body
                            .lines()
                            .nth(line - 1)
                            .map(|l| l.len())
                            .unwrap_or(0);
                        state.compose.body_cursor = (line - 1, prev_len);
                    }
                }
            }
            (crossterm::event::KeyCode::Right, _) => {
                if state.compose.focused == 3 {
                    let (line, col) = state.compose.body_cursor;
                    let lines: Vec<&str> = state.compose.body.lines().collect();
                    let line_len = lines.get(line).map(|l| l.len()).unwrap_or(0);
                    if col < line_len {
                        state.compose.body_cursor.1 += 1;
                    } else if line + 1 < lines.len() {
                        state.compose.body_cursor = (line + 1, 0);
                    }
                }
            }
            (crossterm::event::KeyCode::Up, _) => {
                if state.compose.focused == 3 {
                    let (line, col) = state.compose.body_cursor;
                    if line > 0 {
                        let prev_len = state
                            .compose
                            .body
                            .lines()
                            .nth(line - 1)
                            .map(|l| l.len())
                            .unwrap_or(0);
                        state.compose.body_cursor = (line - 1, col.min(prev_len));
                    }
                }
            }
            (crossterm::event::KeyCode::Down, _) => {
                if state.compose.focused == 3 {
                    let (line, col) = state.compose.body_cursor;
                    let lines: Vec<&str> = state.compose.body.lines().collect();
                    if line + 1 < lines.len() {
                        let next_len = lines[line + 1].len();
                        state.compose.body_cursor = (line + 1, col.min(next_len));
                    }
                }
            }
            _ => {}
        }
    }
}

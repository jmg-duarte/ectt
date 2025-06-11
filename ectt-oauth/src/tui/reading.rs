use crossterm::event::{Event, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use crate::{Screen, ScreenState};

pub struct ReadingFields {
    from: String,
    cc: Vec<String>,
    bcc: Vec<String>,
    body: String,
    scroll: u16,
}

impl Default for ReadingFields {
    fn default() -> Self {
        Self {
            from: "alice@example.com".to_string(),
            cc: vec!["bob@example.com".to_string()],
            bcc: vec!["carol@example.com".to_string()],
            body: "This is the email body.\nIt can be very long and should wrap and scroll."
                .to_string(),
            scroll: 0,
        }
    }
}

impl ReadingFields {
    pub fn render_reading(&self, f: &mut Frame) {
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
            ("To", &self.from),
            ("Cc", &self.cc.join(", ")),
            ("Bcc", &self.bcc.join(", ")),
        ];
        for (i, (label, value)) in fields.iter().enumerate() {
            let block = Block::default().borders(Borders::ALL).title(*label);
            let para = Paragraph::new(value.as_str()).block(block);
            f.render_widget(para, chunks[i]);
        }
        let body_block = Block::default().borders(Borders::ALL).title("Body");
        let para = Paragraph::new(self.body.as_str())
            .block(body_block)
            .wrap(Wrap { trim: false })
            .scroll((self.scroll, 0));
        f.render_widget(para, chunks[3]);
        let help = Paragraph::new("[Esc] Back | [Up/Down] Scroll")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, chunks[4]);
    }
}

pub fn handle_reading(state: &mut ScreenState, event: Event) {
    if let Event::Key(KeyEvent { code, .. }) = event {
        match code {
            crossterm::event::KeyCode::Esc => state.screen = Screen::Main,
            crossterm::event::KeyCode::Down => state.reading.scroll += 1,
            crossterm::event::KeyCode::Up => {
                if state.reading.scroll > 0 {
                    state.reading.scroll -= 1;
                }
            }
            _ => {}
        }
    }
}

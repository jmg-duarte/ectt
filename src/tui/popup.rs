use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

#[derive(Debug)]
pub struct Popup {
    message: String,
}

impl Popup {
    pub const fn new(message: String) -> Self {
        Self { message }
    }
}

impl Widget for Popup {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // take up a third of the screen vertically and half horizontally
        let popup_area = Rect {
            x: area.width / 2 - ((self.message.len() + 2) as u16 / 2),
            y: area.height / 3,
            width: (self.message.len() + 2) as u16,
            height: 3,
        };
        Clear.render(popup_area, buf);

        Paragraph::new(self.message)
            .wrap(Wrap { trim: true })
            .style(Style::new().yellow())
            .block(
                Block::new()
                    .borders(Borders::ALL)
                    .border_style(Style::new().red()),
            )
            .render(popup_area, buf);
    }
}

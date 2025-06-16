use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

const DISMISS_MESSAGE: &str = "Press Enter to dismiss";

#[derive(Debug)]
pub struct Popup {
    message: String,
    dismissable: bool,
}

impl Popup {
    pub const fn new(message: String, dismissable: bool) -> Self {
        Self {
            message,
            dismissable,
        }
    }
}

impl Widget for Popup {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        let message_with_padding = if self.dismissable {
            (self.message.len() + 2).max(DISMISS_MESSAGE.len() + 2) as u16
        } else {
            (self.message.len() + 2) as u16
        };

        let popup_area = Rect {
            x: area.width / 2 - (message_with_padding / 2),
            y: area.height / 3,
            width: message_with_padding,
            height: if self.dismissable { 4 } else { 3 },
        };
        Clear.render(popup_area, buf);

        let text = if self.dismissable {
            vec![Line::from(self.message), Line::from(DISMISS_MESSAGE)]
        } else {
            vec![Line::from(self.message)]
        };

        Paragraph::new(text)
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

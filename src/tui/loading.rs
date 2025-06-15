use ratatui::{
    layout::Rect,
    style::{Style, Stylize},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};

#[derive(Debug)]
pub struct LoadingPopup;

impl Widget for LoadingPopup {
    fn render(self, area: ratatui::prelude::Rect, buf: &mut ratatui::prelude::Buffer)
    where
        Self: Sized,
    {
        // take up a third of the screen vertically and half horizontally
        let popup_area = Rect {
            x: area.width / 2 - (("Loading more emails!".len() + 2) as u16 / 2),
            y: area.height / 3,
            width: ("Loading more emails!".len() + 2) as u16,
            height: 3,
        };
        Clear.render(popup_area, buf);

        Paragraph::new("Loading more emails!")
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

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::prelude::*;
use ratatui::widgets::*;
use crate::tui::utils::centered_rect;

pub fn draw_centered_popup(f: &mut Frame, title: &str, button_select: &mut bool) {
    let mut yes = Text::from("Yes").bold().alignment(Alignment::Center);
    let mut no = Text::from("No").bold().alignment(Alignment::Center);
    if *button_select {
        yes = yes.fg(Color::Green);
    } else {
        no = no.fg(Color::Green);
    }

    let container = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_alignment(Alignment::Center);
    let centered_rect = centered_rect(60, 20, f.size());
    let half_y = centered_rect.height / 2;
    let half_x = centered_rect.width / 2;
    let left_side = Rect::new(centered_rect.x, centered_rect.y + half_y, half_x, 1);
    let right_side = Rect::new(
        centered_rect.x + half_x,
        centered_rect.y + half_y,
        half_x,
        1,
    );

    f.render_widget(container, centered_rect);
    f.render_widget(yes, left_side);
    f.render_widget(no, right_side);
}

use ratatui::layout::{Constraint, Direction, Layout, Rect};

#[allow(dead_code)]
pub fn get_longest_len<'a, T>(str_iter: T) -> u16
where
    T: Iterator<Item = &'a str>,
{
    str_iter
        .map(|l| l.len())
        .max()
        .map_or(0, |len| u16::try_from(len).unwrap_or(u16::MAX))
}

// Source: https://github.com/fdehau/tui-rs/blob/335f5a4563342f9a4ee19e2462059e1159dcbf25/examples/popup.rs#L104C1-L128C2
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

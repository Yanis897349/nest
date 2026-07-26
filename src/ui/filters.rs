use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem},
};

use crate::app::AppState;

pub(super) fn render(frame: &mut Frame<'_>, app: &AppState, area: Rect) {
    let [categories, options] =
        Layout::horizontal([Constraint::Percentage(32), Constraint::Percentage(68)]).areas(area);
    let groups = app.groups();
    let category_items = groups.iter().enumerate().map(|(index, group)| {
        let style = if index == app.category_index {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        ListItem::new(group.title.clone()).style(style)
    });
    frame.render_widget(
        List::new(category_items).block(Block::default().title(" Filters ").borders(Borders::ALL)),
        categories,
    );
    let selected_group = groups.get(app.category_index);
    let option_items = selected_group
        .into_iter()
        .flat_map(|group| group.options.iter().enumerate())
        .map(|(index, option)| {
            let marker = if option.selected { "[x]" } else { "[ ]" };
            let mut style = if option.selected {
                Style::default().fg(Color::Green)
            } else {
                Style::default()
            };
            if index == app.option_index {
                style = style.bg(Color::DarkGray).add_modifier(Modifier::BOLD);
            }
            ListItem::new(format!("{marker} {}", option.label)).style(style)
        });
    frame.render_widget(
        List::new(option_items).block(Block::default().title(" Options ").borders(Borders::ALL)),
        options,
    );
}

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Styled, Stylize},
    widgets::{Block, Borders, List, ListItem, ListState, Widget},
};
use std::{io, thread, time::Duration};

use crate::request::RssFeed;
pub fn get_title_list_items(feed: &RssFeed) -> Vec<ListItem> {
    let items = feed.channel.item.as_deref().unwrap_or(&[]);
    items
        .iter()
        .map(|item| {
            let content = item.title.clone().unwrap_or_else(|| "Untitled".to_string());
            ListItem::new(content)
        })
        .collect()
}

pub fn get_date_list_items(feed: &RssFeed) -> Vec<ListItem> {
    feed.channel
        .item
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .map(|item| {
            let content = item.pubDate.clone().unwrap().to_string();
            ListItem::new(content)
        })
        .collect()
}
pub fn render_item_list(
    frame: &mut Frame,
    area: Rect,
    list_items: &[ListItem],
    list_dates: &[ListItem],
    list_state: &mut ListState,
    feed: &RssFeed,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
        .split(area);
    let channel_title = feed.channel.title.as_deref().unwrap_or("Untitled");

    let title_list = List::new(list_items.iter().cloned())
        .block(Block::default().title(channel_title).borders(Borders::ALL))
        .highlight_style(Style::new().on_white().black())
        .highlight_symbol("> ".black())
        .scroll_padding(1)
        .repeat_highlight_symbol(true);
    let date_list = List::new(list_dates.iter().cloned())
        .block(
            Block::default()
                .title("Date Published")
                .borders(Borders::ALL),
        )
        .highlight_style(Style::new().on_white().black())
        .scroll_padding(1);

    frame.render_stateful_widget(title_list, chunks[0], list_state);
    frame.render_stateful_widget(date_list, chunks[1], list_state);
}

#[allow(clippy::collapsible_if)]
pub fn ui(feed: &RssFeed) -> Result<(), io::Error> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut list_state = ListState::default();
    list_state.select_first();
    let titles = get_title_list_items(feed);
    let dates = get_date_list_items(feed);
    loop {
        terminal.draw(|f| {
            let area = f.area();
            render_item_list(f, area, &titles, &dates, &mut list_state, feed);
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up => list_state.select_previous(),
                    KeyCode::Down => list_state.select_next(),
                    _ => {}
                }
            }
        }
    }
    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}

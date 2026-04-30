use crate::{
    config::Config,
    request::{RssFeed, parse_xml, request_channel},
};
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
use std::error::Error;
use std::{io, time::Duration};
pub async fn get_feed(link: &str) -> Result<RssFeed, Box<dyn Error>> {
    let response = request_channel(link).await?;
    let body = response.text().await?;
    Ok(parse_xml(&body)?)
}
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
            let content = item
                .pubDate
                .unwrap_or_default()
                .format("%d-%m-%Y %H:%M")
                .to_string();
            ListItem::new(content)
        })
        .collect()
}
pub fn render_item_list(
    frame: &mut Frame,
    area: Rect,
    list_items: &[ListItem],
    list_dates: &[ListItem],
    channel_title: &str,
    list_state: &mut ListState,
    feed: &RssFeed,
) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)].as_ref())
        .split(area);

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
pub async fn ui() -> Result<(), io::Error> {
    let config = Config::load();
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    // setup feed
    let mut feed_index = 0;
    let mut list_state = ListState::default();

    let mut feed = get_feed(&config.feeds[feed_index].link).await.unwrap();
    list_state.select_first();
    let mut titles = get_title_list_items(&feed);
    let mut dates = get_date_list_items(&feed);
    let mut channel_title = config.feeds[feed_index].title.clone();

    loop {
        terminal.draw(|f| {
            let area = f.area();
            render_item_list(
                f,
                area,
                &titles,
                &dates,
                &channel_title,
                &mut list_state,
                &feed,
            );
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up => list_state.select_previous(),
                    KeyCode::Down => list_state.select_next(),
                    KeyCode::Char('n') => {
                        feed_index = (feed_index + 1) % config.feeds.len();
                        feed = get_feed(&config.feeds[feed_index].link).await.unwrap();
                        titles = get_title_list_items(&feed);
                        dates = get_date_list_items(&feed);

                        channel_title = config.feeds[feed_index].title.clone();
                        list_state.select_first();
                    }
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

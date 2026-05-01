use crate::{
    config::{Config, ConfigFeed},
    request::{RssFeed, parse_xml, request_channel},
};
use chrono::{Duration, TimeDelta, Utc};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, HorizontalAlignment, Layout, Rect},
    style::{Style, Stylize},
    text::Text,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use std::io;
use std::time;
use std::{error::Error, fmt::Alignment};

pub enum Period {
    Week,
    TwoWeeks,
    Month,
    Year,
}
impl Period {
    fn to_days(&self) -> i64 {
        match self {
            Period::Week => 7,
            Period::TwoWeeks => 14,
            Period::Month => 30,
            Period::Year => 365,
        }
    }
    fn next(&self) -> Period {
        match self {
            Period::Week => Period::TwoWeeks,
            Period::TwoWeeks => Period::Month,
            Period::Month => Period::Year,
            Period::Year => Period::Week,
        }
    }
}
pub async fn get_feed(link: &str) -> Result<RssFeed, Box<dyn Error>> {
    let response = request_channel(link).await?;
    let body = response.text().await?;
    Ok(parse_xml(&body)?)
}
#[allow(clippy::collapsible_if)]
pub fn get_title_list_items<'a>(feed: &'a RssFeed, period: &Period) -> Vec<ListItem<'a>> {
    let now = chrono::Utc::now();
    let period_days = period.to_days();

    let items = feed.channel.item.as_deref().unwrap_or(&[]);
    items
        .iter()
        .filter_map(|item| {
            if let Some(pub_date) = item.pub_date {
                if now.signed_duration_since(pub_date) <= Duration::days(period_days) {
                    let content = item.title.as_deref().unwrap_or("Untitled");
                    return Some(ListItem::new(content));
                }
            }
            None
        })
        .collect()
}

#[allow(clippy::collapsible_if)]
pub fn get_date_list_items<'a>(feed: &'a RssFeed, period: &Period) -> Vec<ListItem<'a>> {
    let now = chrono::Utc::now();
    let period_days = period.to_days();
    feed.channel
        .item
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter_map(|item| {
            if let Some(pub_date) = item.pub_date {
                if now.signed_duration_since(pub_date) <= Duration::days(period_days) {
                    let content = pub_date.format("%d-%m-%Y %H:%M").to_string();
                    return Some(ListItem::new(content));
                }
            }
            None
        })
        .collect()
}
pub fn render_item_screen(
    frame: &mut Frame,
    area: Rect,
    list_items: &[ListItem],
    list_dates: &[ListItem],
    channel_title: &str,
    list_state: &mut ListState,
    period: &Period,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(95), Constraint::Percentage(5)].as_ref())
        .split(area);
    let list_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Max(22)].as_ref())
        .split(chunks[0]);
    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Max(8)].as_ref())
        .split(chunks[1]);
    let title_list = List::new(list_items.iter().cloned())
        .block(
            Block::default()
                .title(format!(
                    "{}  {:?}/{}",
                    channel_title,
                    list_state.selected().unwrap_or(0) + 1,
                    list_items.len()
                ))
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .scroll_padding(1)
        .repeat_highlight_symbol(true);
    let date_list = List::new(list_dates.iter().cloned())
        .block(
            Block::default()
                .title("Date Published")
                .borders(Borders::ALL),
        )
        .highlight_symbol("> ")
        .scroll_padding(1);
    let keybinds_text = [
        "Q, Esc: Quit",
        "N: Next Feed",
        "T: Change Period",
        "Enter: Show Details",
        "↑: Move Up",
        "↓: Move Down",
    ];

    let keybinds_paragraph = Paragraph::new(Text::from(keybinds_text.join(" | ")))
        .block(Block::default().borders(Borders::ALL).title("Keybinds"));
    let period_paragraph = Paragraph::new(Text::from(period.to_days().to_string()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Period")
                .title_alignment(HorizontalAlignment::Center),
        )
        .style(Style::new().green())
        .alignment(HorizontalAlignment::Center);
    frame.render_stateful_widget(title_list, list_chunks[0], list_state);
    frame.render_stateful_widget(date_list, list_chunks[1], list_state);
    frame.render_widget(keybinds_paragraph, bottom_chunks[0]);
    frame.render_widget(period_paragraph, bottom_chunks[1]);
}
pub fn render_item_details(frame: &mut Frame, area: Rect, feed: &RssFeed, list_state: &ListState) {
    if let Some(index) = list_state.selected() {
        // Check if the feed contains items
        if let Some(items) = &feed.channel.item {
            // Ensure the index is within bounds
            if let Some(item) = items.get(index) {
                // Extract the fields, using default values if they are None
                let title = item.title.as_deref().unwrap_or("No Title Available");
                let link = item.link.as_deref().unwrap_or("No Link Available");
                let description = item
                    .description
                    .as_deref()
                    .unwrap_or("No Description Available");
                let pub_date = item
                    .pub_date
                    .unwrap_or_default()
                    .format("%d-%m-%Y %H:%M")
                    .to_string();
                // Combine the details into a single text string
                let detail_text = [
                    "Title: ".to_string() + title + "\n",
                    "Link: ".to_string() + link + "\n",
                    "Description: ".to_string() + description + "\n",
                    "Date Published: ".to_string() + &pub_date,
                ];

                let popup_block = Block::bordered().title("Details");
                let centered_area =
                    area.centered(Constraint::Percentage(70), Constraint::Percentage(40));
                frame.render_widget(Clear, centered_area);
                let detail_paragraph = Paragraph::new(detail_text.join(""))
                    .wrap(Wrap { trim: true })
                    .block(popup_block);
                frame.render_widget(detail_paragraph, centered_area);
            }
        }
    }
}
#[allow(clippy::collapsible_if)]
pub async fn ui(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<(), io::Error> {
    let mut config = Config::load();
    // setup terminal

    // setup feed
    let mut feed_index = 0;
    let mut list_state = ListState::default();
    let mut period = Period::Year;
    let mut feed = get_feed(&config.feeds[feed_index].link).await.unwrap();
    list_state.select_first();
    let mut titles = get_title_list_items(&feed, &period);
    let mut dates = get_date_list_items(&feed, &period);
    let mut channel_title = config.feeds[feed_index].title.clone();
    let mut details_popup = false;
    loop {
        terminal.draw(|f| {
            let area = f.area();
            render_item_screen(
                f,
                area,
                &titles,
                &dates,
                &channel_title,
                &mut list_state,
                &period,
            );
            if details_popup {
                render_item_details(f, area, &feed, &list_state);
            };
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up => list_state.select_previous(),
                    KeyCode::Down => list_state.select_next(),
                    KeyCode::Char('n') => {
                        feed_index = (feed_index + 1) % config.feeds.len();
                        feed = get_feed(&config.feeds[feed_index].link).await.unwrap();
                        titles = get_title_list_items(&feed, &period);
                        dates = get_date_list_items(&feed, &period);

                        channel_title = config.feeds[feed_index].title.clone();
                        list_state.select_first();
                    }
                    KeyCode::Enter => details_popup = !details_popup,
                    KeyCode::Char('t') => {
                        period = period.next();
                        titles = get_title_list_items(&feed, &period);
                        dates = get_date_list_items(&feed, &period);
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

use crate::{
    config::{Config, ConfigFeed},
    request::{RssFeed, parse_xml, request_channel},
};
use chrono::Duration;
use futures::future::join_all;
use ratatui::crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode},
    execute,
    terminal::{LeaveAlternateScreen, disable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, HorizontalAlignment, Layout, Rect},
    style::Style,
    text::Text,
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};
use ratatui_textarea::{Input, TextArea};
use std::error::Error;
use std::io;
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

pub struct AddFeedForm<'a> {
    pub title: TextArea<'a>,
    pub link: TextArea<'a>,
    pub focused: Field,
}
impl AddFeedForm<'_> {
    pub fn focus_next(&mut self) {
        self.focused = match self.focused {
            Field::Title => Field::Link,
            Field::Link => Field::Title,
        };
    }
}
pub enum Field {
    Title,
    Link,
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
            // If there's a date, check the period. If no date, show it anyway
            let should_show = if let Some(pub_date) = item.pub_date {
                now.signed_duration_since(pub_date) <= Duration::days(period_days)
            } else {
                true // Show items without dates
            };

            if should_show {
                let content = item.title.as_deref().unwrap_or("Untitled");
                Some(ListItem::new(content))
            } else {
                None
            }
        })
        .collect()
}
pub fn get_date_list_items<'a>(feed: &'a RssFeed, period: &Period) -> Vec<ListItem<'a>> {
    let now = chrono::Utc::now();
    let period_days = period.to_days();

    feed.channel
        .item
        .as_deref()
        .unwrap_or(&[])
        .iter()
        .filter_map(|item| {
            let should_show = if let Some(pub_date) = item.pub_date {
                now.signed_duration_since(pub_date) <= Duration::days(period_days)
            } else {
                true // Show items without dates
            };

            if should_show {
                let content = if let Some(pub_date) = item.pub_date {
                    pub_date.format("%d-%m-%Y %H:%M").to_string()
                } else {
                    "No date".to_string()
                };
                Some(ListItem::new(content))
            } else {
                None
            }
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
    message: &str,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage(92),
                Constraint::Percentage(5),
                Constraint::Percentage(1),
            ]
            .as_ref(),
        )
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
        "N: Add New Feed",
        "T: Change Period",
        "L: List Feeds",
        "Enter: Show Details",
        "↑: Move Up",
        "↓: Move Down",
        "←: Previous Feed",
        "→: Next Feed",
        "O: Open Link",
        "R: Refresh",
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
    let messages_paragraph = Paragraph::new(Text::from(message));
    frame.render_stateful_widget(title_list, list_chunks[0], list_state);
    frame.render_stateful_widget(date_list, list_chunks[1], list_state);
    frame.render_widget(keybinds_paragraph, bottom_chunks[0]);
    frame.render_widget(period_paragraph, bottom_chunks[1]);
    frame.render_widget(messages_paragraph, chunks[2]);
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
                    "Title: ".to_string() + title + "\n\n",
                    "Link: ".to_string() + link + "\n\n",
                    "Description: ".to_string() + description + "\n\n",
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
pub fn render_add_channel(frame: &mut Frame, area: Rect, form: &mut AddFeedForm) {
    let popup = area.centered(Constraint::Percentage(50), Constraint::Percentage(20));

    frame.render_widget(Clear, popup);

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Length(3),
        Constraint::Length(3),
    ])
    .margin(1)
    .split(popup);
    // Update the blocks based on focus
    let title_block =
        Block::default()
            .borders(Borders::ALL)
            .title("Title")
            .style(match form.focused {
                Field::Title => Style::default().fg(ratatui::style::Color::Green), // Highlighted
                Field::Link => Style::default(),
            });

    let link_block =
        Block::default()
            .borders(Borders::ALL)
            .title("Link")
            .style(match form.focused {
                Field::Link => Style::default().fg(ratatui::style::Color::Green), // Highlighted
                Field::Title => Style::default(),
            });

    form.title.set_block(title_block);
    form.link.set_block(link_block);
    frame.render_widget(&form.title, chunks[0]);
    frame.render_widget(&form.link, chunks[1]);

    let hint = Paragraph::new("Enter = save | Tab = switch | Esc = cancel");
    frame.render_widget(hint, chunks[2]);
}

pub fn render_channel_list(
    frame: &mut Frame,
    area: Rect,
    config: &Config,
    list_state: &mut ListState,
) {
    let popup = area.centered(Constraint::Percentage(50), Constraint::Percentage(50));
    frame.render_widget(Clear, popup);
    let chunks = Layout::vertical([Constraint::Percentage(95), Constraint::Percentage(5)])
        .margin(1)
        .split(popup);
    let items = &config.feeds;
    let titles: Vec<ListItem<'_>> = items
        .iter()
        .map(|feed| {
            let content = feed.title.clone().to_string();
            ListItem::new(content)
        })
        .collect();
    let list = List::new(titles.iter().cloned())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Available Feeds"),
        )
        .highlight_symbol("> ");
    let hint = Paragraph::new("Enter: Select | Del: Delete | S: Save");
    frame.render_stateful_widget(list, chunks[0], list_state);
    frame.render_widget(hint, chunks[1]);
}

pub fn empty_feed_check(titles: &[ListItem], message: &mut String) {
    if titles.is_empty() {
        *message = "Feed is empty!".to_string();
    } else {
        *message = " ".to_string();
    }
}
async fn build_feed_cache(feeds: &[ConfigFeed]) -> Vec<RssFeed> {
    let futures: Vec<_> = feeds.iter().map(|feed| get_feed(&feed.link)).collect();
    join_all(futures)
        .await
        .into_iter()
        .filter_map(|result| match result {
            Ok(feed) => Some(feed),
            Err(e) => {
                eprintln!("Failed to fetch feed: {}", e);
                None
            }
        })
        .collect()
}
#[allow(clippy::collapsible_if)]
pub async fn ui(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
) -> Result<(), io::Error> {
    let mut config = Config::load();
    // setup terminal

    // setup feed
    let mut message = String::new();
    let mut feed_index = 0;
    let mut items_list_state = ListState::default();
    let mut channels_list_state = ListState::default();
    channels_list_state.select_first();
    let mut period = Period::Year;
    let mut feed_cache = build_feed_cache(&config.feeds).await;
    if feed_cache.is_empty() {
        message = "Failed to load feeds!".to_string();
    }

    items_list_state.select_first();
    let mut titles = if !feed_cache.is_empty() {
        get_title_list_items(&feed_cache[feed_index], &period)
    } else {
        Vec::new()
    };
    let mut dates = if !feed_cache.is_empty() {
        get_date_list_items(&feed_cache[feed_index], &period)
    } else {
        Vec::new()
    };
    empty_feed_check(&titles, &mut message);
    let mut channel_title = config
        .feeds
        .get(feed_index)
        .map(|f| f.title.clone())
        .unwrap_or_default();
    let mut details_popup = false;
    let mut new_channel_popup = false;
    let mut channel_list_popup = false;
    let mut add_form = AddFeedForm {
        title: TextArea::default(),
        link: TextArea::default(),
        focused: Field::Title,
    };
    add_form
        .title
        .set_block(Block::default().borders(Borders::ALL).title("Title"));
    add_form
        .link
        .set_block(Block::default().borders(Borders::ALL).title("Link"));
    loop {
        terminal.draw(|f| {
            let area = f.area();
            render_item_screen(
                f,
                area,
                &titles,
                &dates,
                &channel_title,
                &mut items_list_state,
                &period,
                &message,
            );

            if details_popup {
                render_item_details(f, area, &feed_cache[feed_index], &items_list_state);
            };
            if new_channel_popup {
                render_add_channel(f, area, &mut add_form);
            }
            if channel_list_popup {
                render_channel_list(f, area, &config, &mut channels_list_state);
            }
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if new_channel_popup {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('q') => {
                            new_channel_popup = false;
                            message = "Form saved!".to_string();
                        }

                        KeyCode::Tab => {
                            add_form.focus_next();
                        }

                        KeyCode::Enter => {
                            let title = add_form.title.lines().join("\n").trim().to_string();
                            let link = add_form.link.lines().join("\n").trim().to_string();

                            if !title.is_empty() && !link.is_empty() {
                                config.feeds.push(ConfigFeed { title, link });
                                config.write().ok();
                            }

                            new_channel_popup = false;
                            message = "New feed added succesfully!".to_string();
                            feed_cache = build_feed_cache(&config.feeds).await;
                            titles = get_title_list_items(&feed_cache[feed_index], &period);
                            dates = get_date_list_items(&feed_cache[feed_index], &period);
                        }
                        _ => {
                            let input: Input = key.into();
                            match add_form.focused {
                                Field::Title => &mut add_form.title.input(input),
                                Field::Link => &mut add_form.link.input(input),
                            };
                        }
                    }
                    continue;
                }
                if channel_list_popup {
                    match key.code {
                        KeyCode::Up => channels_list_state.select_previous(),
                        KeyCode::Down => channels_list_state.select_next(),
                        KeyCode::Char('l') | KeyCode::Char('q') | KeyCode::Esc => {
                            channel_list_popup = !channel_list_popup;
                            channels_list_state.select_first();
                        }
                        KeyCode::Enter => {
                            feed_index = channels_list_state.selected().unwrap();
                            channel_list_popup = !channel_list_popup;
                            titles = get_title_list_items(&feed_cache[feed_index], &period);
                            dates = get_date_list_items(&feed_cache[feed_index], &period);
                            empty_feed_check(&titles, &mut message);
                            channel_title = config.feeds[feed_index].title.clone();
                            items_list_state.select_first();
                        }
                        KeyCode::Delete => {
                            config.feeds.remove(channels_list_state.selected().unwrap());
                        }
                        KeyCode::Char('s') => {
                            config.write()?;
                            message = "Config saved!".to_string();
                        }

                        _ => {}
                    }
                    continue;
                }

                async fn build_feed_cache(feeds: &[ConfigFeed]) -> Vec<RssFeed> {
                    let mut cache = Vec::new();
                    for feed in feeds {
                        cache.push(get_feed(&feed.link).await.unwrap());
                    }
                    cache
                }
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up => items_list_state.select_previous(),
                    KeyCode::Down => items_list_state.select_next(),
                    KeyCode::Right => {
                        if !feed_cache.is_empty() {
                            feed_index = (feed_index + 1) % config.feeds.len();
                            titles = get_title_list_items(&feed_cache[feed_index], &period);
                            dates = get_date_list_items(&feed_cache[feed_index], &period);
                            empty_feed_check(&titles, &mut message);
                            channel_title = config.feeds[feed_index].title.clone();
                            items_list_state.select_first();
                        }
                    }
                    KeyCode::Left => {
                        if !feed_cache.is_empty() {
                            feed_index = (feed_index + config.feeds.len() - 1) % config.feeds.len();
                            titles = get_title_list_items(&feed_cache[feed_index], &period);
                            dates = get_date_list_items(&feed_cache[feed_index], &period);
                            empty_feed_check(&titles, &mut message);
                            channel_title = config.feeds[feed_index].title.clone();
                            items_list_state.select_first();
                        }
                    }
                    KeyCode::Enter => details_popup = !details_popup,
                    KeyCode::Char('t') => {
                        period = period.next();
                        titles = get_title_list_items(&feed_cache[feed_index], &period);
                        dates = get_date_list_items(&feed_cache[feed_index], &period);

                        empty_feed_check(&titles, &mut message);
                        items_list_state.select_first();
                    }
                    KeyCode::Char('n') => new_channel_popup = !new_channel_popup,
                    KeyCode::Char('l') => channel_list_popup = !channel_list_popup,
                    KeyCode::Char('o') => {
                        if let Some(index) = items_list_state.selected() {
                            if let Some(items) = &feed_cache[feed_index].channel.item {
                                if let Some(item) = items.get(index) {
                                    if let Some(link) = &item.link {
                                        let link = link.clone();
                                        std::thread::spawn(move || {
                                            if let Err(e) = open::that(link) {
                                                eprintln!("Failed to open link: {}", e);
                                            }
                                        });
                                    } else {
                                        message = "Could not open the link!".to_string();
                                    }
                                }
                            }
                        }
                    }
                    KeyCode::Char('r') => {
                        feed_cache = build_feed_cache(&config.feeds).await;
                        titles = get_title_list_items(&feed_cache[feed_index], &period);
                        dates = get_date_list_items(&feed_cache[feed_index], &period);
                        message = "Refreshed!".to_string();
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

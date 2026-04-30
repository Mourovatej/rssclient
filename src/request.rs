use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;
use serde_xml_rs::from_str;

#[derive(Debug, Deserialize)]
pub struct Item {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, rename = "pubDate", deserialize_with = "deserialize_rfc2822")]
    pub pub_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct RssFeed {
    pub channel: Channel,
}
#[derive(Debug, Deserialize)]
pub struct Channel {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub item: Option<Vec<Item>>,
}

pub fn deserialize_rfc2822<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        Some(date_str) => match DateTime::parse_from_rfc2822(&date_str) {
            Ok(dt) => Ok(Some(dt.with_timezone(&Utc))),
            Err(err) => Err(serde::de::Error::custom(err)),
        },
        None => Ok(None),
    }
}
pub async fn request_channel(link: &str) -> Result<reqwest::Response, reqwest::Error> {
    let client = Client::new();
    client
        .get(link)
        .header("User-Agent", "rss-client/1.0")
        .send()
        .await
}

pub fn parse_xml(data: &str) -> Result<RssFeed, serde_xml_rs::Error> {
    from_str(data)
}

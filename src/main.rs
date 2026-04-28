use chrono::{DateTime, Local, Utc};
use http::{Request, Response};
use reqwest;
use reqwest::Client;
use serde::Deserialize;
use serde_xml_rs::from_str;
use tokio;

#[derive(Debug, Deserialize)]
struct Book {
    title: String,
    link: String,
    description: String,
    #[serde(rename = "pubDate", deserialize_with = "deserialize_rfc2822")]
    add_date: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct RssFeed {
    channel: Channel,
}
#[derive(Debug, Deserialize)]
struct Channel {
    title: String,
    link: String,
    item: Vec<Book>,
}
// Custom deserializer for RFC 2822 dates
fn deserialize_rfc2822<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc2822(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}
async fn request_channel(link: &str) -> Result<reqwest::Response, reqwest::Error> {
    let client = Client::new();
    client
        .get(link)
        .header("User-Agent", "rss-client/1.0")
        .send()
        .await
}

fn parse_xml(data: &str) -> Result<RssFeed, serde_xml_rs::Error> {
    from_str(data)
}
#[tokio::main]
async fn main() {
    let rss_link = "https://www.techlib.cz/public/feeds/books20_cs.xml";
    match request_channel(rss_link).await {
        Ok(response) => match response.text().await {
            Ok(body) => match parse_xml(&body) {
                Ok(feed) => {
                    println!("Channel name: {}", feed.channel.title);
                    for (i, book) in feed.channel.item.iter().enumerate() {
                        let local_add_date = book.add_date.with_timezone(&Local);
                        println!("Book num: {}, Name: {}", i, book.title);
                        println!("Added on: {}", local_add_date);
                        println!("--------------");
                    }
                }
                Err(e) => eprintln!("Failed to parse XML: {}", e),
            },
            Err(e) => eprintln!("Failed to get body: {}", e),
        },
        Err(e) => eprintln!("Failed to request: {}", e),
    }
}

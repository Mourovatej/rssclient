mod request;
mod tui;
use chrono::Local;
use request::{parse_xml, request_channel};
#[tokio::main]
async fn main() {
    let rss_link = "https://katalog.upce.cz/rss";
    match request_channel(rss_link).await {
        Ok(response) => match response.text().await {
            Ok(body) => match parse_xml(&body) {
                Ok(feed) => {
                    println!("Channel name: {:?}", feed.channel.title);
                    for (i, book) in feed.channel.item.iter().enumerate() {
                        let local_add_date = book.add_date.with_timezone(&Local);
                        println!("Book num: {:?}, Name: {:?}", i, book.title);
                        println!("Added on: {:?}", local_add_date);
                        println!("--------------");
                    }
                }
                Err(e) => eprintln!("Failed to parse XML: {:?}", e),
            },
            Err(e) => eprintln!("Failed to get body: {:?}", e),
        },
        Err(e) => eprintln!("Failed to request: {:?}", e),
    }
}

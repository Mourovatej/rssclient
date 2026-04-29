use crate::request::{parse_xml, request_channel};
use std::error::Error;
mod request;
mod ui;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let rss_link = "https://www.techlib.cz/public/feeds/books20_cs.xml";
    let response = request_channel(rss_link).await?;
    let body = response.text().await?;
    let feed = parse_xml(&body)?;
    let _ = ui::ui(&feed);
    Ok(())
}

use crate::{
    config::Config,
    request::{parse_xml, request_channel},
};
use std::error::Error;
mod config;
mod request;
mod ui;
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let _ = ui::ui().await;
    Ok(())
}

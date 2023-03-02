use bot::BranikBot;

pub mod bot;
pub mod comment_reader;
pub mod config;
pub mod parser;

#[tokio::main]
async fn main() {
    let mut bot = BranikBot::respawn().await;
    let _ = bot.run().await;
}


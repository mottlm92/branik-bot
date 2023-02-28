use bot::BranikBot;

pub mod parser;
pub mod comment_reader;
pub mod bot;
pub mod config;


#[tokio::main]
async fn main() {
    let bot = BranikBot::respawn().await;
    let _ = bot.run().await;
}

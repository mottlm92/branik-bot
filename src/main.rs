use std::thread;

use bot::BranikBot;
use tokio::time;

pub mod bot;
pub mod comment_reader;
pub mod config;
pub mod parser;

#[tokio::main]
async fn main() {
    loop {
        let mut bot = BranikBot::respawn().await;
        let _ = bot.run().await;
        drop(bot);
        println!("Restart in 10 seconds");
        thread::sleep(time::Duration::from_secs(10));
    }
}


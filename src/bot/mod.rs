use core::time;
use std::{thread, fs, io::Write};

use roux::{Reddit, Me, comment::CommentData};
use crate::{parser::{Parser, ParseResult}, comment_reader::CommentReader};
use super::config::Config;

pub struct BranikBot {
    config: Config,
    reddit_client: Me,
    comment_reader: CommentReader,
    parser: Parser,
}

enum BranikAmount {
    Pet(u32),
    Pack(u32),
    Palett(u32, u32)
}

const ONE_BRANIK: f32 = 43.9;

impl BranikBot {
    pub async fn respawn() -> Self {
        let config = match Config::load() {
            Err(_) => panic!("Couldn't respawn BranikBot! Failed to load config file!"),
            Ok(c) => c
        };
        let reddit_client = Self::login(&config).await;
        let parser = Parser::new();
        let comment_reader = CommentReader { 
            subreddit: config.subreddit.to_string(),
            last_comment_storage_path: "./data/last_comment".to_string()
        };
        BranikBot { config, reddit_client, comment_reader, parser }
    }

    async fn login(config: &Config) -> Me {
        let client = Reddit::new(&config.user_agent, &config.client_id, &config.client_secret)
            .username(&config.user_name)
            .password(&config.password)
            .login().await;
        match client {
            Err(_) => panic!("Couldn't login to reddit!"),
            Ok(me) => me
        } 
    }

    pub async fn run(&self) -> Result<(), ()> {
        let mut count = 0;
        loop {
            println!("\nRead new comments!");
            let comments = self.comment_reader.read_latest_comments().await.ok_or(())?;
            println!("Found {} new comments!", comments.len());
            self.parse_comments_and_create_responses(comments).await;
            count += 1;
            if count == 12 {
                break;
            }
            thread::sleep(time::Duration::from_secs(60 * 5));
        }
        Ok(())
    }


async fn parse_comments_and_create_responses(&self, comments: Vec<CommentData>) {
    for comment in comments.iter() {
        // lets not react to my own comments here
        if comment.author.clone().unwrap() == self.config.user_name {
            continue;
        }
        match &comment.body {
            None => continue,
            Some(comment_body) => {
                match &self.parser.parse(&comment_body) {
                    None => continue,
                    Some (matches) => {
                        if matches.len() == 0 {
                            continue;
                        }
                        self.post_response(
                            &BranikBot::generate_message_for_parse_results(&matches),
                            &comment.name.clone().unwrap().to_string()).await;
                    },
                }
            },
        }
    }
}

    fn generate_message_for_parse_results(parse_results: &Vec<ParseResult>) -> String {
        let mut result_message = "".to_string();
        for result in parse_results {
            result_message += &BranikBot::generate_parse_result_row(result);
        }
        result_message += &format!("\n\n^(Jsem bot, doufam, ze poskytnuta informace byla uzitecna. Podnety - Stiznosti - QA na r/branicek)").to_string();
        result_message
    }

    fn generate_parse_result_row(parse_result: &ParseResult) -> String {
        let row = format!("> {}\n\n", parse_result.parsed_value);
        match BranikBot::get_branik_amount(parse_result.result_value) {
            BranikAmount::Pet(amount) => {
                if amount == 0 {
                    format!("{}Je mi to lito, ale to neni ani na jeden 2L Branik ve sleve\n\n", row)
                } else {
                    format!("{}To je dost na {} 2L Branika ve sleve!\n\n", row, amount)
                }
            },
            BranikAmount::Pack(amount) => {
                format!("{}To je dost na {} baliku 2L Branika ve sleve!\n\n", row, amount)
            },
            BranikAmount::Palett(amount, pack_amount) => {
                format!("{}To je dost na vic jak {} palet{} ({} baliku) 2L Branika ve sleve!\n\n",
                    row,
                    amount,
                    match amount {
                        1 => "u",
                        2..=4 => "y",
                        _ => ""
                    },
                    pack_amount)
            }
        }
    }

    fn get_branik_amount(cash: f32) -> BranikAmount {
        // TODO: get current lowest branik price from the web!
        let amount = (cash / ONE_BRANIK) as u32;
        match amount {
            0 => BranikAmount::Pet(0), 
            // 144 = half of a palett
            1..=144 => BranikAmount::Pet(amount),
            // 288 = full palett
            145..=288 => BranikAmount::Pack(amount / 6),
            289.. => BranikAmount::Palett(amount / 288, amount / 6)
        }
    }

    async fn post_response(&self, response: &str, comment_id: &str) {
        if self.config.post_response {
            println!("\nPosted response {}\nto comment {}", response, comment_id);
            let _ = self.reddit_client.comment(response, comment_id).await;
        }
        if self.config.save_response {
            let open_file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .append(true)
                .open("./data/comments_from_bot");
            match open_file {
                Err(e) => println!("Cant open file! {}", e.to_string()),
                Ok(mut file) => {
                    let _ =file.write_all(response.as_bytes()); 
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_result_row() {
        let parse_result = ParseResult {parsed_value: "20 kc".to_string(), result_value: 20.0};
        let response_row = BranikBot::generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 20 kc\n\nJe mi to lito, ale to neni ani na jeden 2L Branik ve sleve\n\n"));
        let parse_result = ParseResult {parsed_value: "650kc".to_string(), result_value: 650.0};
        let response_row = BranikBot::generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 650kc\n\nTo je dost na {} 2L Branika ve sleve!\n\n", (650.0 / ONE_BRANIK) as i32));
        let parse_result = ParseResult {parsed_value: "10k".to_string(), result_value: 10000.0};
        let response_row = BranikBot::generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 10k\n\nTo je dost na {} baliku 2L Branika ve sleve!\n\n", (10000.0 / ONE_BRANIK / 6.0) as i32));
        let parse_result = ParseResult {parsed_value: "20k".to_string(), result_value: 20000.0};
        let response_row = BranikBot::generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 20k\n\nTo je dost na vic jak {} paletu ({} baliku) 2L Branika ve sleve!\n\n", (20000.0 / (12.0*8.0*3.0*ONE_BRANIK)) as i32, (20000.0 / ONE_BRANIK / 6.0) as i32));
        let parse_result = ParseResult {parsed_value: "30k".to_string(), result_value: 30000.0};
        let response_row = BranikBot::generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 30k\n\nTo je dost na vic jak {} palety ({} baliku) 2L Branika ve sleve!\n\n", (30000.0 / (12.0*8.0*3.0*ONE_BRANIK)) as i32, (30000.0 / ONE_BRANIK / 6.0) as i32));
        let parse_result = ParseResult {parsed_value: "150k".to_string(), result_value: 150000.0};
        let response_row = BranikBot::generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 150k\n\nTo je dost na vic jak {} palet ({} baliku) 2L Branika ve sleve!\n\n", (150000.0 / (12.0*8.0*3.0*ONE_BRANIK)) as i32, (150000.0 / ONE_BRANIK / 6.0) as i32));
    }
}

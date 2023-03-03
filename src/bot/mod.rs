use core::time;
use std::{thread, fs, io::Write};

use roux::{Reddit, Me, comment::CommentData, Subreddit, User};
use crate::{parser::{Parser, ParseResult}, comment_reader::CommentReader};
use self::price_reader::PriceReader;

use super::config::Config;

pub mod price_reader;

pub struct BranikBot {
    config: Config,
    reddit_client: Me,
    comment_reader: CommentReader,
    price_reader: PriceReader,
    parser: Parser,
    user: User,
    needs_update_price: bool,
    branik_price: f32
}

enum BranikAmount {
    Pet(u32),
    Pack(u32),
    Palett(u32, u32)
}

impl BranikBot {

    const MAX_CYCLES: i32 = 120;

    pub async fn respawn() -> Self {
        let config = Config::load();
        let reddit_client = Self::login(&config).await;
        let parser = Parser::new();
        let price_reader = PriceReader {};
        let comment_reader = CommentReader { 
            subreddit: Subreddit::new(&config.subreddit),
            last_comment_storage_path: "./data/last_comment".to_string()
        };
        let user = User::new(&config.user_name);
        let default_price = config.default_price;
        BranikBot { 
            config,
            reddit_client,
            comment_reader,
            parser,
            user,
            price_reader,
            needs_update_price: true,
            branik_price: default_price
        }
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

    pub async fn run(&mut self) {
        let mut count = 0;
        loop {
            if self.needs_update_price {
                self.update_price().await;
                println!("Price updated!");
            }
            println!("\nRead new comments!");
            match self.comment_reader.read_latest_comments().await {
                None => {
                    self.sleep();
                    continue;
                },
                Some(comments) => {
                    println!("Found {} new comments!", comments.len());
                    self.parse_comments_and_create_responses(comments).await;
                }
            }
            count += 1;
            self.needs_update_price = count % 12 == 0;
            println!("Cycle compeleted, {} cycles left", Self::MAX_CYCLES - count);
            if count == Self::MAX_CYCLES {
                break;
            }
            self.sleep(); 
        }
    }

    fn sleep(&self) {
        thread::sleep(time::Duration::from_secs(60 * 5));
    }

    async fn update_price(&mut self) {
        println!("Update price!");
        let price = if let Ok(p) = self.price_reader.load_and_parse_branik_price(self.config.default_price).await {
            p
        } else {
            self.config.default_price
        };
        self.branik_price = price;
    }

    async fn load_post_ids_for_posted_comments(&self) -> Vec<String> {
        let mut post_ids_for_bot_comments: Vec<String> = vec![];
        match &self.user.comments(
            Some(roux::util::FeedOption {
                after: None,
                before: None,
                limit: None,
                count: None,
                period: Some(roux::util::TimePeriod::Today) 
            })).await {
                Ok(comments_from_bot) => {
                    for comment in comments_from_bot.data.children.iter() {
                        post_ids_for_bot_comments.push(comment.data.link_id.clone().unwrap());
                    }
                },
                Err(_) => println!("Wasn't able to load comments from bot")
            }
        post_ids_for_bot_comments
    }

    async fn parse_comments_and_create_responses(&self, comments: Vec<CommentData>) {
        let post_ids_for_posted_comments = self.load_post_ids_for_posted_comments().await;
        for comment in comments.iter() {
            // lets not react to my own comments here
            if comment.author.clone().unwrap() == self.config.user_name {
                continue;
            }
            let comments_on_post_count = post_ids_for_posted_comments.iter()
                // count current comment "LINK_ID (= post id)" occurencies in bot comments
                .filter(|pid| pid.to_owned() == &comment.link_id.clone().unwrap_or("".to_string())).count();
            if comments_on_post_count >= self.config.comments_per_post_limit {
                println!("Already posted {} comments on this post {}, limit is {}, skipping...",
                    comments_on_post_count, &comment.link_url.clone().unwrap(), self.config.comments_per_post_limit);
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
                                &self.generate_message_for_parse_results(&matches),
                                &comment.name.clone().unwrap().to_string()).await;
                        },
                    }
                },
            }
        }
    }

    fn generate_message_for_parse_results(&self, parse_results: &Vec<ParseResult>) -> String {
        let mut result_message = "".to_string();
        for result in parse_results {
            result_message += &self.generate_parse_result_row(result);
        }
        result_message += &format!("\n\n^(Jsem bot, doufam, ze poskytnuta informace byla uzitecna. Podnety - Stiznosti - QA na r/branicek)").to_string();
        result_message
    }

    fn generate_parse_result_row(&self, parse_result: &ParseResult) -> String {
        let row = format!("> {}\n\n", parse_result.parsed_value);
        match self.get_branik_amount(parse_result.result_value) {
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

    fn get_branik_amount(&self, cash: f32) -> BranikAmount {
        // TODO: get current lowest branik price from the web!
        let amount = (cash / self.branik_price) as u32;
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

    #[tokio::test]
    async fn test_result_row() {

        let test_bot = BranikBot ::respawn().await;


        let parse_result = ParseResult {parsed_value: "20 kc".to_string(), result_value: 20.0};
        let response_row = test_bot.generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 20 kc\n\nJe mi to lito, ale to neni ani na jeden 2L Branik ve sleve\n\n"));
        let parse_result = ParseResult {parsed_value: "650kc".to_string(), result_value: 650.0};
        let response_row = test_bot.generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 650kc\n\nTo je dost na {} 2L Branika ve sleve!\n\n", (650.0 / test_bot.config.default_price) as i32));
        let parse_result = ParseResult {parsed_value: "10k".to_string(), result_value: 10000.0};
        let response_row = test_bot.generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 10k\n\nTo je dost na {} baliku 2L Branika ve sleve!\n\n", (10000.0 / test_bot.config.default_price / 6.0) as i32));
        let parse_result = ParseResult {parsed_value: "20k".to_string(), result_value: 20000.0};
        let response_row = test_bot.generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 20k\n\nTo je dost na vic jak {} paletu ({} baliku) 2L Branika ve sleve!\n\n", (20000.0 / (12.0*8.0*3.0*test_bot.config.default_price)) as i32, (20000.0 / test_bot.config.default_price / 6.0) as i32));
        let parse_result = ParseResult {parsed_value: "30k".to_string(), result_value: 30000.0};
        let response_row = test_bot.generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 30k\n\nTo je dost na vic jak {} palety ({} baliku) 2L Branika ve sleve!\n\n", (30000.0 / (12.0*8.0*3.0*test_bot.config.default_price)) as i32, (30000.0 / test_bot.config.default_price / 6.0) as i32));
        let parse_result = ParseResult {parsed_value: "150k".to_string(), result_value: 150000.0};
        let response_row = test_bot.generate_parse_result_row(&parse_result);
        assert_eq!(response_row, format!("> 150k\n\nTo je dost na vic jak {} palet ({} baliku) 2L Branika ve sleve!\n\n", (150000.0 / (12.0*8.0*3.0*test_bot.config.default_price)) as i32, (150000.0 / test_bot.config.default_price / 6.0) as i32));
    }
}

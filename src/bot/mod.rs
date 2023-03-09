use core::time;
use std::{thread, fs, io::Write};

use roux::{Reddit, Me, comment::CommentData, Subreddit, User};
use crate::{parser::{Parser, ParseResult}, comment_reader::CommentReader};
use self::price_reader::PriceReader;

use super::config::Config;

pub mod price_reader;

pub struct BranikBot {
    config: Config,
    reddit_client: Option<Me>,
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

    const RESPONSE_PREFIX: &str = "To by stacilo na ";
    const RESPONSE_SUFFIX: &str = "Branika ve sleve!";

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

    async fn login(config: &Config) -> Option<Me> { 
        let client = Reddit::new(&config.user_agent, &config.client_id, &config.client_secret)
            .username(&config.user_name)
            .password(&config.password)
            .login().await;
        match client {
            Err(_) => {
                if config.post_response {
                    panic!("Couldn't login to reddit and POST_RESPONSE is set to true");
                }
                None
            },
            Ok(me) => Some(me)
        } 
    }

    pub async fn run(&mut self) {
        // if set to 0, will run indefinitely
        let max_cycles: i32 = self.config.run_cycles;
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
            if max_cycles > 0 {
                println!("Cycle completed, {} cycles left", max_cycles - count);
            } else {
                println!("{} cycles completed.", count);
            }
            if count == max_cycles {
                break;
            }
            // TODO: better logic for this - update before each cycle right after midnight,
            // no need to update so often
            self.needs_update_price = count % 12 == 0;
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
            let comment_body = if let Some(cb) = &comment.body {
                cb
            } else {
                // No comment body - nothing to parse
                continue;
            };
            let matches = if let Some(m) = self.parser.parse(&comment_body) {
                m
            } else {
                // No matches in comment 
                continue;
            };
            self.post_response(
                &self.generate_message_for_results(&matches),
                &comment.name.clone().unwrap().to_string()).await;
        }
    }

    fn generate_message_for_results(&self, parse_results: &Vec<ParseResult>) -> String {
        let mut result_message = "".to_string();
        for result in parse_results {
            result_message += &self.generate_result_row(result);
        }
        result_message += &format!("\n\n^(Jsem bot, doufam, ze poskytnuta informace byla uzitecna. Podnety - Stiznosti - QA na r/branicek)").to_string();
        result_message
    }

    fn generate_result_row(&self, parse_result: &ParseResult) -> String {
        match parse_result {
            ParseResult::Keyword => self.generate_keyword_result_row(),
            ParseResult::Value(parsed_value, result_value) => self.generate_value_result_row(parsed_value, *result_value)
        }
    }

    fn generate_keyword_result_row(&self) -> String {
        format!("Dvoulitrovka Branika ve sleve aktualne stoji {} korun.", self.branik_price)
    }

    fn generate_value_result_row(&self, parsed_value: &String, parsed_result: f32) -> String {
        let row = format!("> {}\n\n", parsed_value);
        match self.get_branik_amount(parsed_result) {
            BranikAmount::Pet(amount) => {
                if amount == 0 {
                    format!("{}Je mi to lito, ale to neni ani na jednu dvoulitrovku Branika ve sleve.\n\n", row)
                } else {
                    format!("{}{}{} dvoulitrov{} {}\n\n",
                        row,
                        Self::RESPONSE_PREFIX,
                        amount,
                        match amount {
                            1 => "ku",
                            2..=4 => "ky",
                            _ => "ek" 
                        },
                        Self::RESPONSE_SUFFIX)
                }
            },
            BranikAmount::Pack(amount) => {
                format!("{}{}{} baliku dvoulitrovek {}\n\n",
                    row,
                    Self::RESPONSE_PREFIX,
                    amount,
                    Self::RESPONSE_SUFFIX)
            },
            BranikAmount::Palett(amount, pack_amount) => {
                format!("{}{}vic jak {} palet{} ({} baliku) dvoulitrovek {}\n\n",
                    row,
                    Self::RESPONSE_PREFIX,
                    amount,
                    match amount {
                        1 => "u",
                        2..=4 => "y",
                        _ => ""
                    },
                    pack_amount,
                    Self::RESPONSE_SUFFIX)
            }
        }
    }

    fn get_branik_amount(&self, cash: f32) -> BranikAmount {
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
            let r = self.reddit_client.as_ref().expect("Expected reddit client being logged in").comment(response, comment_id).await;
            match r {
                Ok(_) => (),
                Err(err_response) => println!("Error posting response {}", err_response.to_string())
            }
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
                    let _ = file.write_all(response.as_bytes()); 
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

        let parse_result = ParseResult::Value( "20 kc".to_string(), 20.0);
        let response_row = test_bot.generate_result_row(&parse_result);
        assert_eq!(response_row, format!("> 20 kc\n\nJe mi to lito, ale to neni ani na jednu dvoulitrovku Branika ve sleve.\n\n"));
        let parse_result = ParseResult::Value( "50kc".to_string(), 50.0);
        let response_row = test_bot.generate_result_row(&parse_result);
        assert_eq!(response_row, format!("> 50kc\n\n{}{} dvoulitrovku {}\n\n", BranikBot::RESPONSE_PREFIX, (50.0 / test_bot.config.default_price) as i32, BranikBot::RESPONSE_SUFFIX));
        let parse_result = ParseResult::Value( "150kc".to_string(), 150.0);
        let response_row = test_bot.generate_result_row(&parse_result);
        assert_eq!(response_row, format!("> 150kc\n\n{}{} dvoulitrovky {}\n\n", BranikBot::RESPONSE_PREFIX, (150.0 / test_bot.config.default_price) as i32, BranikBot::RESPONSE_SUFFIX));
        let parse_result = ParseResult::Value( "650kc".to_string(), 650.0);
        let response_row = test_bot.generate_result_row(&parse_result);
        assert_eq!(response_row, format!("> 650kc\n\n{}{} dvoulitrovek {}\n\n", BranikBot::RESPONSE_PREFIX, (650.0 / test_bot.config.default_price) as i32, BranikBot::RESPONSE_SUFFIX));
        let parse_result = ParseResult::Value("10k".to_string(), 10000.0);
        let response_row = test_bot.generate_result_row(&parse_result);
        assert_eq!(response_row, format!("> 10k\n\n{}{} baliku dvoulitrovek {}\n\n", BranikBot::RESPONSE_PREFIX, (10000.0 / test_bot.config.default_price / 6.0) as i32, BranikBot::RESPONSE_SUFFIX));
        let parse_result = ParseResult::Value( "20k".to_string(), 20000.0);
        let response_row = test_bot.generate_result_row(&parse_result);
        assert_eq!(response_row, format!("> 20k\n\n{}vic jak {} paletu ({} baliku) dvoulitrovek {}\n\n", BranikBot::RESPONSE_PREFIX, (20000.0 / (12.0*8.0*3.0*test_bot.config.default_price)) as i32, (20000.0 / test_bot.config.default_price / 6.0) as i32, BranikBot::RESPONSE_SUFFIX));
        let parse_result = ParseResult::Value("30k".to_string(), 30000.0);
        let response_row = test_bot.generate_result_row(&parse_result);
        assert_eq!(response_row, format!("> 30k\n\n{}vic jak {} palety ({} baliku) dvoulitrovek {}\n\n", BranikBot::RESPONSE_PREFIX, (30000.0 / (12.0*8.0*3.0*test_bot.config.default_price)) as i32, (30000.0 / test_bot.config.default_price / 6.0) as i32, BranikBot::RESPONSE_SUFFIX));
        let parse_result = ParseResult::Value("150k".to_string(), 150000.0);
        let response_row = test_bot.generate_result_row(&parse_result);
        assert_eq!(response_row, format!("> 150k\n\n{}vic jak {} palet ({} baliku) dvoulitrovek {}\n\n", BranikBot::RESPONSE_PREFIX, (150000.0 / (12.0*8.0*3.0*test_bot.config.default_price)) as i32, (150000.0 / test_bot.config.default_price / 6.0) as i32, BranikBot::RESPONSE_SUFFIX));
    }
}

use core::time;
use std::{thread, fs, io::Write};

use roux::{Reddit, Me, comment::CommentData};
use crate::{parser::{Parser, ParseResult}, comment_reader::CommentReader};
use super::config::Config;

pub struct BranikBot {
    reddit_client: Me,
    comment_reader: CommentReader,
    parser: Parser,
}

impl BranikBot {
    pub async fn respawn() -> Self {
        let config = match Config::load() {
            Err(_) => panic!("Couldn't respawn BranikBot! Failed to load config file!"),
            Ok(c) => c
        };
        let reddit_client = Self::login(config).await;
        let parser = Parser::new();
        let comment_reader = CommentReader { 
            subreddit: "branicek".to_string(),
            last_comment_storage_path: "../data/last_comment".to_string()
        };
        BranikBot { reddit_client, comment_reader, parser }
    }

    async fn login(config: Config) -> Me {
        let client = Reddit::new(&config.user_agent, &config.client_id, &config.client_secret)
            .username(&config.user_name)
            .password(&config.password)
            .login().await;
        match client {
            Err(_) => panic!("Couldn't login to reddit!"),
            Ok(me) => me
        } 
    }

    pub async fn run(&self) {
        let mut count = 0;
        loop {
            match self.comment_reader.read_latest_comments().await {
                None => (),
                Some(comments) => {
                    self.parse_comments_and_create_responses(comments).await;
                },
            }
            count += 1;
            if count == 6 {
                break;
            }
            thread::sleep(time::Duration::from_secs(60 * 5));
        }
    }


    async fn parse_comments_and_create_responses(&self, comments: Vec<CommentData>) {
        for comment in comments.iter() {
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
                                &self.generate_message_for_parse_results(matches, comment_body),
                                &comment.id.clone().unwrap()).await;
                        },
                    }
                },
            }
        }
    }

    fn generate_message_for_parse_results(&self, parse_results: &Vec<ParseResult>, comment: &str) -> String {
        let mut result_message = "".to_string();
        for result in parse_results {
            result_message += &self.generate_parse_result_row(result);
        }
        result_message += &format!("\n\nI did this in response to {}\n\n\n", comment).to_string();
        result_message
    }

    fn generate_parse_result_row(&self, parse_result: &ParseResult) -> String {
        format!("> {}\n\nTo je dost na {:.0} 2L Branika ve sleve!\n", parse_result.parsed_value, self.get_branik_amount(parse_result.result_value))    
    }

    fn get_branik_amount(&self, cash: f32) -> f32 {
        // TODO: get current lowest branik price from the web!
        cash / 39.9
    }

    async fn post_response(&self, response: &str, comment_id: &str) {

        match  self.reddit_client.comment(response, &format!("t1_{}", comment_id)).await {
            Ok(_) => (),
            Err(_) => ()
        };

        let open_file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open("../data/comments_from_bot");
        match open_file {
            Err(e) => println!("Cant open file! {}", e.to_string()),
            Ok(mut file) => {
                match file.write_all(response.as_bytes()) {
                    Ok(_) => (), 
                    Err(_) => ()
                }
            },
        }
    }
}

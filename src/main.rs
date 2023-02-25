pub mod parser;
pub mod comment_reader;

use std::{fs, io::Write};

use parser::{Parser, ParseResult};
use comment_reader::CommentReader;
use roux::comment::CommentData;


#[tokio::main]
async fn main() {
    let parser = Parser::new();
    match CommentReader::read_latest_comments().await {
        Some(comments) => {
            parse_comments_and_create_responses(parser, comments);
        },
        None => ()
    }
}

fn parse_comments_and_create_responses(parser: Parser, comments: Vec<CommentData>) {
    let mut responses = 0;
    for comment in comments.iter() {
        match &comment.body {
            Some(comment_body) => {
                println!("Comment: {}", comment_body);
                match parser.parse(&comment_body) {
                    Some (matches) => {
                        if matches.len() == 0 {
                            continue;
                        }
                        post_response(
                            &generate_message_for_parse_results(matches, comment_body),
                            &comment.id.clone().unwrap());
                        responses += 1;
                    },
                    None => continue
                }
            },
            None => continue
        }
    }
    println!("Saved {} responses!", responses);
}

fn generate_message_for_parse_results(parse_results: Vec<ParseResult>, comment: &str) -> String {
    let mut result_message = "".to_string();
    for result in parse_results {
        result_message += &generate_parse_result_row(result);
    }
    result_message += &format!("\n\nI did this in response to {}", comment).to_string();
    result_message
}

fn generate_parse_result_row(parse_result: ParseResult) -> String {
    format!("> {}\nTo je dost na {:.0} 2L Braníku ve slevě!\n", parse_result.parsed_value, get_branik_amount(parse_result.result_value))    
}

fn get_branik_amount(cash: f32) -> f32 {
    // TODO: get current lowest branik price from the web!
    cash / 39.9
}

fn post_response(response: &str, _comment_id: &str) {
    // TODO: actually post using reddit client, for now, write to debug file
    let open_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open("./data/comments_from_bot");
    match open_file {
        Ok(mut file) => {
            match file.write_all(response.as_bytes()) {
                Ok(_) => (), 
                Err(_) => (), 
            }
        },
        Err(_) => ()
    }
}

pub mod parser;
pub mod comment_reader;

use parser::Parser;
use comment_reader::CommentReader;


#[tokio::main]
async fn main() {
    let comments = CommentReader::read_latest_comments().await;
    for comment in comments.unwrap().iter() {
        println!("Comment id: {}, text: {}", comment.id.clone().unwrap(), comment.body.clone().unwrap());
    }
}

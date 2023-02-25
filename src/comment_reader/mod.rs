use std::{fs, io::Write};
use roux::{Subreddit, comment::CommentData};

pub struct CommentReader {
}

impl CommentReader {
    pub async fn read_latest_comments() -> Option<Vec<CommentData>> {
        let subreddit = Subreddit::new("czech");
        let latest_comments = subreddit.latest_comments(None, Some(20)).await;
        match latest_comments {
            Ok(comments) => {
                // load last read comment id from file in order to not read it again
                let last_read_comment_id = match Self::load_last_read_comment() {
                    Some(comment_id) => comment_id,
                    None => "".to_string()
                }; 
                let mut result: Vec<CommentData> =  vec![];
                for comment in comments.data.children {
                    if comment.data.id.clone().unwrap() == last_read_comment_id {
                        break;
                    }
                    result.push(comment.data);
                }
                if result.len() == 0 {
                    return None;
                }
                // save id of first comment we received
                let latest_comment_id = &result[0].id.clone().unwrap();
                Self::save_latest_read_comment(&latest_comment_id);
                return Some(result);
            },
            Err(_) => return None
        }
    }

    fn load_last_read_comment() -> Option<String> {
        match fs::read_to_string("./data/last_comment") {
            Ok(text) => return Some(text),
            Err(_) => return None
        };
    }

    fn save_latest_read_comment(comment_id: &str) {
        let open_file = fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open("./data/last_comment");
        match open_file {
            Ok(mut file) => {
                match file.write_all(comment_id.as_bytes()) {
                    Ok(_) => (),
                    Err(_) => (),
                }
            },
            Err(_) => ()
        }
    }
}

use std::fs;

pub struct Config {
    pub user_agent: String,
    pub client_id: String,
    pub client_secret: String,
    pub user_name: String,
    pub password: String,
    pub subreddit: String,
    pub post_response: bool,
    pub save_response: bool
}

impl Config {
    pub fn load() -> Result<Config, ()> {
        match fs::read_to_string("./.config") {
            Err(_) => Err(()),
            Ok(text) => {
                Ok(Self::read_config_file(&text))
            } 
        }
    }

    fn read_config_file(config_str: &str) -> Config {
        let mut config_lines = config_str.lines();
        Config {
            user_agent: config_lines.next().expect("Expected to have user agent on index 0 in the config!").to_string(),
            client_id: config_lines.next().expect("Expected to have client id on index 1 in the config!").to_string(),
            client_secret: config_lines.next().expect("Expected to have client secret on index 2 in the config!").to_string(),
            user_name: config_lines.next().expect("Expected to have username on index 3 in the config!").to_string(),
            password: config_lines.next().expect("Expected to have password on index 4 in the config!").to_string(),
            subreddit: config_lines.next().expect("Expected to have subreddit on index 4 in the config!").to_string(),
            post_response: config_lines.next().expect("Expected to have password on index 5 in the config!").to_string() == "true",
            save_response: config_lines.next().expect("Expected to have subreddit on index 6 in the config!").to_string() == "true",
        }
    }
}

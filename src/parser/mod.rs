use std::fmt::Display;

use regex::Regex;

pub struct Parser {
    main_regex: Regex,
    value_regex: Regex,
    unit_regex: Regex,
}

pub struct ParseResult {
    pub parsed_value: String,
    pub result_value: f32,
}

impl Display for ParseResult {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "ParseResult - value {} extracted from {}", self.result_value, self.parsed_value)
    }
}

impl Parser {
    pub fn new() -> Parser {
        let parser = Parser {
            main_regex: Regex::new(r"\d+[ ](kc|czk|kč)|\d+(kc|czk|kč)|( |^)\d+[k]|( |^)\d+[.|,]\d+[k]|\d+(,-)").unwrap(),
            value_regex: Regex::new(r"\d*[,|.]\d+|\d+").unwrap(),
            unit_regex: Regex::new(r"([^\d]+)$").unwrap()
        };
        parser
    }

    pub fn parse(&self, text: &str) -> Option<Vec<ParseResult>> {
        let binding = text.to_lowercase();
        let is_match = self.main_regex.is_match(&binding);
        if !is_match {
            return None;
        }
        let mut parsed_results: Vec<ParseResult> = vec![];
        let captures = self.main_regex.captures_iter(&binding);
        for cap in captures {
            let cap = &cap[0].trim();
            let value = self.get_value_from_match(cap);
            match value {
                Some(v) => {
                    let v = self.get_true_value(v, cap);
                    let result = ParseResult {
                        parsed_value: cap.to_string(),
                        result_value: v
                    };
                    parsed_results.push(result);
                } 
                None => continue
            }
        }
        Some(parsed_results)
    }

    fn get_value_from_match(&self, match_str: &str) -> Option<f32> {
        let capture = self.value_regex.captures(&match_str).unwrap();
        match capture[0].replace(",", ".").parse() {
            Ok(v) => if v > 0.0 {
                return Some(v)
            },
            Err(_) => return None
        };
        None 
    }

    fn get_true_value(&self, value: f32, match_str: &str) -> f32 {
        let capture = self.unit_regex.captures(match_str).unwrap();
        match &capture[0] {
            "k" => return value * 1000.0,
            _ => return value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let test_parser = Parser::new();
        let test_data = "Test data for currency detection, 500czk or 600 CZK shouldn't matter, you should also be able to use 6000Kc or 5999kč just as you should be able to use 42 kc or 69 Kč... Last but not least, let's check some shortened values! Like 5k or 2.5k, oh and 2,5k should work as well! What shouldn't work though, is just loose numbers like 69420 without any currency specification, same with like this range 9-5, time 10PM or 9 AM but this 100,- should get captured!";
        let results = test_parser.parse(test_data).unwrap();
        assert_eq!(results.len(), 10);
    }

    #[test]
    fn test_parse_k() {
        let test_parser = Parser::new();
        let test_data = "Let's see if all 200k, 1.5k and 6,9k are correct.. and these totally random numbers 69 420 should be ignored..";
        let results = test_parser.parse(test_data).unwrap();
        assert_eq!(results.len(), 3);
        let result = &results[0];
        assert_eq!("200k", result.parsed_value);
        assert_eq!(200000.0, result.result_value);
        let result = &results[1];
        assert_eq!("1.5k", result.parsed_value);
        assert_eq!(1500.0, result.result_value);
        let result = &results[2];
        assert_eq!("6,9k", result.parsed_value);
        assert_eq!(6900.0, result.result_value);
    }
}

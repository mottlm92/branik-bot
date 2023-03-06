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
            main_regex: Regex::new(r"( |^)(((\d+[ ,.]?)+?(kc|kč|czk|mega|korun))|(\d+[,.]?\d+(k))|(\d+[k]))+(\b)|((\d+[ .|,]?)+(,-))").unwrap(),
            value_regex: Regex::new(r"(\d+[ ,.]?)+(\d+)?").unwrap(),
            unit_regex: Regex::new(r"([\p{L}+]+)|(mega)|(,-)").unwrap()
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
            let value = self.get_value_from_match(cap)?;
            let value = self.get_true_value(value, cap);
            if value == 0.0 {
                continue;
            }
            let result = ParseResult {
                parsed_value: cap.to_string(),
                result_value: value
            };
            if parsed_results.iter().any(|r| r.result_value == result.result_value) {
                continue;
            }
            parsed_results.push(result);
        }
        if parsed_results.len() == 0 {
            return None;
        }
        Some(parsed_results)
    }

    fn get_value_from_match(&self, match_str: &str) -> Option<f32> {
        let capture = self.value_regex.captures(&match_str).unwrap();
        if match_str.ends_with("k") || match_str.ends_with("mega") {
            // if value doesn't end with exact unit only remove whitespace
            capture[0].replace(",", ".").replace(" ", "").parse::<f32>().ok()
        } else {
            // if value ends with remove all punctuation and whitespace
            capture[0].replace(",", "").replace(" ", "").replace(".", "").parse::<f32>().ok()
        }
    }

    fn get_true_value(&self, value: f32, match_str: &str) -> f32 {
        let capture = self.unit_regex.captures(match_str).unwrap();
        match &capture[0] {
            "k" => return value * 1000.0,
            "mega" => return value * 1000000.0,
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
        let test_data = "Test data for currency detection, 500czk or 600 CZK shouldn't matter, you should also be able to use 6000Kc or 5999kč just as you should be able to use 42 kc or 69 Kč...
                         Last but not least, let's check some shortened values! Like 5k or 2.5k, oh and 5,5k should work as well! What shouldn't work though,
                         is just loose numbers like 69420 without any currency specification, same with like this range 9-5, time 10PM or 9 AM but this 100,- should get captured!";
        let results = test_parser.parse(test_data).unwrap();
        assert_eq!(results.len(), 10);
    }

    #[test]
    fn test_parse_unit() {
        let test_parser = Parser::new();
        let test_data = "Let's see if all 200k, 1.5k and 6,9k are correct.. and these totally random numbers 69 420 should be ignored..
But 2 mega should not! also add 60 kc and 100kc, but should be implemented 3 000 kc and this 1.900,- is parsed, 3.000.000 kc, 6 000 000 czk what about like 3.5 mega?";
        let results = test_parser.parse(test_data).unwrap();
        assert_eq!(results.len(), 11);
        let result = &results[0];
        assert_eq!("200k", result.parsed_value);
        assert_eq!(200000.0, result.result_value);
        let result = &results[1];
        assert_eq!("1.5k", result.parsed_value);
        assert_eq!(1500.0, result.result_value);
        let result = &results[2];
        assert_eq!("6,9k", result.parsed_value);
        assert_eq!(6900.0, result.result_value);
        let result = &results[3];
        assert_eq!("2 mega", result.parsed_value);
        assert_eq!(2000000.0, result.result_value);
        let result = &results[4];
        assert_eq!("60 kc", result.parsed_value);
        assert_eq!(60.0, result.result_value);
        let result = &results[5];
        assert_eq!("100kc", result.parsed_value);
        assert_eq!(100.0, result.result_value);
        let result = &results[6];
        assert_eq!("3 000 kc", result.parsed_value);
        assert_eq!(3000.0, result.result_value);
        let result = &results[7];
        assert_eq!("1.900,-", result.parsed_value);
        assert_eq!(1900.0, result.result_value);
        let result = &results[8];
        assert_eq!("3.000.000 kc", result.parsed_value);
        assert_eq!(3000000.0, result.result_value);
        let result = &results[9];
        assert_eq!("6 000 000 czk", result.parsed_value);
        assert_eq!(6000000.0, result.result_value);
        let result = &results[10];
        assert_eq!("3.5 mega", result.parsed_value);
        assert_eq!(3500000.0, result.result_value);
    }

    #[test]
    fn test_get_true_value(){
        let test_parser = Parser::new();
        let value = 100.0;
        let match_str = "100kc";
        let true_value = test_parser.get_true_value(value, match_str);
        assert_eq!(value, true_value);
        let value = 399.0;
        let match_str = "399 kč";
        let true_value = test_parser.get_true_value(value, match_str);
        assert_eq!(value, true_value);
        let value = 1.5;
        let match_str = "1,5k";
        let true_value = test_parser.get_true_value(value, match_str);
        assert_ne!(value, true_value);
        assert_eq!(value * 1000.0, true_value);
        let value = 5.0;
        let match_str = "5 mega";
        let true_value = test_parser.get_true_value(value, match_str);
        assert_ne!(value, true_value);
        assert_eq!(value * 1000000.0, true_value);
        let value = 3000.0;
        let match_str = "3 000 kc";
        let true_value = test_parser.get_true_value(value, match_str);
        assert_eq!(value, true_value);
        let value = 1900.0;
        let match_str = "1.900,-";
        let true_value = test_parser.get_true_value(value, match_str);
        assert_eq!(value, true_value);
        let value = 3000000.0;
        let match_str = "3.000.000 kc";
        let true_value = test_parser.get_true_value(value, match_str);
        assert_eq!(value, true_value);
        let value = 6000000.0;
        let match_str = "6 000 000 czk";
        let true_value = test_parser.get_true_value(value, match_str);
        assert_eq!(value, true_value);
        let value = 69420.0;
        let match_str = "69.420kc";
        let true_value = test_parser.get_true_value(value, match_str);
        assert_eq!(value, true_value);
    }
}

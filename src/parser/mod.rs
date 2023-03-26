use regex::Regex;

pub struct Parser {
    main_regex: Regex,
    value_regex: Regex,
    unit_regex: Regex,
}

pub enum ParseResult {
    // parsed some cash value
    Value(String, f32),
    // no cash value, keyword detected
    Keyword
}

impl PartialEq for ParseResult {
    fn eq(&self, other: &Self) -> bool {
        match self {
            ParseResult::Value(_, value) => {
                match other {
                    ParseResult::Keyword => false,
                    // texts don't need to match, we only care about value
                    ParseResult::Value(_, other_value) => other_value == value
                }
            }
            ParseResult::Keyword => {
                match other {
                    ParseResult::Keyword => true,
                    ParseResult::Value(_, _) => false
                }
            }
        }
    }
}

impl Parser {
    pub fn new() -> Parser {
        let parser = Parser {
            main_regex: Regex::new(r"( |^)(((\d+[ ,.]?)+?(kc|kč|czk|mega|korun))|(\d+[,.]?\d+(k))|(\d+[k]))+(\b)|((\d+[ .|,]?)+(,-))").unwrap(),
            value_regex: Regex::new(r"(\d?[ ,.]?)+(\d+)").unwrap(),
            unit_regex: Regex::new(r"([\p{L}+]+)|(mega)|(,-)").unwrap()
        };
        parser
    }

    const KEYWORDS: [&str; 6] = [
        "branik",
        "braník",
        "bráník",
        "branicek",
        "braníček",
        "bráníček"];

    pub fn parse(&self, text: &str) -> Option<Vec<ParseResult>> {
        let binding = text.to_lowercase();
        let is_match = self.main_regex.is_match(&binding);
        // no value in the text
        if !is_match {
            // check for keywords
            match Self::check_for_keyword(&binding) {
                false => return None,
                true => return Some(vec![ParseResult::Keyword])
            }
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
            let result = ParseResult::Value(cap.to_string(), value);
            if parsed_results.iter().any(|r| r == &result) {
                continue;
            }
            parsed_results.push(result);
        }
        if parsed_results.len() == 0 {
            return None;
        }
        Some(parsed_results)
    }

    fn check_for_keyword(text: &str) -> bool {
        text.split(" ").any(|word| Self::KEYWORDS.contains(&word))
    }

    fn get_value_from_match(&self, match_str: &str) -> Option<f32> {
        println!("MATCH = {}", &match_str);
        let capture = self.value_regex.captures(&match_str).unwrap();
        println!("CAPTURE = {}", capture[0].trim());
        if match_str.ends_with("k") || match_str.ends_with("mega") {
            // if value doesn't end with exact unit only remove whitespace
            capture[0].replace(",", ".").replace(" ", "").parse::<f32>().ok()
        } else if !capture[0].contains(",") {
            // remove whitespace and "." which are used as whitespace to improve readability but
            // don't have any real purpose
            capture[0].replace(" ", "").replace(".", "").parse::<f32>().ok()
        } else {
            // value contains "," which is used as delimiter for decimal values - i.e. 42,50 Kc
            capture[0].replace(",", ".").parse::<f32>().ok()
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
is just loose numbers like 69420 without any currency specification, same with like this range 9-5, time 10PM or 9 AM but this 100,- should get captured!, 30 - 50 kč";
        let results = test_parser.parse(test_data).unwrap();
        assert_eq!(results.len(), 11);
        let test_data = "Some text without any value or keyword";
        let results = test_parser.parse(test_data);
        assert_eq!(true, results.is_none());
        let test_data = "Some text without any value, but containing branicek keyword";
        let results = test_parser.parse(test_data);
        assert_eq!(true, results.is_some());
        let results = results.unwrap();
        assert_eq!(1, results.len());
    }

    #[test]
    fn test_parse_unit_from_value_result() {
        let test_parser = Parser::new();
        let test_data = "Let's see if all 200k, 1.5k and 6,9k are correct.. and these totally random numbers 69 420 should be ignored.. But 2 mega should not! also add 60 kc and 100kc,
but should be implemented 3 000 kc and this 1.900,- is parsed, 3.000.000 kc, 6 000 000 czk what about like 3.5 mega? 30 - 50 kč what about 42,50 kc and 0,90kc";
        let results = test_parser.parse(test_data).unwrap();
        assert_eq!(results.len(), 14);
        let ParseResult::Value(str, value) = &results[0] else {panic!()};
        assert_eq!("200k", str);
        assert_eq!(200000.0, *value);
        let ParseResult::Value(str, value) = &results[1] else {panic!()};
        assert_eq!("1.5k", str);
        assert_eq!(1500.0, *value);
        let ParseResult::Value(str, value) = &results[2] else {panic!()};
        assert_eq!("6,9k", str);
        assert_eq!(6900.0, *value);
        let ParseResult::Value(str, value) = &results[3] else {panic!()};
        assert_eq!("2 mega", str);
        assert_eq!(2000000.0, *value);
        let ParseResult::Value(str, value) = &results[4] else {panic!()};
        assert_eq!("60 kc", str);
        assert_eq!(60.0, *value);
        let ParseResult::Value(str, value) = &results[5] else {panic!()};
        assert_eq!("100kc", str);
        assert_eq!(100.0, *value);
        let ParseResult::Value(str, value) = &results[6] else {panic!()};
        assert_eq!("3 000 kc", str);
        assert_eq!(3000.0, *value);
        let ParseResult::Value(str, value) = &results[7] else {panic!()};
        assert_eq!("1.900,-", str);
        assert_eq!(1900.0, *value);
        let ParseResult::Value(str, value) = &results[8] else {panic!()};
        assert_eq!("3.000.000 kc", str);
        assert_eq!(3000000.0, *value);
        let ParseResult::Value(str, value) = &results[9] else {panic!()};
        assert_eq!("6 000 000 czk", str);
        assert_eq!(6000000.0, *value);
        let ParseResult::Value(str, value) = &results[10] else {panic!()};
        assert_eq!("3.5 mega", str);
        assert_eq!(3500000.0, *value);
        let ParseResult::Value(str, value) = &results[11] else {panic!()};
        assert_eq!("50 kč", str);
        assert_eq!(50.0, *value);
        let ParseResult::Value(str, value) = &results[12] else {panic!()};
        assert_eq!("42,50 kc", str);
        assert_eq!(42.5, *value);
        let ParseResult::Value(str, value) = &results[13] else {panic!()};
        assert_eq!("0,90kc", str);
        assert_eq!(0.9, *value);
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

    #[test]
    fn check_text_result() {
        let text = "sample text without any keyword";
        let result = Parser::check_for_keyword(text);
        assert_eq!(false, result);
        let text = "sample text with branik in it";
        let result = Parser::check_for_keyword(text);
        assert_eq!(true, result);
    }

    #[test]
    fn test_parse_result_compare() {
        let result1 = ParseResult::Keyword;
        let result2 = ParseResult::Keyword;
        assert_eq!(true, result2 == result1);
        assert_eq!(true, result1 == result2);
        let result1 = ParseResult::Value("100kc".to_string(), 100.0);
        assert_eq!(false, result1 == result2);
        assert_eq!(false, result2 == result1);
        let result2 = ParseResult::Value("3 000 kc".to_string(), 3000.0);
        assert_eq!(false, result1 == result2);
        assert_eq!(false, result2 == result1);
        let result2 = ParseResult::Value("100,-".to_string(), 100.0);
        assert_eq!(true, result2 == result1);
        assert_eq!(true, result1 == result2);
    }
}

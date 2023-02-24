pub mod parser;

use parser::Parser;

fn main() {
    let parser = Parser::new();

    let test_strings = ["500kc 750 kč ve věku 25-30? 100k? Nebo 1.5k or 2,4k? 100,- je tak akorat"];
    for s in test_strings {
        let parsed_values = parser.parse(s);
        for value in parsed_values.unwrap() {
            println!("{}", value);
        }
    }
}

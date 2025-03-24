mod der_parser;

use der_parser::DerParser;

fn main() {
    let data: &[u8] = &[0x02, 0x01, 0x05]; // Example: INTEGER with value 5
    let mut parser = DerParser::new(data);

    println!("Starting DER parse test:\n---------------------------");

    while !parser.is_done() {
        match parser.read_byte() {
            Some(byte) => println!("Read byte: 0x{:02x}", byte),
            None => println!("No more bytes to read."),
        }
    }

    println!("\nDone");
}

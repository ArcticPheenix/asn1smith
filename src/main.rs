// src/main.rs

mod der_parser;
mod format;
use der_parser::DerParser;
use std::env;
use std::fs::File;
use std::io::{self, Read};

fn main() {
    if env::args().any(|arg| arg == "--help" || arg == "-h") {
        println!("Usage: asn1smith [OPTIONS] [FILE] 
");
        println!("Options:");
        println!("  --pretty     Pretty-print parsed ASN.1 structures (default)");
        println!("  --raw        Output raw hex values only");
        println!("  --help, -h   Show this help message");
        println!("
If FILE is not provided, input is read from STDIN.");
        return;
    }
    let args: Vec<String> = env::args().collect();
    let mut buffer = Vec::new();
    let mut base64_buffer = String::new();
    let mut pretty = false;
    let mut raw = false;
    let mut filename = None;

    for arg in &args[1..] {
        match arg.as_str() {
            "--pretty" => pretty = true,
            "--raw" => raw = true,
            _ => filename = Some(arg.clone()),
        }
    }

    if let Some(file) = filename {
        let mut f = File::open(file).expect("Failed to open file");
        let mut contents = String::new();
        f.read_to_string(&mut contents).expect("Failed to read file");

        if contents.contains("-----BEGIN") {
            let begin = contents.find("-----BEGIN").unwrap();
            let end = contents.find("-----END").unwrap();
            let start_line = contents[begin..].find('\n').unwrap() + begin + 1;
            let end_line = contents[..end].rfind('\n').unwrap();
            let b64 = &contents[start_line..end_line];
            let cleaned = b64.lines().collect::<String>();
            buffer = base64::decode(cleaned.as_bytes()).expect("Failed to decode PEM");
        } else {
            buffer = contents.into_bytes();
        }
    } else {
        io::stdin().read_to_string(&mut base64_buffer).expect("Failed to read from STDIN");

        if base64_buffer.contains("-----BEGIN") {
            let begin = base64_buffer.find("-----BEGIN").unwrap();
            let end = base64_buffer.find("-----END").unwrap();
            let start_line = base64_buffer[begin..].find('\n').unwrap() + begin + 1;
            let end_line = base64_buffer[..end].rfind('\n').unwrap();
            let b64 = &base64_buffer[start_line..end_line];
            let cleaned = b64.lines().collect::<String>();
            buffer = base64::decode(cleaned.as_bytes()).expect("Failed to decode PEM");
        } else {
            buffer = base64_buffer.as_bytes().to_vec();
        }
    }

    let mut parser = DerParser::new(&buffer);
    match parser.parse_all() {
        Ok(objects) => {
            for obj in &objects {
                format::print_asn1_object(obj, 0, !raw);
            }
        }
        Err(err) => {
            eprintln!("Failed to parse DER input: {:?}", err);
        }
    }
}

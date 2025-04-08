// src/main.rs

mod der_parser;

use der_parser::{ASN1Object, ASN1Value, DerParser, TagClass};
use std::env;
use std::fs::File;
use std::io::{self, Read};

fn print_asn1_object(obj: &ASN1Object, indent: usize, pretty: bool) {
    let indent_str = "  ".repeat(indent);
    let class_str = match &obj.tag.class {
        TagClass::Universal => "Universal",
        TagClass::Application => "Application",
        TagClass::ContextSpecific => "ContextSpecific",
        TagClass::Private => "Private",
    };

    let tag_name = match (&obj.tag.class, obj.tag.number) {
        (TagClass::Universal, 1) => Some("BOOLEAN"),
        (TagClass::Universal, 2) => Some("INTEGER"),
        (TagClass::Universal, 3) => Some("BIT STRING"),
        (TagClass::Universal, 4) => Some("OCTET STRING"),
        (TagClass::Universal, 5) => Some("NULL"),
        (TagClass::Universal, 6) => Some("OBJECT IDENTIFIER"),
        (TagClass::Universal, 10) => Some("ENUMERATED"),
        (TagClass::Universal, 16) => Some("SEQUENCE"),
        (TagClass::Universal, 17) => Some("SET"),
        (TagClass::Universal, 19) => Some("PrintableString"),
        (TagClass::Universal, 20) => Some("T61String"),
        (TagClass::Universal, 22) => Some("IA5String"),
        (TagClass::Universal, 23) => Some("UTCTime"),
        (TagClass::Universal, 24) => Some("GeneralizedTime"),
        _ => None,
    };

    let tag_display = if let Some(name) = tag_name {
        format!("{} ({})", obj.tag.number, name)
    } else {
        obj.tag.number.to_string()
    };

    let tag_color = "\x1b[1;34m"; // Bold Blue
    let reset = "\x1b[0m";

    println!(
        "{}{}Tag:{} class={}, constructed={}, number={}",
        indent_str, tag_color, reset, class_str, obj.tag.constructed, tag_display
    );

    match &obj.value {
        ASN1Value::Primitive(bytes) => {
            if pretty {
                println!(
                    "{}  {}Primitive:{} ({} bytes): {:02X?}",
                    indent_str, tag_color, reset, bytes.len(), bytes
                );
            } else {
                println!("{:02X?}", bytes);
            }
        }
        ASN1Value::Constructed(children) => {
            if pretty {
                println!("{}  {}Constructed:{} {} children:", indent_str, tag_color, reset, children.len());
                for child in children {
                    print_asn1_object(child, indent + 1, pretty);
                }
            } else {
                for child in children {
                    print_asn1_object(child, indent, pretty);
                }
            }
        }
    }
}

fn main() {
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
                print_asn1_object(obj, 0, !raw);
            }
        }
        Err(err) => {
            eprintln!("Failed to parse DER input: {:?}", err);
        }
    }
}
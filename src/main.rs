// src/main.rs

mod der_parser;

use der_parser::{ASN1Object, ASN1Value, DerParser, TagClass};
use std::env;
use std::fs::File;
use std::io::{self, Read};

fn print_asn1_object(obj: &ASN1Object, indent: usize, pretty: bool) {
    let indent_str = "  ".repeat(indent);
    print_tag_header(obj, &indent_str);
    print_tag_value(obj, &indent_str, pretty);
}

fn print_tag_header(obj: &ASN1Object, indent_str: &str) {
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

    let tag_color = "[1;34m"; // Bold Blue
    let reset = "[0m";

    println!(
        "{}{}Tag:{} class={}, constructed={}, number={}",
        indent_str, tag_color, reset, class_str, obj.tag.constructed, tag_display
    );
}

fn print_tag_value(obj: &ASN1Object, indent_str: &str, pretty: bool) {
    match &obj.value {
        ASN1Value::Primitive(bytes) => {
            interpret_value(obj, indent_str, pretty, bytes);
        },
        ASN1Value::Constructed(children) => {
            let tag_color = "[1;34m";
            let reset = "[0m";
            if pretty {
                println!("{}  {}Constructed:{} {} children:", indent_str, tag_color, reset, children.len());
                for child in children {
                    print_asn1_object(child, indent_str.len() / 2 + 1, pretty);
                }
            } else {
                for child in children {
                    print_asn1_object(child, indent_str.len() / 2, pretty);
                }
            }
        }
    }
}

fn interpret_value(obj: &ASN1Object, indent_str: &str, pretty: bool, bytes: &[u8]) {
    let tag_color = "[1;34m";
    let reset = "[0m";

    if !pretty {
        println!("{:02X?}", bytes);
        return;
    }

    match obj.tag.class {
        TagClass::Universal => match obj.tag.number {
            1 => {
                let value = !bytes.is_empty() && bytes[0] != 0;
                println!("{}  {}BOOLEAN:{} {}", indent_str, tag_color, reset, value);
            },
            2 => {
                let value = num_bigint::BigUint::from_bytes_be(bytes);
                println!("{}  {}INTEGER:{} {} ({} bytes)", indent_str, tag_color, reset, value, bytes.len());
            },
            3 => {
                if let Some((&padding_bits, bits)) = bytes.split_first() {
                    println!(
                        "{}  {}BIT STRING:{} ({} bits, {} padding): {:02X?}",
                        indent_str,
                        tag_color,
                        reset,
                        bits.len() * 8 - (padding_bits as usize),
                        padding_bits,
                        bits
                    );
                } else {
                    println!("{}  {}BIT STRING:{} <empty>", indent_str, tag_color, reset);
                }
            },
            4 => {
                println!("{}  {}OCTET STRING:{} ({} bytes): {:02X?}", indent_str, tag_color, reset, bytes.len(), bytes);
            },
            5 => {
                println!("{}  {}NULL:{} (0 bytes)", indent_str, tag_color, reset);
            },
            6 => {
                if let Some(first) = bytes.first() {
                    let mut oid = vec![];
                    let first_byte = *first;
                    oid.push((first_byte / 40).to_string());
                    oid.push((first_byte % 40).to_string());

                    let mut value: u32 = 0;
                    for &b in &bytes[1..] {
                        value = (value << 7) | (b & 0x7F) as u32;
                        if b & 0x80 == 0 {
                            oid.push(value.to_string());
                            value = 0;
                        }
                    }

                    println!("{}  {}OID:{} {} ({} bytes)", indent_str, tag_color, reset, oid.join("."), bytes.len());
                } else {
                    println!("{}  {}OID:{} <empty>", indent_str, tag_color, reset);
                }
            },
            19 | 20 | 22 => {
                match std::str::from_utf8(bytes) {
                    Ok(text) => println!("{}  {}String:{} '{}' ({} bytes)", indent_str, tag_color, reset, text, bytes.len()),
                    Err(_) => println!("{}  {}String:{} <invalid UTF-8> ({:?})", indent_str, tag_color, reset, bytes),
                }
            },
            23 | 24 => {
                match std::str::from_utf8(bytes) {
                    Ok(time_str) => println!("{}  {}Time:{} '{}' ({} bytes)", indent_str, tag_color, reset, time_str, bytes.len()),
                    Err(_) => println!("{}  {}Time:{} <invalid UTF-8> ({:?})", indent_str, tag_color, reset, bytes),
                }
            },
            _ => {
                println!("{}  {}Primitive:{} ({} bytes): {:02X?}", indent_str, tag_color, reset, bytes.len(), bytes);
            }
        },
        _ => {
            println!("{}  {}Primitive:{} ({} bytes): {:02X?}", indent_str, tag_color, reset, bytes.len(), bytes);
        }
    }
}


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
                print_asn1_object(obj, 0, !raw);
            }
        }
        Err(err) => {
            eprintln!("Failed to parse DER input: {:?}", err);
        }
    }
}

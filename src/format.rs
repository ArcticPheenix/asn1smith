// src/format.rs

use crate::der_parser::{ASN1Object, TagClass, ASN1Value};

pub fn print_asn1_object(obj: &ASN1Object, indent: usize, pretty: bool) {
    let indent_str = "  ".repeat(indent);
    print_tag_header(obj, &indent_str);
    print_tag_value(obj, &indent_str, pretty);
}

pub fn print_tag_header(obj: &ASN1Object, indent_str: &str) {
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

    let tag_color = "\x1b[1;34m";
    let reset = "\x1b[0m";

    println!(
        "{}{}Tag:{} class={}, constructed={}, number={}",
        indent_str, tag_color, reset, class_str, obj.tag.constructed, tag_display
    );
}

pub fn print_tag_value(obj: &ASN1Object, indent_str: &str, pretty: bool) {
    match &obj.value {
        ASN1Value::Primitive(bytes) => interpret_value(obj, indent_str, pretty, bytes),
        ASN1Value::Constructed(children) => {
            let tag_color = "\x1b[1;34m";
            let reset = "\x1b[0m";
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
    let tag_color = "\x1b[1;34m";
    let reset = "\x1b[0m";

    if !pretty {
        println!("{:02X?}", bytes);
        return;
    }

    match obj.tag.class {
        TagClass::Universal => match obj.tag.number {
            1 => {
                let value = !bytes.is_empty() && bytes[0] != 0;
                println!("{}  {}BOOLEAN:{} {}", indent_str, tag_color, reset, value);
            }
            2 => {
                let value = num_bigint::BigUint::from_bytes_be(bytes);
                println!("{}  {}INTEGER:{} {} ({} bytes)", indent_str, tag_color, reset, value, bytes.len());
            }
            3 => {
                if let Some((&padding_bits, bits)) = bytes.split_first() {
                    let bit_len = bits.len().saturating_mul(8).saturating_sub(padding_bits as usize);
                    println!("{}  {}BIT STRING:{} ({} bits, {} padding): {:02X?}", indent_str, tag_color, reset, bit_len, padding_bits, bits);
                } else {
                    println!("{}  {}BIT STRING:{} <empty>", indent_str, tag_color, reset);
                }
            }
            4 => {
                println!("{}  {}OCTET STRING:{} ({} bytes): {:02X?}", indent_str, tag_color, reset, bytes.len(), bytes);
            }
            5 => {
                println!("{}  {}NULL:{} (0 bytes)", indent_str, tag_color, reset);
            }
            6 => {
                if let Some(first) = bytes.first() {
                    let mut oid = vec![(first / 40).to_string(), (first % 40).to_string()];
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
            }
            19 | 20 | 22 => match std::str::from_utf8(bytes) {
                Ok(text) => println!("{}  {}String:{} '{}' ({} bytes)", indent_str, tag_color, reset, text, bytes.len()),
                Err(_) => println!("{}  {}String:{} <invalid UTF-8> ({:?})", indent_str, tag_color, reset, bytes),
            },
            23 | 24 => match std::str::from_utf8(bytes) {
                Ok(time) => println!("{}  {}Time:{} '{}' ({} bytes)", indent_str, tag_color, reset, time, bytes.len()),
                Err(_) => println!("{}  {}Time:{} <invalid UTF-8> ({:?})", indent_str, tag_color, reset, bytes),
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

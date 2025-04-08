// src/der_parser.rs

pub struct DerParser<'a> {
    input: &'a [u8],
    position: usize,
}

#[derive(Debug, PartialEq)]
pub enum TagClass {
    Universal,
    Application,
    ContextSpecific,
    Private,
}

#[derive(Debug, PartialEq)]
pub struct Tag {
    pub class: TagClass,
    pub constructed: bool,
    pub number: u32,
}

#[derive(Debug, PartialEq)]
pub struct ASN1Object<'a> {
    pub tag: Tag,
    pub value: ASN1Value<'a>,
}

#[derive(Debug, PartialEq)]
pub enum ASN1Value<'a> {
    Primitive(&'a [u8]),
    Constructed(Vec<ASN1Object<'a>>),
}

#[derive(Debug, PartialEq)]
pub enum ASN1Error {
    UnexpectedEOF,
    InvalidTag,
    InvalidLength,
    IndefiniteLengthNotAllowed,
    TrailingData,
}

impl<'a> DerParser<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            position: 0,
        }
    }

    pub fn peek(&self) -> Option<u8> {
        self.input.get(self.position).copied()
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        if self.position < self.input.len() {
            let byte = self.input[self.position];
            self.position += 1;
            Some(byte)
        } else {
            None
        }
    }

    pub fn read_n(&mut self, n: usize) -> Option<&'a [u8]> {
        if self.position + n <= self.input.len() {
            let slice = &self.input[self.position..self.position + n];
            self.position += n;
            Some(slice)
        } else {
            None
        }
    }

    pub fn is_done(&self) -> bool {
        self.position >= self.input.len()
    }

    pub fn read_tag(&mut self) -> Option<Tag> {
        let first_byte = self.read_byte()?;

        let class = match first_byte >> 6 {
            0b00 => TagClass::Universal,
            0b01 => TagClass::Application,
            0b10 => TagClass::ContextSpecific,
            0b11 => TagClass::Private,
            _ => unreachable!(),
        };

        let constructed = (first_byte & 0b0010_0000) != 0;
        let mut number = (first_byte & 0b0001_1111) as u32;

        if number == 0b0001_1111 {
            number = 0;
            loop {
                let byte = self.read_byte()? as u32;
                number = (number << 7) | (byte & 0b0111_1111);
                if (byte & 0b1000_0000) == 0 {
                    break;
                }
            }
        }

        Some(Tag {
            class,
            constructed,
            number,
        })
    }

    pub fn read_length(&mut self) -> Option<usize> {
        let first = self.read_byte()? as usize;

        if first & 0x80 == 0 {
            // Short form: length is in the lower 7 bits
            Some(first)
        } else {
            let num_bytes = first & 0x7F;
            if num_bytes == 0 {
                // Indefinite length not allowed by DER
                return None;
            }

            let bytes = self.read_n(num_bytes)?;
            let mut length = 0usize;

            for &b in bytes {
                length = (length << 8) | b as usize;
            }
            Some(length)
        }
    }

    pub fn read_value(&mut self, length: usize) -> Option<&'a [u8]> {
        self.read_n(length)   
    }

    pub fn parse_tlv(&mut self) -> Result<ASN1Object<'a>, ASN1Error> {
        let tag = self.read_tag().ok_or(ASN1Error::InvalidTag)?;
        let length = self.read_length().ok_or(ASN1Error::InvalidLength)?;
        let value = self.read_value(length).ok_or(ASN1Error::UnexpectedEOF)?;
        let value = if tag.constructed {
            let mut parser = DerParser::new(value);
            let result = parser.parse_all()?;
            ASN1Value::Constructed(result)
        } else {
            ASN1Value::Primitive(value)
        };
        Ok(ASN1Object {
            tag,
            value,
        })
    }

    pub fn parse_all(&mut self) -> Result<Vec<ASN1Object<'a>>, ASN1Error> {
        let mut der_data = Vec::new();
        while !self.is_done() {
            let object = self.parse_tlv();
            match object {
                Ok(object) => der_data.push(object),
                Err(err) => return Err(err)
            }
        }
        Ok(der_data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_byte() {
        let data = [0xDE, 0xAD, 0xBE, 0xEF];
        let mut parser = DerParser::new(&data);

        assert_eq!(parser.read_byte(), Some(0xDE));
        assert_eq!(parser.read_byte(), Some(0xAD));
        assert_eq!(parser.read_byte(), Some(0xBE));
        assert_eq!(parser.read_byte(), Some(0xEF));
        assert_eq!(parser.read_byte(), None);
    }

    #[test]
    fn test_read_n() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05];
        let mut parser = DerParser::new(&data);

        assert_eq!(parser.read_n(3), Some(&[0x01, 0x02, 0x03][..]));
        assert_eq!(parser.read_n(2), Some(&[0x04, 0x05][..]));
        assert_eq!(parser.read_n(1), None);
    }

    #[test]
    fn test_peek_and_is_done() {
        let data = [0xAA];
        let mut parser = DerParser::new(&data);

        assert_eq!(parser.peek(), Some(0xAA));
        assert!(!parser.is_done());
        assert_eq!(parser.read_byte(), Some(0xAA));
        assert!(parser.is_done());
        assert_eq!(parser.peek(), None);
    }

    #[test]
    fn test_read_tag_simple() {
        let data = [0x30]; // SEQUENCE (constructed, universal, tag number 16)
        let mut parser = DerParser::new(&data);

        let tag = parser.read_tag().unwrap();
        assert_eq!(tag.class, TagClass::Universal);
        assert_eq!(tag.constructed, true);
        assert_eq!(tag.number, 16);
    }

    #[test]
    fn test_read_tag_long_form() {
        let data = [0x1F, 0x85, 0x01]; // Universal, primitive, tag number 0x0281 = 641
        let mut parser = DerParser::new(&data);

        let tag = parser.read_tag().unwrap();
        assert_eq!(tag.class, TagClass::Universal);
        assert_eq!(tag.constructed, false);
        assert_eq!(tag.number, 0x0281);
    }

    #[test]
    fn test_read_length_short() {
        let data = [0x0A]; // short-form: length = 10
        let mut parser = DerParser::new(&data);
        assert_eq!(parser.read_length(), Some(10));
    }

    #[test]
    fn test_read_length_long() {
        let data = [0x82, 0x01, 0xF4]; // long-form: 0x01F4 = 500
        let mut parser = DerParser::new(&data);
        assert_eq!(parser.read_length(), Some(500));
    }

    #[test]
    fn test_read_length_invalid_indefinite() {
        let data = [0x80]; // indefinite-length not allowed in DER
        let mut parser = DerParser::new(&data);
        assert_eq!(parser.read_length(), None);
    }

    #[test]
    fn test_parse_tlv_primitive_integer() {
        let data = [0x02, 0x01, 0x05]; // INTEGER, length 1, value 5
        let mut parser = DerParser::new(&data);
        let obj = parser.parse_tlv().unwrap();

        assert_eq!(obj.tag.class, TagClass::Universal);
        assert!(!obj.tag.constructed);
        assert_eq!(obj.tag.number, 2);

        match obj.value {
            ASN1Value::Primitive(val) => assert_eq!(val, &[0x05]),
            _ => panic!("Expected primitive value"),
        }
    }


    #[test]
    fn test_parse_tlv_constructed_sequence() {
        let data = [0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x02];
        let mut parser = DerParser::new(&data);
        let obj = parser.parse_tlv().unwrap();

        assert_eq!(obj.tag.class, TagClass::Universal);
        assert!(obj.tag.constructed);
        assert_eq!(obj.tag.number, 16); // SEQUENCE

        match obj.value {
            ASN1Value::Constructed(children) => {
                assert_eq!(children.len(), 2);
                assert_eq!(children[0].tag.number, 2); // INTEGER
                assert_eq!(children[1].tag.number, 2); // INTEGER
            },
            _ => panic!("Expected constructed value"),
        }
    }

    #[test]
    fn test_parse_all_multiple_primitive_integers() {
        let data = [
            0x02, 0x01, 0x01, // INTEGER 1
            0x02, 0x01, 0x02, // INTEGER 2
            0x02, 0x01, 0x03  // INTEGER 3
        ];
        let mut parser = DerParser::new(&data);
        let result = parser.parse_all().unwrap();

        assert_eq!(result.len(), 3);

        for (i, obj) in result.iter().enumerate() {
            assert_eq!(obj.tag.class, TagClass::Universal);
            assert!(!obj.tag.constructed);
            assert_eq!(obj.tag.number, 2); // INTEGER

            match obj.value {
                ASN1Value::Primitive(val) => assert_eq!(val, &[i as u8 + 1]),
                _ => panic!("Expected primitive value"),
            }
        }
    }
}

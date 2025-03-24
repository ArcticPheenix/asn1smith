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
        let data = [0x1F, 0x85, 0x01]; // Universal, primitive, tag number 0x0501 = 1281
        let mut parser = DerParser::new(&data);

        let tag = parser.read_tag().unwrap();
        assert_eq!(tag.class, TagClass::Universal);
        assert_eq!(tag.constructed, false);
        assert_eq!(tag.number, 0x0501);
    }
}

use crate::helpers::*;

#[derive(Copy, Clone, Debug)]
pub struct Question<'a> {
    pub buffer: &'a [u8],
    pub offset: usize,
}

impl<'a> Question<'a> {
    // https://stackoverflow.com/a/47258845
    pub fn qname(&self) -> Vec<String> {
        parse_name(&self.buffer, self.offset).0
    }

    pub fn qtype(&self) -> u16 {
        parse_u16(&self.buffer[self.offset..], self.len() - 4)
    }

    pub fn get_type(&self) -> ResourceRecordType {
        parse_type_code(self.qtype())
    }

    pub fn qclass(&self) -> u16 {
        parse_u16(&self.buffer[self.offset..], self.len() - 2)
    }

    pub fn len(&self) -> usize {
        parse_name(&self.buffer, self.offset).1 + 4
    }
}

use crate::helpers::*;

#[derive(Copy, Clone, Debug)]
pub struct Question<'a> {
    pub buffer: &'a [u8],
    pub offset: usize,
}

impl<'a> Question<'a> {
    // https://stackoverflow.com/a/47258845
    pub fn qname(&self) -> Option<Vec<String>> {
        Some(parse_name(&self.buffer, self.offset)?.0)
    }

    pub fn qtype(&self) -> Option<u16> {
        Some(parse_u16(&self.buffer[self.offset..], self.len()? - 4)?)
    }

    pub fn get_type(&self) -> Option<ResourceRecordType> {
        Some(parse_type_code(self.qtype()?))
    }

    pub fn qclass(&self) -> Option<u16> {
        Some(parse_u16(&self.buffer[self.offset..], self.len()? - 2)?)
    }

    pub fn len(&self) -> Option<usize> {
        Some(parse_name(&self.buffer, self.offset)?.1 + 4)
    }
}

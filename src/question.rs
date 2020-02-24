use crate::helpers::*;
use std::error::Error;

#[derive(Copy, Clone, Debug)]
pub struct Question<'a> {
    pub buffer: &'a [u8],
    pub offset: usize,
}

impl<'a> Question<'a> {
    // https://stackoverflow.com/a/47258845
    pub fn qname(&self) -> Result<Vec<String>, Box<dyn Error>> {
        Ok(parse_name(&self.buffer, self.offset)?.0)
    }

    pub fn qtype(&self) -> Result<u16, Box<dyn Error>> {
        Ok(parse_u16(
            &self
                .buffer
                .get(self.offset..)
                .ok_or(MalformedMessageError)?,
            self.len()? - 4,
        )?)
    }

    pub fn get_type(&self) -> Result<ResourceRecordType, Box<dyn Error>> {
        Ok(parse_type_code(self.qtype()?))
    }

    pub fn qclass(&self) -> Result<u16, Box<dyn Error>> {
        Ok(parse_u16(
            &self
                .buffer
                .get(self.offset..)
                .ok_or(MalformedMessageError)?,
            self.len()? - 2,
        )?)
    }

    pub fn len(&self) -> Result<usize, Box<dyn Error>> {
        Ok(parse_name(&self.buffer, self.offset)?.1 + 4)
    }
}

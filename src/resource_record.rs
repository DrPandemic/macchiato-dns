use crate::helpers::*;

// https://tools.ietf.org/html/rfc6891#section-6.1.2
// RRs with type Opt(41), are not parsed the same. It's the same format, but different names.
// However, TTL seems to be parsed in multiple entries.
#[derive(Debug)]
pub struct ResourceRecord {
    pub name: Vec<String>,
    pub type_code: u16,
    pub class: u16,
    pub ttl: u32,
    pub rdlength: u16,
    pub rdata: Vec<u8>,
    pub size: usize,
}

impl ResourceRecord {
    pub fn get_type(&self) -> ResourceRecordType {
        parse_type_code(self.type_code)
    }

    pub fn a_data(&self) -> Option<u32> {
        if self.type_code != 1 || self.rdlength != 4 {
            None
        } else {
            Some(parse_u32(&self.rdata, 0))
        }
    }

    pub fn get_buffer(&self) -> Vec<u8> {
        let mut buffer = vec![];

        let mut name = encode_name(self.name.clone());
        buffer.append(&mut name);
        let type_code = split_u16_into_u8(self.type_code);
        buffer.append(&mut type_code.to_vec());
        let class = split_u16_into_u8(self.class);
        buffer.append(&mut class.to_vec());
        let ttl = split_u32_into_u8(self.ttl);
        buffer.append(&mut ttl.to_vec());
        let rdlength = split_u16_into_u8(self.rdlength);
        buffer.append(&mut rdlength.to_vec());
        buffer.append(&mut self.rdata.clone());

        buffer
    }
}

pub fn generate_answer_a(name: &Vec<String>, address: Vec<u8>) -> ResourceRecord {
    ResourceRecord {
        name: name.clone(),
        type_code: 1,
        class: 1,
        ttl: 86400,
        rdlength: 4,
        rdata: address,
        size: name.into_iter().map(|a| a.len()).sum::<usize>() + 4 + 10 + 1,
    }
}

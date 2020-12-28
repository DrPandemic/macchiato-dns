use crate::helpers::*;
use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::error::Error;

// https://tools.ietf.org/html/rfc6891#section-6.1.2 RRs with type Opt(41), are not parsed the same way. It's the same
// format, but different names. However, TTL seems to be parsed in multiple entries.

// https://en.wikipedia.org/wiki/Extension_mechanisms_for_DNS#Mechanism
// > The mechanism is backward compatible, because older DNS responders ignore any RR of the unknown OPT type in a
// request and a newer DNS responder never includes an OPT in a response unless there was one in the request. The
// presence of the OPT in the request signifies a newer requester that knows what to do with an OPT in the response.
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

impl Serialize for ResourceRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("ResourceRecord", 7)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("type_code", &self.type_code)?;
        state.serialize_field("class", &self.class)?;
        state.serialize_field("ttl", &self.ttl)?;
        state.serialize_field("rdlength", &self.rdlength)?;
        state.serialize_field(
            "rdata",
            &self
                .rdata
                .iter()
                .map(|n| format!("{}", n))
                .collect::<Vec<String>>()
                .join("."),
        )?;
        state.serialize_field("size", &self.size)?;
        state.end()
    }
}

impl ResourceRecord {
    pub fn get_type(&self) -> ResourceRecordType {
        parse_type_code(self.type_code)
    }

    pub fn a_data(&self) -> Option<u32> {
        if self.type_code != 1 || self.rdlength != 4 {
            None
        } else {
            Some(parse_u32(&self.rdata, 0).ok()?)
        }
    }

    pub fn get_buffer(&self) -> Result<Vec<u8>, Box<dyn Error>> {
        let mut buffer = vec![];

        let mut name = encode_name(self.name.clone());
        buffer.append(&mut name);
        let type_code = split_u16_into_u8(self.type_code)?;
        buffer.append(&mut type_code.to_vec());
        let class = split_u16_into_u8(self.class)?;
        buffer.append(&mut class.to_vec());
        let ttl = split_u32_into_u8(self.ttl)?;
        buffer.append(&mut ttl.to_vec());
        let rdlength = split_u16_into_u8(self.rdlength)?;
        buffer.append(&mut rdlength.to_vec());
        buffer.append(&mut self.rdata.clone());

        Ok(buffer)
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

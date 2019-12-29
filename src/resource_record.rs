// https://tools.ietf.org/html/rfc6891#section-6.1.2
// RRs with type Opt(41), are not parsed the same. It's the same format, but different names.
// However, TTL seems to be parsed in multiple entries.
pub struct ResourceRecord<'a> {
    pub buffer: &'a[u8],
    pub name: Vec<String>,
    pub type_code: u16,
    pub class: u16,
    pub ttl: u32,
    pub rdlength: u16,
    pub rdata: &'a[u8],
    pub size: usize,
}

impl <'a> ResourceRecord<'a> {
    pub fn get_type(&self) -> ResourceRecordType {
        match self.type_code {
            1 => ResourceRecordType::A,
            5 => ResourceRecordType::CName,
            41 => ResourceRecordType::Opt,
            _ => ResourceRecordType::Other,
        }
    }
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum ResourceRecordType {
    A,
    CName,
    Opt,
    Other,
}


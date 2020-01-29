use std::convert::TryFrom;

pub fn split_u16_into_u8(data: u16) -> [u8; 2] {
    let a: u8 = u8::try_from(data >> 8).unwrap();
    let b: u8 = u8::try_from((data << 8) >> 8).unwrap();
    [a, b]
}

pub fn split_u32_into_u8(data: u32) -> [u8; 4] {
    let a: u8 = u8::try_from(data >> 24).unwrap();
    let b: u8 = u8::try_from((data << 8) >> 24).unwrap();
    let c: u8 = u8::try_from((data << 16) >> 24).unwrap();
    let d: u8 = u8::try_from((data << 24) >> 24).unwrap();
    [a, b, c, d]
}

pub fn parse_u16(buffer: &[u8], position: usize) -> u16 {
    (u16::from(buffer[position]) << 8) | u16::from(buffer[position + 1])
}

pub fn parse_u32(buffer: &[u8], position: usize) -> u32 {
    (u32::from(buffer[position]) << 24)
        | (u32::from(buffer[position + 1]) << 16)
        | (u32::from(buffer[position + 2]) << 8)
        | u32::from(buffer[position + 3])
}

pub fn parse_name(buffer: &[u8], offset: usize) -> (Vec<String>, usize) {
    let mut strings = vec![];
    let mut i = offset;
    loop {
        let size = buffer[i];
        if size == 0 {
            i += 1;
            break;
        } else if size == 192 {
            let pointer: u16 = (parse_u16(buffer, i) << 2) >> 2;
            let (mut other_names, _) = parse_name(&buffer, pointer as usize);
            strings.append(&mut other_names);
            i += 2;
            break;
        } else {
            let name: String = buffer[i + 1..i + 1 + size as usize]
                .iter()
                .cloned()
                .map(char::from)
                .collect();
            strings.push(name);
            i += (1 + size) as usize;
        }
    }
    (strings, i - offset)
}

pub fn encode_name(name: Vec<String>) -> Vec<u8> {
    let mut buffer = name.into_iter().fold(vec![], |mut acc, part| {
        acc.push(part.len() as u8);
        part.into_bytes()
            .into_iter()
            .for_each(|byte| acc.push(byte));
        acc
    });

    buffer.push(0);

    buffer
}

pub fn parse_type_code(code: u16) -> ResourceRecordType {
    match code {
        1 => ResourceRecordType::A,
        5 => ResourceRecordType::CName,
        41 => ResourceRecordType::Opt,
        _ => ResourceRecordType::Other,
    }
}

#[derive(PartialEq, Debug)]
pub enum ResourceRecordType {
    A,
    CName,
    Opt,
    Other,
}

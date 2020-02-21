use std::convert::TryFrom;

pub fn split_u16_into_u8(data: u16) -> Option<[u8; 2]> {
    let a: u8 = u8::try_from(data.checked_shr(8)?).ok()?;
    let b: u8 = u8::try_from(data.checked_shl(8)?.checked_shr(8)?).ok()?;
    Some([a, b])
}

pub fn split_u32_into_u8(data: u32) -> Option<[u8; 4]> {
    let a: u8 = u8::try_from(data.checked_shr(24)?).ok()?;
    let b: u8 = u8::try_from(data.checked_shl(8)?.checked_shr(24)?).ok()?;
    let c: u8 = u8::try_from(data.checked_shl(16)?.checked_shr(24)?).ok()?;
    let d: u8 = u8::try_from(data.checked_shl(24)?.checked_shr(24)?).ok()?;
    Some([a, b, c, d])
}

pub fn parse_u16(buffer: &[u8], position: usize) -> Option<u16> {
    Some(
        (u16::from(*buffer.get(position)?).checked_shl(8)?) | u16::from(*buffer.get(position + 1)?),
    )
}

pub fn parse_u32(buffer: &[u8], position: usize) -> Option<u32> {
    Some(
        (u32::from(*buffer.get(position)?).checked_shl(24)?)
            | (u32::from(*buffer.get(position + 1)?).checked_shl(16)?)
            | (u32::from(*buffer.get(position + 2)?).checked_shl(8)?)
            | u32::from(*buffer.get(position + 3)?),
    )
}

pub fn parse_name(buffer: &[u8], offset: usize) -> Option<(Vec<String>, usize)> {
    let mut strings = vec![];
    let mut i = offset;
    loop {
        let size = buffer[i];
        if size == 0 {
            i += 1;
            break;
        } else if size == 192 {
            let pointer: u16 = (parse_u16(buffer, i)? << 2) >> 2;
            let (mut other_names, _) = parse_name(&buffer, pointer as usize)?;
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
    Some((strings, i - offset))
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

pub fn log_error(message: &str, verbosity: u8) {
    if verbosity > 2 {
        println!("{:?}", message);
    }
}

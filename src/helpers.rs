use std::convert::TryFrom;
use std::error;
use std::error::Error;
use std::fmt;
use std::net::{Ipv4Addr, SocketAddr};
use std::collections::HashMap;

pub fn split_u16_into_u8(data: u16) -> Result<[u8; 2], Box<dyn Error>> {
    let a: u8 = u8::try_from(data.checked_shr(8).ok_or(DataTransformationError)?)?;
    let b: u8 = u8::try_from(
        data.checked_shl(8)
            .ok_or(DataTransformationError)?
            .checked_shr(8)
            .ok_or(DataTransformationError)?,
    )?;
    Ok([a, b])
}

pub fn split_u32_into_u8(data: u32) -> Result<[u8; 4], Box<dyn Error>> {
    let a: u8 = u8::try_from(data.checked_shr(24).ok_or(DataTransformationError)?)?;
    let b: u8 = u8::try_from(
        data.checked_shl(8)
            .ok_or(DataTransformationError)?
            .checked_shr(24)
            .ok_or(DataTransformationError)?,
    )?;
    let c: u8 = u8::try_from(
        data.checked_shl(16)
            .ok_or(DataTransformationError)?
            .checked_shr(24)
            .ok_or(DataTransformationError)?,
    )?;
    let d: u8 = u8::try_from(
        data.checked_shl(24)
            .ok_or(DataTransformationError)?
            .checked_shr(24)
            .ok_or(DataTransformationError)?,
    )?;
    Ok([a, b, c, d])
}

pub fn parse_u16(buffer: &[u8], position: usize) -> Result<u16, Box<dyn Error>> {
    Ok((u16::from(*buffer.get(position).ok_or(DataTransformationError)?)
        .checked_shl(8)
        .ok_or(DataTransformationError)?)
        | u16::from(*buffer.get(position + 1).ok_or(DataTransformationError)?))
}

pub fn parse_u32(buffer: &[u8], position: usize) -> Result<u32, Box<dyn Error>> {
    Ok((u32::from(*buffer.get(position).ok_or(DataTransformationError)?)
        .checked_shl(24)
        .ok_or(DataTransformationError)?)
        | (u32::from(*buffer.get(position + 1).ok_or(DataTransformationError)?)
            .checked_shl(16)
            .ok_or(DataTransformationError)?)
        | (u32::from(*buffer.get(position + 2).ok_or(DataTransformationError)?)
            .checked_shl(8)
            .ok_or(DataTransformationError)?)
        | u32::from(*buffer.get(position + 3).ok_or(DataTransformationError)?))
}

pub fn parse_name(buffer: &[u8], offset: usize) -> Result<(Vec<String>, usize), Box<dyn Error>> {
    let mut strings = vec![];
    let mut i = offset;
    loop {
        let size = buffer.get(i).ok_or(MalformedMessageError)?;
        if *size == 0 {
            i += 1;
            break;
        } else if *size == 192 {
            let pointer: u16 = (parse_u16(buffer, i)? << 2) >> 2;
            let (mut other_names, _) = parse_name(&buffer, pointer as usize)?;
            strings.append(&mut other_names);
            i += 2;
            break;
        } else {
            let name: String = buffer
                .get(i + 1..i + 1 + *size as usize)
                .ok_or(MalformedMessageError)?
                .iter()
                .cloned()
                .map(char::from)
                .collect();
            strings.push(name);
            i += (1 + size) as usize;
        }
    }
    Ok((strings, i - offset))
}

pub fn encode_name(name: Vec<String>) -> Vec<u8> {
    let mut buffer = name.into_iter().fold(vec![], |mut acc, part| {
        acc.push(part.len() as u8);
        part.into_bytes().into_iter().for_each(|byte| acc.push(byte));
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

#[derive(PartialEq, Debug, serde::Serialize)]
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

#[derive(Debug, Clone)]
pub struct MalformedMessageError;

impl fmt::Display for MalformedMessageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "couldn't interpret some data.")
    }
}

impl error::Error for MalformedMessageError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

#[derive(Debug, Clone)]
pub struct DataTransformationError;

impl fmt::Display for DataTransformationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "couldn't cast the data.")
    }
}

impl error::Error for DataTransformationError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        None
    }
}

pub fn ip_in_cidr(ip: SocketAddr, cidr: &str) -> bool {
    if let SocketAddr::V4(ipv4_addr) = ip {
        let ip = *ipv4_addr.ip();

        let parts: Vec<&str> = cidr.split('/').collect();
        if parts.len() != 2 {
            return false;
        }

        let network_ip: Ipv4Addr = match parts[0].parse() {
            Ok(ip) => ip,
            Err(_) => return false,
        };

        let prefix_len: u8 = match parts[1].parse() {
            Ok(len) if len <= 32 => len,
            _ => return false,
        };

        let mask = if prefix_len == 0 {
            0u32
        } else {
            !((1u32 << (32 - prefix_len)) - 1)
        };

        let ip_num = u32::from(ip);
        let network_num = u32::from(network_ip);

        (ip_num & mask) == (network_num & mask)
    } else {
        false
    }
}

pub fn client_in_override_network(client_addr: SocketAddr, network_overrides: &HashMap<String, HashMap<String, Vec<u8>>>) -> bool {
    for cidr in network_overrides.keys() {
        if ip_in_cidr(client_addr, cidr) {
            return true;
        }
    }
    false
}

pub fn has_network_override_for_domain(client_addr: SocketAddr, domain: &str, network_overrides: &HashMap<String, HashMap<String, Vec<u8>>>) -> bool {
    for (cidr, domain_overrides) in network_overrides {
        if ip_in_cidr(client_addr, cidr) && domain_overrides.contains_key(domain) {
            return true;
        }
    }
    false
}

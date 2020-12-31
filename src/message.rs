use serde::ser::{Serialize, SerializeStruct, Serializer};
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use tokio::net::udp::SendHalf;

use crate::helpers::*;
use crate::question::*;
use crate::resource_record::*;

#[derive(Clone, Debug)]
pub struct Message {
    pub buffer: Vec<u8>,
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub enum OpCode {
    QUERY,
    IQUERY,
    STATUS,
    Other,
}

#[derive(Debug, PartialEq, serde::Serialize)]
pub enum RCode {
    NoError,
    FormatError,
    ServerFailure,
    NameError,
    NotImplemented,
    Refused,
    Other,
}

impl Message {
    pub fn id(&self) -> Result<u16, Box<dyn Error>> {
        Ok(parse_u16(&self.buffer, 0)?)
    }

    pub fn set_id(&mut self, id: u16) -> Result<(), Box<dyn Error>> {
        self.write_buffer(0, &split_u16_into_u8(id)?)?;

        Ok(())
    }

    pub fn qr(&self) -> Result<bool, Box<dyn Error>> {
        Ok((self.buffer.get(2).ok_or(MalformedMessageError)? >> 7) == 0b1u8)
    }

    pub fn set_qr(&mut self, answer: bool) -> Result<(), Box<dyn Error>> {
        let data = self.buffer.get_mut(2).ok_or(MalformedMessageError)?;
        if answer {
            *data |= 0b10000000;
        } else {
            *data &= 0b01111111;
        }
        Ok(())
    }

    pub fn opcode(&self) -> Result<OpCode, Box<dyn Error>> {
        Ok(
            match ((self.buffer.get(2).ok_or(MalformedMessageError)? << 1) as u8) >> 4 {
                0b0 => OpCode::QUERY,
                0b1 => OpCode::IQUERY,
                0b10 => OpCode::STATUS,
                _ => OpCode::Other,
            },
        )
    }

    pub fn aa(&self) -> Result<bool, Box<dyn Error>> {
        let data: &u8 = self.buffer.get(2).ok_or(MalformedMessageError)?;
        Ok((*data << 5) >> 7 == 1)
    }

    pub fn tc(&self) -> Result<bool, Box<dyn Error>> {
        let data: &u8 = self.buffer.get(2).ok_or(MalformedMessageError)?;
        Ok((*data << 6) >> 7 == 0b1)
    }

    pub fn rd(&self) -> Result<bool, Box<dyn Error>> {
        let data: &u8 = self.buffer.get(2).ok_or(MalformedMessageError)?;
        Ok((*data << 7) >> 7 == 0b1)
    }

    pub fn ra(&self) -> Result<bool, Box<dyn Error>> {
        let data: &u8 = self.buffer.get(3).ok_or(MalformedMessageError)?;
        Ok(*data >> 7 == 0b1)
    }

    pub fn z(&self) -> Result<bool, Box<dyn Error>> {
        let data: &u8 = self.buffer.get(3).ok_or(MalformedMessageError)?;
        Ok((*data << 1) >> 7 == 0b1)
    }

    // http://www.networksorcery.com/enp/rfc/rfc3655.txt
    // https://tools.ietf.org/html/rfc6840#page-10
    pub fn ad(&self) -> Result<bool, Box<dyn Error>> {
        let data: &u8 = self.buffer.get(3).ok_or(MalformedMessageError)?;
        Ok((*data << 2) >> 7 == 0b1)
    }

    pub fn set_ad(&mut self, ad: bool) -> Result<(), Box<dyn Error>> {
        let data = self.buffer.get_mut(3).ok_or(MalformedMessageError)?;
        if ad {
            *data |= 0b00100000;
        } else {
            *data &= 0b11011111;
        }
        Ok(())
    }

    pub fn cd(&self) -> Result<bool, Box<dyn Error>> {
        let data: &u8 = self.buffer.get(3).ok_or(MalformedMessageError)?;
        Ok((*data << 3) >> 7 == 0b1)
    }

    pub fn rcode(&self) -> Result<RCode, Box<dyn Error>> {
        let data: &u8 = self.buffer.get(3).ok_or(MalformedMessageError)?;
        Ok(match (*data << 4) >> 4_u8 {
            0b0 => RCode::NoError,
            0b1 => RCode::FormatError,
            0b10 => RCode::ServerFailure,
            0b11 => RCode::NameError,
            0b100 => RCode::NotImplemented,
            0b101 => RCode::Refused,
            _ => RCode::Other,
        })
    }

    pub fn qdcount(&self) -> Result<u16, Box<dyn Error>> {
        Ok(parse_u16(&self.buffer, 4)?)
    }

    pub fn ancount(&self) -> Result<u16, Box<dyn Error>> {
        Ok(parse_u16(&self.buffer, 6)?)
    }

    pub fn set_ancount(&mut self, ancount: u16) -> Result<(), Box<dyn Error>> {
        self.write_buffer(6, &split_u16_into_u8(ancount)?)?;
        Ok(())
    }

    pub fn nscount(&self) -> Result<u16, Box<dyn Error>> {
        Ok(parse_u16(&self.buffer, 8)?)
    }

    pub fn arcount(&self) -> Result<u16, Box<dyn Error>> {
        Ok(parse_u16(&self.buffer, 10)?)
    }

    fn questions(&self) -> Result<Vec<Question>, Box<dyn Error>> {
        (0..self.qdcount()?)
            .fold(Ok((vec![], 12)), |maybe_acc, _| {
                let (mut acc, offset) = maybe_acc?;
                let name_end = self
                    .buffer
                    .get(offset..)
                    .ok_or(MalformedMessageError)?
                    .iter()
                    .position(|&c| c == 0b0)
                    .ok_or(MalformedMessageError)?;
                acc.push(Question {
                    buffer: &self
                        .buffer
                        .get(..(offset + name_end + 4 + 1))
                        .ok_or(MalformedMessageError)?,
                    offset: offset,
                });
                Ok((acc, offset + name_end + 4 + 1))
            })
            .map(|x| x.0)
    }

    // https://stackoverflow.com/a/4083071 multiple questions is not really supported
    pub fn question(&self) -> Result<Question, Box<dyn Error>> {
        Ok(self.questions()?[0])
    }

    pub fn name(&self) -> Result<String, Box<dyn Error>> {
        Ok(if let Ok(question) = self.question() {
            question.qname()?.join(".")
        } else {
            String::from("")
        })
    }

    fn parse_rr(&self, offset: usize) -> Result<ResourceRecord, Box<dyn Error>> {
        let (name, consumed_bytes) = parse_name(&self.buffer, offset)?;
        let post_name: usize = consumed_bytes + offset;
        let rdlength = parse_u16(&self.buffer, post_name + 8)?;
        let post_header = post_name + 10;
        // TODO: Class 41 shouldn't be parse as a RR. Maybe create a new struct for Opt?
        Ok(ResourceRecord {
            name: name,
            rdlength: rdlength,
            type_code: parse_u16(&self.buffer, post_name)?,
            class: parse_u16(&self.buffer, post_name + 2)?,
            ttl: parse_u32(&self.buffer, post_name + 4)?,
            rdata: self
                .buffer
                .get(post_header..post_header + rdlength as usize)
                .ok_or(MalformedMessageError)?
                .to_vec(),
            size: consumed_bytes + rdlength as usize + 10,
        })
    }

    pub fn set_response_ttl(&mut self, ttl: u32) -> Result<(), Box<dyn Error>> {
        let data = split_u32_into_u8(ttl)?;
        (0..self.ancount()?).fold(self.resource_records_offset(), |maybe_offset, _| {
            let offset = maybe_offset?;
            let (_, consumed_bytes) = parse_name(&self.buffer, offset)?;
            let ttl_offset: usize = consumed_bytes + offset + 4;

            self.write_buffer(ttl_offset, &data)?;

            let rr = self.parse_rr(offset)?;
            Ok(offset + rr.size)
        })?;

        Ok(())
    }

    fn resource_records_offset(&self) -> Result<usize, Box<dyn Error>> {
        Ok(12
            + self
                .questions()?
                .iter()
                .fold(Ok(0), |acc: Result<usize, Box<dyn Error>>, q| Ok(acc? + q.len()?))?)
    }

    pub fn resource_records(
        &self,
    ) -> Result<(Vec<ResourceRecord>, Vec<ResourceRecord>, Vec<ResourceRecord>), Box<dyn Error>> {
        let question_offset = self.resource_records_offset()?;
        let answers = (0..self.ancount()?).fold(
            Ok((vec![], question_offset)),
            |maybe_acc: Result<(Vec<ResourceRecord>, usize), Box<dyn Error>>, _| {
                let (mut acc, offset) = maybe_acc?;
                let rr = self.parse_rr(offset)?;
                let size = rr.size;
                acc.push(rr);
                Ok((acc, offset + size))
            },
        )?;
        let authorities = (0..self.nscount()?).fold(
            Ok((vec![], answers.1)),
            |maybe_acc: Result<(Vec<ResourceRecord>, usize), Box<dyn Error>>, _| {
                let (mut acc, offset) = maybe_acc?;
                let rr = self.parse_rr(offset)?;
                let size = rr.size;
                acc.push(rr);
                Ok((acc, offset + size))
            },
        )?;
        let additionals = (0..self.arcount()?).fold(
            Ok((vec![], authorities.1)),
            |maybe_acc: Result<(Vec<ResourceRecord>, usize), Box<dyn Error>>, _| {
                let (mut acc, offset) = maybe_acc?;
                let rr = self.parse_rr(offset)?;
                let size = rr.size;
                acc.push(rr);
                Ok((acc, offset + size))
            },
        )?;

        Ok((answers.0, authorities.0, additionals.0))
    }

    pub fn add_answer(&mut self, answer: ResourceRecord) -> Result<(), Box<dyn Error>> {
        self.set_ancount(self.ancount()? + 1)?;
        let new_buffer = self.buffer.clone();
        let split_point = 12
            + self
                .questions()
                .unwrap()
                .into_iter()
                .fold(Ok(0), |acc: Result<usize, Box<dyn Error>>, q| Ok(acc? + q.len()?))?;
        let (first, last) = new_buffer.split_at(split_point);
        self.buffer = vec![];
        self.buffer.extend_from_slice(&first);
        self.buffer.extend_from_slice(&answer.get_buffer()?);
        self.buffer.extend_from_slice(&last);

        Ok(())
    }

    pub async fn send_to(&self, socket: &mut SendHalf, target: &SocketAddr) -> io::Result<usize> {
        socket.send_to(&self.buffer, target).await
    }

    fn write_buffer(&mut self, position: usize, data: &[u8]) -> Result<(), Box<dyn Error>> {
        if (position + data.len()) > self.buffer.len() {
            return Err(Box::new(MalformedMessageError));
        }
        for (i, datum) in data.into_iter().enumerate() {
            *self.buffer.get_mut(position + i).ok_or(MalformedMessageError)? = *datum;
        }
        Ok(())
    }
}

pub fn parse_message(query: Vec<u8>) -> Message {
    Message { buffer: query }
}

pub fn generate_deny_response<'a>(query: &'a Message) -> Result<Message, Box<dyn Error>> {
    let mut message = Message {
        buffer: query.buffer.clone(),
    };

    message.set_qr(true)?;
    // Means don't understand DNSSEC. AD bit
    message.set_ad(false)?;
    message.add_answer(generate_answer_a(&query.question()?.qname()?, vec![0, 0, 0, 0]))?;

    Ok(message)
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Message", 17)?;
        state.serialize_field("id", &self.id().map_err(|e| serde::ser::Error::custom(e.to_string()))?)?;
        state.serialize_field("qr", &self.qr().map_err(|e| serde::ser::Error::custom(e.to_string()))?)?;
        state.serialize_field(
            "opcode",
            &self.opcode().map_err(|e| serde::ser::Error::custom(e.to_string()))?,
        )?;
        state.serialize_field("aa", &self.aa().map_err(|e| serde::ser::Error::custom(e.to_string()))?)?;
        state.serialize_field("tc", &self.tc().map_err(|e| serde::ser::Error::custom(e.to_string()))?)?;
        state.serialize_field("rd", &self.rd().map_err(|e| serde::ser::Error::custom(e.to_string()))?)?;
        state.serialize_field("ra", &self.z().map_err(|e| serde::ser::Error::custom(e.to_string()))?)?;
        state.serialize_field("ad", &self.ad().map_err(|e| serde::ser::Error::custom(e.to_string()))?)?;
        state.serialize_field("cd", &self.cd().map_err(|e| serde::ser::Error::custom(e.to_string()))?)?;
        state.serialize_field(
            "rcode",
            &self.rcode().map_err(|e| serde::ser::Error::custom(e.to_string()))?,
        )?;
        state.serialize_field(
            "qdcount",
            &self.qdcount().map_err(|e| serde::ser::Error::custom(e.to_string()))?,
        )?;
        state.serialize_field(
            "ancount",
            &self.ancount().map_err(|e| serde::ser::Error::custom(e.to_string()))?,
        )?;
        state.serialize_field(
            "nscount",
            &self.nscount().map_err(|e| serde::ser::Error::custom(e.to_string()))?,
        )?;
        state.serialize_field(
            "arcount",
            &self.arcount().map_err(|e| serde::ser::Error::custom(e.to_string()))?,
        )?;
        state.serialize_field(
            "question",
            &self.question().map_err(|e| serde::ser::Error::custom(e.to_string()))?,
        )?;
        state.serialize_field(
            "name",
            &self.name().map_err(|e| serde::ser::Error::custom(e.to_string()))?,
        )?;
        state.serialize_field(
            "resource_records",
            &self
                .resource_records()
                .map_err(|e| serde::ser::Error::custom(e.to_string()))?,
        )?;
        state.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMATEAPOT_QUESTION: [u8; 46] = [
        57, 32, 1, 32, 0, 1, 0, 0, 0, 0, 0, 1, 3, 119, 119, 119, 9, 105, 109, 97, 116, 101, 97, 112, 111, 116, 3, 111,
        114, 103, 0, 0, 1, 0, 1, 0, 0, 41, 16, 0, 0, 0, 0, 0, 0, 0,
    ];
    const IMATEAPOT_ANSWER: [u8; 95] = [
        57, 32, 129, 128, 0, 1, 0, 2, 0, 0, 0, 1, 3, 119, 119, 119, 9, 105, 109, 97, 116, 101, 97, 112, 111, 116, 3,
        111, 114, 103, 0, 0, 1, 0, 1, 192, 12, 0, 5, 0, 1, 0, 0, 84, 64, 0, 21, 5, 115, 104, 111, 112, 115, 9, 109,
        121, 115, 104, 111, 112, 105, 102, 121, 3, 99, 111, 109, 0, 192, 47, 0, 1, 0, 1, 0, 0, 5, 23, 0, 4, 23, 227,
        38, 64, 0, 0, 41, 2, 0, 0, 0, 0, 0, 0, 0,
    ];
    #[test]
    fn test_question_attributes() {
        let buffer = IMATEAPOT_QUESTION.to_vec();
        let message = parse_message(buffer);
        let question = message.question().expect("couldn't parse questions");
        let rrs = message.resource_records().expect("couldn't parse RRs");

        assert_eq!(message.id().unwrap(), 14624);
        assert_eq!(message.qr().unwrap(), false);
        assert_eq!(message.opcode().unwrap(), OpCode::QUERY);
        assert_eq!(message.aa().unwrap(), false);
        assert_eq!(message.tc().unwrap(), false);
        assert_eq!(message.rd().unwrap(), true);
        assert_eq!(message.ra().unwrap(), false);
        assert_eq!(message.z().unwrap(), false);
        assert_eq!(message.ad().unwrap(), true);
        assert_eq!(message.cd().unwrap(), false);
        assert_eq!(message.rcode().unwrap(), RCode::NoError);
        assert_eq!(message.qdcount().unwrap(), 1);
        assert_eq!(message.ancount().unwrap(), 0);
        assert_eq!(message.nscount().unwrap(), 0);
        assert_eq!(message.arcount().unwrap(), 1);
        assert_eq!(
            question.qname().unwrap(),
            vec![String::from("www"), String::from("imateapot"), String::from("org")]
        );
        assert_eq!(question.qtype().unwrap(), 1);
        assert_eq!(question.qclass().unwrap(), 1);

        assert_eq!(rrs.2[0].name, Vec::<String>::new());
        assert_eq!(rrs.2[0].get_type(), ResourceRecordType::Opt);
        assert_eq!(rrs.2[0].class, 4096);
    }

    #[test]
    fn test_answer_attributes() {
        let buffer = IMATEAPOT_ANSWER.to_vec();
        let message = parse_message(buffer);
        let question = message.question().expect("couldn't parse questions");
        let rrs = message.resource_records().expect("couldn't parse RRs");

        assert_eq!(message.id().unwrap(), 14624);
        assert_eq!(message.qr().unwrap(), true);
        assert_eq!(message.opcode().unwrap(), OpCode::QUERY);
        assert_eq!(message.aa().unwrap(), false);
        assert_eq!(message.tc().unwrap(), false);
        assert_eq!(message.rd().unwrap(), true);
        assert_eq!(message.ra().unwrap(), true);
        assert_eq!(message.z().unwrap(), false);
        assert_eq!(message.ad().unwrap(), false);
        assert_eq!(message.cd().unwrap(), false);
        assert_eq!(message.rcode().unwrap(), RCode::NoError);
        assert_eq!(message.qdcount().unwrap(), 1);
        assert_eq!(message.ancount().unwrap(), 2);
        assert_eq!(message.nscount().unwrap(), 0);
        assert_eq!(message.arcount().unwrap(), 1);
        assert_eq!(
            question.qname().unwrap(),
            vec![String::from("www"), String::from("imateapot"), String::from("org")]
        );
        assert_eq!(question.qtype().unwrap(), 1);
        assert_eq!(question.qclass().unwrap(), 1);

        assert_eq!(rrs.0[0].name, ["www", "imateapot", "org"]);
        assert_eq!(rrs.0[0].get_type(), ResourceRecordType::CName);
        assert_eq!(rrs.0[0].class, 1);
        assert_eq!(rrs.0[0].ttl, 21568);
        assert_eq!(rrs.0[0].rdlength, 21);

        assert_eq!(rrs.0[1].name, ["shops", "myshopify", "com"]);
        assert_eq!(rrs.0[1].get_type(), ResourceRecordType::A);
        assert_eq!(rrs.0[1].class, 1);
        assert_eq!(rrs.0[1].ttl, 1303);
        assert_eq!(rrs.0[1].rdlength, 4);

        assert_eq!(rrs.2[0].name, Vec::<String>::new());
        assert_eq!(rrs.2[0].get_type(), ResourceRecordType::Opt);
        assert_eq!(rrs.2[0].class, 512);
    }

    #[test]
    fn test_generate_deny_response() {
        let buffer = IMATEAPOT_QUESTION.to_vec();
        let message = parse_message(buffer);
        let answer = generate_deny_response(&message).unwrap();
        let question = answer.question().expect("couldn't parse questions");
        let rrs = answer.resource_records().expect("couldn't parse RRs");

        assert_eq!(answer.id().unwrap(), message.id().unwrap());
        assert_eq!(answer.qr().unwrap(), true);
        assert_eq!(answer.opcode().unwrap(), OpCode::QUERY);
        assert_eq!(answer.aa().unwrap(), false);
        assert_eq!(answer.tc().unwrap(), false);
        assert_eq!(answer.rd().unwrap(), true);
        assert_eq!(answer.ra().unwrap(), false);
        assert_eq!(answer.z().unwrap(), false);
        assert_eq!(answer.ad().unwrap(), false);
        assert_eq!(answer.cd().unwrap(), false);
        assert_eq!(answer.rcode().unwrap(), RCode::NoError);
        assert_eq!(answer.qdcount().unwrap(), 1);
        assert_eq!(answer.ancount().unwrap(), 1);
        assert_eq!(answer.nscount().unwrap(), 0);
        assert_eq!(answer.arcount().unwrap(), 1);
        assert_eq!(
            question.qname().unwrap(),
            vec![String::from("www"), String::from("imateapot"), String::from("org")]
        );
        assert_eq!(question.qtype().unwrap(), 1);
        assert_eq!(question.qclass().unwrap(), 1);

        assert_eq!(rrs.0[0].name, ["www", "imateapot", "org"]);
        assert_eq!(rrs.0[0].get_type(), ResourceRecordType::A);
        assert_eq!(rrs.0[0].class, 1);
        assert_eq!(rrs.0[0].ttl, 86400);
        assert_eq!(rrs.0[0].rdlength, 4);
        assert_eq!(rrs.0[0].a_data(), Some(0));
    }

    #[test]
    fn test_set_ttl() {
        let buffer = IMATEAPOT_ANSWER.to_vec();
        let mut message = parse_message(buffer);
        let rrs0 = message.resource_records().expect("couldn't parse RRs");

        assert_ne!(rrs0.0[0].ttl, 42);

        if message.set_response_ttl(42).is_err() {
            panic!("failed to set ttl");
        }
        let rrs1 = message.resource_records().expect("couldn't parse RRs");
        assert_eq!(rrs1.0[0].ttl, 42);
    }
}

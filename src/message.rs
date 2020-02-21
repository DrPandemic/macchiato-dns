use std::io;
use std::net::SocketAddr;
use tokio::net::udp::SendHalf;

use crate::helpers::*;
use crate::question::*;
use crate::resource_record::*;

#[derive(Clone)]
pub struct Message {
    pub buffer: Vec<u8>,
}

#[derive(Debug, PartialEq)]
pub enum OpCode {
    QUERY,
    IQUERY,
    STATUS,
    Other,
}

#[derive(Debug, PartialEq)]
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
    pub fn id(&self) -> Option<u16> {
        Some(parse_u16(&self.buffer, 0)?)
    }

    pub fn set_id(&mut self, id: u16) -> Option<()> {
        let data = split_u16_into_u8(id)?;
        self.buffer[0] = data[0];
        self.buffer[1] = data[1];

        Some(())
    }

    pub fn qr(&self) -> bool {
        (self.buffer[2] >> 7) == 0b1u8
    }

    pub fn set_qr(&mut self, answer: bool) {
        if answer {
            self.buffer[2] |= 0b10000000;
        } else {
            self.buffer[2] &= 0b01111111;
        }
    }

    pub fn opcode(&self) -> OpCode {
        match (self.buffer[2] << 1) >> 4 {
            0b0 => OpCode::QUERY,
            0b1 => OpCode::IQUERY,
            0b10 => OpCode::STATUS,
            _ => OpCode::Other,
        }
    }

    pub fn aa(&self) -> bool {
        (self.buffer[2] << 5) >> 7 == 0b1
    }

    pub fn tc(&self) -> bool {
        (self.buffer[2] << 6) >> 7 == 0b1
    }

    pub fn rd(&self) -> bool {
        (self.buffer[2] << 7) >> 7 == 0b1
    }

    pub fn ra(&self) -> bool {
        self.buffer[3] >> 7 == 0b1
    }

    pub fn z(&self) -> bool {
        (self.buffer[3] << 1) >> 7 == 0b1
    }

    // http://www.networksorcery.com/enp/rfc/rfc3655.txt
    // https://tools.ietf.org/html/rfc6840#page-10
    pub fn ad(&self) -> bool {
        (self.buffer[3] << 2) >> 7 == 0b1
    }

    pub fn set_ad(&mut self, ad: bool) {
        if ad {
            self.buffer[3] |= 0b00100000;
        } else {
            self.buffer[3] &= 0b11011111;
        }
    }

    pub fn cd(&self) -> bool {
        (self.buffer[3] << 3) >> 7 == 0b1
    }

    pub fn rcode(&self) -> RCode {
        match (self.buffer[3] << 4) >> 4 {
            0b0 => RCode::NoError,
            0b1 => RCode::FormatError,
            0b10 => RCode::ServerFailure,
            0b11 => RCode::NameError,
            0b100 => RCode::NotImplemented,
            0b101 => RCode::Refused,
            _ => RCode::Other,
        }
    }

    pub fn qdcount(&self) -> Option<u16> {
        Some(parse_u16(&self.buffer, 4)?)
    }

    pub fn ancount(&self) -> Option<u16> {
        Some(parse_u16(&self.buffer, 6)?)
    }

    pub fn set_ancount(&mut self, ancount: u16) -> Option<()> {
        let data = split_u16_into_u8(ancount)?;
        self.buffer[6] = data[0];
        self.buffer[7] = data[1];

        Some(())
    }

    pub fn nscount(&self) -> Option<u16> {
        Some(parse_u16(&self.buffer, 8)?)
    }

    pub fn arcount(&self) -> Option<u16> {
        Some(parse_u16(&self.buffer, 10)?)
    }

    fn questions(&self) -> Option<Vec<Question>> {
        (0..self.qdcount()?)
            .fold(Some((vec![], 12)), |maybe_acc, _| match maybe_acc {
                Some((mut acc, offset)) => {
                    let name_end = self.buffer[offset..].iter().position(|&c| c == 0b0)?;
                    acc.push(Question {
                        buffer: &self.buffer[..(offset + name_end + 4 + 1)],
                        offset: offset,
                    });
                    Some((acc, offset + name_end + 4 + 1))
                }
                _ => None,
            })
            .map(|x| x.0)
    }

    // https://stackoverflow.com/a/4083071 multiple questions is not really supported
    pub fn question(&self) -> Option<Question> {
        Some(self.questions()?[0])
    }

    pub fn name(&self) -> Option<String> {
        Some(if let Some(question) = self.question() {
            question.qname()?.join(".")
        } else {
            String::from("")
        })
    }

    fn parse_rr(&self, offset: usize) -> Option<ResourceRecord> {
        let (name, consumed_bytes) = parse_name(&self.buffer, offset)?;
        let post_name: usize = consumed_bytes + offset;
        let rdlength = parse_u16(&self.buffer, post_name + 8)?;
        let post_header = post_name + 10;
        // Class 41 shouldn't be parse as a RR. Maybe create a new struct for Opt?
        Some(ResourceRecord {
            name: name,
            rdlength: rdlength,
            type_code: parse_u16(&self.buffer, post_name)?,
            class: parse_u16(&self.buffer, post_name + 2)?,
            ttl: parse_u32(&self.buffer, post_name + 4)?,
            rdata: self.buffer[post_header..post_header + rdlength as usize].to_vec(),
            size: consumed_bytes + rdlength as usize + 10,
        })
    }

    pub fn set_response_ttl(&mut self, ttl: u32) -> Option<()> {
        let data = split_u32_into_u8(ttl)?;
        (0..self.ancount()?).fold(self.resouce_records_offset(), |maybe_offset, _| {
            let offset = maybe_offset?;
            let (_, consumed_bytes) = parse_name(&self.buffer, offset)?;
            let ttl_offset: usize = consumed_bytes + offset + 4;

            self.buffer[ttl_offset] = data[0];
            self.buffer[ttl_offset + 1] = data[1];
            self.buffer[ttl_offset + 2] = data[2];
            self.buffer[ttl_offset + 3] = data[3];

            let rr = self.parse_rr(offset)?;
            Some(offset + rr.size)
        })?;

        Some(())
    }

    fn resouce_records_offset(&self) -> Option<usize> {
        Some(
            12 + self
                .questions()?
                .iter()
                .fold(Some(0), |acc, q| Some(acc? + q.len()?))?,
        )
    }

    pub fn resource_records(
        &self,
    ) -> Option<(
        Vec<ResourceRecord>,
        Vec<ResourceRecord>,
        Vec<ResourceRecord>,
    )> {
        let question_offset = self.resouce_records_offset()?;
        let answers =
            (0..self.ancount()?).fold(Some((vec![], question_offset)), |maybe_acc, _| {
                let (mut acc, offset) = maybe_acc?;
                let rr = self.parse_rr(offset)?;
                let size = rr.size;
                acc.push(rr);
                Some((acc, offset + size))
            })?;
        let authorities =
            (0..self.nscount()?).fold(Some((vec![], answers.1)), |maybe_acc, _| {
                let (mut acc, offset) = maybe_acc?;
                let rr = self.parse_rr(offset)?;
                let size = rr.size;
                acc.push(rr);
                Some((acc, offset + size))
            })?;
        let additionals =
            (0..self.arcount()?).fold(Some((vec![], authorities.1)), |maybe_acc, _| {
                let (mut acc, offset) = maybe_acc?;
                let rr = self.parse_rr(offset)?;
                let size = rr.size;
                acc.push(rr);
                Some((acc, offset + size))
            })?;

        Some((answers.0, authorities.0, additionals.0))
    }

    pub fn add_answer(&mut self, answer: ResourceRecord) -> Option<()> {
        self.set_ancount(self.ancount()? + 1)?;
        let new_buffer = self.buffer.clone();
        let split_point = 12
            + self
                .questions()
                .unwrap()
                .into_iter()
                .fold(Some(0), |acc, q| Some(acc? + q.len()?))?;
        let (first, last) = new_buffer.split_at(split_point);
        self.buffer = vec![];
        self.buffer.extend_from_slice(&first);
        self.buffer.extend_from_slice(&answer.get_buffer()?);
        self.buffer.extend_from_slice(&last);

        Some(())
    }

    pub async fn send_to(&self, socket: &mut SendHalf, target: &SocketAddr) -> io::Result<usize> {
        socket.send_to(&self.buffer, target).await
    }
}

pub fn parse_message(query: Vec<u8>) -> Message {
    Message { buffer: query }
}

pub fn generate_deny_response<'a>(query: &'a Message) -> Option<Message> {
    let mut message = Message {
        buffer: query.buffer.clone(),
    };

    message.set_qr(true);
    // Means don't understand DNSSEC. AD bit
    message.set_ad(false);
    message.add_answer(generate_answer_a(
        &query.question()?.qname()?,
        vec![0, 0, 0, 0],
    ))?;

    Some(message)
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMATEAPOT_QUESTION: [u8; 46] = [
        57, 32, 1, 32, 0, 1, 0, 0, 0, 0, 0, 1, 3, 119, 119, 119, 9, 105, 109, 97, 116, 101, 97,
        112, 111, 116, 3, 111, 114, 103, 0, 0, 1, 0, 1, 0, 0, 41, 16, 0, 0, 0, 0, 0, 0, 0,
    ];
    const IMATEAPOT_ANSWER: [u8; 95] = [
        57, 32, 129, 128, 0, 1, 0, 2, 0, 0, 0, 1, 3, 119, 119, 119, 9, 105, 109, 97, 116, 101, 97,
        112, 111, 116, 3, 111, 114, 103, 0, 0, 1, 0, 1, 192, 12, 0, 5, 0, 1, 0, 0, 84, 64, 0, 21,
        5, 115, 104, 111, 112, 115, 9, 109, 121, 115, 104, 111, 112, 105, 102, 121, 3, 99, 111,
        109, 0, 192, 47, 0, 1, 0, 1, 0, 0, 5, 23, 0, 4, 23, 227, 38, 64, 0, 0, 41, 2, 0, 0, 0, 0,
        0, 0, 0,
    ];
    #[test]
    fn test_question_attributes() {
        let buffer = IMATEAPOT_QUESTION.to_vec();
        let message = parse_message(buffer);
        let question = message.question().expect("couldn't parse questions");
        let rrs = message.resource_records().expect("couldn't parse RRs");

        assert_eq!(message.id(), Some(14624));
        assert_eq!(message.qr(), false);
        assert_eq!(message.opcode(), OpCode::QUERY);
        assert_eq!(message.aa(), false);
        assert_eq!(message.tc(), false);
        assert_eq!(message.rd(), true);
        assert_eq!(message.ra(), false);
        assert_eq!(message.z(), false);
        assert_eq!(message.ad(), true);
        assert_eq!(message.cd(), false);
        assert_eq!(message.rcode(), RCode::NoError);
        assert_eq!(message.qdcount(), Some(1));
        assert_eq!(message.ancount(), Some(0));
        assert_eq!(message.nscount(), Some(0));
        assert_eq!(message.arcount(), Some(1));
        assert_eq!(
            question.qname(),
            Some(vec![
                String::from("www"),
                String::from("imateapot"),
                String::from("org")
            ])
        );
        assert_eq!(question.qtype(), Some(1));
        assert_eq!(question.qclass(), Some(1));

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

        assert_eq!(message.id(), Some(14624));
        assert_eq!(message.qr(), true);
        assert_eq!(message.opcode(), OpCode::QUERY);
        assert_eq!(message.aa(), false);
        assert_eq!(message.tc(), false);
        assert_eq!(message.rd(), true);
        assert_eq!(message.ra(), true);
        assert_eq!(message.z(), false);
        assert_eq!(message.ad(), false);
        assert_eq!(message.cd(), false);
        assert_eq!(message.rcode(), RCode::NoError);
        assert_eq!(message.qdcount(), Some(1));
        assert_eq!(message.ancount(), Some(2));
        assert_eq!(message.nscount(), Some(0));
        assert_eq!(message.arcount(), Some(1));
        assert_eq!(
            question.qname(),
            Some(vec![
                String::from("www"),
                String::from("imateapot"),
                String::from("org")
            ])
        );
        assert_eq!(question.qtype(), Some(1));
        assert_eq!(question.qclass(), Some(1));

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

        assert_eq!(answer.id(), answer.id());
        assert_eq!(answer.qr(), true);
        assert_eq!(answer.opcode(), OpCode::QUERY);
        assert_eq!(answer.aa(), false);
        assert_eq!(answer.tc(), false);
        assert_eq!(answer.rd(), true);
        assert_eq!(answer.ra(), false);
        assert_eq!(answer.z(), false);
        assert_eq!(answer.ad(), false);
        assert_eq!(answer.cd(), false);
        assert_eq!(answer.rcode(), RCode::NoError);
        assert_eq!(answer.qdcount(), Some(1));
        assert_eq!(answer.ancount(), Some(1));
        assert_eq!(answer.nscount(), Some(0));
        assert_eq!(answer.arcount(), Some(1));
        assert_eq!(
            question.qname(),
            Some(vec![
                String::from("www"),
                String::from("imateapot"),
                String::from("org")
            ])
        );
        assert_eq!(question.qtype(), Some(1));
        assert_eq!(question.qclass(), Some(1));

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

        message.set_response_ttl(42);
        let rrs1 = message.resource_records().expect("couldn't parse RRs");
        assert_eq!(rrs1.0[0].ttl, 42);
    }
}

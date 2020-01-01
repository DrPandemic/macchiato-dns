use crate::resource_record::*;

// TODO: Move rather than reference
pub struct Message<'a> {
    buffer: &'a Vec<u8>,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum OpCode {
    QUERY,
    IQUERY,
    STATUS,
    Other,
}

#[derive(Debug)]
#[derive(PartialEq)]
pub enum RCode {
    NoError,
    FormatError,
    ServerFailure,
    NameError,
    NotImplemented,
    Refused,
    Other,
}

impl<'a> Message<'a> {
    pub fn id(&self) -> u16 {
        parse_u16(&self.buffer, 0)
    }

    pub fn qr(&self) -> bool {
        (self.buffer[2] >> 7) == 0b1u8
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

    pub fn qdcount(&self) -> u16 {
        parse_u16(&self.buffer, 4)
    }

    pub fn ancount(&self) -> u16 {
        parse_u16(&self.buffer, 6)
    }

    pub fn nscount(&self) -> u16 {
        parse_u16(&self.buffer, 8)
    }

    pub fn arcount(&self) -> u16 {
        parse_u16(&self.buffer, 10)
    }

    fn questions(&self) -> Option<Vec<Question>> {
        (0..self.qdcount()).fold(Some((vec![], 12)), |maybe_acc, _| {
            match maybe_acc {
                Some((mut acc, offset)) => {
                    let name_end = self.buffer[offset..].iter().position(|&c| c == 0b0)?;
                    acc.push(Question{buffer: &self.buffer[..(offset + name_end + 4 + 1)], offset: offset});
                    Some((acc, offset + name_end + 4 + 1))
                },
                _ => None
            }
        }).map(|x| x.0)
    }

    // https://stackoverflow.com/a/4083071 multiple questions is not really supported
    pub fn question(&self) -> Option<Question> {
        Some(self.questions()?[0])
    }

    fn parse_rr(&self, offset: usize) -> ResourceRecord {
        let (name, consumed_bytes) = parse_name(&self.buffer, offset);
        let post_name: usize = consumed_bytes + offset;
        let rdlength = parse_u16(&self.buffer, post_name + 8);
        // Class 41 shouldn't be parse as a RR. Maybe create a new struct for Opt?
        ResourceRecord {
            buffer: &self.buffer[offset..post_name + 10 + rdlength as usize],
            name: name,
            rdlength: rdlength,
            type_code: parse_u16(&self.buffer, post_name),
            class: parse_u16(&self.buffer, post_name + 2),
            ttl: parse_u32(&self.buffer, post_name + 4),
            rdata: &self.buffer[post_name..post_name + rdlength as usize],
            size: consumed_bytes + rdlength as usize + 10,
        }
    }

    pub fn resource_records(&self) -> Option<(Vec<ResourceRecord>, Vec<ResourceRecord>, Vec<ResourceRecord>)> {
        let question_offset = 12 + self.questions()?.iter().map(|q| q.len()).sum::<usize>();
        let answers = (0..self.ancount()).fold((vec![], question_offset), |(mut acc, offset), _| {
            // TODO: one byte too far
            let rr = self.parse_rr(offset);
            let size = rr.size;
            acc.push(rr);
            (acc, offset + size)
        });
        let authorities = (0..self.nscount()).fold((vec![], answers.1), |(mut acc, offset), _| {
            let rr = self.parse_rr(offset);
            let size = rr.size;
            acc.push(rr);
            (acc, offset + size)
        });
        let additionals = (0..self.arcount()).fold((vec![], authorities.1), |(mut acc, offset), _| {
            let rr = self.parse_rr(offset);
            let size = rr.size;
            acc.push(rr);
            (acc, offset + size)
        });

        Some((answers.0, authorities.0, additionals.0))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Question<'a> {
    buffer: &'a[u8],
    offset: usize,
}

impl<'a> Question<'a> {
    // https://stackoverflow.com/a/47258845
    pub fn qname(&self) -> Vec<String> {
        parse_name(&self.buffer, self.offset).0
    }

    pub fn qtype(&self) -> u16 {
        parse_u16(&self.buffer[self.offset..], self.len() - 4)
    }

    pub fn qclass(&self) -> u16 {
        parse_u16(&self.buffer[self.offset..], self.len() - 2)
    }

    pub fn len(&self) -> usize {
        parse_name(&self.buffer, self.offset).1 + 4
    }
}

fn parse_u16(buffer: &[u8], position: usize) -> u16 {
    (u16::from(buffer[position]) << 8) | u16::from(buffer[position + 1])
}

fn parse_u32(buffer: &[u8], position: usize) -> u32 {
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
            let name: String = buffer[i + 1..i + 1 + size as usize].iter().cloned().map(char::from).collect();
            strings.push(name);
            i += (1 + size) as usize;
        }
    }
    (strings, i - offset)
}

pub fn parse_message<'a>(query: &'a Vec<u8>) -> Message {
    Message { buffer: &query }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IMATEAPOT_QUESTION: [u8; 46] = [57, 32, 1, 32, 0, 1, 0, 0, 0, 0, 0, 1, 3, 119, 119, 119, 9, 105, 109, 97, 116,
                                          101, 97, 112, 111, 116, 3, 111, 114, 103, 0, 0, 1, 0, 1, 0, 0, 41, 16, 0, 0, 0,
                                          0, 0, 0, 0];
    const IMATEAPOT_ANSWER: [u8; 95] = [57, 32, 129, 128, 0, 1, 0, 2, 0, 0, 0, 1, 3, 119, 119, 119, 9, 105, 109, 97, 116,
                                        101, 97, 112, 111, 116, 3, 111, 114, 103, 0, 0, 1, 0, 1, 192, 12, 0, 5, 0, 1, 0,
                                        0, 84, 64, 0, 21, 5, 115, 104, 111, 112, 115, 9, 109, 121, 115, 104, 111, 112,
                                        105, 102, 121, 3, 99, 111, 109, 0, 192, 47, 0, 1, 0, 1, 0, 0, 5, 23, 0, 4, 23,
                                        227, 38, 64, 0, 0, 41, 2, 0, 0, 0, 0, 0, 0, 0];
    #[test]
    fn test_question_attributes() {
        let buffer = IMATEAPOT_QUESTION.to_vec();
        let message = parse_message(&buffer);
        let question = message.question().expect("couldn't parse questions");
        let rrs = message.resource_records().expect("couldn't parse RRs");

        assert_eq!(message.id(), 14624);
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
        assert_eq!(message.qdcount(), 1);
        assert_eq!(message.ancount(), 0);
        assert_eq!(message.nscount(), 0);
        assert_eq!(message.arcount(), 1);
        assert_eq!(question.qname(), ["www", "imateapot", "org"]);
        assert_eq!(question.qtype(), 1);
        assert_eq!(question.qclass(), 1);
        assert_eq!(rrs.2[0].name, Vec::<String>::new());
        assert_eq!(rrs.2[0].get_type(), ResourceRecordType::Opt);
        assert_eq!(rrs.2[0].class, 4096);
    }

    #[test]
    fn test_answer_attributes() {
        let buffer = IMATEAPOT_ANSWER.to_vec();
        let message = parse_message(&buffer);
        let question = message.question().expect("couldn't parse questions");
        let rrs = message.resource_records().expect("couldn't parse RRs");

        assert_eq!(message.id(), 14624);
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
        assert_eq!(message.qdcount(), 1);
        assert_eq!(message.ancount(), 2);
        assert_eq!(message.nscount(), 0);
        assert_eq!(message.arcount(), 1);
        assert_eq!(question.qname(), ["www", "imateapot", "org"]);
        assert_eq!(question.qtype(), 1);
        assert_eq!(question.qclass(), 1);

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
}

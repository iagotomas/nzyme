use std::{sync::Arc, net::{Ipv4Addr, Ipv6Addr}};
use crate::ethernet::{packets::{UDPPacket, DNSPacket, DNSData}, types::{DNSType, DNSDataType, DNSClass}};
use anyhow::{Result, bail};
use bitvec::{view::BitView, order::Msb0};
use byteorder::{BigEndian, ByteOrder};
use bitreader::BitReader;
use chrono::Utc;
use entropy::shannon_entropy;

#[allow(clippy::cast_possible_truncation)]
pub fn parse(udp: &Arc<UDPPacket>) -> Result<DNSPacket> {

    if udp.payload.len() < 13 {
        bail!("Payload too short to hold a DNS request or response.");
    }

    // We are currently only looking at first bit but may parse all flags in the future.
    let mut flags = BitReader::new(&udp.payload[2..4]);
    let dns_type = match flags.read_bool() {
        Ok(b) => {
            if b {
                DNSType::QueryResponse
            } else {
                DNSType::Query
            }
        },
        Err(e) => bail!("Unexpected type flag: {}", e)
    };

    let question_count = BigEndian::read_u16(&udp.payload[4..6]);
    let answer_count = BigEndian::read_u16(&udp.payload[6..8]);

    // Queries.
    let mut cursor: usize = 12;
    let queries = if question_count > 0 {
        let mut query_list = Vec::new();
        for _ in 0..question_count {
            if udp.payload.len() < cursor+5 {
                bail!("Payload too short to fit next question.");
            }

            match parse_query_element(&udp.payload[cursor..udp.payload.len()]) {
                Ok((pos, data)) => {
                    cursor += pos;
                    query_list.push(data);
                },
                Err(e) => bail!("Could not parse DNS query data element: {}", e)
            };
        }
        
        Option::Some(query_list)
    } else {
        Option::None
    };

    // Responses.
    let responses = if answer_count > 0 {
        let mut response_list = Vec::new();
        for _ in 0..answer_count {
            if udp.payload.len() < cursor+5 {
                bail!("Payload too short to fit next answer.");
            }

            match parse_answer_element(&udp.payload[cursor..udp.payload.len()], &udp.payload) {
                Ok((pos, data)) => {
                    cursor += pos;
                    response_list.push(data);
                },
                Err(e) => bail!("Could not parse DNS response data element: {}", e)
            };
        }
        
        Option::Some(response_list)
    } else {
        Option::None
    };

    let size = udp.payload.len() as u32;

    Ok(DNSPacket {
        source_mac: udp.source_mac.clone(),
        destination_mac: udp.destination_mac.clone(),
        source_address: udp.source_address.clone(),
        destination_address: udp.destination_address.clone(),
        source_port: udp.source_port,
        destination_port: udp.destination_port,
        dns_type,
        question_count,
        answer_count,
        queries,
        responses,
        size,
        timestamp: Utc::now()
    })

}

fn is_pointer(mask: u8) -> bool {
    let mask = mask.view_bits::<Msb0>();
    *mask.get(0).unwrap() && *mask.get(1).unwrap()
}

fn parse_label(data: &[u8], full_packet: &[u8]) -> Result<(usize, String)> {
    if is_pointer(data[0]) {
        // Value is a pointer.
        if data.len() < 3 {
            bail!("Label pointer too short.");
        }

        let offset = BigEndian::read_u16(&[data[0] & 0b0011_1111, data[1]]);
        return match parse_string(&full_packet[(offset as usize)..full_packet.len()], full_packet) {
            Ok((_, s)) => Ok((2, s)), // 2 is the pointer length
            Err(e) => bail!("Could not parse answer name: {}", e)
        };
    }

    parse_string(data, full_packet)
}

fn parse_string(data: &[u8], full_packet: &[u8]) -> Result<(usize, String)> {
    let mut cursor: usize = 0;
    let mut chars = Vec::new();
    let mut section_length = 0;
    let mut initial = true;
    for b in data {
        cursor += 1;

        if *b == 0 {
            // Reached the end of the complete label.
            break;
        }
    
        if is_pointer(*b) {
            let offset = BigEndian::read_u16(&[data[cursor-1] & 0b0011_1111, data[cursor]]);

            if full_packet.len() < offset as usize {
                bail!("Offset <{}> does not match provided full packet length <{}>.", offset as usize, full_packet.len());
            }

            let (_,res) = match parse_string(&full_packet[(offset as usize)..full_packet.len()], full_packet) {
                Ok((c,s)) => (c,s),
                Err(e) => bail!("Recursive pointer parsing failed: {}", e)
            };

            let prev = String::from_utf8_lossy(&chars).to_string();
            let full = prev + "." + res.as_str();
            cursor += 2; // The pointer size.

            return Ok((cursor, full));
        }

        if section_length == 0 {
            if initial {
                initial = false;
            } else {
                // A new label section begins.
                chars.push(46);
            }

            section_length = *b;
            continue;
        }

        chars.push(*b);
        section_length -= 1;
    }

    Ok((cursor, String::from_utf8_lossy(&chars).to_string()))
}

fn parse_query_element(data: &[u8]) -> Result<(usize, DNSData)> {
    let mut cursor;
    let name;
    match parse_string(&data[0..data.len()], data) {
        Ok((c,s)) => {
            cursor = c;
            name = s;
        },
        Err(e) => bail!("Could not parse query name: {}", e)
    };

    if data.len() < cursor+4 {
        bail!("DNS data element too short.");
    }

    let dns_type_num = BigEndian::read_u16(&data[cursor..cursor+2]);
    let Ok(dns_type) = DNSDataType::try_from(dns_type_num) else {
        bail!("Unknown DNS type <{}>.", dns_type_num)
    };
    cursor += 2;
   
    let class_num = BigEndian::read_u16(&data[cursor..cursor+2]);
    let Ok(class) = DNSClass::try_from(class_num) else {
        bail!("Unknown DNS class <{}>.", class_num)
    };
    cursor += 2;

    let entropy = shannon_entropy(name.as_bytes());

    Ok((cursor, DNSData {
       name,
       dns_type,
       class,
       entropy: Option::Some(entropy),
       value: Option::None,
       ttl: Option::None
    }))
}

#[allow(clippy::too_many_lines)]
fn parse_answer_element(data: &[u8], full_packet: &[u8]) -> Result<(usize, DNSData)> {
    let (mut cursor, name) = match parse_label(data, full_packet) {
        Ok(x) => x,
        Err(e) => bail!("Could not parse answer name: {}", e)
    };

    if data.len() < cursor+11 {
       bail!("DNS answer element too short"); 
    }

    let dns_type_num = BigEndian::read_u16(&data[cursor..cursor+2]);
    let Ok(dns_type) = DNSDataType::try_from(dns_type_num) else {
        bail!("Unknown DNS type <{}>.", dns_type_num)
    };
    cursor += 2;
   
    let class_num = BigEndian::read_u16(&data[cursor..cursor+2]);
    let Ok(class) = DNSClass::try_from(class_num) else {
        bail!("Unknown DNS class <{}>.", class_num)
    };
    cursor += 2;

    let ttl = BigEndian::read_u32(&data[cursor..cursor+4]);
    cursor += 4;

    let data_length = BigEndian::read_u16(&data[cursor..cursor+2]) as usize;
    cursor += 2;
   
    if data.len() < cursor+data_length {
        bail!("DNS answer element too short to fit announced data length.");
    }

    // Not collapsing branches here because logic might become more specific in the future.
    let dns_type_has_entropy;
    let value = match dns_type {
        DNSDataType::A => {
            dns_type_has_entropy = false;
            match parse_ipv4(&data[cursor..cursor+data_length]) {
                Ok(addr) => {
                    Option::Some(addr)
                },
                Err(e) => bail!("Could not parse A value: {}", e) 
            }
        },
        DNSDataType::CNAME => {
            dns_type_has_entropy = true;
            match parse_string(&data[cursor..cursor+data_length], full_packet) {
                Ok((_, cname)) => {
                    Option::Some(cname) 
                },
                Err(e) => bail!("Could not parse CNAME value: {}", e)
            }
        },
        DNSDataType::AAAA => {
            dns_type_has_entropy = false;
            match parse_ipv6(&data[cursor..cursor+data_length]) {
                Ok(addr) => {
                    Option::Some(addr)
                },
                Err(e) => bail!("Could not parse AAAA value: {}", e) 
            }
        },
        DNSDataType::NS => {
            dns_type_has_entropy = true;
            match parse_string(&data[cursor..cursor+data_length], full_packet) {
                Ok((_, ns)) => {
                    Option::Some(ns) 
                },
                Err(e) => bail!("Could not parse NS value: {}", e)
            }
        },
        DNSDataType::PTR => {
            dns_type_has_entropy = true;
            match parse_string(&data[cursor..cursor+data_length], full_packet) {
                Ok((_, ptr)) => {
                    Option::Some(ptr) 
                },
                Err(e) => bail!("Could not parse PTR value: {}", e)
            }
        },
        DNSDataType::MX => {
            dns_type_has_entropy = true;
            if data_length < 3 {
                bail!("MX data is too short.");
            }

            // We are skipping two bytes because we don't parse the MX priority.
            match parse_string(&data[cursor+2..cursor+data_length], full_packet) {
                Ok((_, mx)) => {
                    Option::Some(mx) 
                },
                Err(e) => bail!("Could not parse MX value: {}", e)
            }
        },
        DNSDataType::TXT => {
            dns_type_has_entropy = true;
            match parse_string(&data[cursor..cursor+data_length], full_packet) {
                Ok((_, txt)) => {
                    Option::Some(txt) 
                },
                Err(e) => bail!("Could not parse TXT value: {}", e)
            }
        },
        _ => {
            dns_type_has_entropy = false;
            Option::None
        }
    };

    let entropy = match value.clone() {
        Some(v) => {
            if dns_type_has_entropy {
                Option::Some(shannon_entropy(v.as_bytes()))
            } else {
                Option::None
            }
        },
        None => Option::None
    };

    cursor += data_length;
    
    Ok((cursor, DNSData {
        name,
        class,
        dns_type,
        value,
        entropy,
        ttl: Option::Some(ttl)
    }))
}

fn parse_ipv4(data: &[u8]) -> Result<String> {
    if data.len() != 4 {
        bail!("Invalid IPv4 address length.");
    }

    Ok(Ipv4Addr::new(data[0], data[1], data[2], data[3]).to_string())
}

fn parse_ipv6(data: &[u8]) -> Result<String> {
    if data.len() != 16 {
        bail!("Invalid IPv4 address length.");
    }
    
    Ok(Ipv6Addr::new(
            BigEndian::read_u16(&[data[0], data[1]]),
            BigEndian::read_u16(&[data[2], data[3]]),
            BigEndian::read_u16(&[data[4], data[5]]),
            BigEndian::read_u16(&[data[6], data[7]]),
            BigEndian::read_u16(&[data[8], data[9]]),
            BigEndian::read_u16(&[data[10], data[11]]),
            BigEndian::read_u16(&[data[12], data[13]]),
            BigEndian::read_u16(&[data[14], data[15]])
    ).to_string())
}

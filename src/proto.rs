/// Lightweight dynamic Protobuf wire decoder and query helper

#[derive(Debug, Clone, PartialEq)]
pub enum ProtoValue {
    Varint(u64),
    LengthDelimited(Vec<u8>),
    Fixed64([u8; 8]),
    Fixed32([u8; 4]),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtoField {
    pub tag: u32,
    pub wire_type: u32,
    pub value: ProtoValue,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProtoMessage {
    pub fields: Vec<ProtoField>,
}

impl ProtoMessage {
    pub fn parse(data: &[u8]) -> Self {
        let mut fields = Vec::new();
        let mut i = 0;

        while i < data.len() {
            // Read tag and wire type
            let (key, bytes_read) = match read_varint(&data[i..]) {
                Some(res) => res,
                None => break,
            };
            i += bytes_read;

            let tag = (key >> 3) as u32;
            let wire_type = (key & 0x07) as u32;

            match wire_type {
                0 => {
                    // Varint
                    let (val, varint_len) = match read_varint(&data[i..]) {
                        Some(res) => res,
                        None => break,
                    };
                    i += varint_len;
                    fields.push(ProtoField {
                        tag,
                        wire_type,
                        value: ProtoValue::Varint(val),
                    });
                }
                1 => {
                    // 64-bit
                    if i + 8 > data.len() {
                        break;
                    }
                    let mut arr = [0u8; 8];
                    arr.copy_from_slice(&data[i..i + 8]);
                    i += 8;
                    fields.push(ProtoField {
                        tag,
                        wire_type,
                        value: ProtoValue::Fixed64(arr),
                    });
                }
                2 => {
                    // Length-delimited
                    let (len, len_bytes) = match read_varint(&data[i..]) {
                        Some(res) => res,
                        None => break,
                    };
                    i += len_bytes;
                    let len = len as usize;
                    if i + len > data.len() {
                        break;
                    }
                    let bytes = data[i..i + len].to_vec();
                    i += len;
                    fields.push(ProtoField {
                        tag,
                        wire_type,
                        value: ProtoValue::LengthDelimited(bytes),
                    });
                }
                5 => {
                    // 32-bit
                    if i + 4 > data.len() {
                        break;
                    }
                    let mut arr = [0u8; 4];
                    arr.copy_from_slice(&data[i..i + 4]);
                    i += 4;
                    fields.push(ProtoField {
                        tag,
                        wire_type,
                        value: ProtoValue::Fixed32(arr),
                    });
                }
                _ => {
                    // Unknown/unsupported wire type
                    break;
                }
            }
        }

        ProtoMessage { fields }
    }

    /// Get all fields matching a specific tag
    pub fn get_fields(&self, tag: u32) -> Vec<&ProtoField> {
        self.fields.iter().filter(|f| f.tag == tag).collect()
    }

    /// Get the first field matching a specific tag
    pub fn get_field(&self, tag: u32) -> Option<&ProtoField> {
        self.fields.iter().find(|f| f.tag == tag)
    }

    /// Get value as u64 varint
    pub fn get_varint(&self, tag: u32) -> Option<u64> {
        match self.get_field(tag)?.value {
            ProtoValue::Varint(v) => Some(v),
            _ => None,
        }
    }

    /// Get value as UTF-8 string
    pub fn get_string(&self, tag: u32) -> Option<String> {
        match &self.get_field(tag)?.value {
            ProtoValue::LengthDelimited(bytes) => String::from_utf8(bytes.clone()).ok(),
            _ => None,
        }
    }

    /// Get value as nested ProtoMessage
    pub fn get_message(&self, tag: u32) -> Option<ProtoMessage> {
        match &self.get_field(tag)?.value {
            ProtoValue::LengthDelimited(bytes) => Some(ProtoMessage::parse(bytes)),
            _ => None,
        }
    }

    /// Get all nested ProtoMessages for repeated fields
    pub fn get_messages(&self, tag: u32) -> Vec<ProtoMessage> {
        self.get_fields(tag)
            .into_iter()
            .filter_map(|f| match &f.value {
                ProtoValue::LengthDelimited(bytes) => Some(ProtoMessage::parse(bytes)),
                _ => None,
            })
            .collect()
    }

    /// Get raw bytes
    #[allow(dead_code)]
    pub fn get_bytes(&self, tag: u32) -> Option<&[u8]> {
        match &self.get_field(tag)?.value {
            ProtoValue::LengthDelimited(bytes) => Some(bytes),
            _ => None,
        }
    }
}

fn read_varint(data: &[u8]) -> Option<(u64, usize)> {
    let mut val: u64 = 0;
    let mut shift = 0;
    let mut read = 0;

    for &b in data {
        read += 1;
        val |= ((b & 0x7F) as u64) << shift;
        shift += 7;
        if (b & 0x80) == 0 {
            return Some((val, read));
        }
        if shift >= 64 {
            return None;
        }
    }

    None
}

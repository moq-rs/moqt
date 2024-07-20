use crate::message::message_parser::ErrorCode;
use crate::message::FilterType;
use crate::serde::parameters::ParameterKey;
use crate::{Deserializer, Error, Parameters, Result, Serializer};
use bytes::{Buf, BufMut};

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub struct Subscribe {
    pub subscribe_id: u64,

    pub track_alias: u64,
    pub track_namespace: String,
    pub track_name: String,

    pub filter_type: FilterType,

    pub authorization_info: Option<String>,
}

impl Deserializer for Subscribe {
    fn deserialize<R: Buf>(r: &mut R) -> Result<(Self, usize)> {
        let (subscribe_id, sil) = u64::deserialize(r)?;

        let (track_alias, tal) = u64::deserialize(r)?;
        let (track_namespace, tnsl) = String::deserialize(r)?;
        let (track_name, tnl) = String::deserialize(r)?;

        let (filter_type, ftl) = FilterType::deserialize(r)?;

        let mut authorization_info: Option<String> = None;
        let (num_params, mut pl) = u64::deserialize(r)?;
        // Parse parameters
        for _ in 0..num_params {
            let (key, kl) = u64::deserialize(r)?;
            pl += kl;
            let (size, sl) = usize::deserialize(r)?;
            pl += sl;

            if r.remaining() < size {
                return Err(Error::ErrBufferTooShort);
            }

            if key == ParameterKey::AuthorizationInfo as u64 {
                if authorization_info.is_some() {
                    return Err(Error::ErrParseError(
                        ErrorCode::ProtocolViolation,
                        "AUTHORIZATION_INFO parameter appears twice in SUBSCRIBE".to_string(),
                    ));
                }
                let mut buf = vec![0; size];
                r.copy_to_slice(&mut buf);
                pl += size;

                authorization_info = Some(String::from_utf8(buf)?);
            }
        }

        Ok((
            Self {
                subscribe_id,

                track_alias,
                track_namespace,
                track_name,

                filter_type,

                authorization_info,
            },
            sil + tal + tnsl + tnl + ftl + pl,
        ))
    }
}

impl Serializer for Subscribe {
    fn serialize<W: BufMut>(&self, w: &mut W) -> Result<usize> {
        let mut l = self.subscribe_id.serialize(w)?;

        l += self.track_alias.serialize(w)?;
        l += self.track_namespace.serialize(w)?;
        l += self.track_name.serialize(w)?;

        l += self.filter_type.serialize(w)?;

        if let Some(authorization_info) = self.authorization_info.as_ref() {
            let mut parameters = Parameters::new();
            parameters.insert(
                ParameterKey::AuthorizationInfo,
                authorization_info.to_string(),
            )?;
            l += parameters.serialize(w)?;
        }

        Ok(l)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::message::{ControlMessage, FullSequence};
    use std::io::Cursor;

    #[test]
    fn test_subscribe() -> Result<()> {
        let expected_packet: Vec<u8> = vec![
            0x03, 0x01, 0x02, // id and alias
            0x03, 0x66, 0x6f, 0x6f, // track_namespace = "foo"
            0x04, 0x61, 0x62, 0x63, 0x64, // track_name = "abcd"
            0x03, // Filter type: Absolute Start
            0x04, // start_group = 4 (relative previous)
            0x01, // start_object = 1 (absolute)
            // No EndGroup or EndObject
            0x01, // 1 parameter
            0x02, 0x03, 0x62, 0x61, 0x72, // authorization_info = "bar"
        ];

        let expected_message = ControlMessage::Subscribe(Subscribe {
            subscribe_id: 1,
            track_alias: 2,
            track_namespace: "foo".to_string(),
            track_name: "abcd".to_string(),
            filter_type: FilterType::AbsoluteStart(FullSequence {
                group_id: 4,
                object_id: 1,
            }),
            authorization_info: Some("bar".to_string()),
        });

        let mut cursor: Cursor<&[u8]> = Cursor::new(expected_packet.as_ref());
        let (actual_message, actual_len) = ControlMessage::deserialize(&mut cursor)?;
        assert_eq!(expected_message, actual_message);
        assert_eq!(expected_packet.len(), actual_len);

        let mut actual_packet = vec![];
        let _ = expected_message.serialize(&mut actual_packet)?;
        assert_eq!(expected_packet, actual_packet);

        Ok(())
    }
}

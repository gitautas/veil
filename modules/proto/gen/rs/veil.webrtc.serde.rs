// @generated
impl serde::Serialize for IceCandidate {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.candidate.is_empty() {
            len += 1;
        }
        if !self.sdp_mid.is_empty() {
            len += 1;
        }
        if self.sdp_mline_index != 0 {
            len += 1;
        }
        if !self.username_fragment.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.webrtc.IceCandidate", len)?;
        if !self.candidate.is_empty() {
            struct_ser.serialize_field("candidate", &self.candidate)?;
        }
        if !self.sdp_mid.is_empty() {
            struct_ser.serialize_field("sdpMid", &self.sdp_mid)?;
        }
        if self.sdp_mline_index != 0 {
            struct_ser.serialize_field("sdpMlineIndex", &self.sdp_mline_index)?;
        }
        if !self.username_fragment.is_empty() {
            struct_ser.serialize_field("usernameFragment", &self.username_fragment)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for IceCandidate {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "candidate",
            "sdp_mid",
            "sdpMid",
            "sdp_mline_index",
            "sdpMlineIndex",
            "username_fragment",
            "usernameFragment",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Candidate,
            SdpMid,
            SdpMlineIndex,
            UsernameFragment,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "candidate" => Ok(GeneratedField::Candidate),
                            "sdpMid" | "sdp_mid" => Ok(GeneratedField::SdpMid),
                            "sdpMlineIndex" | "sdp_mline_index" => Ok(GeneratedField::SdpMlineIndex),
                            "usernameFragment" | "username_fragment" => Ok(GeneratedField::UsernameFragment),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = IceCandidate;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.webrtc.IceCandidate")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<IceCandidate, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut candidate__ = None;
                let mut sdp_mid__ = None;
                let mut sdp_mline_index__ = None;
                let mut username_fragment__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Candidate => {
                            if candidate__.is_some() {
                                return Err(serde::de::Error::duplicate_field("candidate"));
                            }
                            candidate__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SdpMid => {
                            if sdp_mid__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sdpMid"));
                            }
                            sdp_mid__ = Some(map_.next_value()?);
                        }
                        GeneratedField::SdpMlineIndex => {
                            if sdp_mline_index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sdpMlineIndex"));
                            }
                            sdp_mline_index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::UsernameFragment => {
                            if username_fragment__.is_some() {
                                return Err(serde::de::Error::duplicate_field("usernameFragment"));
                            }
                            username_fragment__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(IceCandidate {
                    candidate: candidate__.unwrap_or_default(),
                    sdp_mid: sdp_mid__.unwrap_or_default(),
                    sdp_mline_index: sdp_mline_index__.unwrap_or_default(),
                    username_fragment: username_fragment__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("veil.webrtc.IceCandidate", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SessionDescription {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.r#type != 0 {
            len += 1;
        }
        if !self.sdp.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.webrtc.SessionDescription", len)?;
        if self.r#type != 0 {
            let v = session_description::Type::try_from(self.r#type)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.r#type)))?;
            struct_ser.serialize_field("type", &v)?;
        }
        if !self.sdp.is_empty() {
            struct_ser.serialize_field("sdp", &self.sdp)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SessionDescription {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "type",
            "sdp",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Type,
            Sdp,
        }
        impl<'de> serde::Deserialize<'de> for GeneratedField {
            fn deserialize<D>(deserializer: D) -> std::result::Result<GeneratedField, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct GeneratedVisitor;

                impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
                    type Value = GeneratedField;

                    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        write!(formatter, "expected one of: {:?}", &FIELDS)
                    }

                    #[allow(unused_variables)]
                    fn visit_str<E>(self, value: &str) -> std::result::Result<GeneratedField, E>
                    where
                        E: serde::de::Error,
                    {
                        match value {
                            "type" => Ok(GeneratedField::Type),
                            "sdp" => Ok(GeneratedField::Sdp),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SessionDescription;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.webrtc.SessionDescription")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SessionDescription, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut r#type__ = None;
                let mut sdp__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Type => {
                            if r#type__.is_some() {
                                return Err(serde::de::Error::duplicate_field("type"));
                            }
                            r#type__ = Some(map_.next_value::<session_description::Type>()? as i32);
                        }
                        GeneratedField::Sdp => {
                            if sdp__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sdp"));
                            }
                            sdp__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(SessionDescription {
                    r#type: r#type__.unwrap_or_default(),
                    sdp: sdp__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("veil.webrtc.SessionDescription", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for session_description::Type {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "UNSPECIFIED",
            Self::Offer => "OFFER",
            Self::Answer => "ANSWER",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for session_description::Type {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "UNSPECIFIED",
            "OFFER",
            "ANSWER",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = session_description::Type;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(formatter, "expected one of: {:?}", &FIELDS)
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Signed(v), &self)
                    })
            }

            fn visit_u64<E>(self, v: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                i32::try_from(v)
                    .ok()
                    .and_then(|x| x.try_into().ok())
                    .ok_or_else(|| {
                        serde::de::Error::invalid_value(serde::de::Unexpected::Unsigned(v), &self)
                    })
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                match value {
                    "UNSPECIFIED" => Ok(session_description::Type::Unspecified),
                    "OFFER" => Ok(session_description::Type::Offer),
                    "ANSWER" => Ok(session_description::Type::Answer),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}

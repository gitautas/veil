// @generated
impl serde::Serialize for StartSessionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.command.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.compositor.StartSessionRequest", len)?;
        if !self.command.is_empty() {
            struct_ser.serialize_field("command", &self.command)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StartSessionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "command",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Command,
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
                            "command" => Ok(GeneratedField::Command),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StartSessionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.compositor.StartSessionRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StartSessionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut command__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Command => {
                            if command__.is_some() {
                                return Err(serde::de::Error::duplicate_field("command"));
                            }
                            command__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(StartSessionRequest {
                    command: command__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("veil.compositor.StartSessionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for StartSessionResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.session_id.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.compositor.StartSessionResponse", len)?;
        if !self.session_id.is_empty() {
            struct_ser.serialize_field("sessionId", &self.session_id)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for StartSessionResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "session_id",
            "sessionId",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            SessionId,
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
                            "sessionId" | "session_id" => Ok(GeneratedField::SessionId),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = StartSessionResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.compositor.StartSessionResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<StartSessionResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut session_id__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::SessionId => {
                            if session_id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("sessionId"));
                            }
                            session_id__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(StartSessionResponse {
                    session_id: session_id__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("veil.compositor.StartSessionResponse", FIELDS, GeneratedVisitor)
    }
}

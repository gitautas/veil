// @generated
impl serde::Serialize for AudioStream {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.encoding != 0 {
            len += 1;
        }
        if !self.channels.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.AudioStream", len)?;
        if self.encoding != 0 {
            let v = audio_stream::Codec::try_from(self.encoding)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.encoding)))?;
            struct_ser.serialize_field("encoding", &v)?;
        }
        if !self.channels.is_empty() {
            let v = self.channels.iter().cloned().map(|v| {
                audio_stream::Channel::try_from(v)
                    .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", v)))
                }).collect::<std::result::Result<Vec<_>, _>>()?;
            struct_ser.serialize_field("channels", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for AudioStream {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "encoding",
            "channels",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Encoding,
            Channels,
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
                            "encoding" => Ok(GeneratedField::Encoding),
                            "channels" => Ok(GeneratedField::Channels),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = AudioStream;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.AudioStream")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<AudioStream, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut encoding__ = None;
                let mut channels__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Encoding => {
                            if encoding__.is_some() {
                                return Err(serde::de::Error::duplicate_field("encoding"));
                            }
                            encoding__ = Some(map_.next_value::<audio_stream::Codec>()? as i32);
                        }
                        GeneratedField::Channels => {
                            if channels__.is_some() {
                                return Err(serde::de::Error::duplicate_field("channels"));
                            }
                            channels__ = Some(map_.next_value::<Vec<audio_stream::Channel>>()?.into_iter().map(|x| x as i32).collect());
                        }
                    }
                }
                Ok(AudioStream {
                    encoding: encoding__.unwrap_or_default(),
                    channels: channels__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("veil.AudioStream", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for audio_stream::Channel {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Mono => "MONO",
            Self::FrontLeft => "FRONT_LEFT",
            Self::FrontRight => "FRONT_RIGHT",
            Self::FrontCenter => "FRONT_CENTER",
            Self::RearCenter => "REAR_CENTER",
            Self::RearLeft => "REAR_LEFT",
            Self::RearRight => "REAR_RIGHT",
            Self::Lfe => "LFE",
            Self::FrontLeftOfCenter => "FRONT_LEFT_OF_CENTER",
            Self::FrontRightOfCenter => "FRONT_RIGHT_OF_CENTER",
            Self::SideLeft => "SIDE_LEFT",
            Self::SideRight => "SIDE_RIGHT",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for audio_stream::Channel {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "MONO",
            "FRONT_LEFT",
            "FRONT_RIGHT",
            "FRONT_CENTER",
            "REAR_CENTER",
            "REAR_LEFT",
            "REAR_RIGHT",
            "LFE",
            "FRONT_LEFT_OF_CENTER",
            "FRONT_RIGHT_OF_CENTER",
            "SIDE_LEFT",
            "SIDE_RIGHT",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = audio_stream::Channel;

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
                    "MONO" => Ok(audio_stream::Channel::Mono),
                    "FRONT_LEFT" => Ok(audio_stream::Channel::FrontLeft),
                    "FRONT_RIGHT" => Ok(audio_stream::Channel::FrontRight),
                    "FRONT_CENTER" => Ok(audio_stream::Channel::FrontCenter),
                    "REAR_CENTER" => Ok(audio_stream::Channel::RearCenter),
                    "REAR_LEFT" => Ok(audio_stream::Channel::RearLeft),
                    "REAR_RIGHT" => Ok(audio_stream::Channel::RearRight),
                    "LFE" => Ok(audio_stream::Channel::Lfe),
                    "FRONT_LEFT_OF_CENTER" => Ok(audio_stream::Channel::FrontLeftOfCenter),
                    "FRONT_RIGHT_OF_CENTER" => Ok(audio_stream::Channel::FrontRightOfCenter),
                    "SIDE_LEFT" => Ok(audio_stream::Channel::SideLeft),
                    "SIDE_RIGHT" => Ok(audio_stream::Channel::SideRight),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for audio_stream::Codec {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Opus => "OPUS",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for audio_stream::Codec {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "OPUS",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = audio_stream::Codec;

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
                    "OPUS" => Ok(audio_stream::Codec::Opus),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for CreateSessionRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("veil.CreateSessionRequest", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CreateSessionRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CreateSessionRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.CreateSessionRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CreateSessionRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(CreateSessionRequest {
                })
            }
        }
        deserializer.deserialize_struct("veil.CreateSessionRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for CreateSessionResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.offer.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.CreateSessionResponse", len)?;
        if let Some(v) = self.offer.as_ref() {
            struct_ser.serialize_field("offer", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for CreateSessionResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "offer",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Offer,
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
                            "offer" => Ok(GeneratedField::Offer),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = CreateSessionResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.CreateSessionResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<CreateSessionResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut offer__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Offer => {
                            if offer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("offer"));
                            }
                            offer__ = map_.next_value()?;
                        }
                    }
                }
                Ok(CreateSessionResponse {
                    offer: offer__,
                })
            }
        }
        deserializer.deserialize_struct("veil.CreateSessionResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Display {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.resolution.is_some() {
            len += 1;
        }
        if self.refresh_rate != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.Display", len)?;
        if let Some(v) = self.resolution.as_ref() {
            struct_ser.serialize_field("resolution", v)?;
        }
        if self.refresh_rate != 0 {
            struct_ser.serialize_field("refreshRate", &self.refresh_rate)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Display {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "resolution",
            "refresh_rate",
            "refreshRate",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Resolution,
            RefreshRate,
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
                            "resolution" => Ok(GeneratedField::Resolution),
                            "refreshRate" | "refresh_rate" => Ok(GeneratedField::RefreshRate),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Display;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.Display")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Display, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut resolution__ = None;
                let mut refresh_rate__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Resolution => {
                            if resolution__.is_some() {
                                return Err(serde::de::Error::duplicate_field("resolution"));
                            }
                            resolution__ = map_.next_value()?;
                        }
                        GeneratedField::RefreshRate => {
                            if refresh_rate__.is_some() {
                                return Err(serde::de::Error::duplicate_field("refreshRate"));
                            }
                            refresh_rate__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Display {
                    resolution: resolution__,
                    refresh_rate: refresh_rate__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("veil.Display", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Error {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.code != 0 {
            len += 1;
        }
        if !self.message.is_empty() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.Error", len)?;
        if self.code != 0 {
            let v = error::Code::try_from(self.code)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.code)))?;
            struct_ser.serialize_field("code", &v)?;
        }
        if !self.message.is_empty() {
            struct_ser.serialize_field("message", &self.message)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Error {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "code",
            "message",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Code,
            Message,
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
                            "code" => Ok(GeneratedField::Code),
                            "message" => Ok(GeneratedField::Message),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Error;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.Error")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Error, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut code__ = None;
                let mut message__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Code => {
                            if code__.is_some() {
                                return Err(serde::de::Error::duplicate_field("code"));
                            }
                            code__ = Some(map_.next_value::<error::Code>()? as i32);
                        }
                        GeneratedField::Message => {
                            if message__.is_some() {
                                return Err(serde::de::Error::duplicate_field("message"));
                            }
                            message__ = Some(map_.next_value()?);
                        }
                    }
                }
                Ok(Error {
                    code: code__.unwrap_or_default(),
                    message: message__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("veil.Error", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for error::Code {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::ErrorUnspecified => "ERROR_UNSPECIFIED",
            Self::ErrorServer => "ERROR_SERVER",
            Self::ErrorProtocol => "ERROR_PROTOCOL",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for error::Code {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "ERROR_UNSPECIFIED",
            "ERROR_SERVER",
            "ERROR_PROTOCOL",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = error::Code;

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
                    "ERROR_UNSPECIFIED" => Ok(error::Code::ErrorUnspecified),
                    "ERROR_SERVER" => Ok(error::Code::ErrorServer),
                    "ERROR_PROTOCOL" => Ok(error::Code::ErrorProtocol),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for Gamepad {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if !self.id.is_empty() {
            len += 1;
        }
        if self.index != 0 {
            len += 1;
        }
        if self.mapping != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.Gamepad", len)?;
        if !self.id.is_empty() {
            struct_ser.serialize_field("id", &self.id)?;
        }
        if self.index != 0 {
            struct_ser.serialize_field("index", &self.index)?;
        }
        if self.mapping != 0 {
            let v = gamepad::Mapping::try_from(self.mapping)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.mapping)))?;
            struct_ser.serialize_field("mapping", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Gamepad {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "id",
            "index",
            "mapping",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Id,
            Index,
            Mapping,
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
                            "id" => Ok(GeneratedField::Id),
                            "index" => Ok(GeneratedField::Index),
                            "mapping" => Ok(GeneratedField::Mapping),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Gamepad;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.Gamepad")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Gamepad, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut id__ = None;
                let mut index__ = None;
                let mut mapping__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Id => {
                            if id__.is_some() {
                                return Err(serde::de::Error::duplicate_field("id"));
                            }
                            id__ = Some(map_.next_value()?);
                        }
                        GeneratedField::Index => {
                            if index__.is_some() {
                                return Err(serde::de::Error::duplicate_field("index"));
                            }
                            index__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Mapping => {
                            if mapping__.is_some() {
                                return Err(serde::de::Error::duplicate_field("mapping"));
                            }
                            mapping__ = Some(map_.next_value::<gamepad::Mapping>()? as i32);
                        }
                    }
                }
                Ok(Gamepad {
                    id: id__.unwrap_or_default(),
                    index: index__.unwrap_or_default(),
                    mapping: mapping__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("veil.Gamepad", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for gamepad::Mapping {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Unspecified => "UNSPECIFIED",
            Self::Standard => "STANDARD",
            Self::StandardXr => "STANDARD_XR",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for gamepad::Mapping {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "UNSPECIFIED",
            "STANDARD",
            "STANDARD_XR",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = gamepad::Mapping;

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
                    "UNSPECIFIED" => Ok(gamepad::Mapping::Unspecified),
                    "STANDARD" => Ok(gamepad::Mapping::Standard),
                    "STANDARD_XR" => Ok(gamepad::Mapping::StandardXr),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}
impl serde::Serialize for RenegotiateRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.offer.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.RenegotiateRequest", len)?;
        if let Some(v) = self.offer.as_ref() {
            struct_ser.serialize_field("offer", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RenegotiateRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "offer",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Offer,
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
                            "offer" => Ok(GeneratedField::Offer),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RenegotiateRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.RenegotiateRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RenegotiateRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut offer__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Offer => {
                            if offer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("offer"));
                            }
                            offer__ = map_.next_value()?;
                        }
                    }
                }
                Ok(RenegotiateRequest {
                    offer: offer__,
                })
            }
        }
        deserializer.deserialize_struct("veil.RenegotiateRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for RenegotiateResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.answer.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.RenegotiateResponse", len)?;
        if let Some(v) = self.answer.as_ref() {
            struct_ser.serialize_field("answer", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for RenegotiateResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "answer",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Answer,
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
                            "answer" => Ok(GeneratedField::Answer),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = RenegotiateResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.RenegotiateResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<RenegotiateResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut answer__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Answer => {
                            if answer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("answer"));
                            }
                            answer__ = map_.next_value()?;
                        }
                    }
                }
                Ok(RenegotiateResponse {
                    answer: answer__,
                })
            }
        }
        deserializer.deserialize_struct("veil.RenegotiateResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for Resolution {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.width != 0 {
            len += 1;
        }
        if self.height != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.Resolution", len)?;
        if self.width != 0 {
            struct_ser.serialize_field("width", &self.width)?;
        }
        if self.height != 0 {
            struct_ser.serialize_field("height", &self.height)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for Resolution {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "width",
            "height",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Width,
            Height,
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
                            "width" => Ok(GeneratedField::Width),
                            "height" => Ok(GeneratedField::Height),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = Resolution;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.Resolution")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<Resolution, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut width__ = None;
                let mut height__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Width => {
                            if width__.is_some() {
                                return Err(serde::de::Error::duplicate_field("width"));
                            }
                            width__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                        GeneratedField::Height => {
                            if height__.is_some() {
                                return Err(serde::de::Error::duplicate_field("height"));
                            }
                            height__ = 
                                Some(map_.next_value::<::pbjson::private::NumberDeserialize<_>>()?.0)
                            ;
                        }
                    }
                }
                Ok(Resolution {
                    width: width__.unwrap_or_default(),
                    height: height__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("veil.Resolution", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SetAnswerRequest {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.answer.is_some() {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.SetAnswerRequest", len)?;
        if let Some(v) = self.answer.as_ref() {
            struct_ser.serialize_field("answer", v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SetAnswerRequest {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "answer",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Answer,
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
                            "answer" => Ok(GeneratedField::Answer),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SetAnswerRequest;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.SetAnswerRequest")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SetAnswerRequest, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut answer__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Answer => {
                            if answer__.is_some() {
                                return Err(serde::de::Error::duplicate_field("answer"));
                            }
                            answer__ = map_.next_value()?;
                        }
                    }
                }
                Ok(SetAnswerRequest {
                    answer: answer__,
                })
            }
        }
        deserializer.deserialize_struct("veil.SetAnswerRequest", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for SetAnswerResponse {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let len = 0;
        let struct_ser = serializer.serialize_struct("veil.SetAnswerResponse", len)?;
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for SetAnswerResponse {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
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
                            Err(serde::de::Error::unknown_field(value, FIELDS))
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = SetAnswerResponse;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.SetAnswerResponse")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<SetAnswerResponse, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                while map_.next_key::<GeneratedField>()?.is_some() {
                    let _ = map_.next_value::<serde::de::IgnoredAny>()?;
                }
                Ok(SetAnswerResponse {
                })
            }
        }
        deserializer.deserialize_struct("veil.SetAnswerResponse", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for VideoStream {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut len = 0;
        if self.display.is_some() {
            len += 1;
        }
        if self.codec != 0 {
            len += 1;
        }
        let mut struct_ser = serializer.serialize_struct("veil.VideoStream", len)?;
        if let Some(v) = self.display.as_ref() {
            struct_ser.serialize_field("display", v)?;
        }
        if self.codec != 0 {
            let v = video_stream::Codec::try_from(self.codec)
                .map_err(|_| serde::ser::Error::custom(format!("Invalid variant {}", self.codec)))?;
            struct_ser.serialize_field("codec", &v)?;
        }
        struct_ser.end()
    }
}
impl<'de> serde::Deserialize<'de> for VideoStream {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "display",
            "codec",
        ];

        #[allow(clippy::enum_variant_names)]
        enum GeneratedField {
            Display,
            Codec,
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
                            "display" => Ok(GeneratedField::Display),
                            "codec" => Ok(GeneratedField::Codec),
                            _ => Err(serde::de::Error::unknown_field(value, FIELDS)),
                        }
                    }
                }
                deserializer.deserialize_identifier(GeneratedVisitor)
            }
        }
        struct GeneratedVisitor;
        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = VideoStream;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("struct veil.VideoStream")
            }

            fn visit_map<V>(self, mut map_: V) -> std::result::Result<VideoStream, V::Error>
                where
                    V: serde::de::MapAccess<'de>,
            {
                let mut display__ = None;
                let mut codec__ = None;
                while let Some(k) = map_.next_key()? {
                    match k {
                        GeneratedField::Display => {
                            if display__.is_some() {
                                return Err(serde::de::Error::duplicate_field("display"));
                            }
                            display__ = map_.next_value()?;
                        }
                        GeneratedField::Codec => {
                            if codec__.is_some() {
                                return Err(serde::de::Error::duplicate_field("codec"));
                            }
                            codec__ = Some(map_.next_value::<video_stream::Codec>()? as i32);
                        }
                    }
                }
                Ok(VideoStream {
                    display: display__,
                    codec: codec__.unwrap_or_default(),
                })
            }
        }
        deserializer.deserialize_struct("veil.VideoStream", FIELDS, GeneratedVisitor)
    }
}
impl serde::Serialize for video_stream::Codec {
    #[allow(deprecated)]
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let variant = match self {
            Self::Avc => "AVC",
            Self::Hevc => "HEVC",
            Self::Av1 => "AV1",
            Self::Vp8 => "VP8",
            Self::Vp9 => "VP9",
        };
        serializer.serialize_str(variant)
    }
}
impl<'de> serde::Deserialize<'de> for video_stream::Codec {
    #[allow(deprecated)]
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        const FIELDS: &[&str] = &[
            "AVC",
            "HEVC",
            "AV1",
            "VP8",
            "VP9",
        ];

        struct GeneratedVisitor;

        impl<'de> serde::de::Visitor<'de> for GeneratedVisitor {
            type Value = video_stream::Codec;

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
                    "AVC" => Ok(video_stream::Codec::Avc),
                    "HEVC" => Ok(video_stream::Codec::Hevc),
                    "AV1" => Ok(video_stream::Codec::Av1),
                    "VP8" => Ok(video_stream::Codec::Vp8),
                    "VP9" => Ok(video_stream::Codec::Vp9),
                    _ => Err(serde::de::Error::unknown_variant(value, FIELDS)),
                }
            }
        }
        deserializer.deserialize_any(GeneratedVisitor)
    }
}

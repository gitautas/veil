// @generated
// This file is @generated by prost-build.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct Resolution {
    #[prost(int32, tag="1")]
    pub width: i32,
    #[prost(int32, tag="2")]
    pub height: i32,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct Display {
    #[prost(message, optional, tag="1")]
    pub resolution: ::core::option::Option<Resolution>,
    #[prost(int32, tag="2")]
    pub refresh_rate: i32,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct VideoStream {
    #[prost(message, optional, tag="1")]
    pub display: ::core::option::Option<Display>,
    #[prost(enumeration="video_stream::Codec", tag="2")]
    pub codec: i32,
}
/// Nested message and enum types in `VideoStream`.
pub mod video_stream {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Codec {
        Unspecified = 0,
        Avc = 1,
        Hevc = 2,
        Av1 = 3,
        Vp8 = 4,
        Vp9 = 5,
    }
    impl Codec {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Self::Unspecified => "CODEC_UNSPECIFIED",
                Self::Avc => "AVC",
                Self::Hevc => "HEVC",
                Self::Av1 => "AV1",
                Self::Vp8 => "VP8",
                Self::Vp9 => "VP9",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "CODEC_UNSPECIFIED" => Some(Self::Unspecified),
                "AVC" => Some(Self::Avc),
                "HEVC" => Some(Self::Hevc),
                "AV1" => Some(Self::Av1),
                "VP8" => Some(Self::Vp8),
                "VP9" => Some(Self::Vp9),
                _ => None,
            }
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AudioStream {
    #[prost(enumeration="audio_stream::Codec", tag="1")]
    pub encoding: i32,
    #[prost(enumeration="audio_stream::Channel", repeated, tag="2")]
    pub channels: ::prost::alloc::vec::Vec<i32>,
}
/// Nested message and enum types in `AudioStream`.
pub mod audio_stream {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Channel {
        Unspecified = 0,
        Mono = 1,
        FrontLeft = 2,
        FrontRight = 3,
        FrontCenter = 4,
        RearCenter = 5,
        RearLeft = 6,
        RearRight = 7,
        Lfe = 8,
        FrontLeftOfCenter = 9,
        FrontRightOfCenter = 10,
        SideLeft = 11,
        SideRight = 12,
    }
    impl Channel {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Self::Unspecified => "CHANNEL_UNSPECIFIED",
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
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "CHANNEL_UNSPECIFIED" => Some(Self::Unspecified),
                "MONO" => Some(Self::Mono),
                "FRONT_LEFT" => Some(Self::FrontLeft),
                "FRONT_RIGHT" => Some(Self::FrontRight),
                "FRONT_CENTER" => Some(Self::FrontCenter),
                "REAR_CENTER" => Some(Self::RearCenter),
                "REAR_LEFT" => Some(Self::RearLeft),
                "REAR_RIGHT" => Some(Self::RearRight),
                "LFE" => Some(Self::Lfe),
                "FRONT_LEFT_OF_CENTER" => Some(Self::FrontLeftOfCenter),
                "FRONT_RIGHT_OF_CENTER" => Some(Self::FrontRightOfCenter),
                "SIDE_LEFT" => Some(Self::SideLeft),
                "SIDE_RIGHT" => Some(Self::SideRight),
                _ => None,
            }
        }
    }
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Codec {
        Unspecified = 0,
        Opus = 1,
    }
    impl Codec {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Self::Unspecified => "CODEC_UNSPECIFIED",
                Self::Opus => "OPUS",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "CODEC_UNSPECIFIED" => Some(Self::Unspecified),
                "OPUS" => Some(Self::Opus),
                _ => None,
            }
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Gamepad {
    #[prost(string, tag="1")]
    pub id: ::prost::alloc::string::String,
    #[prost(uint32, tag="2")]
    pub index: u32,
    #[prost(enumeration="gamepad::Mapping", tag="3")]
    pub mapping: i32,
}
/// Nested message and enum types in `Gamepad`.
pub mod gamepad {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Mapping {
        Unspecified = 0,
        Standard = 1,
        StandardXr = 2,
    }
    impl Mapping {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Self::Unspecified => "MAPPING_UNSPECIFIED",
                Self::Standard => "STANDARD",
                Self::StandardXr => "STANDARD_XR",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "MAPPING_UNSPECIFIED" => Some(Self::Unspecified),
                "STANDARD" => Some(Self::Standard),
                "STANDARD_XR" => Some(Self::StandardXr),
                _ => None,
            }
        }
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Error {
    #[prost(enumeration="error::Code", tag="1")]
    pub code: i32,
    #[prost(string, tag="2")]
    pub message: ::prost::alloc::string::String,
}
/// Nested message and enum types in `Error`.
pub mod error {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, ::prost::Enumeration)]
    #[repr(i32)]
    pub enum Code {
        ErrorUnspecified = 0,
        ErrorServer = 10,
        ErrorProtocol = 20,
    }
    impl Code {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                Self::ErrorUnspecified => "ERROR_UNSPECIFIED",
                Self::ErrorServer => "ERROR_SERVER",
                Self::ErrorProtocol => "ERROR_PROTOCOL",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "ERROR_UNSPECIFIED" => Some(Self::ErrorUnspecified),
                "ERROR_SERVER" => Some(Self::ErrorServer),
                "ERROR_PROTOCOL" => Some(Self::ErrorProtocol),
                _ => None,
            }
        }
    }
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct CreateSessionRequest {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CreateSessionResponse {
    #[prost(message, optional, tag="1")]
    pub offer: ::core::option::Option<webrtc::SessionDescription>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SetAnswerRequest {
    #[prost(message, optional, tag="1")]
    pub answer: ::core::option::Option<webrtc::SessionDescription>,
}
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct SetAnswerResponse {
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RenegotiateRequest {
    #[prost(message, optional, tag="1")]
    pub offer: ::core::option::Option<webrtc::SessionDescription>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RenegotiateResponse {
    #[prost(message, optional, tag="1")]
    pub answer: ::core::option::Option<webrtc::SessionDescription>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrickleIceRequest {
    #[prost(message, optional, tag="1")]
    pub candidate: ::core::option::Option<webrtc::IceCandidate>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrickleIceResponse {
    #[prost(message, optional, tag="1")]
    pub candidate: ::core::option::Option<webrtc::IceCandidate>,
}
include!("veil.serde.rs");
include!("veil.tonic.rs");
// @@protoc_insertion_point(module)
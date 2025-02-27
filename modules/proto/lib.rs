// Import generated Protobuf code
// This file re-exports the generated protobuf code

// First include all message definitions
mod proto_defs {
    // Include base definitions
    pub mod veil {
        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/rs/veil.rs"));
        
        // WebRTC submodule
        pub mod webrtc {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/rs/veil.webrtc.rs"));
        }
        
        // Compositor submodule
        pub mod compositor {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/rs/veil.compositor.rs"));
        }
        
        // Media submodule
        pub mod media {
            include!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/rs/veil.media.rs"));
        }
    }
}

// Re-export all message definitions at the crate root
pub use proto_defs::*;

// Create separate modules for tonic implementations which will have access to the message types
pub mod tonic_mod {
    use crate::veil::*; // Import all message types
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/rs/veil.tonic.rs"));
}

pub mod compositor_tonic_mod {
    use crate::veil::compositor::*; // Import compositor types
    include!(concat!(env!("CARGO_MANIFEST_DIR"), "/gen/rs/veil.compositor.tonic.rs"));
}

// Re-export the service clients and servers from tonic modules
pub use tonic_mod::rtc_service_client;
pub use tonic_mod::rtc_service_server;
pub use compositor_tonic_mod::compositor_service_client;
pub use compositor_tonic_mod::compositor_service_server;

// Any additional traits or extensions can be added here
// These won't be affected by code regeneration 
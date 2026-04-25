//! English Partner - Core types and utilities shared across all crates.

pub mod error;
pub mod protocol;
pub mod speech;

/// Generated protobuf types (from `proto/ep/v1/conversation.proto`).
pub mod pb {
    pub mod v1 {
        include!(concat!(env!("OUT_DIR"), "/ep.v1.rs"));
    }
}

pub use error::*;
pub use protocol::*;
pub use speech::*;

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::pb::v1::{AudioChunk, StartRequest, TurnResponse};

    #[test]
    fn test_protobuf_roundtrip() {
        let req = StartRequest {
            scenario_id: "restaurant".to_string(),
        };
        let bytes = req.encode_to_vec();
        let decoded = StartRequest::decode(&bytes[..]).unwrap();
        assert_eq!(decoded.scenario_id, "restaurant");
    }

    #[test]
    fn test_turn_response_serialize() {
        let resp = TurnResponse {
            transcript: "hello".to_string(),
            ai_text: "hi there".to_string(),
            audio_wav: b"wav".to_vec(),
        };
        let bytes = resp.encode_to_vec();
        assert!(!bytes.is_empty());

        let decoded = TurnResponse::decode(&bytes[..]).unwrap();
        assert_eq!(decoded.transcript, "hello");
        assert_eq!(decoded.audio_wav, b"wav");
    }

    #[test]
    fn test_audio_chunk_fields() {
        let chunk = AudioChunk {
            pcm_data: vec![0, 1, 2],
            sample_rate: 16000,
            timestamp_ms: 1000,
        };
        assert_eq!(chunk.sample_rate, 16000);
        assert_eq!(chunk.timestamp_ms, 1000);
    }

    /// Cross-language compatibility test:
    /// The bytes below are the protobuf payload produced by Python for
    /// `StartRequest(scenario_id="restaurant")`.
    #[test]
    fn test_python_compatibility() {
        let python_bytes: &[u8] = &[0x0a, 0x0a, 0x72, 0x65, 0x73, 0x74, 0x61, 0x75, 0x72, 0x61, 0x6e, 0x74];
        let decoded = StartRequest::decode(python_bytes).unwrap();
        assert_eq!(decoded.scenario_id, "restaurant");
    }
}

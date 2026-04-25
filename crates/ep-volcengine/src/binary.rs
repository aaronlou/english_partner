//! Binary protocol encoding/decoding for Volcengine Realtime Speech API.
//!
//! Protocol: 4-byte header + optional fields + 4-byte payload_size + payload
//!
//! Header (4 bytes):
//!   Byte 0: [4bit Protocol Version=1] [4bit Header Size=4]
//!   Byte 1: [4bit Message Type] [4bit Message Type Specific Flags]
//!   Byte 2: [4bit Serialization] [4bit Compression]
//!   Byte 3: Reserved (0x00)
//!
//! Message Types:
//!   0x1 = Full-client request
//!   0x2 = Audio-only client request
//!   0x9 = Full-server response
//!   0xB = Audio-only server response
//!   0xF = Error information
//!
//! Flags (lower nibble of Byte 1):
//!   0x4 = Event flag: the next 4 bytes are the event ID

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgType {
    FullClientRequest = 0x1,
    AudioClientRequest = 0x2,
    FullServerResponse = 0x9,
    AudioServerResponse = 0xB,
    ErrorInfo = 0xF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Serialization {
    Raw = 0x0,
    Json = 0x1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Compression {
    None = 0x0,
    Gzip = 0x1,
}

const EVENT_FLAG: u8 = 0x4;

/// Build a 4-byte header.
fn header(msg_type: MsgType, ser: Serialization, comp: Compression, flags: u8) -> [u8; 4] {
    let b0: u8 = 0x11; // Protocol v1, Header 4 bytes
    let b1: u8 = ((msg_type as u8) << 4) | (flags & 0x0F);
    let b2: u8 = ((ser as u8) << 4) | (comp as u8);
    [b0, b1, b2, 0x00]
}

// ── Encoding ──────────────────────────────────────────────────────────────

/// Encode a JSON event with no session_id (Connection-level events).
pub fn encode_connect_event(event_id: u32, json: &str) -> Vec<u8> {
    let hdr = header(MsgType::FullClientRequest, Serialization::Json, Compression::None, EVENT_FLAG);
    let payload = json.as_bytes();
    let mut buf = Vec::with_capacity(12 + payload.len());
    buf.extend_from_slice(&hdr);
    buf.extend_from_slice(&event_id.to_be_bytes());       // event_id
    buf.extend_from_slice(&(payload.len() as u32).to_be_bytes()); // payload_size
    buf.extend_from_slice(payload);                        // payload
    buf
}

/// Encode a JSON event with a session_id.
pub fn encode_session_event(event_id: u32, session_id: &str, json: &str) -> Vec<u8> {
    let hdr = header(MsgType::FullClientRequest, Serialization::Json, Compression::None, EVENT_FLAG);
    let sid = session_id.as_bytes();
    let payload = json.as_bytes();
    let mut buf = Vec::with_capacity(16 + sid.len() + payload.len());
    buf.extend_from_slice(&hdr);
    buf.extend_from_slice(&event_id.to_be_bytes());        // event_id
    buf.extend_from_slice(&(sid.len() as u32).to_be_bytes()); // session_id_size
    buf.extend_from_slice(sid);                             // session_id
    buf.extend_from_slice(&(payload.len() as u32).to_be_bytes()); // payload_size
    buf.extend_from_slice(payload);                         // payload
    buf
}

/// Encode an audio-only event (Raw serialization) with session_id.
/// Used for TaskRequest (event 200).
pub fn encode_audio_event(session_id: &str, pcm_data: &[u8]) -> Vec<u8> {
    let hdr = header(MsgType::AudioClientRequest, Serialization::Raw, Compression::None, EVENT_FLAG);
    let sid = session_id.as_bytes();
    let event_id: u32 = 200; // TaskRequest
    let mut buf = Vec::with_capacity(16 + sid.len() + pcm_data.len());
    buf.extend_from_slice(&hdr);
    buf.extend_from_slice(&event_id.to_be_bytes());        // event_id
    buf.extend_from_slice(&(sid.len() as u32).to_be_bytes()); // session_id_size
    buf.extend_from_slice(sid);                             // session_id
    buf.extend_from_slice(&(pcm_data.len() as u32).to_be_bytes()); // payload_size
    buf.extend_from_slice(pcm_data);                        // payload
    buf
}

// ── Decoding ──────────────────────────────────────────────────────────────

/// A decoded frame from the server.
#[derive(Debug, Clone)]
pub enum ServerFrame {
    Json {
        msg_type: MsgType,
        event_id: u32,
        session_id: Option<String>,
        payload: serde_json::Value,
    },
    Audio {
        event_id: u32,
        session_id: Option<String>,
        data: Vec<u8>,
    },
    Error {
        code: u32,
        message: String,
    },
}

/// Decode a binary frame received from the server WebSocket.
pub fn decode_frame(data: &[u8]) -> anyhow::Result<ServerFrame> {
    if data.len() < 4 {
        anyhow::bail!("frame too short: {} bytes", data.len());
    }

    let msg_type = match (data[1] >> 4) & 0x0F {
        0x9 => MsgType::FullServerResponse,
        0xB => MsgType::AudioServerResponse,
        0xF => MsgType::ErrorInfo,
        other => anyhow::bail!("unknown server msg_type: 0x{:X}", other),
    };

    let flags = data[1] & 0x0F;
    let has_event = (flags & EVENT_FLAG) != 0;
    let serialization = (data[2] >> 4) & 0x0F;

    let mut pos: usize = 4;

    // Error code (only for error frames, before event_id)
    let error_code = if msg_type == MsgType::ErrorInfo && data.len() >= pos + 4 {
        let code = u32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]);
        pos += 4;
        code
    } else {
        0
    };

    // Event ID
    let event_id = if has_event && data.len() >= pos + 4 {
        let eid = u32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]);
        pos += 4;
        eid
    } else {
        0
    };

    // Session ID — only for Session-level events (event_id >= 100).
    // Connect-level events (1, 2, 50, 51, 52) may have connect_id instead.
    let is_session_event = event_id >= 100;
    let is_connect_event = event_id < 100;

    let session_id: Option<String> = if is_session_event && data.len() >= pos + 4 {
        let sid_size =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        pos += 4;
        if sid_size > 0 && sid_size < 256 && data.len() >= pos + sid_size {
            let sid = std::str::from_utf8(&data[pos..pos + sid_size])
                .ok()
                .map(|s| s.to_string());
            pos += sid_size;
            sid
        } else {
            anyhow::bail!("invalid session_id in session event {}", event_id);
        }
    } else {
        None
    };

    // Connect ID — for Connect-level events, server may echo the connect_id.
    let _connect_id: Option<String> = if is_connect_event && data.len() >= pos + 4 {
        let cid_size =
            u32::from_be_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]) as usize;
        // connect_ids are typically hex strings < 64 chars. Check if this looks like a size field
        // by verifying the data after it looks like ASCII.
        if cid_size > 0 && cid_size < 64 && data.len() >= pos + 4 + cid_size {
            let peek = &data[pos + 4..pos + 4 + cid_size];
            let looks_like_id = peek.iter().all(|&b| b.is_ascii_alphanumeric() || b == b'-');
            if looks_like_id {
                pos += 4;
                let cid = std::str::from_utf8(&data[pos..pos + cid_size])
                    .ok()
                    .map(|s| s.to_string());
                pos += cid_size;
                cid
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    // Payload
    if data.len() < pos + 4 {
        anyhow::bail!("no payload in frame");
    }
    let payload_size = u32::from_be_bytes([data[pos], data[pos+1], data[pos+2], data[pos+3]]) as usize;
    pos += 4;

    if data.len() < pos + payload_size {
        anyhow::bail!("incomplete payload: need {} bytes, have {}", payload_size, data.len() - pos);
    }

    let payload = &data[pos..pos + payload_size];

    match msg_type {
        MsgType::ErrorInfo => {
            let msg = std::str::from_utf8(payload)
                .unwrap_or("<non-utf8 error payload>");
            Ok(ServerFrame::Error {
                code: error_code,
                message: msg.to_string(),
            })
        }
        MsgType::AudioServerResponse => {
            Ok(ServerFrame::Audio {
                event_id,
                session_id,
                data: payload.to_vec(),
            })
        }
        _ => {
            // JSON payload
            if serialization == 0x1 {
                let text = std::str::from_utf8(payload)?;
                let json: serde_json::Value = serde_json::from_str(text)?;
                Ok(ServerFrame::Json {
                    msg_type,
                    event_id,
                    session_id,
                    payload: json,
                })
            } else {
                // Unknown serialization for text frame — try JSON anyway
                let text = std::str::from_utf8(payload)?;
                let json: serde_json::Value = serde_json::from_str(text)?;
                Ok(ServerFrame::Json {
                    msg_type,
                    event_id,
                    session_id,
                    payload: json,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_connect_event() {
        let frame = encode_connect_event(1, "{}");
        // [17 20 16 0, 0 0 0 1, 0 0 0 2, 123 125]
        assert_eq!(frame, vec![0x11, 0x14, 0x10, 0x00, 0,0,0,1, 0,0,0,2, b'{', b'}']);
    }

    #[test]
    fn test_encode_session_event() {
        let sid = "75a6126e-427f-49a1-a2c1-621143cb9db3";
        let json = r#"{"dialog":{"bot_name":"豆包"}}"#;
        let frame = encode_session_event(100, sid, json);
        assert_eq!(&frame[0..4], &[0x11, 0x14, 0x10, 0x00]);
        assert_eq!(&frame[4..8], &100u32.to_be_bytes());
        assert_eq!(&frame[8..12], &36u32.to_be_bytes());
    }

    #[test]
    fn test_decode_connect_started() {
        // ConnectionStarted: event 50, payload "{}"
        let data = [0x11, 0x94, 0x10, 0x00, 0,0,0,50, 0,0,0,2, b'{', b'}'];
        let frame = decode_frame(&data).unwrap();
        match frame {
            ServerFrame::Json { event_id, session_id, .. } => {
                assert_eq!(event_id, 50);
                assert!(session_id.is_none());
            }
            _ => panic!("expected Json frame"),
        }
    }

    #[test]
    fn test_decode_session_started() {
        // SessionStarted: event 150, sid="abc", payload={"dialog_id":"x"}
        let sid = b"abc";
        let payload = br#"{"dialog_id":"x"}"#;
        let mut data = vec![0x11, 0x94, 0x10, 0x00];
        data.extend_from_slice(&150u32.to_be_bytes());
        data.extend_from_slice(&(sid.len() as u32).to_be_bytes());
        data.extend_from_slice(sid);
        data.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        data.extend_from_slice(payload);

        let frame = decode_frame(&data).unwrap();
        match frame {
            ServerFrame::Json { event_id, session_id, .. } => {
                assert_eq!(event_id, 150);
                assert_eq!(session_id, Some("abc".to_string()));
            }
            _ => panic!("expected Json frame"),
        }
    }

    #[test]
    fn test_decode_audio_response() {
        // TTSResponse: event 352, sid="x", audio data
        let sid = b"x";
        let audio = vec![1u8, 2, 3, 4];
        let mut data = vec![0x11, 0xB4, 0x00, 0x00]; // AudioServerResponse + event flag
        data.extend_from_slice(&352u32.to_be_bytes());
        data.extend_from_slice(&(sid.len() as u32).to_be_bytes());
        data.extend_from_slice(sid);
        data.extend_from_slice(&(audio.len() as u32).to_be_bytes());
        data.extend_from_slice(&audio);

        let frame = decode_frame(&data).unwrap();
        match frame {
            ServerFrame::Audio { event_id, session_id, data: d } => {
                assert_eq!(event_id, 352);
                assert_eq!(session_id, Some("x".to_string()));
                assert_eq!(d, vec![1, 2, 3, 4]);
            }
            _ => panic!("expected Audio frame"),
        }
    }

    #[test]
    fn test_decode_error() {
        // Error frame: msg_type 0xF,encoding: error_code + event + session + payload
        // Simple error with json {"error":"test"}
        let payload = br#"{"error":"test error"}"#;
        let mut data = vec![0x11, 0xF4, 0x10, 0x00]; // ErrorInfo + event flag, JSON
        data.extend_from_slice(&0u32.to_be_bytes());  // error_code
        data.extend_from_slice(&0u32.to_be_bytes());  // event_id (0)
        data.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        data.extend_from_slice(payload);

        let frame = decode_frame(&data).unwrap();
        match frame {
            ServerFrame::Error { code, message } => {
                assert_eq!(code, 0);
                assert!(message.contains("test error"));
            }
            _ => panic!("expected Error frame"),
        }
    }
}
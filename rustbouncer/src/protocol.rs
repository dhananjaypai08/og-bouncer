use bytes::{Buf, BytesMut};
use std::collections::HashMap;

#[derive(Debug)]
pub enum StartupKind {
    SslRequest,
    CancelRequest,
    StartupMessage,
}

#[derive(Debug)]
pub struct PgMessage {
    pub tag: Option<u8>, // None for startup-phase messages
    pub payload: BytesMut,
    pub startup_kind: Option<StartupKind>,
}

/// Try to parse exactly ONE PostgreSQL message from the buffer.
/// Returns None if not enough data yet.
pub fn try_parse_message(buf: &mut BytesMut, startup: bool) -> Option<PgMessage> {
    if startup {
        // Startup / SSL / Cancel messages have NO tag byte
        if buf.len() < 8 {
            return None;
        }

        let len = (&buf[..4]).get_u32() as usize;
        if buf.len() < len {
            return None;
        }

        let code = (&buf[4..8]).get_u32();

        // Remove length
        buf.advance(4);
        let payload = buf.split_to(len - 4);

        let startup_kind = match code {
            80877103 => Some(StartupKind::SslRequest),
            80877102 => Some(StartupKind::CancelRequest),
            _ => Some(StartupKind::StartupMessage),
        };

        return Some(PgMessage {
            tag: None,
            payload,
            startup_kind,
        });
    }

    // Normal messages (tagged)
    if buf.len() < 5 {
        return None;
    }

    let tag = buf[0];
    let len = (&buf[1..5]).get_u32() as usize;

    if buf.len() < len + 1 {
        return None;
    }

    buf.advance(5);
    let payload = buf.split_to(len - 4);

    Some(PgMessage {
        tag: Some(tag),
        payload,
        startup_kind: None,
    })
}

/// Parse key/value pairs from a StartupMessage payload
pub fn parse_startup_params(payload: &BytesMut) -> HashMap<String, String> {
    let mut params = HashMap::new();

    // First 4 bytes = protocol version
    let mut i = 4;

    while i < payload.len() {
        if payload[i] == 0 {
            break;
        }

        let key_start = i;
        while payload[i] != 0 {
            i += 1;
        }
        let key = String::from_utf8_lossy(&payload[key_start..i]).to_string();
        i += 1;

        let val_start = i;
        while payload[i] != 0 {
            i += 1;
        }
        let val = String::from_utf8_lossy(&payload[val_start..i]).to_string();
        i += 1;

        params.insert(key, val);
    }

    params
}

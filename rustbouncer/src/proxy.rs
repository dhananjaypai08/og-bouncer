use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use bytes::BytesMut;
use tracing::info;
use std::sync::Arc;

use crate::protocol::{
    try_parse_message,
    parse_startup_params,
    StartupKind,
};

#[derive(Debug, Clone, Copy)]
enum AuthState {
    NotStarted,
    InProgress,
    Ok,
}

pub async fn proxy(client: TcpStream) -> anyhow::Result<()> {
    let server = TcpStream::connect("127.0.0.1:5432").await?;

    let (mut cr, mut cw) = client.into_split();
    let (mut sr, mut sw) = server.into_split();

    let mut client_buf = BytesMut::with_capacity(8192);
    let mut server_buf = BytesMut::with_capacity(8192);

    let mut startup_phase = true;
    let auth_state = Arc::new(Mutex::new(AuthState::NotStarted));

    // ---- client → server (startup parsing)
    let client_to_server = async {
        loop {
            let n = cr.read_buf(&mut client_buf).await?;
            if n == 0 {
                break;
            }

            while let Some(msg) = try_parse_message(&mut client_buf, startup_phase) {
                if startup_phase {
                    if let Some(kind) = msg.startup_kind {
                        match kind {
                            StartupKind::SslRequest => {
                                info!("SSLRequest received");
                            }
                            StartupKind::CancelRequest => {
                                info!("CancelRequest received");
                            }
                            StartupKind::StartupMessage => {
                                let params = parse_startup_params(&msg.payload);
                                info!("Startup params: {:?}", params);
                                startup_phase = false;
                                *auth_state.lock().await = AuthState::InProgress;
                            }
                        }
                    }
                }

                // forward bytes
                if let Some(tag) = msg.tag {
                    sw.write_u8(tag).await?;
                }
                sw.write_u32((msg.payload.len() + 4) as u32).await?;
                sw.write_all(&msg.payload).await?;
            }
        }

        sw.shutdown().await?;
        Ok::<_, anyhow::Error>(())
    };

    // ---- server → client (auth parsing)
    let server_to_client = async {
        loop {
            let n = sr.read_buf(&mut server_buf).await?;
            if n == 0 {
                break;
            }

            while let Some(msg) = try_parse_message(&mut server_buf, false) {
                if let Some(b'R') = msg.tag {
                    let auth_code = u32::from_be_bytes(
                        msg.payload[..4].try_into().unwrap()
                    );

                    match auth_code {
                        0 => {
                            info!("AuthenticationOk");
                            *auth_state.lock().await = AuthState::Ok;
                        }
                        3 => info!("AuthenticationCleartextPassword"),
                        5 => info!("AuthenticationMD5Password"),
                        10 => info!("AuthenticationSASL"),
                        11 => info!("AuthenticationSASLContinue"),
                        12 => info!("AuthenticationSASLFinal"),
                        _ => info!("AuthenticationUnknown({})", auth_code),
                    }
                }

                // forward bytes
                if let Some(tag) = msg.tag {
                    cw.write_u8(tag).await?;
                }
                cw.write_u32((msg.payload.len() + 4) as u32).await?;
                cw.write_all(&msg.payload).await?;
            }
        }

        cw.shutdown().await?;
        Ok::<_, anyhow::Error>(())
    };

    tokio::try_join!(client_to_server, server_to_client)?;
    Ok(())
}

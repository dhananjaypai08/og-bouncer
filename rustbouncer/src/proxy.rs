use bytes::BytesMut;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::info;

use crate::protocol::{StartupKind, parse_startup_params, try_parse_message};

pub async fn proxy(client: TcpStream) -> anyhow::Result<()> {
    let server = TcpStream::connect("127.0.0.1:5432").await?;

    let (mut cr, mut cw) = client.into_split();
    let (mut sr, mut sw) = server.into_split();

    let mut client_buf = BytesMut::with_capacity(8192);
    let mut startup_phase = true;

    // ---- Client → Server (with protocol inspection)
    let client_to_server = async {
        loop {
            let n = cr.read_buf(&mut client_buf).await?;
            if n == 0 {
                break;
            }

            while let Some(msg) = try_parse_message(&mut client_buf, startup_phase) {
                if startup_phase && let Some(kind) = msg.startup_kind {
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
                        }
                    }
                }

                // Forward EXACT bytes to Postgres
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

    // ---- Server → Client (pure streaming)
    let server_to_client = async {
        tokio::io::copy(&mut sr, &mut cw).await?;
        cw.shutdown().await?;
        Ok::<_, anyhow::Error>(())
    };

    tokio::try_join!(client_to_server, server_to_client)?;
    Ok(())
}

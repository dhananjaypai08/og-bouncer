mod protocol;
mod proxy;

use tokio::net::TcpListener;
use tracing::{error, info};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind("0.0.0.0:6432").await?;
    info!("rustbouncer listening on 6432");

    loop {
        let (socket, addr) = listener.accept().await?;
        info!("client connected: {}", addr);

        tokio::spawn(async move {
            if let Err(e) = proxy::proxy(socket).await {
                error!("connection error: {:?}", e);
            }
        });
    }
}

use tokio::net::{TcpListener, TcpStream};
use tracing::{info, error};
use std::env;
use tokio::io::{self, AsyncWriteExt};

const PORT: i32 = 6432;
const POSTGRES_PORT: i32 = 5432;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let addr = format!("0.0.0.0:{}", PORT.to_string());
    let listener = TcpListener::bind(addr).await?;
    info!("{} listening on: {PORT}", env::var("CARGO_PKG_NAME").unwrap());
    
    loop {
        let (client, addr) = listener.accept().await?;
        info!("client connected: {}", addr);

        tokio::spawn(async move {
            if let Err(e) = proxy(client).await {
                error!("connection error: {:?}", e);
            }
        });
    }
}

async fn proxy(mut client: TcpStream) -> anyhow::Result<()> {
    let postgres_addr = format!("127.0.0.1:{}", POSTGRES_PORT.to_string());
    let mut server = TcpStream::connect(postgres_addr).await?;
    info!(
        "connected client {} -> postgres {}",
        client.peer_addr()?,
        server.peer_addr()?
    );

    let (mut cr, mut cw) = client.split();
    let (mut sr, mut sw) = server.split();

    let client_to_server = async {
        io::copy(&mut cr, &mut sw).await?;
        sw.shutdown().await?;
        Ok::<_, anyhow::Error>(())
    };

    let server_to_client = async {
        io::copy(&mut sr, &mut cw).await?;
        cw.shutdown().await?;
        Ok::<_, anyhow::Error>(())
    };

    // Run both directions concurrently
    tokio::try_join!(client_to_server, server_to_client)?;

    info!("connection closed cleanly");
    Ok(())
}
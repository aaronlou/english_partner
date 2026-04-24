use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("English Partner starting...");

    let api_key = std::env::var("SONIOX_API_KEY")
        .expect("SONIOX_API_KEY environment variable must be set");

    let _soniox = ep_soniox::SonioxClient::new(api_key);

    info!("Soniox client initialized. Ready for voice conversations.");

    // TODO: main event loop
    Ok(())
}

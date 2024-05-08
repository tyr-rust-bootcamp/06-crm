use anyhow::Result;
use crm::{AppConfig, CrmService};
use tonic::transport::Server;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::{fmt::Layer, layer::SubscriberExt, util::SubscriberInitExt, Layer as _};

#[tokio::main]
async fn main() -> Result<()> {
    let layer = Layer::new().with_filter(LevelFilter::INFO);
    tracing_subscriber::registry().with(layer).init();

    let config = AppConfig::load().expect("Failed to load config");
    let addr = config.server.port;
    let addr = format!("[::1]:{}", addr).parse().unwrap();
    info!("CRM service listening on {}", addr);
    let svc = CrmService::try_new(config).await?.into_server();
    Server::builder().add_service(svc).serve(addr).await?;
    Ok(())
}

use anyhow::Result;
use crm::pb::{crm_client::CrmClient, WelcomeRequestBuilder};
use tonic::Request;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = CrmClient::connect("http://[::1]:50000").await?;

    let req = WelcomeRequestBuilder::default()
        .id(Uuid::new_v4().to_string())
        .interval(93u32)
        .content_ids([1u32, 2, 3])
        .build()?;

    let response = client.welcome(Request::new(req)).await?.into_inner();
    println!("Response: {:?}", response);
    Ok(())
}

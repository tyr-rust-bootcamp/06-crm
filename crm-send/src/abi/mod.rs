mod email;
mod in_app;
mod sms;

use chrono::Utc;
use crm_metadata::{pb::Content, Tpl};
use futures::{Stream, StreamExt};
use prost_types::Timestamp;
use std::{ops::Deref, sync::Arc, time::Duration};
use tokio::{sync::mpsc, time::sleep};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{Response, Status};
use tracing::{info, warn};
use uuid::Uuid;

const CHANNEL_SIZE: usize = 1024;

use crate::{
    pb::{
        notification_server::NotificationServer, send_request::Msg, EmailMessage, SendRequest,
        SendResponse,
    },
    AppConfig, NotificationService, NotificationServiceInner, ResponseStream, ServiceResult,
};

pub trait Sender {
    async fn send(self, svc: NotificationService) -> Result<SendResponse, Status>;
}

impl NotificationService {
    pub fn new(config: AppConfig) -> Self {
        let sender = dummy_send();
        let inner = NotificationServiceInner { config, sender };
        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn into_server(self) -> NotificationServer<Self> {
        NotificationServer::new(self)
    }

    pub async fn send(
        &self,
        mut stream: impl Stream<Item = Result<SendRequest, Status>> + Send + 'static + Unpin,
    ) -> ServiceResult<ResponseStream> {
        let (tx, rx) = mpsc::channel(CHANNEL_SIZE);
        let notif = self.clone();
        tokio::spawn(async move {
            while let Some(Ok(req)) = stream.next().await {
                let notif_clone = notif.clone();
                let res = match req.msg {
                    Some(Msg::Email(email)) => email.send(notif_clone).await,
                    Some(Msg::Sms(sms)) => sms.send(notif_clone).await,
                    Some(Msg::InApp(in_app)) => in_app.send(notif_clone).await,
                    None => {
                        warn!("Invalid request");
                        Err(Status::invalid_argument("Invalid request"))
                    }
                };
                tx.send(res).await.unwrap();
            }
        });

        let stream = ReceiverStream::new(rx);
        Ok(Response::new(Box::pin(stream)))
    }
}

impl Deref for NotificationService {
    type Target = NotificationServiceInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl SendRequest {
    pub fn new(
        subject: String,
        sender: String,
        recipients: &[String],
        contents: &[Content],
    ) -> Self {
        let tpl = Tpl(contents);
        let msg = Msg::Email(EmailMessage {
            message_id: Uuid::new_v4().to_string(),
            subject,
            sender,
            recipients: recipients.to_vec(),
            body: tpl.to_body(),
        });

        SendRequest { msg: Some(msg) }
    }
}

fn dummy_send() -> mpsc::Sender<Msg> {
    let (tx, mut rx) = mpsc::channel(CHANNEL_SIZE * 100);

    tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            info!("Sending message: {:?}", msg);
            sleep(Duration::from_millis(300)).await;
        }
    });
    tx
}

fn to_ts() -> Timestamp {
    let now = Utc::now();
    Timestamp {
        seconds: now.timestamp(),
        nanos: now.timestamp_subsec_nanos() as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        pb::{EmailMessage, InAppMessage, SmsMessage},
        AppConfig,
    };
    use anyhow::Result;

    #[tokio::test]
    async fn send_should_work() -> Result<()> {
        let config = AppConfig::load()?;
        let service = NotificationService::new(config);
        let stream = tokio_stream::iter(vec![
            Ok(EmailMessage::fake().into()),
            Ok(SmsMessage::fake().into()),
            Ok(InAppMessage::fake().into()),
        ]);

        let response = service.send(stream).await?;
        let ret = response.into_inner().collect::<Vec<_>>().await;
        assert_eq!(ret.len(), 3);

        Ok(())
    }
}

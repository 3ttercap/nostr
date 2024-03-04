// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2024 Rust Nostr Developers
// Distributed under the MIT software license

use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use async_utility::thread;
use nostr_ffi::{
    ClientMessage, Event, EventBuilder, EventId, FileMetadata, Filter, Metadata, PublicKey,
    RelayMessage, Timestamp,
};
use nostr_sdk::client::Client as ClientSdk;
use nostr_sdk::pool::RelayPoolNotification as RelayPoolNotificationSdk;
use nostr_sdk::{block_on, spawn_blocking, UncheckedUrl};
use uniffi::Object;

mod builder;
mod options;
pub mod signer;
pub mod zapper;

pub use self::builder::ClientBuilder;
pub use self::options::Options;
pub use self::signer::NostrSigner;
use self::zapper::{ZapDetails, ZapEntity};
use crate::error::Result;
use crate::relay::options::NegentropyOptions;
use crate::{NostrDatabase, Relay};

#[derive(Object)]
pub struct Client {
    inner: ClientSdk,
}

impl From<ClientSdk> for Client {
    fn from(inner: ClientSdk) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl Client {
    #[uniffi::constructor]
    pub fn new(signer: Option<Arc<NostrSigner>>) -> Self {
        Self::with_opts(signer, Arc::new(Options::new()))
    }

    #[uniffi::constructor]
    pub fn with_opts(signer: Option<Arc<NostrSigner>>, opts: Arc<Options>) -> Self {
        Self {
            inner: match signer {
                Some(signer) => ClientSdk::with_opts(
                    signer.as_ref().deref().clone(),
                    opts.as_ref().deref().clone(),
                ),
                None => nostr_sdk::ClientBuilder::new()
                    .opts(opts.as_ref().deref().clone())
                    .build(),
            },
        }
    }

    pub fn update_difficulty(&self, difficulty: u8) {
        self.inner.update_difficulty(difficulty);
    }

    pub fn signer(&self) -> Result<NostrSigner> {
        block_on(async move { Ok(self.inner.signer().await?.into()) })
    }

    pub fn database(&self) -> Arc<NostrDatabase> {
        Arc::new(self.inner.database().into())
    }

    pub fn start(&self) {
        block_on(async move { self.inner.start().await })
    }

    pub fn stop(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.stop().await?) })
    }

    pub fn shutdown(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.clone().shutdown().await?) })
    }

    pub fn relays(&self) -> HashMap<String, Arc<Relay>> {
        block_on(async move {
            self.inner
                .relays()
                .await
                .into_iter()
                .map(|(u, r)| (u.to_string(), Arc::new(r.into())))
                .collect()
        })
    }

    pub fn relay(&self, url: String) -> Result<Arc<Relay>> {
        block_on(async move { Ok(Arc::new(self.inner.relay(url).await?.into())) })
    }

    pub fn add_relay(&self, url: String) -> Result<bool> {
        block_on(async move { Ok(self.inner.add_relay(url).await?) })
    }

    pub fn add_relays(&self, relays: Vec<String>) -> Result<()> {
        block_on(async move { Ok(self.inner.add_relays(relays).await?) })
    }

    pub fn remove_relay(&self, url: String) -> Result<()> {
        block_on(async move { Ok(self.inner.remove_relay(url).await?) })
    }

    pub fn connect_relay(&self, url: String) -> Result<()> {
        block_on(async move { Ok(self.inner.connect_relay(url).await?) })
    }

    pub fn disconnect_relay(&self, url: String) -> Result<()> {
        block_on(async move { Ok(self.inner.disconnect_relay(url).await?) })
    }

    pub fn connect(&self) {
        block_on(async move { self.inner.connect().await })
    }

    pub fn disconnect(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.disconnect().await?) })
    }

    pub fn subscribe(&self, filters: Vec<Arc<Filter>>) {
        let filters = filters
            .into_iter()
            .map(|f| f.as_ref().deref().clone())
            .collect();
        block_on(async move { self.inner.subscribe(filters).await })
    }

    // TODO: add subscribe_with_custom_wait

    pub fn unsubscribe(&self) {
        block_on(async move { self.inner.unsubscribe().await })
    }

    // TODO: add unsubscribe_with_custom_wait

    pub fn get_events_of(
        &self,
        filters: Vec<Arc<Filter>>,
        timeout: Option<Duration>,
    ) -> Result<Vec<Arc<Event>>> {
        let filters = filters
            .into_iter()
            .map(|f| f.as_ref().deref().clone())
            .collect();
        block_on(async move {
            Ok(self
                .inner
                .get_events_of(filters, timeout)
                .await?
                .into_iter()
                .map(|e| Arc::new(e.into()))
                .collect())
        })
    }

    // TODO: add get_events_of_with_opts

    /// Get events of filters from specific relays
    ///
    /// Get events both from **local database** and **relays**
    ///
    /// If no relay is specified, will be queried only the database.
    pub fn get_events_from(
        &self,
        urls: Vec<String>,
        filters: Vec<Arc<Filter>>,
        timeout: Option<Duration>,
    ) -> Result<Vec<Arc<Event>>> {
        let filters = filters
            .into_iter()
            .map(|f| f.as_ref().deref().clone())
            .collect();
        block_on(async move {
            Ok(self
                .inner
                .get_events_from(urls, filters, timeout)
                .await?
                .into_iter()
                .map(|e| Arc::new(e.into()))
                .collect())
        })
    }

    pub fn req_events_of(&self, filters: Vec<Arc<Filter>>, timeout: Option<Duration>) {
        let filters = filters
            .into_iter()
            .map(|f| f.as_ref().deref().clone())
            .collect();
        block_on(async move { self.inner.req_events_of(filters, timeout).await })
    }

    // TODO: add req_events_of_with_opts

    pub fn send_msg(&self, msg: ClientMessage) -> Result<()> {
        block_on(async move { Ok(self.inner.send_msg(msg.into()).await?) })
    }

    pub fn send_msg_to(&self, urls: Vec<String>, msg: ClientMessage) -> Result<()> {
        block_on(async move { Ok(self.inner.send_msg_to(urls, msg.into()).await?) })
    }

    pub fn send_event(&self, event: Arc<Event>) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .send_event(event.as_ref().deref().clone())
                    .await?
                    .into(),
            ))
        })
    }

    pub fn send_event_to(&self, urls: Vec<String>, event: Arc<Event>) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .send_event_to(urls, event.as_ref().deref().clone())
                    .await?
                    .into(),
            ))
        })
    }

    /// Signs the `EventBuilder` into an `Event` using the `NostrSigner`
    pub fn sign_event_builder(&self, builder: Arc<EventBuilder>) -> Result<Arc<Event>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .sign_event_builder(builder.as_ref().deref().clone())
                    .await?
                    .into(),
            ))
        })
    }

    /// Take an [`EventBuilder`], sign it by using the [`NostrSigner`] and broadcast to all relays.
    ///
    /// Rise an error if the [`NostrSigner`] is not set.
    pub fn send_event_builder(&self, builder: Arc<EventBuilder>) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .send_event_builder(builder.as_ref().deref().clone())
                    .await?
                    .into(),
            ))
        })
    }

    /// Take an [`EventBuilder`], sign it by using the [`NostrSigner`] and broadcast to specific relays.
    ///
    /// Rise an error if the [`NostrSigner`] is not set.
    pub fn send_event_builder_to(
        &self,
        urls: Vec<String>,
        builder: Arc<EventBuilder>,
    ) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .send_event_builder_to(urls, builder.as_ref().deref().clone())
                    .await?
                    .into(),
            ))
        })
    }

    pub fn set_metadata(&self, metadata: Arc<Metadata>) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .set_metadata(metadata.as_ref().deref())
                    .await?
                    .into(),
            ))
        })
    }

    pub fn send_direct_msg(
        &self,
        receiver: Arc<PublicKey>,
        msg: String,
        reply: Option<Arc<EventId>>,
    ) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .send_direct_msg(**receiver, msg, reply.map(|r| **r))
                    .await?
                    .into(),
            ))
        })
    }

    /// Repost
    pub fn repost(&self, event: Arc<Event>, relay_url: Option<String>) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .repost(event.as_ref().deref(), relay_url.map(UncheckedUrl::from))
                    .await?
                    .into(),
            ))
        })
    }

    /// Like event
    ///
    /// <https://github.com/nostr-protocol/nips/blob/master/25.md>
    pub fn like(&self, event: Arc<Event>) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner.like(event.as_ref().deref()).await?.into(),
            ))
        })
    }

    /// Disike event
    ///
    /// <https://github.com/nostr-protocol/nips/blob/master/25.md>
    pub fn dislike(&self, event: Arc<Event>) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner.dislike(event.as_ref().deref()).await?.into(),
            ))
        })
    }

    /// React to an [`Event`]
    ///
    /// <https://github.com/nostr-protocol/nips/blob/master/25.md>
    pub fn reaction(&self, event: Arc<Event>, reaction: String) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .reaction(event.as_ref().deref(), reaction)
                    .await?
                    .into(),
            ))
        })
    }

    /// Send a Zap!
    ///
    /// This method automatically create a split zap to support Rust Nostr development.
    pub fn zap(
        &self,
        to: Arc<ZapEntity>,
        satoshi: u64,
        details: Option<Arc<ZapDetails>>,
    ) -> Result<()> {
        block_on(async move {
            Ok(self
                .inner
                .zap(**to, satoshi, details.map(|d| d.as_ref().deref().clone()))
                .await?)
        })
    }

    /// Gift Wrap
    ///
    /// <https://github.com/nostr-protocol/nips/blob/master/59.md>
    pub fn gift_wrap(
        &self,
        receiver: Arc<PublicKey>,
        rumor: Arc<EventBuilder>,
        expiration: Option<Arc<Timestamp>>,
    ) -> Result<()> {
        block_on(async move {
            Ok(self
                .inner
                .gift_wrap(
                    **receiver,
                    rumor.as_ref().deref().clone(),
                    expiration.map(|t| **t),
                )
                .await?)
        })
    }

    /// Send GiftWrapper Sealed Direct message
    pub fn send_sealed_msg(
        &self,
        receiver: Arc<PublicKey>,
        message: String,
        expiration: Option<Arc<Timestamp>>,
    ) -> Result<()> {
        block_on(async move {
            Ok(self
                .inner
                .send_sealed_msg(**receiver, message, expiration.map(|t| **t))
                .await?)
        })
    }

    pub fn file_metadata(
        &self,
        description: String,
        metadata: Arc<FileMetadata>,
    ) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .file_metadata(description, metadata.as_ref().deref().clone())
                    .await?
                    .into(),
            ))
        })
    }

    pub fn reconcile(&self, filter: Arc<Filter>, opts: Arc<NegentropyOptions>) -> Result<()> {
        block_on(async move {
            Ok(self
                .inner
                .reconcile(filter.as_ref().deref().clone(), **opts)
                .await?)
        })
    }

    pub fn handle_notifications(
        self: Arc<Self>,
        handler: Box<dyn HandleNotification>,
    ) -> Result<()> {
        thread::spawn(async move {
            let handler = Arc::new(handler);
            self.inner
                .handle_notifications(|notification| async {
                    match notification {
                        RelayPoolNotificationSdk::Message { relay_url, message } => {
                            let h = handler.clone();
                            let _ = spawn_blocking(move || {
                                h.handle_msg(relay_url.to_string(), Arc::new(message.into()))
                            })
                            .await;
                        }
                        RelayPoolNotificationSdk::Event { relay_url, event } => {
                            let h = handler.clone();
                            let _ = spawn_blocking(move || {
                                h.handle(relay_url.to_string(), Arc::new(event.into()))
                            })
                            .await;
                        }
                        _ => (),
                    }
                    Ok(false)
                })
                .await
        })?;
        Ok(())
    }
}

#[uniffi::export(callback_interface)]
pub trait HandleNotification: Send + Sync + Debug {
    fn handle_msg(&self, relay_url: String, msg: Arc<RelayMessage>);
    fn handle(&self, relay_url: String, event: Arc<Event>);
}

// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: GPL-3.0-or-later WITH Classpath-exception-2.0

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Notification service implementation.

#![allow(unused)]

// TODO: remove allow(unused)
// TODO: try to remove code duplication

use crate::{
	error,
	protocol::notifications::handler::{
		NotificationsSink, NotificationsSinkMessage, ASYNC_NOTIFICATIONS_BUFFER_SIZE,
	},
	service::traits::{
		ClonableNotificationService, NotificationEvent, NotificationService, ValidationResult,
	},
	types::ProtocolName,
};

use futures::{
	stream::{FuturesUnordered, Stream},
	SinkExt, StreamExt,
};
use libp2p::PeerId;
use sc_utils::mpsc::{tracing_unbounded, TracingUnboundedReceiver, TracingUnboundedSender};
use tokio::sync::{mpsc, oneshot};

use sc_network_common::role::ObservedRole;

use std::{
	collections::HashMap,
	fmt::Debug,
	sync::{Arc, Mutex},
};

/// Inner notification event to deal with `NotificationsSinks` without exposing that
/// implementation detail to [`NotificationService`] consumers.
#[derive(Debug)]
enum InnerNotificationEvent {
	/// Validate inbound substream.
	ValidateInboundSubstream {
		/// Peer ID.
		peer: PeerId,

		/// Received handshake.
		handshake: Vec<u8>,

		/// `oneshot::Sender` for sending validation result back to `Notifications`
		result_tx: oneshot::Sender<ValidationResult>,
	},

	/// Remote identified by `PeerId` opened a substream and sent `Handshake`.
	/// Validate `Handshake` and report status (accept/reject) to `Notifications`.
	NotificationStreamOpened {
		/// Peer ID.
		peer: PeerId,

		/// Role of the peer.
		role: ObservedRole,

		/// Negotiated fallback.
		negotiated_fallback: Option<ProtocolName>,

		/// Notification sink.
		sink: NotificationsSink,
	},

	/// Substream was closed.
	NotificationStreamClosed {
		/// Peer Id.
		peer: PeerId,
	},

	/// Notification was received from the substream.
	NotificationReceived {
		/// Peer ID.
		peer: PeerId,

		/// Received notification.
		notification: Vec<u8>,
	},
}

/// Notification commands.
///
/// Sent by the installed protocols to `Notifications` to open/close/modify substreams.
pub enum NotificationCommand {
	/// Instruct `Notifications` to open a substream to peer.
	OpenSubstream(PeerId),

	/// Instruct `Notifications` to close the substream to peer.
	CloseSubstream(PeerId),

	/// Set handshake for the notifications protocol.
	SetHandshake(Vec<u8>),
}

/// Handle that is passed on to the notifications protocol.
#[derive(Debug)]
pub struct NotificationHandle {
	/// TX channel for sending commands to `Notifications`.
	tx: mpsc::Sender<NotificationCommand>,

	/// RX channel for receiving events from `Notifications`.
	rx: TracingUnboundedReceiver<InnerNotificationEvent>,

	/// Connected peers.
	peers: HashMap<PeerId, NotificationsSink>,
}

impl NotificationHandle {
	/// Create new [`NotificationHandle`].
	fn new(
		tx: mpsc::Sender<NotificationCommand>,
		rx: TracingUnboundedReceiver<InnerNotificationEvent>,
	) -> Self {
		Self { tx, rx, peers: HashMap::new() }
	}
}

#[async_trait::async_trait]
impl NotificationService for NotificationHandle {
	/// Instruct `Notifications` to open a new substream for `peer`.
	async fn open_substream(&mut self, _peer: PeerId) -> Result<(), ()> {
		todo!("support for opening substreams not implemented yet");
	}

	/// Instruct `Notifications` to close substream for `peer`.
	async fn close_substream(&mut self, _peer: PeerId) -> Result<(), ()> {
		todo!("support for closing substreams not implemented yet");
	}

	/// Send synchronous `notification` to `peer`.
	fn send_sync_notification(
		&self,
		peer: &PeerId,
		notification: Vec<u8>,
	) -> Result<(), error::Error> {
		self.peers
			.get(&peer)
			.ok_or_else(|| error::Error::PeerDoesntExist(*peer))?
			.send_sync_notification(notification);
		Ok(())
	}

	/// Send asynchronous `notification` to `peer`, allowing sender to exercise backpressure.
	async fn send_async_notification(
		&self,
		peer: &PeerId,
		notification: Vec<u8>,
	) -> Result<(), error::Error> {
		self.peers
			.get(&peer)
			// TODO: check what the current implementation does in case the peer doesn't exist
			.ok_or_else(|| error::Error::PeerDoesntExist(*peer))?
			.reserve_notification()
			.await
			.map_err(|_| error::Error::ConnectionClosed)?
			.send(notification)
			.map_err(|_| error::Error::ChannelClosed)
	}

	/// Set handshake for the notification protocol replacing the old handshake.
	async fn set_hanshake(&mut self, handshake: Vec<u8>) -> Result<(), ()> {
		self.tx.send(NotificationCommand::SetHandshake(handshake)).await.map_err(|_| ())
	}

	/// Get next event from the `Notifications` event stream.
	async fn next_event(&mut self) -> Option<NotificationEvent> {
		match self.rx.next().await? {
			InnerNotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx } =>
				Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }),
			InnerNotificationEvent::NotificationStreamOpened {
				peer,
				role,
				negotiated_fallback,
				sink,
			} => {
				self.peers.insert(peer, sink);
				Some(NotificationEvent::NotificationStreamOpened {
					peer,
					role,
					negotiated_fallback,
				})
			},
			InnerNotificationEvent::NotificationStreamClosed { peer } => {
				self.peers.remove(&peer);
				Some(NotificationEvent::NotificationStreamClosed { peer })
			},
			InnerNotificationEvent::NotificationReceived { peer, notification } =>
				Some(NotificationEvent::NotificationReceived { peer, notification }),
		}
	}
}

/// Handle that is passed on to the notifications protocol.
#[derive(Debug)]
pub struct ClonableNotificationHandle {
	/// TX channel for sending commands to `Notifications`.
	tx: mpsc::Sender<NotificationCommand>,

	/// RX channel for receiving events from `Notifications`.
	rx: TracingUnboundedReceiver<InnerNotificationEvent>,

	/// All subscribers of `NotificationEvent`s.
	subscribers: Arc<Mutex<Vec<TracingUnboundedSender<InnerNotificationEvent>>>>,

	/// Connected peers.
	peers: HashMap<PeerId, NotificationsSink>,
}

impl ClonableNotificationHandle {
	/// Create new [`ClonableNotificationHandle`].
	fn new(
		tx: mpsc::Sender<NotificationCommand>,
		rx: TracingUnboundedReceiver<InnerNotificationEvent>,
		subscribers: Arc<Mutex<Vec<TracingUnboundedSender<InnerNotificationEvent>>>>,
	) -> Self {
		Self { tx, rx, subscribers, peers: HashMap::new() }
	}
}

#[async_trait::async_trait]
impl NotificationService for ClonableNotificationHandle {
	/// Instruct `Notifications` to open a new substream for `peer`.
	async fn open_substream(&mut self, _peer: PeerId) -> Result<(), ()> {
		todo!("support for opening substreams not implemented yet");
	}

	/// Instruct `Notifications` to close substream for `peer`.
	async fn close_substream(&mut self, _peer: PeerId) -> Result<(), ()> {
		todo!("support for closing substreams not implemented yet");
	}

	/// Send synchronous `notification` to `peer`.
	fn send_sync_notification(
		&self,
		peer: &PeerId,
		notification: Vec<u8>,
	) -> Result<(), error::Error> {
		self.peers
			.get(&peer)
			.ok_or_else(|| error::Error::PeerDoesntExist(*peer))?
			.send_sync_notification(notification);
		Ok(())
	}

	/// Send asynchronous `notification` to `peer`, allowing sender to exercise backpressure.
	async fn send_async_notification(
		&self,
		peer: &PeerId,
		notification: Vec<u8>,
	) -> Result<(), error::Error> {
		self.peers
			.get(&peer)
			// TODO: check what the current implementation does in case the peer doesn't exist
			.ok_or_else(|| error::Error::PeerDoesntExist(*peer))?
			.reserve_notification()
			.await
			.map_err(|_| error::Error::ConnectionClosed)?
			.send(notification)
			.map_err(|_| error::Error::ChannelClosed)
	}

	/// Set handshake for the notification protocol replacing the old handshake.
	async fn set_hanshake(&mut self, handshake: Vec<u8>) -> Result<(), ()> {
		self.tx.send(NotificationCommand::SetHandshake(handshake)).await.map_err(|_| ())
	}

	/// Get next event from the `Notifications` event stream.
	async fn next_event(&mut self) -> Option<NotificationEvent> {
		match self.rx.next().await? {
			InnerNotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx } =>
				Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }),
			InnerNotificationEvent::NotificationStreamOpened {
				peer,
				role,
				negotiated_fallback,
				sink,
			} => {
				self.peers.insert(peer, sink);
				Some(NotificationEvent::NotificationStreamOpened {
					peer,
					role,
					negotiated_fallback,
				})
			},
			InnerNotificationEvent::NotificationStreamClosed { peer } => {
				self.peers.remove(&peer);
				Some(NotificationEvent::NotificationStreamClosed { peer })
			},
			InnerNotificationEvent::NotificationReceived { peer, notification } =>
				Some(NotificationEvent::NotificationReceived { peer, notification }),
		}
	}
}

#[async_trait::async_trait]
impl ClonableNotificationService for ClonableNotificationHandle {
	fn clone(&mut self) -> Box<dyn ClonableNotificationService> {
		let mut subscribers = self.subscribers.lock().expect("lock to not be poisoned");
		let (event_tx, event_rx) =
			tracing_unbounded("mpsc-notification-to-protocol-clonable", 100_000);
		subscribers.push(event_tx);
		drop(subscribers);

		let subscribers = self.subscribers.clone();
		let cmd_tx = self.tx.clone();
		let peers = self.peers.clone();

		Box::new(ClonableNotificationHandle { tx: cmd_tx, rx: event_rx, peers, subscribers })
	}
}

/// Type describing how many subscriber the `NotificationService` has.
#[derive(Debug)]
enum SubscriberType {
	/// Single subscriber for notifications events.
	Single(TracingUnboundedSender<InnerNotificationEvent>),

	/// Multiple subscriber for notification events.
	Multi(Arc<Mutex<Vec<TracingUnboundedSender<InnerNotificationEvent>>>>),
}

/// Channel pair which allows `Notifications` to interact with a protocol.
#[derive(Debug)]
pub struct ProtocolHandlePair(SubscriberType, mpsc::Receiver<NotificationCommand>);

impl ProtocolHandlePair {
	/// Create new [`ProtocolHandlePair`].
	fn new(subscriber_type: SubscriberType, rx: mpsc::Receiver<NotificationCommand>) -> Self {
		Self(subscriber_type, rx)
	}

	/// Consume `self` and split [`ProtocolHandlePair`] into a handle which allows it to send events
	/// to the protocol into a stream of commands received from the protocol.
	pub fn split(
		self,
	) -> (ProtocolHandle, Box<dyn Stream<Item = NotificationCommand> + Send + Unpin>) {
		(ProtocolHandle::new(self.0), Box::new(tokio_stream::wrappers::ReceiverStream::new(self.1)))
	}
}

/// Handle that is passed on to `Notifications` and allows it to directly communicate
/// with the protocol.
#[derive(Debug)]
pub struct ProtocolHandle {
	/// TX channel for sending events to protocol.
	subscriber_type: SubscriberType,
}

impl ProtocolHandle {
	fn new(subscriber_type: SubscriberType) -> Self {
		Self { subscriber_type }
	}

	/// Report to the protocol that a substream has been opened and it must be validated by the
	/// protocol.
	///
	/// Return `oneshot::Receiver` which allows `Notifications` to poll for the validation result
	/// from protocol.
	pub fn report_incoming_substream(
		&self,
		peer: PeerId,
		handshake: Vec<u8>,
	) -> Result<oneshot::Receiver<ValidationResult>, ()> {
		match &self.subscriber_type {
			SubscriberType::Single(subscriber) => {
				let (result_tx, rx) = oneshot::channel();
				return subscriber
					.unbounded_send(InnerNotificationEvent::ValidateInboundSubstream {
						peer,
						handshake,
						result_tx,
					})
					.map(|_| rx)
					.map_err(|_| ())
			},
			SubscriberType::Multi(subscribers) => {
				let mut results: FuturesUnordered<_> = subscribers
					.lock()
					.expect("mutex to be unpoisoned")
					.iter()
					.map(|subscriber| {
						let (result_tx, rx) = oneshot::channel();

						let _ = subscriber
							.unbounded_send(InnerNotificationEvent::ValidateInboundSubstream {
								peer,
								handshake: handshake.clone(),
								result_tx,
							})
							.map_err(|_| ());
						rx
					})
					.collect();

				let (tx, rx) = oneshot::channel();
				tokio::spawn(async move {
					while let Some(event) = results.next().await {
						match event {
							Err(_) | Ok(ValidationResult::Reject) =>
								return tx.send(ValidationResult::Reject),
							Ok(ValidationResult::Accept) => {},
						}
					}

					return tx.send(ValidationResult::Accept)
				});

				Ok(rx)
			},
		}
	}

	/// Report to the protocol that a substream has been opened and that it can now use the handle
	/// to send notifications to the remote peer.
	pub fn report_substream_opened(
		&self,
		peer: PeerId,
		role: ObservedRole,
		negotiated_fallback: Option<ProtocolName>,
		sink: NotificationsSink,
	) -> Result<(), ()> {
		match &self.subscriber_type {
			SubscriberType::Single(subscriber) => subscriber
				.unbounded_send(InnerNotificationEvent::NotificationStreamOpened {
					peer,
					role,
					negotiated_fallback,
					sink,
				})
				.map_err(|_| ()),
			SubscriberType::Multi(subscribers) => {
				subscribers
					.lock()
					.expect("mutex to be unpoisoned")
					.iter()
					.for_each(|subscriber| {
						let _ = subscriber
							.unbounded_send(InnerNotificationEvent::NotificationStreamOpened {
								peer,
								role: role.clone(),
								negotiated_fallback: negotiated_fallback.clone(),
								sink: sink.clone(),
							})
							.map_err(|_| ());
					});
				// TODO: process results
				Ok(())
			},
		}
	}

	/// Substream was closed.
	pub fn report_substream_closed(&mut self, peer: PeerId) -> Result<(), ()> {
		match &self.subscriber_type {
			SubscriberType::Single(subscriber) => subscriber
				.unbounded_send(InnerNotificationEvent::NotificationStreamClosed { peer })
				.map_err(|_| ()),
			SubscriberType::Multi(subscribers) => {
				subscribers
					.lock()
					.expect("mutex to be unpoisoned")
					.iter()
					.for_each(|subscriber| {
						let _ = subscriber
							.unbounded_send(InnerNotificationEvent::NotificationStreamClosed {
								peer,
							})
							.map_err(|_| ());
					});
				// TODO: process results
				Ok(())
			},
		}
	}

	/// Notification was received from the substream.
	pub fn report_notification_received(
		&mut self,
		peer: PeerId,
		notification: Vec<u8>,
	) -> Result<(), ()> {
		match &self.subscriber_type {
			SubscriberType::Single(subscriber) => subscriber
				.unbounded_send(InnerNotificationEvent::NotificationReceived { peer, notification })
				.map_err(|_| ()),
			SubscriberType::Multi(subscribers) => {
				subscribers
					.lock()
					.expect("mutex to be unpoisoned")
					.iter()
					.for_each(|subscriber| {
						let _ = subscriber
							.unbounded_send(InnerNotificationEvent::NotificationReceived {
								peer,
								notification: notification.clone(),
							})
							.map_err(|_| ());
					});
				// TODO: process results
				Ok(())
			},
		}
	}
}

/// Create new (protocol, notification) handle pair.
///
/// Handle pair allows `Notifications` and the protocol to communicate with each other directly.
pub fn notification_service() -> (ProtocolHandlePair, Box<dyn NotificationService>) {
	let (cmd_tx, cmd_rx) = mpsc::channel(64); // TODO: zzz
	let (event_tx, event_rx) = tracing_unbounded("mpsc-notification-to-protocol", 100_000);

	(
		ProtocolHandlePair::new(SubscriberType::Single(event_tx), cmd_rx),
		Box::new(NotificationHandle::new(cmd_tx, event_rx)),
	)
}

/// Create new (protocol, notification) handle pair.
///
/// Handle pair allows `Notifications` and the protocol to communicate with each other directly.
///
/// This version of the pair is clonable meaning the protocol that has created the pair, can clone
/// the resulting `NotificationService` and interact with `Notificatoins` through any of the clones
/// and receive the events `Notifications` sends to each copy of the service.
pub fn clonable_notification_service() -> (ProtocolHandlePair, Box<dyn ClonableNotificationService>)
{
	let (cmd_tx, cmd_rx) = mpsc::channel(64); // TODO: zzz
	let (event_tx, event_rx) = tracing_unbounded("mpsc-notification-to-protocol-clonable", 100_000);
	let subscribers = Arc::new(Mutex::new(vec![event_tx]));

	(
		ProtocolHandlePair::new(SubscriberType::Multi(subscribers.clone()), cmd_rx),
		Box::new(ClonableNotificationHandle::new(cmd_tx, event_rx, subscribers)),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use futures::prelude::*;

	#[tokio::test]
	async fn validate_and_accept_substream() {
		let (proto, mut notif) = notification_service();
		let (mut handle, stream) = proto.split();

		let peer_id = PeerId::random();
		let rx = handle.report_incoming_substream(peer_id, vec![1, 3, 3, 7]).unwrap();

		if let Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }) =
			notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(handshake, vec![1, 3, 3, 7]);
			let _ = result_tx.send(ValidationResult::Accept).unwrap();
		} else {
			panic!("invalid event received");
		}

		assert_eq!(rx.await.unwrap(), ValidationResult::Accept);
	}

	#[tokio::test]
	async fn substream_opened() {
		let (proto, mut notif) = notification_service();
		let (sink, _, _) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();

		let peer_id = PeerId::random();
		handle.report_substream_opened(peer_id, ObservedRole::Full, None, sink).unwrap();

		if let Some(NotificationEvent::NotificationStreamOpened {
			peer,
			role,
			negotiated_fallback,
		}) = notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(negotiated_fallback, None);
			assert_eq!(role, ObservedRole::Full);
		} else {
			panic!("invalid event received");
		}
	}

	#[tokio::test]
	async fn send_sync_notification() {
		let (proto, mut notif) = notification_service();
		let (sink, _, mut sync_rx) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();
		let peer_id = PeerId::random();

		// validate inbound substream
		let result_rx = handle.report_incoming_substream(peer_id, vec![1, 3, 3, 7]).unwrap();

		if let Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }) =
			notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(handshake, vec![1, 3, 3, 7]);
			let _ = result_tx.send(ValidationResult::Accept).unwrap();
		} else {
			panic!("invalid event received");
		}
		assert_eq!(result_rx.await.unwrap(), ValidationResult::Accept);

		// report that a substream has been opened
		handle.report_substream_opened(peer_id, ObservedRole::Full, None, sink).unwrap();

		if let Some(NotificationEvent::NotificationStreamOpened {
			peer,
			role,
			negotiated_fallback,
		}) = notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(negotiated_fallback, None);
			assert_eq!(role, ObservedRole::Full);
		} else {
			panic!("invalid event received");
		}

		notif.send_sync_notification(&peer_id, vec![1, 3, 3, 8]);
		assert_eq!(
			sync_rx.next().await,
			Some(NotificationsSinkMessage::Notification { message: vec![1, 3, 3, 8] })
		);
	}

	#[tokio::test]
	async fn send_async_notification() {
		let (proto, mut notif) = notification_service();
		let (sink, mut async_rx, _) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();
		let peer_id = PeerId::random();

		// validate inbound substream
		let result_rx = handle.report_incoming_substream(peer_id, vec![1, 3, 3, 7]).unwrap();

		if let Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }) =
			notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(handshake, vec![1, 3, 3, 7]);
			let _ = result_tx.send(ValidationResult::Accept).unwrap();
		} else {
			panic!("invalid event received");
		}
		assert_eq!(result_rx.await.unwrap(), ValidationResult::Accept);

		// report that a substream has been opened
		handle.report_substream_opened(peer_id, ObservedRole::Full, None, sink).unwrap();

		if let Some(NotificationEvent::NotificationStreamOpened {
			peer,
			role,
			negotiated_fallback,
		}) = notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(negotiated_fallback, None);
			assert_eq!(role, ObservedRole::Full);
		} else {
			panic!("invalid event received");
		}

		notif.send_async_notification(&peer_id, vec![1, 3, 3, 9]).await.unwrap();
		assert_eq!(
			async_rx.next().await,
			Some(NotificationsSinkMessage::Notification { message: vec![1, 3, 3, 9] })
		);
	}

	#[tokio::test]
	async fn send_sync_notification_to_non_existent_peer() {
		let (proto, mut notif) = notification_service();
		let (sink, _, mut sync_rx) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();
		let peer = PeerId::random();

		if let Err(error::Error::PeerDoesntExist(peer_id)) =
			notif.send_sync_notification(&peer, vec![1, 3, 3, 7])
		{
			assert_eq!(peer, peer_id);
		} else {
			panic!("invalid error received from `send_sync_notification()`");
		}
	}

	#[tokio::test]
	async fn send_async_notification_to_non_existent_peer() {
		let (proto, mut notif) = notification_service();
		let (sink, _, mut sync_rx) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();
		let peer = PeerId::random();

		if let Err(error::Error::PeerDoesntExist(peer_id)) =
			notif.send_async_notification(&peer, vec![1, 3, 3, 7]).await
		{
			assert_eq!(peer, peer_id);
		} else {
			panic!("invalid error received from `send_sync_notification()`");
		}
	}

	#[tokio::test]
	async fn receive_notification() {
		let (proto, mut notif) = notification_service();
		let (sink, _, mut sync_rx) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();
		let peer_id = PeerId::random();

		// validate inbound substream
		let result_rx = handle.report_incoming_substream(peer_id, vec![1, 3, 3, 7]).unwrap();

		if let Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }) =
			notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(handshake, vec![1, 3, 3, 7]);
			let _ = result_tx.send(ValidationResult::Accept).unwrap();
		} else {
			panic!("invalid event received");
		}
		assert_eq!(result_rx.await.unwrap(), ValidationResult::Accept);

		// report that a substream has been opened
		handle.report_substream_opened(peer_id, ObservedRole::Full, None, sink).unwrap();

		if let Some(NotificationEvent::NotificationStreamOpened {
			peer,
			role,
			negotiated_fallback,
		}) = notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(negotiated_fallback, None);
			assert_eq!(role, ObservedRole::Full);
		} else {
			panic!("invalid event received");
		}

		// notification is received
		handle.report_notification_received(peer_id, vec![1, 3, 3, 8]).unwrap();

		if let Some(NotificationEvent::NotificationReceived { peer, notification }) =
			notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(notification, vec![1, 3, 3, 8]);
		} else {
			panic!("invalid event received");
		}
	}

	#[tokio::test]
	async fn backpressure_works() {
		let (proto, mut notif) = notification_service();
		let (sink, mut async_rx, _) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();
		let peer_id = PeerId::random();

		// validate inbound substream
		let result_rx = handle.report_incoming_substream(peer_id, vec![1, 3, 3, 7]).unwrap();

		if let Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }) =
			notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(handshake, vec![1, 3, 3, 7]);
			let _ = result_tx.send(ValidationResult::Accept).unwrap();
		} else {
			panic!("invalid event received");
		}
		assert_eq!(result_rx.await.unwrap(), ValidationResult::Accept);

		// report that a substream has been opened
		handle.report_substream_opened(peer_id, ObservedRole::Full, None, sink).unwrap();

		if let Some(NotificationEvent::NotificationStreamOpened {
			peer,
			role,
			negotiated_fallback,
		}) = notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(negotiated_fallback, None);
			assert_eq!(role, ObservedRole::Full);
		} else {
			panic!("invalid event received");
		}

		// fill the message buffer with messages
		for i in 0..=ASYNC_NOTIFICATIONS_BUFFER_SIZE {
			assert!(
				futures::poll!(notif.send_async_notification(&peer_id, vec![1, 3, 3, i as u8]))
					.is_ready()
			);
		}

		// try to send one more message and verify that the call blocks
		assert!(
			futures::poll!(notif.send_async_notification(&peer_id, vec![1, 3, 3, 9])).is_pending()
		);

		// release one slot from the buffer for new message
		assert_eq!(
			async_rx.next().await,
			Some(NotificationsSinkMessage::Notification { message: vec![1, 3, 3, 0] })
		);

		// verify that a message can be sent
		assert!(
			futures::poll!(notif.send_async_notification(&peer_id, vec![1, 3, 3, 9])).is_ready()
		);
	}

	#[tokio::test]
	async fn peer_disconnects_then_sync_notification_is_sent() {
		let (proto, mut notif) = notification_service();
		let (sink, _, mut sync_rx) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();
		let peer_id = PeerId::random();

		// validate inbound substream
		let result_rx = handle.report_incoming_substream(peer_id, vec![1, 3, 3, 7]).unwrap();

		if let Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }) =
			notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(handshake, vec![1, 3, 3, 7]);
			let _ = result_tx.send(ValidationResult::Accept).unwrap();
		} else {
			panic!("invalid event received");
		}
		assert_eq!(result_rx.await.unwrap(), ValidationResult::Accept);

		// report that a substream has been opened
		handle.report_substream_opened(peer_id, ObservedRole::Full, None, sink).unwrap();

		if let Some(NotificationEvent::NotificationStreamOpened {
			peer,
			role,
			negotiated_fallback,
		}) = notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(negotiated_fallback, None);
			assert_eq!(role, ObservedRole::Full);
		} else {
			panic!("invalid event received");
		}

		// report that a substream has been closed but don't poll `notif` to receive this
		// information
		handle.report_substream_closed(peer_id).unwrap();
		drop(sync_rx);

		// as per documentation, error is not reported but the notification is silently dropped
		assert!(notif.send_sync_notification(&peer_id, vec![1, 3, 3, 7]).is_ok());
	}

	#[tokio::test]
	async fn peer_disconnects_then_async_notification_is_sent() {
		let (proto, mut notif) = notification_service();
		let (sink, mut async_rx, _) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();
		let peer_id = PeerId::random();

		// validate inbound substream
		let result_rx = handle.report_incoming_substream(peer_id, vec![1, 3, 3, 7]).unwrap();

		if let Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }) =
			notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(handshake, vec![1, 3, 3, 7]);
			let _ = result_tx.send(ValidationResult::Accept).unwrap();
		} else {
			panic!("invalid event received");
		}
		assert_eq!(result_rx.await.unwrap(), ValidationResult::Accept);

		// report that a substream has been opened
		handle.report_substream_opened(peer_id, ObservedRole::Full, None, sink).unwrap();

		if let Some(NotificationEvent::NotificationStreamOpened {
			peer,
			role,
			negotiated_fallback,
		}) = notif.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(negotiated_fallback, None);
			assert_eq!(role, ObservedRole::Full);
		} else {
			panic!("invalid event received");
		}

		// report that a substream has been closed but don't poll `notif` to receive this
		// information
		handle.report_substream_closed(peer_id).unwrap();
		drop(async_rx);

		// as per documentation, error is not reported but the notification is silently dropped
		if let Err(error::Error::ConnectionClosed) =
			notif.send_async_notification(&peer_id, vec![1, 3, 3, 7]).await
		{
		} else {
			panic!("invalid state after calling `send_async_notificatio()` on closed connection")
		}
	}

	#[tokio::test]
	async fn cloned_service_opening_substream_works() {
		let (proto, mut notif1) = clonable_notification_service();
		let (sink, mut async_rx, _) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();
		let mut notif2 = notif1.clone();
		let peer_id = PeerId::random();

		// validate inbound substream
		let mut result_rx = handle.report_incoming_substream(peer_id, vec![1, 3, 3, 7]).unwrap();

		// verify that `stream1` also gets the event
		if let Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }) =
			notif1.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(handshake, vec![1, 3, 3, 7]);
			let _ = result_tx.send(ValidationResult::Accept).unwrap();
		} else {
			panic!("invalid event received");
		}

		// verify that because only one listener has thus far send their result, the result is
		// pending
		assert!(result_rx.try_recv().is_err());

		// verify that `stream2` also gets the event
		if let Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }) =
			notif2.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(handshake, vec![1, 3, 3, 7]);
			result_tx.send(ValidationResult::Accept);
		} else {
			panic!("invalid event received");
		}

		assert_eq!(result_rx.await.unwrap(), ValidationResult::Accept);
	}

	#[tokio::test]
	async fn cloned_service_one_service_rejects_substream() {
		let (proto, mut notif1) = clonable_notification_service();
		let (sink, mut async_rx, _) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();
		let mut notif2 = notif1.clone();
		let mut notif3 = notif2.clone();
		let peer_id = PeerId::random();

		// validate inbound substream
		let mut result_rx = handle.report_incoming_substream(peer_id, vec![1, 3, 3, 7]).unwrap();

		for notif in vec![&mut notif1, &mut notif2] {
			if let Some(NotificationEvent::ValidateInboundSubstream {
				peer,
				handshake,
				result_tx,
			}) = notif.next_event().await
			{
				assert_eq!(peer_id, peer);
				assert_eq!(handshake, vec![1, 3, 3, 7]);
				let _ = result_tx.send(ValidationResult::Accept).unwrap();
			} else {
				panic!("invalid event received");
			}
		}

		// `notif3` has not yet sent their validation result
		assert!(result_rx.try_recv().is_err());

		if let Some(NotificationEvent::ValidateInboundSubstream { peer, handshake, result_tx }) =
			notif3.next_event().await
		{
			assert_eq!(peer_id, peer);
			assert_eq!(handshake, vec![1, 3, 3, 7]);
			let _ = result_tx.send(ValidationResult::Reject).unwrap();
		} else {
			panic!("invalid event received");
		}
		assert_eq!(result_rx.await.unwrap(), ValidationResult::Reject);
	}

	#[tokio::test]
	async fn cloned_service_opening_substream_sending_and_receiving_notifications_work() {
		let (proto, mut notif1) = clonable_notification_service();
		let (sink, _, mut sync_rx) = NotificationsSink::new(PeerId::random());
		let (mut handle, stream) = proto.split();
		let mut notif2 = notif1.clone();
		let mut notif3 = notif1.clone();
		let peer_id = PeerId::random();

		// validate inbound substream
		let mut result_rx = handle.report_incoming_substream(peer_id, vec![1, 3, 3, 7]).unwrap();

		for notif in vec![&mut notif1, &mut notif2, &mut notif3] {
			// accept the inbound substream for all services
			if let Some(NotificationEvent::ValidateInboundSubstream {
				peer,
				handshake,
				result_tx,
			}) = notif.next_event().await
			{
				assert_eq!(peer_id, peer);
				assert_eq!(handshake, vec![1, 3, 3, 7]);
				let _ = result_tx.send(ValidationResult::Accept).unwrap();
			} else {
				panic!("invalid event received");
			}
		}
		assert_eq!(result_rx.await.unwrap(), ValidationResult::Accept);

		// report that then notification stream has been opened
		handle.report_substream_opened(peer_id, ObservedRole::Full, None, sink).unwrap();

		for notif in vec![&mut notif1, &mut notif2, &mut notif3] {
			if let Some(NotificationEvent::NotificationStreamOpened {
				peer,
				role,
				negotiated_fallback,
			}) = notif.next_event().await
			{
				assert_eq!(peer_id, peer);
				assert_eq!(negotiated_fallback, None);
				assert_eq!(role, ObservedRole::Full);
			} else {
				panic!("invalid event received");
			}
		}
		// receive a notification from peer and verify all services receive it
		handle.report_notification_received(peer_id, vec![1, 3, 3, 8]).unwrap();

		for notif in vec![&mut notif1, &mut notif2, &mut notif3] {
			if let Some(NotificationEvent::NotificationReceived { peer, notification }) =
				notif.next_event().await
			{
				assert_eq!(peer_id, peer);
				assert_eq!(notification, vec![1, 3, 3, 8]);
			} else {
				panic!("invalid event received");
			}
		}

		for (i, notif) in vec![&mut notif1, &mut notif2, &mut notif3].iter().enumerate() {
			// send notification from each service and verify peer receives it
			notif.send_sync_notification(&peer_id, vec![1, 3, 3, i as u8]);
			assert_eq!(
				sync_rx.next().await,
				Some(NotificationsSinkMessage::Notification { message: vec![1, 3, 3, i as u8] })
			);
		}

		// close the substream for peer and verify all services receive the event
		handle.report_substream_closed(peer_id).unwrap();

		for notif in vec![&mut notif1, &mut notif2, &mut notif3] {
			if let Some(NotificationEvent::NotificationStreamClosed { peer }) =
				notif.next_event().await
			{
				assert_eq!(peer_id, peer);
			} else {
				panic!("invalid event received");
			}
		}
	}
}

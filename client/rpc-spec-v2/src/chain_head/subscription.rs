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

//! Subscription management for tracking subscription IDs to pinned blocks.

use futures::channel::oneshot;
use parking_lot::RwLock;
use sc_client_api::Backend;
use sp_blockchain::Error;
use sp_runtime::traits::Block as BlockT;
use std::{
	collections::{hash_map::Entry, HashMap, HashSet},
	sync::Arc,
};

/// Subscription management error.
#[derive(Debug)]
pub enum SubscriptionManagementError {
	/// The block cannot be pinned into memory because
	/// the subscription has exceeded the maximum number
	/// of blocks pinned.
	ExceededLimits,
	/// Error originated from the blockchain (client or backend).
	Blockchain(Error),
	/// The database does not contain a block number.
	BlockNumberAbsent,
	/// The database does not contain a block hash.
	BlockHashAbsent,
	/// Custom error.
	Custom(String),
}

impl From<Error> for SubscriptionManagementError {
	fn from(err: Error) -> Self {
		SubscriptionManagementError::Blockchain(err)
	}
}

/// Inner subscription data structure.
struct SubscriptionInner<Block: BlockT> {
	/// The `runtime_updates` parameter flag of the subscription.
	runtime_updates: bool,
	/// Signals the "Stop" event.
	tx_stop: Option<oneshot::Sender<()>>,
	/// The blocks pinned.
	blocks: HashSet<Block::Hash>,
	/// The maximum number of pinned blocks allowed per subscription.
	max_pinned_blocks: usize,
}

/// Manage the blocks of a specific subscription ID.
#[derive(Clone)]
pub struct SubscriptionHandle<Block: BlockT, BE: Backend<Block> + 'static> {
	inner: Arc<RwLock<SubscriptionInner<Block>>>,
	/// Backend pinning / unpinning blocks.
	///
	/// The `Arc` is handled one level-above, but substrate exposes the backend as Arc<T>.
	backend: Arc<BE>,
}

impl<Block: BlockT, BE: Backend<Block> + 'static> SubscriptionHandle<Block, BE> {
	/// Construct a new [`SubscriptionHandle`].
	fn new(
		runtime_updates: bool,
		tx_stop: oneshot::Sender<()>,
		max_pinned_blocks: usize,
		backend: Arc<BE>,
	) -> Self {
		SubscriptionHandle {
			inner: Arc::new(RwLock::new(SubscriptionInner {
				runtime_updates,
				tx_stop: Some(tx_stop),
				blocks: HashSet::new(),
				max_pinned_blocks,
			})),
			backend,
		}
	}

	/// Trigger the stop event for the current subscription.
	///
	/// This can happen on internal failure (ie, the pruning deleted the block from memory)
	/// or if the user exceeded the amount of available pinned blocks.
	pub fn stop(&self) {
		let mut inner = self.inner.write();

		if let Some(tx_stop) = inner.tx_stop.take() {
			let _ = tx_stop.send(());
		}
	}

	/// Pin a new block for the current subscription ID.
	///
	/// Returns whether the value was newly inserted if the block can be pinned.
	/// Otherwise, returns an error if the maximum number of blocks has been exceeded.
	pub fn pin_block(&self, hash: Block::Hash) -> Result<bool, SubscriptionManagementError> {
		let mut inner = self.inner.write();

		if inner.blocks.len() == inner.max_pinned_blocks {
			// We reached the limit. However, the block could already be pinned.

			if inner.blocks.contains(&hash) {
				return Ok(false)
			} else {
				return Err(SubscriptionManagementError::ExceededLimits)
			}
		}

		self.backend
			.pin_block(hash)
			.map_err(|err| SubscriptionManagementError::Custom(err.to_string()))?;
		Ok(inner.blocks.insert(hash))
	}

	/// Unpin a new block for the current subscription ID.
	///
	/// Returns whether the value was present in the set.
	pub fn unpin_block(&self, hash: &Block::Hash) -> bool {
		let mut inner = self.inner.write();

		if inner.blocks.remove(hash) {
			// Unpin the block if it was previously pinned.
			self.backend.unpin_block(*hash);
			true
		} else {
			false
		}
	}

	/// Check if the block hash is present for the provided subscription ID.
	///
	/// Returns `true` if the set contains the block.
	pub fn contains_block(&self, hash: &Block::Hash) -> bool {
		let inner = self.inner.read();
		inner.blocks.contains(hash)
	}

	/// Get the `runtime_updates` flag of this subscription.
	pub fn has_runtime_updates(&self) -> bool {
		let inner = self.inner.read();
		inner.runtime_updates
	}
}

impl<Block: BlockT, BE: Backend<Block> + 'static> Drop for SubscriptionHandle<Block, BE> {
	fn drop(&mut self) {
		let mut inner = self.inner.write();

		// Unpin any remaining blocks of this subscription.
		for hash in inner.blocks.drain() {
			self.backend.unpin_block(hash);
		}
	}
}

/// Manage block pinning / unpinning for subscription IDs.
pub struct SubscriptionManagement<Block: BlockT, BE: Backend<Block> + 'static> {
	/// Manage subscription by mapping the subscription ID
	/// to a set of block hashes.
	inner: RwLock<HashMap<String, Arc<SubscriptionHandle<Block, BE>>>>,
}

impl<Block: BlockT, BE: Backend<Block> + 'static> SubscriptionManagement<Block, BE> {
	/// Construct a new [`SubscriptionManagement`].
	pub fn new() -> Self {
		SubscriptionManagement { inner: RwLock::new(HashMap::new()) }
	}

	/// Insert a new subscription ID.
	///
	/// If the subscription was not previously inserted, the method returns a tuple of
	/// the receiver that is triggered upon the "Stop" event and the subscription
	/// handle. Otherwise, when the subscription ID was already inserted returns none.
	pub fn insert_subscription(
		&self,
		subscription_id: String,
		runtime_updates: bool,
		max_pinned_blocks: usize,
		backend: Arc<BE>,
	) -> Option<(oneshot::Receiver<()>, Arc<SubscriptionHandle<Block, BE>>)> {
		let mut subs = self.inner.write();

		if let Entry::Vacant(entry) = subs.entry(subscription_id) {
			let (tx_stop, rx_stop) = oneshot::channel();
			let handle = Arc::new(SubscriptionHandle::<Block, BE>::new(
				runtime_updates,
				tx_stop,
				max_pinned_blocks,
				backend,
			));
			entry.insert(handle.clone());
			Some((rx_stop, handle))
		} else {
			None
		}
	}

	/// Remove the subscription ID with associated pinned blocks.
	pub fn remove_subscription(&self, subscription_id: &String) {
		let mut subs = self.inner.write();
		subs.remove(subscription_id);
	}

	/// Obtain the specific subscription handle.
	pub fn get_subscription(
		&self,
		subscription_id: &String,
	) -> Option<Arc<SubscriptionHandle<Block, BE>>> {
		let subs = self.inner.write();
		subs.get(subscription_id).and_then(|handle| Some(handle.clone()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use futures::executor::block_on;
	use sc_block_builder::BlockBuilderProvider;
	use sp_consensus::BlockOrigin;
	use sp_core::H256;
	use substrate_test_runtime_client::{
		prelude::*, runtime::Block, Backend, ClientBlockImportExt,
	};

	#[test]
	fn subscription_check_id() {
		let subs = SubscriptionManagement::<Block, Backend>::new();
		let builder = TestClientBuilder::new();
		let backend = builder.backend();

		let id = "abc".to_string();
		let hash = H256::random();

		let handle = subs.get_subscription(&id);
		assert!(handle.is_none());

		let (_, handle) = subs.insert_subscription(id.clone(), false, 10, backend).unwrap();
		assert!(!handle.contains_block(&hash));

		subs.remove_subscription(&id);

		let handle = subs.get_subscription(&id);
		assert!(handle.is_none());
	}

	#[test]
	fn subscription_check_block() {
		let subs = SubscriptionManagement::<Block, Backend>::new();
		let builder = TestClientBuilder::new();
		let backend = builder.backend();
		let mut client = Arc::new(builder.build());

		let block = client.new_block(Default::default()).unwrap().build().unwrap().block;
		let hash = block.header.hash();
		block_on(client.import(BlockOrigin::Own, block.clone())).unwrap();

		let id = "abc".to_string();

		// Check with subscription.
		let (_, handle) = subs.insert_subscription(id.clone(), false, 10, backend).unwrap();
		assert!(!handle.contains_block(&hash));
		assert!(!handle.unpin_block(&hash));

		handle.pin_block(hash).unwrap();
		assert!(handle.contains_block(&hash));
		// Unpin an invalid block.
		assert!(!handle.unpin_block(&H256::random()));

		// Unpin the valid block.
		assert!(handle.unpin_block(&hash));
		assert!(!handle.contains_block(&hash));
	}

	#[test]
	fn subscription_check_stop_event() {
		let subs = SubscriptionManagement::<Block, Backend>::new();
		let builder = TestClientBuilder::new();
		let backend = builder.backend();

		let id = "abc".to_string();

		// Check with subscription.
		let (mut rx_stop, handle) =
			subs.insert_subscription(id.clone(), false, 10, backend.clone()).unwrap();

		// Check the stop signal was not received.
		let res = rx_stop.try_recv().unwrap();
		assert!(res.is_none());

		// Inserting a second time returns None.
		let res = subs.insert_subscription(id.clone(), false, 10, backend);
		assert!(res.is_none());

		handle.stop();

		// Check the signal was received.
		let res = rx_stop.try_recv().unwrap();
		assert!(res.is_some());
	}

	#[test]
	fn subscription_check_data() {
		let subs = SubscriptionManagement::<Block, Backend>::new();
		let builder = TestClientBuilder::new();
		let backend = builder.backend();

		let id = "abc".to_string();
		let (_, handle) = subs.insert_subscription(id.clone(), false, 10, backend.clone()).unwrap();
		assert!(!handle.has_runtime_updates());

		let id2 = "abcd".to_string();
		let (_, handle) = subs.insert_subscription(id2.clone(), true, 10, backend).unwrap();
		assert!(handle.has_runtime_updates());
	}

	#[test]
	fn subscription_check_max_pinned() {
		let subs = SubscriptionManagement::<Block, Backend>::new();
		let builder = TestClientBuilder::new();
		let backend = builder.backend();
		let mut client = Arc::new(builder.build());

		let block = client.new_block(Default::default()).unwrap().build().unwrap().block;
		let hash = block.header.hash();
		block_on(client.import(BlockOrigin::Own, block.clone())).unwrap();

		let id = "abc".to_string();
		let hash_2 = H256::random();
		let (_, handle) = subs.insert_subscription(id.clone(), false, 1, backend).unwrap();

		handle.pin_block(hash).unwrap();
		// The same block can be pinned multiple times.
		handle.pin_block(hash).unwrap();
		// Exceeded number of pinned blocks.
		handle.pin_block(hash_2).unwrap_err();
	}
}

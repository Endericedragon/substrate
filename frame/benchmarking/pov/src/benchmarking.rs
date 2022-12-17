// This file is part of Substrate.

// Copyright (C) 2022 Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![cfg(feature = "runtime-benchmarks")]

use super::*;

use frame_support::traits::UnfilteredDispatchable;
use frame_system::{Pallet as System, RawOrigin};
use sp_runtime::traits::Hash;

frame_benchmarking::benchmarks! {
	storage_single_value_read {
		Value::<T>::put(123);
	}: {
		assert_eq!(Value::<T>::get(), Some(123));
	}

	storage_single_value_read_twice {
		Value::<T>::put(123);
	}: {
		assert_eq!(Value::<T>::get(), Some(123));
		assert_eq!(Value::<T>::get(), Some(123));
	}

	storage_single_value_write {
	}: {
		Value::<T>::put(123);
	} verify {
		assert_eq!(Value::<T>::get(), Some(123));
	}

	storage_single_value_kill {
		Value::<T>::put(123);
	}: {
		Value::<T>::kill();
	} verify {
		assert!(!Value::<T>::exists());
	}

	// This benchmark and the following are testing a storage map with adjacent storage items.
	//
	// First a storage map is filled and a specific number of other storage items is
	// created. Then the one value is read from the map. This demonstrates that the number of other
	// nodes in the Trie influences the proof size. The number of inserted nodes can be interpreted
	// as the number of `StorageMap`/`StorageValue` in the whole runtime.
	storage_1m_map_read_one_value_two_additional_layers {
		(0..(1<<10)).for_each(|i| Map1M::<T>::insert(i, i));
		// Assume there are 16-256 other storage items.
		(0..(1u32<<4)).for_each(|i| {
			let k = T::Hashing::hash(&i.to_be_bytes());
			frame_support::storage::unhashed::put(k.as_ref(), &i);
		});
	}: {
		assert_eq!(Map1M::<T>::get(1<<9), Some(1<<9));
	}

	storage_1m_map_read_one_value_three_additional_layers {
		(0..(1<<10)).for_each(|i| Map1M::<T>::insert(i, i));
		// Assume there are 256-4096 other storage items.
		(0..(1u32<<8)).for_each(|i| {
			let k = T::Hashing::hash(&i.to_be_bytes());
			frame_support::storage::unhashed::put(k.as_ref(), &i);
		});
	}: {
		assert_eq!(Map1M::<T>::get(1<<9), Some(1<<9));
	}

	storage_1m_map_read_one_value_four_additional_layers {
		(0..(1<<10)).for_each(|i| Map1M::<T>::insert(i, i));
		// Assume there are 4096-65536 other storage items.
		(0..(1u32<<12)).for_each(|i| {
			let k = T::Hashing::hash(&i.to_be_bytes());
			frame_support::storage::unhashed::put(k.as_ref(), &i);
		});
	}: {
		assert_eq!(Map1M::<T>::get(1<<9), Some(1<<9));
	}

	// Reads from both storage maps each `n` and `m` times. Should result in two linear components.
	storage_map_read_per_component {
		let n in 0 .. 100;
		let m in 0 .. 100;

		(0..m*10).for_each(|i| Map1M::<T>::insert(i, i));
		(0..n*10).for_each(|i| Map16M::<T>::insert(i, i));
	}: {
		(0..m).for_each(|i|
			assert_eq!(Map1M::<T>::get(i*10), Some(i*10)));
		(0..n).for_each(|i|
			assert_eq!(Map16M::<T>::get(i*10), Some(i*10)));
	}

	// Reads the same value from a storage map. Should not result in a component.
	storage_1m_map_one_entry_repeated_read {
		let n in 0 .. 100;
		Map1M::<T>::insert(0, 0);
	}: {
		(0..n).for_each(|i|
			assert_eq!(Map1M::<T>::get(0), Some(0)));
	}

	// Reads the same values from a storage map. Should result in a `1x` linear component.
	storage_1m_map_multiple_entry_repeated_read {
		let n in 0 .. 100;
		(0..n).for_each(|i| Map1M::<T>::insert(i, i));
	}: {
		(0..n).for_each(|i| {
			// Reading the same value 10 times does nothing.
			(0..10).for_each(|j|
				assert_eq!(Map1M::<T>::get(i), Some(i)));
		});
	}

	storage_value_bounded_read {
	}: {
		assert!(BoundedValue::<T>::get().is_none());
	}

	// Reading unbounded values will produce no mathematical worst case PoV size for this component.
	storage_value_unbounded_read {
	}: {
		assert!(UnboundedValue::<T>::get().is_none());
	}

	// Same as above, but we still expect a mathematical worst case PoV size for the bounded one.
	storage_value_bounded_and_unbounded_read {
	}: {
		assert!(UnboundedValue::<T>::get().is_none());
		assert!(BoundedValue::<T>::get().is_none());
	}

	storage_map_unbounded_read {
	}: {
		assert!(UnboundedMap::<T>::get(0).is_none());
	}

	// Emitting an event will not incur any PoV.
	emit_event {
		// Emit a single event.
		let call = Call::<T>::emit_event {  };
	}: { call.dispatch_bypass_filter(RawOrigin::Root.into()).unwrap(); }
	verify {
		assert_eq!(System::<T>::events().len(), 1);
	}

	// A No-OP will not incur any PoV.
	noop {
		let call = Call::<T>::noop { };
	}: {
		call.dispatch_bypass_filter(RawOrigin::Root.into()).unwrap();
	}

	impl_benchmark_test_suite!(
		Pallet,
		mock::new_test_ext(),
		mock::Test,
	);
}

#[cfg(test)]
mod mock {
	use sp_runtime::testing::H256;

	type AccountId = u64;
	type AccountIndex = u32;
	type BlockNumber = u64;

	type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
	type Block = frame_system::mocking::MockBlock<Test>;

	frame_support::construct_runtime!(
		pub enum Test where
			Block = Block,
			NodeBlock = Block,
			UncheckedExtrinsic = UncheckedExtrinsic,
		{
			System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
			Baseline: crate::{Pallet, Call, Storage, Event<T>},
		}
	);

	impl frame_system::Config for Test {
		type BaseCallFilter = frame_support::traits::Everything;
		type BlockWeights = ();
		type BlockLength = ();
		type DbWeight = ();
		type RuntimeOrigin = RuntimeOrigin;
		type Index = AccountIndex;
		type BlockNumber = BlockNumber;
		type RuntimeCall = RuntimeCall;
		type Hash = H256;
		type Hashing = ::sp_runtime::traits::BlakeTwo256;
		type AccountId = AccountId;
		type Lookup = sp_runtime::traits::IdentityLookup<Self::AccountId>;
		type Header = sp_runtime::testing::Header;
		type RuntimeEvent = RuntimeEvent;
		type BlockHashCount = ();
		type Version = ();
		type PalletInfo = PalletInfo;
		type AccountData = ();
		type OnNewAccount = ();
		type OnKilledAccount = ();
		type SystemWeightInfo = ();
		type SS58Prefix = ();
		type OnSetCode = ();
		type MaxConsumers = frame_support::traits::ConstU32<16>;
	}

	impl crate::Config for Test {
		type RuntimeEvent = RuntimeEvent;
	}

	pub fn new_test_ext() -> sp_io::TestExternalities {
		frame_system::GenesisConfig::default().build_storage::<Test>().unwrap().into()
	}
}

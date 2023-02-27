
//! Autogenerated weights for `pallet_nis`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 4.0.0-dev
//! DATE: 2023-02-27, STEPS: `2`, REPEAT: `1`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `i9`, CPU: `13th Gen Intel(R) Core(TM) i9-13900K`
//! EXECUTION: Some(Wasm), WASM-EXECUTION: Compiled, CHAIN: Some("dev"), DB CACHE: 1024

// Executed Command:
// ./target/release/substrate
// benchmark
// pallet
// --chain=dev
// --steps=2
// --repeat=1
// --pallet=pallet_nis
// --extrinsic=*
// --execution=wasm
// --wasm-execution=compiled
// --heap-pages=4096
// --output=./weights-dev-release/frame/nis/src/weights.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]

use frame_support::{traits::Get, weights::Weight};
use sp_std::marker::PhantomData;

/// Weight functions for `pallet_nis`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_nis::WeightInfo for WeightInfo<T> {
	/// Storage: Nis Queues (r:1 w:1)
	/// Proof: Nis Queues (max_values: None, max_size: Some(48022), added: 50497, mode: MaxEncodedLen)
	/// Storage: Balances Reserves (r:1 w:1)
	/// Proof: Balances Reserves (max_values: None, max_size: Some(1249), added: 3724, mode: MaxEncodedLen)
	/// Storage: Nis QueueTotals (r:1 w:1)
	/// Proof: Nis QueueTotals (max_values: Some(1), max_size: Some(6002), added: 6497, mode: MaxEncodedLen)
	/// The range of component `l` is `[0, 999]`.
	fn place_bid(_l: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `42 + l * (54 ±0)`
		//  Estimated: `63688`
		// Minimum execution time: 38_583 nanoseconds.
		Weight::from_ref_time(59_553_000)
			.saturating_add(Weight::from_proof_size(63688))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: Nis Queues (r:1 w:1)
	/// Proof: Nis Queues (max_values: None, max_size: Some(48022), added: 50497, mode: MaxEncodedLen)
	/// Storage: Balances Reserves (r:1 w:1)
	/// Proof: Balances Reserves (max_values: None, max_size: Some(1249), added: 3724, mode: MaxEncodedLen)
	/// Storage: Nis QueueTotals (r:1 w:1)
	/// Proof: Nis QueueTotals (max_values: Some(1), max_size: Some(6002), added: 6497, mode: MaxEncodedLen)
	fn place_bid_max() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `54247`
		//  Estimated: `63688`
		// Minimum execution time: 66_050 nanoseconds.
		Weight::from_ref_time(66_050_000)
			.saturating_add(Weight::from_proof_size(63688))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: Nis Queues (r:1 w:1)
	/// Proof: Nis Queues (max_values: None, max_size: Some(48022), added: 50497, mode: MaxEncodedLen)
	/// Storage: Nis QueueTotals (r:1 w:1)
	/// Proof: Nis QueueTotals (max_values: Some(1), max_size: Some(6002), added: 6497, mode: MaxEncodedLen)
	/// Storage: Balances Reserves (r:1 w:1)
	/// Proof: Balances Reserves (max_values: None, max_size: Some(1249), added: 3724, mode: MaxEncodedLen)
	/// The range of component `l` is `[1, 1000]`.
	fn retract_bid(_l: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6242 + l * (48 ±0)`
		//  Estimated: `63688`
		// Minimum execution time: 37_992 nanoseconds.
		Weight::from_ref_time(53_252_000)
			.saturating_add(Weight::from_proof_size(63688))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: Nis Summary (r:1 w:0)
	/// Proof: Nis Summary (max_values: Some(1), max_size: Some(40), added: 535, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn fund_deficit() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `222`
		//  Estimated: `5118`
		// Minimum execution time: 35_600 nanoseconds.
		Weight::from_ref_time(35_600_000)
			.saturating_add(Weight::from_proof_size(5118))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: Nis Receipts (r:1 w:1)
	/// Proof: Nis Receipts (max_values: None, max_size: Some(81), added: 2556, mode: MaxEncodedLen)
	/// Storage: Nis Summary (r:1 w:1)
	/// Proof: Nis Summary (max_values: Some(1), max_size: Some(40), added: 535, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: Balances Reserves (r:1 w:1)
	/// Proof: Balances Reserves (max_values: None, max_size: Some(1249), added: 3724, mode: MaxEncodedLen)
	fn thaw_private() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `423`
		//  Estimated: `13378`
		// Minimum execution time: 46_469 nanoseconds.
		Weight::from_ref_time(46_469_000)
			.saturating_add(Weight::from_proof_size(13378))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: Nis Receipts (r:1 w:1)
	/// Proof: Nis Receipts (max_values: None, max_size: Some(81), added: 2556, mode: MaxEncodedLen)
	/// Storage: Nis Summary (r:1 w:1)
	/// Proof: Nis Summary (max_values: Some(1), max_size: Some(40), added: 535, mode: MaxEncodedLen)
	/// Storage: Assets Asset (r:1 w:1)
	/// Proof: Assets Asset (max_values: None, max_size: Some(210), added: 2685, mode: MaxEncodedLen)
	/// Storage: Assets Account (r:1 w:1)
	/// Proof: Assets Account (max_values: None, max_size: Some(102), added: 2577, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	fn thaw_communal() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `868`
		//  Estimated: `15906`
		// Minimum execution time: 65_381 nanoseconds.
		Weight::from_ref_time(65_381_000)
			.saturating_add(Weight::from_proof_size(15906))
			.saturating_add(T::DbWeight::get().reads(5))
			.saturating_add(T::DbWeight::get().writes(5))
	}
	/// Storage: Nis Receipts (r:1 w:1)
	/// Proof: Nis Receipts (max_values: None, max_size: Some(81), added: 2556, mode: MaxEncodedLen)
	/// Storage: Nis Summary (r:1 w:1)
	/// Proof: Nis Summary (max_values: Some(1), max_size: Some(40), added: 535, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: Assets Asset (r:1 w:1)
	/// Proof: Assets Asset (max_values: None, max_size: Some(210), added: 2685, mode: MaxEncodedLen)
	/// Storage: Assets Account (r:1 w:1)
	/// Proof: Assets Account (max_values: None, max_size: Some(102), added: 2577, mode: MaxEncodedLen)
	/// Storage: Balances Reserves (r:1 w:1)
	/// Proof: Balances Reserves (max_values: None, max_size: Some(1249), added: 3724, mode: MaxEncodedLen)
	fn privatize() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `930`
		//  Estimated: `20620`
		// Minimum execution time: 75_749 nanoseconds.
		Weight::from_ref_time(75_749_000)
			.saturating_add(Weight::from_proof_size(20620))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: Nis Receipts (r:1 w:1)
	/// Proof: Nis Receipts (max_values: None, max_size: Some(81), added: 2556, mode: MaxEncodedLen)
	/// Storage: Balances Reserves (r:1 w:1)
	/// Proof: Balances Reserves (max_values: None, max_size: Some(1249), added: 3724, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:1)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: Nis Summary (r:1 w:1)
	/// Proof: Nis Summary (max_values: Some(1), max_size: Some(40), added: 535, mode: MaxEncodedLen)
	/// Storage: Assets Asset (r:1 w:1)
	/// Proof: Assets Asset (max_values: None, max_size: Some(210), added: 2685, mode: MaxEncodedLen)
	/// Storage: Assets Account (r:1 w:1)
	/// Proof: Assets Account (max_values: None, max_size: Some(102), added: 2577, mode: MaxEncodedLen)
	fn communify() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `769`
		//  Estimated: `20620`
		// Minimum execution time: 64_304 nanoseconds.
		Weight::from_ref_time(64_304_000)
			.saturating_add(Weight::from_proof_size(20620))
			.saturating_add(T::DbWeight::get().reads(6))
			.saturating_add(T::DbWeight::get().writes(6))
	}
	/// Storage: Nis Summary (r:1 w:1)
	/// Proof: Nis Summary (max_values: Some(1), max_size: Some(40), added: 535, mode: MaxEncodedLen)
	/// Storage: System Account (r:1 w:0)
	/// Proof: System Account (max_values: None, max_size: Some(128), added: 2603, mode: MaxEncodedLen)
	/// Storage: Nis QueueTotals (r:1 w:1)
	/// Proof: Nis QueueTotals (max_values: Some(1), max_size: Some(6002), added: 6497, mode: MaxEncodedLen)
	fn process_queues() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6655`
		//  Estimated: `12605`
		// Minimum execution time: 23_432 nanoseconds.
		Weight::from_ref_time(23_432_000)
			.saturating_add(Weight::from_proof_size(12605))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: Nis Queues (r:1 w:1)
	/// Proof: Nis Queues (max_values: None, max_size: Some(48022), added: 50497, mode: MaxEncodedLen)
	fn process_queue() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `42`
		//  Estimated: `51487`
		// Minimum execution time: 6_033 nanoseconds.
		Weight::from_ref_time(6_033_000)
			.saturating_add(Weight::from_proof_size(51487))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: Nis Receipts (r:0 w:1)
	/// Proof: Nis Receipts (max_values: None, max_size: Some(81), added: 2556, mode: MaxEncodedLen)
	fn process_bid() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 8_975 nanoseconds.
		Weight::from_ref_time(8_975_000)
			.saturating_add(Weight::from_proof_size(0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}

// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
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

//! Test utilities for safe mode pallet.

#![cfg(test)]

use super::*;
use crate as pallet_safe_mode;

use frame_support::{
	parameter_types,
	traits::{ConstU64, Everything, InsideBoth, InstanceFilter, SortedMembers},
};
use frame_system::{EnsureSignedBy, RawOrigin};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

parameter_types! {
	pub const BlockHashCount: u64 = 250;
}
impl frame_system::Config for Test {
	type BaseCallFilter = InsideBoth<Everything, SafeMode>;
	type BlockWeights = ();
	type BlockLength = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = BlockHashCount;
	type DbWeight = ();
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u64>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxReserves: u32 = 10;
}

/// Identifies a hold on an account's balance.
#[derive(
	Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, MaxEncodedLen, Debug, TypeInfo,
)]
pub enum HoldReason {
	/// The safe-mode pallet holds funds since an account either entered or extended the safe-mode.
	SafeMode,
}

impl pallet_balances::Config for Test {
	type Balance = u64;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = ();
	type MaxReserves = MaxReserves;
	type ReserveIdentifier = Self::BlockNumber;
	type HoldIdentifier = HoldReason;
	type FreezeIdentifier = ();
	type MaxHolds = ConstU32<10>;
	type MaxFreezes = ConstU32<0>;
}

impl pallet_utility::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type PalletsOrigin = OriginCaller;
	type WeightInfo = ();
}

#[derive(
	Copy,
	Clone,
	Eq,
	PartialEq,
	Ord,
	PartialOrd,
	Encode,
	Decode,
	RuntimeDebug,
	MaxEncodedLen,
	scale_info::TypeInfo,
)]
pub enum ProxyType {
	Any,
	JustTransfer,
	JustUtility,
}
impl Default for ProxyType {
	fn default() -> Self {
		Self::Any
	}
}
impl InstanceFilter<RuntimeCall> for ProxyType {
	fn filter(&self, c: &RuntimeCall) -> bool {
		match self {
			ProxyType::Any => true,
			ProxyType::JustTransfer => {
				matches!(c, RuntimeCall::Balances(pallet_balances::Call::transfer { .. }))
			},
			ProxyType::JustUtility => matches!(c, RuntimeCall::Utility { .. }),
		}
	}
	fn is_superset(&self, o: &Self) -> bool {
		self == &ProxyType::Any || self == o
	}
}
impl pallet_proxy::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type RuntimeCall = RuntimeCall;
	type Currency = Balances;
	type ProxyType = ProxyType;
	type ProxyDepositBase = ConstU64<1>;
	type ProxyDepositFactor = ConstU64<1>;
	type MaxProxies = ConstU32<4>;
	type WeightInfo = ();
	type CallHasher = BlakeTwo256;
	type MaxPending = ConstU32<2>;
	type AnnouncementDepositBase = ConstU64<1>;
	type AnnouncementDepositFactor = ConstU64<1>;
}

/// Filter to allow all everything except balance calls
pub struct WhitelistedCalls;
impl Contains<RuntimeCall> for WhitelistedCalls {
	fn contains(call: &RuntimeCall) -> bool {
		match call {
			RuntimeCall::Balances(_) => false,
			_ => true,
		}
	}
}

/// An origin that can enable the safe-mode by force.
pub enum ForceEnterOrigin {
	Weak,
	Medium,
	Strong,
}

/// An origin that can extend the safe-mode by force.
pub enum ForceExtendOrigin {
	Weak,
	Medium,
	Strong,
}

impl ForceEnterOrigin {
	/// The duration of how long the safe-mode will be activated.
	pub fn duration(&self) -> u64 {
		match self {
			Self::Weak => 5,
			Self::Medium => 7,
			Self::Strong => 11,
		}
	}

	/// Account id of the origin.
	pub const fn acc(&self) -> u64 {
		match self {
			Self::Weak => 100,
			Self::Medium => 101,
			Self::Strong => 102,
		}
	}

	/// Signed origin.
	pub fn signed(&self) -> <Test as frame_system::Config>::RuntimeOrigin {
		RawOrigin::Signed(self.acc()).into()
	}
}

impl ForceExtendOrigin {
	/// The duration of how long the safe-mode will be extended.
	pub fn duration(&self) -> u64 {
		match self {
			Self::Weak => 13,
			Self::Medium => 17,
			Self::Strong => 19,
		}
	}

	/// Account id of the origin.
	pub const fn acc(&self) -> u64 {
		match self {
			Self::Weak => 200,
			Self::Medium => 201,
			Self::Strong => 202,
		}
	}

	/// Signed origin.
	pub fn signed(&self) -> <Test as frame_system::Config>::RuntimeOrigin {
		RawOrigin::Signed(self.acc()).into()
	}
}

impl<O: Into<Result<RawOrigin<u64>, O>> + From<RawOrigin<u64>>> EnsureOrigin<O>
	for ForceEnterOrigin
{
	type Success = u64;

	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|o| match o {
			RawOrigin::Signed(acc) if acc == ForceEnterOrigin::Weak.acc() =>
				Ok(ForceEnterOrigin::Weak.duration()),
			RawOrigin::Signed(acc) if acc == ForceEnterOrigin::Medium.acc() =>
				Ok(ForceEnterOrigin::Medium.duration()),
			RawOrigin::Signed(acc) if acc == ForceEnterOrigin::Strong.acc() =>
				Ok(ForceEnterOrigin::Strong.duration()),
			r => Err(O::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<O, ()> {
		Ok(O::from(RawOrigin::Signed(ForceEnterOrigin::Strong.acc())))
	}
}

impl<O: Into<Result<RawOrigin<u64>, O>> + From<RawOrigin<u64>>> EnsureOrigin<O>
	for ForceExtendOrigin
{
	type Success = u64;

	fn try_origin(o: O) -> Result<Self::Success, O> {
		o.into().and_then(|o| match o {
			RawOrigin::Signed(acc) if acc == ForceExtendOrigin::Weak.acc() =>
				Ok(ForceExtendOrigin::Weak.duration()),
			RawOrigin::Signed(acc) if acc == ForceExtendOrigin::Medium.acc() =>
				Ok(ForceExtendOrigin::Medium.duration()),
			RawOrigin::Signed(acc) if acc == ForceExtendOrigin::Strong.acc() =>
				Ok(ForceExtendOrigin::Strong.duration()),
			r => Err(O::from(r)),
		})
	}

	#[cfg(feature = "runtime-benchmarks")]
	fn try_successful_origin() -> Result<O, ()> {
		Ok(O::from(RawOrigin::Signed(ForceExtendOrigin::Strong.acc())))
	}
}

parameter_types! {
	pub const EnterDuration: u64 = 3;
	pub const ExtendDuration: u64 = 30;
	pub const EnterStakeAmount: u64 = 100;
	pub const EnterStakeAmountNone: Option<u64> = None;
	pub const ExtendStakeAmount: u64 = 100;
	pub const ForceExitOrigin: u64 = 3;
	pub const StakeSlashOrigin: u64 = 4;
	pub const ReleaseDelay: u64 = 2;
}

// Required impl to use some <Configured Origin>::get() in tests
impl SortedMembers<u64> for ForceExitOrigin {
	fn sorted_members() -> Vec<u64> {
		vec![Self::get()]
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add(_m: &u64) {}
}
impl SortedMembers<u64> for StakeSlashOrigin {
	fn sorted_members() -> Vec<u64> {
		vec![Self::get()]
	}
	#[cfg(feature = "runtime-benchmarks")]
	fn add(_m: &u64) {}
}

parameter_types! {
	pub const SafeModeHoldReason: HoldReason = HoldReason::SafeMode;
}

impl Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type HoldReason = SafeModeHoldReason;
	type WhitelistedCalls = WhitelistedCalls;
	type EnterDuration = EnterDuration;
	type EnterStakeAmount = EnterStakeAmountNone;
	type ExtendDuration = ExtendDuration;
	type ExtendStakeAmount = ExtendStakeAmount;
	type ForceEnterOrigin = ForceEnterOrigin;
	type ForceExtendOrigin = ForceExtendOrigin;
	type ForceExitOrigin = EnsureSignedBy<ForceExitOrigin, Self::AccountId>;
	type StakeSlashOrigin = EnsureSignedBy<StakeSlashOrigin, Self::AccountId>;
	type ReleaseDelay = ReleaseDelay;
	type WeightInfo = ();
}

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Utility: pallet_utility,
		Proxy: pallet_proxy,
		SafeMode: pallet_safe_mode,
	}
);

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		// The 0 account is NOT a special origin, the rest may be.
		balances: vec![(0, 1234), (1, 5678), (2, 5678), (3, 5678), (4, 5678)],
	}
	.assimilate_storage(&mut t)
	.unwrap();

	GenesisBuild::<Test>::assimilate_storage(
		&pallet_safe_mode::GenesisConfig { entered_until: None, _phantom: Default::default() },
		&mut t,
	)
	.unwrap();

	let mut ext = sp_io::TestExternalities::new(t);
	ext.execute_with(|| {
		System::set_block_number(1);
	});
	ext
}

pub fn next_block() {
	SafeMode::on_finalize(System::block_number());
	Balances::on_finalize(System::block_number());
	System::on_finalize(System::block_number());
	System::set_block_number(System::block_number() + 1);
	System::on_initialize(System::block_number());
	Balances::on_initialize(System::block_number());
	SafeMode::on_initialize(System::block_number());
}

pub fn run_to(n: u64) {
	while System::block_number() < n {
		next_block();
	}
}

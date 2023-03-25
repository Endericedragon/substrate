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

#![cfg_attr(not(feature = "std"), no_std)]

mod benchmarking;
#[cfg(test)]
pub mod mock;
#[cfg(test)]
mod tests;
pub mod weights;

use frame_support::{
	pallet_prelude::*,
	traits::{
		fungible::{
			hold::{Inspect as FunHoldInspect, Mutate as FunHoldMutate},
			Inspect as FunInspect,
		},
		tokens::{Fortitude, Precision},
		CallMetadata, Contains, Defensive, GetCallMetadata, PalletInfoAccess,
	},
	weights::Weight,
};
use frame_system::pallet_prelude::*;
use sp_runtime::traits::Saturating;
use sp_std::{convert::TryInto, prelude::*};

pub use pallet::*;
pub use weights::*;

type BalanceOf<T> =
	<<T as Config>::Currency as FunInspect<<T as frame_system::Config>::AccountId>>::Balance;

/// Create a hold reason for a given cause.
pub trait CausalHoldReason<Cause> {
	type Reason: codec::Encode + TypeInfo + 'static;

	fn cause(path: Cause) -> Self::Reason;
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;

	#[pallet::pallet]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Currency type for this pallet, used for Stakes.
		type Currency: FunHoldInspect<Self::AccountId> + FunHoldMutate<Self::AccountId>;

		/// Create a hold reason for a specific cause.
		type HoldReason: CausalHoldReason<
			Self::BlockNumber,
			Reason = <Self::Currency as FunHoldInspect<Self::AccountId>>::Reason,
		>;

		/// Contains all runtime calls in any pallet that can be dispatched even while the safe-mode
		/// is entered.
		///
		/// The safe-mode pallet cannot disable it's own calls, and does not need to be explicitly
		/// added here.
		type WhitelistedCalls: Contains<Self::RuntimeCall>;

		/// For how many blocks the safe-mode will be entered by [`Pallet::enter`].
		#[pallet::constant]
		type EnterDuration: Get<Self::BlockNumber>;

		/// For how many blocks the safe-mode can be extended by each [`Pallet::extend`] call.
		///
		/// This does not impose a hard limit as the safe-mode can be extended multiple times.
		#[pallet::constant]
		type ExtendDuration: Get<Self::BlockNumber>;

		/// The amount that will be reserved upon calling [`Pallet::enter`].
		///
		/// `None` disallows permissionlessly enabling the safe-mode.
		#[pallet::constant]
		type EnterStakeAmount: Get<Option<BalanceOf<Self>>>;

		/// The amount that will be reserved upon calling [`Pallet::extend`].
		///
		/// `None` disallows permissionlessly extending the safe-mode.
		#[pallet::constant]
		type ExtendStakeAmount: Get<Option<BalanceOf<Self>>>;

		/// The origin that may call [`Pallet::force_enter`].
		///
		/// The `Success` value is the number of blocks that this origin can enter safe-mode for.
		type ForceEnterOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::BlockNumber>;

		/// The origin that may call [`Pallet::force_extend`].
		///
		/// The `Success` value is the number of blocks that this origin can extend the safe-mode.
		type ForceExtendOrigin: EnsureOrigin<Self::RuntimeOrigin, Success = Self::BlockNumber>;

		/// The origin that may call [`Pallet::force_enter`].
		type ForceExitOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The origin that may call [`Pallet::force_release_stake`] and
		/// [`Pallet::slash_stake`].
		type StakeSlashOrigin: EnsureOrigin<Self::RuntimeOrigin>;

		/// The minimal duration a deposit will remain reserved after safe-mode is entered or
		/// extended, unless [`Pallet::force_release_stake`] is successfully called sooner.
		///
		/// Every reservation is tied to a specific activation or extension, thus each reservation
		/// can be released independently after the delay for it has passed.
		///
		/// `None` disallows permissionlessly releasing the safe-mode Stakes.
		#[pallet::constant]
		type ReleaseDelay: Get<Option<Self::BlockNumber>>;

		// Weight information for extrinsics in this pallet.
		type WeightInfo: WeightInfo;
	}

	#[pallet::error]
	pub enum Error<T> {
		/// The safe-mode is (already or still) entered.
		Entered,

		/// The safe-mode is (already or still) exited.
		Exited,

		/// This functionality of the pallet is disabled by the configuration.
		NotConfigured,

		/// There is no balance reserved.
		NoReservation,

		/// This reservation cannot be released yet.
		CannotReleaseYet,
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// The safe-mode was entered until inclusively this block.
		Entered { until: T::BlockNumber },

		/// The safe-mode was extended until inclusively this block.
		Extended { until: T::BlockNumber },

		/// Exited the safe-mode for a specific reason.
		Exited { reason: ExitReason },

		/// An account had a reserve released that was reserved at a specific block.
		ReservationReleased { account: T::AccountId, block: T::BlockNumber, amount: BalanceOf<T> },

		/// An account had reserve slashed that was reserved at a specific block.
		ReservationSlashed { account: T::AccountId, block: T::BlockNumber, amount: BalanceOf<T> },
	}

	/// The reason why the safe-mode was deactivated.
	#[derive(Copy, Clone, PartialEq, Eq, RuntimeDebug, Encode, Decode, TypeInfo, MaxEncodedLen)]
	pub enum ExitReason {
		/// The safe-mode was automatically deactivated after it's duration ran out.
		Timeout,

		/// The safe-mode was forcefully deactivated by [`Pallet::force_exit`].
		Force,
	}

	/// Contains the last block number that the safe-mode will stay enter in.
	///
	/// This is set to `None` if the safe-mode is inactive.
	/// The safe-mode is automatically deactivated when the current block number is greater than
	/// this.
	#[pallet::storage]
	#[pallet::getter(fn active_until)]
	pub type EnteredUntil<T: Config> = StorageValue<_, T::BlockNumber, OptionQuery>;

	/// Holds the reserve that was taken from an account at a specific block number.
	#[pallet::storage]
	#[pallet::getter(fn reserves)]
	pub type Stakes<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		T::BlockNumber,
		BalanceOf<T>,
		OptionQuery,
	>;

	/// Configure the initial state of this pallet in the genesis block.
	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		pub entered_until: Option<T::BlockNumber>,
		pub _phantom: PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		// NOTE: `derive(Default)` does not work together with `#[pallet::genesis_config]`.
		// We therefore need to add a trivial default impl.
		fn default() -> Self {
			Self { entered_until: None, _phantom: PhantomData }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			if let Some(block) = self.entered_until {
				EnteredUntil::<T>::put(block);
			}
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// Enter safe-mode permissionlessly for [`Config::EnterDuration`] blocks.
		///
		/// Reserves [`Config::EnterStakeAmount`] from the caller's account.
		/// Emits an [`Event::Entered`] event on success.
		/// Errors with [`Error::Entered`] if the safe-mode is already entered.
		/// Errors with [`Error::NotConfigured`] if the reservation amount is `None` .
		///
		/// ### Safety
		///
		/// This may be called by any signed origin with [`Config::EnterStakeAmount`] free
		/// currency to reserve. This call can be disabled for all origins by configuring
		/// [`Config::EnterStakeAmount`] to `None`.
		#[pallet::call_index(0)]
		#[pallet::weight(T::WeightInfo::enter())]
		pub fn enter(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::do_enter(Some(who), T::EnterDuration::get())
		}

		/// Enter safe-mode by force for a per-origin configured number of blocks.
		///
		/// Emits an [`Event::Entered`] event on success.
		/// Errors with [`Error::Entered`] if the safe-mode is already entered.
		///
		/// Can only be called by the [`Config::ForceEnterOrigin`] origin.
		#[pallet::call_index(1)]
		#[pallet::weight(T::WeightInfo::force_enter())]
		pub fn force_enter(origin: OriginFor<T>) -> DispatchResult {
			let duration = T::ForceEnterOrigin::ensure_origin(origin)?;

			Self::do_enter(None, duration)
		}

		/// Extend the safe-mode permissionlessly for [`Config::ExtendDuration`] blocks.
		///
		/// This accumulates on top of the current remaining duration.
		/// Reserves [`Config::ExtendStakeAmount`] from the caller's account.
		/// Emits an [`Event::Extended`] event on success.
		/// Errors with [`Error::Exited`] if the safe-mode is entered.
		/// Errors with [`Error::NotConfigured`] if the reservation amount is `None` .
		///
		/// This may be called by any signed origin with [`Config::ExtendStakeAmount`] free
		/// currency to reserve. This call can be disabled for all origins by configuring
		/// [`Config::ExtendStakeAmount`] to `None`.
		#[pallet::call_index(2)]
		#[pallet::weight(T::WeightInfo::extend())]
		pub fn extend(origin: OriginFor<T>) -> DispatchResult {
			let who = ensure_signed(origin)?;

			Self::do_extend(Some(who), T::ExtendDuration::get())
		}

		/// Extend the safe-mode by force for a per-origin configured number of blocks.
		///
		/// Emits an [`Event::Extended`] event on success.
		/// Errors with [`Error::Exited`] if the safe-mode is inactive.
		///
		/// Can only be called by the [`Config::ForceExtendOrigin`] origin.
		#[pallet::call_index(3)]
		#[pallet::weight(T::WeightInfo::force_extend())]
		pub fn force_extend(origin: OriginFor<T>) -> DispatchResult {
			let duration = T::ForceExtendOrigin::ensure_origin(origin)?;

			Self::do_extend(None, duration)
		}

		/// Exit safe-mode by force.
		///
		/// Emits an [`Event::Exited`] with [`ExitReason::Force`] event on success.
		/// Errors with [`Error::Exited`] if the safe-mode is inactive.
		///
		/// Note: `safe-mode` will be automatically deactivated by [`Pallet::on_initialize`] hook
		/// after the block height is greater than [`EnteredUntil`] found in storage.
		/// Emits an [`Event::Exited`] with [`ExitReason::Timeout`] event on hook.
		///
		///
		/// ### Safety
		///
		/// Can only be called by the [`Config::ForceExitOrigin`] origin.
		#[pallet::call_index(4)]
		#[pallet::weight(T::WeightInfo::force_exit())]
		pub fn force_exit(origin: OriginFor<T>) -> DispatchResult {
			T::ForceExitOrigin::ensure_origin(origin)?;

			Self::do_deactivate(ExitReason::Force)
		}

		/// Slash a reservation for an account that entered or extended safe-mode at a specific
		/// block earlier.
		///
		/// This can be called while safe-mode is entered.
		///
		/// Emits a [`Event::ReservationSlashed`] event on success.
		/// Errors with [`Error::Entered`] if the safe-mode is entered.
		///
		/// Can only be called by the [`Config::StakeSlashOrigin`] origin.
		#[pallet::call_index(5)]
		#[pallet::weight(T::WeightInfo::force_slash_stake())]
		pub fn force_slash_stake(
			origin: OriginFor<T>,
			account: T::AccountId,
			block: T::BlockNumber,
		) -> DispatchResult {
			T::StakeSlashOrigin::ensure_origin(origin)?;

			Self::do_force_slash(account, block)
		}

		/// Permissionlessly release a reservation for an account that entered safe-mode at a
		/// specific block earlier.
		///
		/// The call can be completely disabled by setting [`Config::ReleaseDelay`] to `None`.
		/// This cannot be called while safe-mode is entered and not until the
		/// [`Config::ReleaseDelay`] block height is passed.
		///
		/// Emits a [`Event::ReservationReleased`] event on success.
		/// Errors with [`Error::Entered`] if the safe-mode is entered.
		/// Errors with [`Error::CannotReleaseYet`] if the [`Config::ReleaseDelay`] .
		/// Errors with [`Error::NoReservation`] if the payee has no reserved currency at the
		/// block specified.
		#[pallet::call_index(6)]
		#[pallet::weight(T::WeightInfo::release_stake())]
		pub fn release_stake(
			origin: OriginFor<T>,
			account: T::AccountId,
			block: T::BlockNumber,
		) -> DispatchResult {
			ensure_signed(origin)?;

			Self::do_release(false, account, block)
		}

		/// Force to release a reservation for an account that entered safe-mode at a specific
		/// block earlier.
		///
		/// This can be called while safe-mode is still entered.
		///
		/// Emits a [`Event::ReservationReleased`] event on success.
		/// Errors with [`Error::Entered`] if the safe-mode is entered.
		/// Errors with [`Error::NoReservation`] if the payee has no reserved currency at the
		/// block specified.
		///
		/// Can only be called by the [`Config::StakeSlashOrigin`] origin.
		#[pallet::call_index(7)]
		#[pallet::weight(T::WeightInfo::force_release_stake())]
		pub fn force_release_stake(
			origin: OriginFor<T>,
			account: T::AccountId,
			block: T::BlockNumber,
		) -> DispatchResult {
			T::StakeSlashOrigin::ensure_origin(origin)?;

			Self::do_release(true, account, block)
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Automatically exits the safe-mode when the period runs out.
		///
		/// Bypasses any call filters to avoid getting rejected by them.
		fn on_initialize(current: T::BlockNumber) -> Weight {
			match EnteredUntil::<T>::get() {
				Some(limit) if current > limit => {
					let _ =
						Self::do_deactivate(ExitReason::Timeout).defensive_proof("Must exit; qed");
					T::WeightInfo::on_initialize_exit()
				},
				_ => T::WeightInfo::on_initialize_noop(),
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	/// Logic for the [`crate::Pallet::enter`] and [`crate::Pallet::force_enter`] calls.
	fn do_enter(who: Option<T::AccountId>, duration: T::BlockNumber) -> DispatchResult {
		ensure!(!EnteredUntil::<T>::exists(), Error::<T>::Entered);

		if let Some(who) = who {
			let reserve = T::EnterStakeAmount::get().ok_or(Error::<T>::NotConfigured)?;
			Self::reserve(who, reserve)?;
		}

		let until = <frame_system::Pallet<T>>::block_number().saturating_add(duration);
		EnteredUntil::<T>::put(until);
		Self::deposit_event(Event::Entered { until });
		Ok(())
	}

	/// Logic for the [`crate::Pallet::extend`] and [`crate::Pallet::force_extend`] calls.
	fn do_extend(who: Option<T::AccountId>, duration: T::BlockNumber) -> DispatchResult {
		let mut until = EnteredUntil::<T>::get().ok_or(Error::<T>::Exited)?;

		if let Some(who) = who {
			let reserve = T::ExtendStakeAmount::get().ok_or(Error::<T>::NotConfigured)?;
			Self::reserve(who, reserve)?;
		}

		until.saturating_accrue(duration);
		EnteredUntil::<T>::put(until);
		Self::deposit_event(Event::<T>::Extended { until });
		Ok(())
	}

	/// Logic for the [`crate::Pallet::force_exit`] call.
	///
	/// Errors if the safe-mode is already exited.
	fn do_deactivate(reason: ExitReason) -> DispatchResult {
		let _until = EnteredUntil::<T>::take().ok_or(Error::<T>::Exited)?;
		Self::deposit_event(Event::Exited { reason });
		Ok(())
	}

	/// Logic for the [`crate::Pallet::release_stake`] and
	/// [`crate::Pallet::force_release_stake`] calls.
	///
	/// Errors if the safe-mode is entered with [`Error::Entered`] when `force` is `false`.
	/// Errors if release is called too soon by anyone but [`Config::StakeSlashOrigin`] with
	/// [`Error::CannotReleaseYet`].
	fn do_release(force: bool, account: T::AccountId, block: T::BlockNumber) -> DispatchResult {
		let amount = Stakes::<T>::take(&account, &block).ok_or(Error::<T>::NoReservation)?;

		if !force {
			ensure!(!Self::is_entered(), Error::<T>::Entered);

			let delay = T::ReleaseDelay::get().ok_or(Error::<T>::NotConfigured)?;
			let now = <frame_system::Pallet<T>>::block_number();
			ensure!(now > (block.saturating_add(delay)), Error::<T>::CannotReleaseYet);
		}

		T::Currency::release(
			&T::HoldReason::cause(block),
			&account,
			amount,
			Precision::BestEffort,
		)?;
		Self::deposit_event(Event::<T>::ReservationReleased { block, account, amount });
		Ok(())
	}

	/// Logic for the [`crate::Pallet::slash_reservation`] call.
	fn do_force_slash(account: T::AccountId, block: T::BlockNumber) -> DispatchResult {
		let amount = Stakes::<T>::take(&account, block).ok_or(Error::<T>::NoReservation)?;

		// FAIL-CI check these args
		T::Currency::burn_held(
			&T::HoldReason::cause(block),
			&account,
			amount,
			Precision::BestEffort,
			Fortitude::Force,
		)?;
		Self::deposit_event(Event::<T>::ReservationSlashed { block, account, amount });
		Ok(())
	}

	/// Reserve `amount` from `who` and store it in `Stakes`.
	fn reserve(who: T::AccountId, amount: BalanceOf<T>) -> DispatchResult {
		let block = <frame_system::Pallet<T>>::block_number();
		T::Currency::hold(&T::HoldReason::cause(block), &who, amount)?;
		let current_reservation = Stakes::<T>::get(&who, block).unwrap_or_default();
		Stakes::<T>::insert(&who, block, current_reservation.saturating_add(amount));
		Ok(())
	}

	/// Return whether the `safe-mode` is entered.
	pub fn is_entered() -> bool {
		EnteredUntil::<T>::exists()
	}

	/// Return whether this call is allowed to be dispatched.
	pub fn is_allowed(call: &T::RuntimeCall) -> bool
	where
		T::RuntimeCall: GetCallMetadata,
	{
		let CallMetadata { pallet_name, .. } = call.get_call_metadata();
		// The `SafeMode` pallet is always allowed.
		if pallet_name == <Pallet<T> as PalletInfoAccess>::name() {
			return true
		}

		if Self::is_entered() {
			T::WhitelistedCalls::contains(call)
		} else {
			true
		}
	}
}

impl<T: pallet::Config> Contains<T::RuntimeCall> for Pallet<T>
where
	T::RuntimeCall: GetCallMetadata,
{
	/// Return whether this call is allowed to be dispatched.
	fn contains(call: &T::RuntimeCall) -> bool {
		Pallet::<T>::is_allowed(call)
	}
}

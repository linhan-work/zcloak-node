//! # Crowdfunding Module
//! A module for user to transfer dots for Ztokens, before transfer, should automatically check whether KYC is verified onchain,if not should reject.
//! Otherwise ,should make a transaction to tranfer dot from user's account to our funding account,
//! and transfer according Ztokens later(to make it immediately later).

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use sp_std::prelude::*;
extern crate alloc;
use frame_support::{
	pallet_prelude::*,
	traits::{Currency, ExistenceRequirement},
};
use primitives_catalog::{inspect::CheckError, types::PublicInputs};

#[cfg(feature = "std")]
use codec::{Decode, Encode};
use sp_std::{
	cmp::{Eq, PartialEq},
	convert::TryInto,
};

#[cfg(all(feature = "std", test))]
mod mock;

#[cfg(all(feature = "std", test))]
mod tests;

// #[derive(Clone, Copy, Encode, Decode, PartialEq, Eq)]
// pub enum CheckError {
// 	//Not on chain
// 	ICOVerifyFailedNotOnChain,
// 	//KYC onchain, but not allowed to do crowdfunding
// 	ICOVerifyFailedNotAllowed,
// 	//KYC onchain, but the corresponding program is not ICOprogram
// 	ICOVerifyFailedTaskProgramWrong,
// }

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::dispatch::DispatchResult;
	use frame_system::pallet_prelude::*;
	// use pallet_starks_verifier::{ClassType};
	use primitives_catalog::types::{ClassType, ProgramOption, ProgramType, Range};
	use sp_runtime::SaturatedConversion;
	use zcloak_support::currency::RegulatedCurrency;
	extern crate zcloak_support;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: frame_system::Config + pallet_assets::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

		/// After a task is verified, it can still be stored on chain for a `StorePeriod` of time
		#[pallet::constant]
		type StorePeriod: Get<Self::BlockNumber>;

		type RegulatedCurrency: zcloak_support::currency::RegulatedCurrency<Self::AccountId>;
	}

	pub type BalanceOf<T> = <<T as pallet_assets::Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	pub type RegulatedBalanceOf<T> =
		<<<T as pallet::Config>::RegulatedCurrency as RegulatedCurrency<
			<T as frame_system::Config>::AccountId,
		>>::RCurrency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		///RegulatedTransferSuccess with source, dest, balance
		RegulatedTransferSuccess(T::AccountId, T::AccountId, T::Balance),
	}

	#[pallet::error]
	#[derive(Clone, PartialEq, Eq)]
	pub enum Error<T> {
		KYCNotPass,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10000)]
		pub fn transfer(
			origin: OriginFor<T>,
			dest: T::AccountId,
			value: T::Balance,
			// class_type: ClassType,
			// public_inputs: PublicInputs,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let class_type =
				ClassType::X1(ProgramType::Age(ProgramOption::Range(Range::LargeThan)));
			let public_inputs = vec![20];
			let option = ExistenceRequirement::KeepAlive;
			let v1 = TryInto::<u128>::try_into(value).ok();
			let v2 = RegulatedBalanceOf::<T>::saturated_from(v1.unwrap());
			let transfer_result =
				T::RegulatedCurrency::transfer(&who, &dest, v2, option, class_type, public_inputs);
			ensure!(
				transfer_result != Err(DispatchError::Other("NotPass".into())),
				Error::<T>::KYCNotPass
			);
			Self::deposit_event(Event::RegulatedTransferSuccess(who, dest, value));
			transfer_result
		}

		#[pallet::weight(10000)]
		pub fn transfer_class(
			origin: OriginFor<T>,
			dest: T::AccountId,
			value: T::Balance,
			class_type: ClassType,
			public_inputs: PublicInputs,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let option = ExistenceRequirement::KeepAlive;
			let v1 = TryInto::<u128>::try_into(value).ok();
			let v2 = RegulatedBalanceOf::<T>::saturated_from(v1.unwrap());
			let transfer_result =
				T::RegulatedCurrency::transfer(&who, &dest, v2, option, class_type, public_inputs);
			ensure!(
				transfer_result != Err(DispatchError::Other("KYCNotPass".into())),
				Error::<T>::KYCNotPass
			);
			Self::deposit_event(Event::RegulatedTransferSuccess(who, dest, value));
			transfer_result
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_block: T::BlockNumber) {}
	}
}

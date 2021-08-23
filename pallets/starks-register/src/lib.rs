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

};
use pallet_starks_verifier::KYCRegister;

#[cfg(feature = "std")]
use sp_std::{
	cmp::{Eq, PartialEq},

};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResult};
	use frame_system::pallet_prelude::*;
	use pallet_starks_verifier::{ClassType};

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type KYCRegister: KYCRegister;
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		///RegulatedTransferSuccess with source, dest, balance
		KYCRegistration(ClassType, [u8; 32]),
	}

	#[pallet::error]
	#[derive(Clone, PartialEq, Eq)]
	pub enum Error<T> {
		RegistrationFail,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10000)]
		pub fn register_kyc_class(
			origin: OriginFor<T>,
			class: ClassType,
			program_hash: [u8; 32],
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let registration_result = T::KYCRegister::register_kyc(&class, &program_hash);
			ensure!(registration_result.is_ok(), Error::<T>::RegistrationFail);
			Self::deposit_event(Event::KYCRegistration(class, program_hash));
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_block: T::BlockNumber) {}
	}
}

//! # Crowdfunding Module
//! A module for user to transfer dots for Ztokens, before transfer, should automatically check whether KYC is verified onchain,if not should reject.
//! Otherwise ,should make a transaction to tranfer dot from user's account to our funding account,
//! and transfer according Ztokens later(to make it immediately later).

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
use sp_std::prelude::*;
extern crate alloc;
use frame_support::pallet_prelude::*;
// use pallet_starks_verifier::KYCRegister;
use primitives_catalog::{
	regist::{ClassError, ClassTypeRegister},
	types::{ClassType, ProgramOption, ProgramType, Range},
};

#[cfg(feature = "std")]
use sp_std::cmp::{Eq, PartialEq};

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::dispatch::DispatchResult;
	use frame_system::pallet_prelude::*;
	// use pallet_starks_verifier::{ClassType};
	use primitives_catalog::types::ClassType;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: frame_system::Config {
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		type Register: ClassTypeRegister;
	}

	#[pallet::storage]
	#[pallet::getter(fn class_type_list)]
	pub(super) type ClassTypeList<T: Config> =
		StorageMap<_, Twox64Concat, ClassType, [u8; 32], ValueQuery>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		///RegulatedTransferSuccess with source, dest, balance
		ClassTypeRegistration(ClassType, [u8; 32]),
		ClassTypeDelete(ClassType),
		ClassTypeModify(ClassType, [u8; 32]),
	}

	#[pallet::error]
	#[derive(Clone, PartialEq, Eq)]
	pub enum Error<T> {
		RegistrationFail,
		DeleteClassTypeFail,
		ModifyClassTypeFail,
		/// ClassTypeListNotHaveThisOne
		ClassTypeListNotHaveThisOne,
		/// ClassTypeListAlreadyHaveThisOne
		ClassTypeListAlreadyHaveThisOne,
		ProgramIsEmpty,
	}

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		_phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			Pallet::<T>::initialize_class_type_list();
		}
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10000)]
		pub fn register_class_type(
			origin: OriginFor<T>,
			class: ClassType,
			program_hash: [u8; 32],
		) -> DispatchResult {
			let _who = ensure_signed(origin)?;
			let registration_result = T::Register::register(&class, &program_hash);
			ensure!(registration_result.is_ok(), Error::<T>::RegistrationFail);
			Self::deposit_event(Event::ClassTypeRegistration(class, program_hash));
			Ok(())
		}

		#[pallet::weight(10000)]
		pub fn delete_class_type(origin: OriginFor<T>, class_type: ClassType) -> DispatchResult {
			ensure_root(origin)?;
			ensure!(T::Register::remove(&class_type).is_ok(), Error::<T>::DeleteClassTypeFail);
			Self::deposit_event(Event::ClassTypeDelete(class_type));
			Ok(())
		}

		#[pallet::weight(10000)]
		pub fn modify_class_type_list(
			origin: OriginFor<T>,
			class_type: ClassType,
			program_hash: [u8; 32],
		) -> DispatchResult {
			ensure_root(origin)?;

			if T::Register::get(&class_type).is_ok() {
				ensure!(T::Register::remove(&class_type).is_ok(), Error::<T>::ModifyClassTypeFail);
				ensure!(
					T::Register::register(&class_type, &program_hash).is_ok(),
					Error::<T>::ModifyClassTypeFail
				);
			} else {
				ensure!(
					T::Register::register(&class_type, &program_hash).is_ok(),
					Error::<T>::ModifyClassTypeFail
				);
			}
			Self::deposit_event(Event::ClassTypeModify(class_type, program_hash));
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(_block: T::BlockNumber) {}
	}
}

impl<T: Config> ClassTypeRegister for Pallet<T> {
	fn register(class_type: &ClassType, program_hash: &[u8; 32]) -> Result<bool, ClassError> {
		if ClassTypeList::<T>::try_get(&class_type).is_ok() {
			return Err(ClassError::ClassAlreadyExist)
		} else {
			ClassTypeList::<T>::insert(class_type, program_hash);
			return Ok(true)
		}
	}

	fn get(class_type: &ClassType) -> Result<[u8; 32], ClassError> {
		let program_hash = ClassTypeList::<T>::try_get(class_type.clone());
		if program_hash.is_ok() {
			return Ok(program_hash.unwrap())
		} else {
			return Err(ClassError::ClassNotExist)
		}
	}

	fn remove(class_type: &ClassType) -> Result<bool, ClassError> {
		if ClassTypeList::<T>::try_get(class_type.clone()).is_ok() {
			<ClassTypeList<T>>::remove(class_type);
			return Ok(true)
		} else {
			return Err(ClassError::ClassNotExist)
		}
	}
}
impl <T: Config> Pallet<T>{
fn initialize_class_type_list() {
		let age_larger_program_hash = [89, 115, 133, 225, 108, 141, 149, 171, 95, 56, 227, 119, 216, 249, 208, 2, 222, 113,
		212, 58, 200, 37, 30, 53, 50, 161, 222, 237, 90, 3, 236, 253,
		];
		let country_eastasia_program_hash = [253,174,66,193,191,97,25,173,194,45,106,148,168,131,
		36,92,193,54,159,37,13,31,235,147,93,140,136,186,4,250,144,25
		];
		let country_number_1_program_hash = [131,88,90,167,212,163,30,194,98,142,76,33,216,242,13,
		208,160,222,41,89,8,71,94,105,52,55,12,122,7,196,50,60
		];
		let country_number_2_program_hash = [101,243,206,66,170,13,101,51,133,202,61,213,147,55,61,30,122,
		105,61,180,30,7,161,145,223,6,103,72,132,175,156,90
		];
		let age_larger_country_number_1_program_hash = [96,66,104,233,134,175,69,174,13,195,146,42,132,156,137,53,
		206,208,189,23,254,48,107,30,245,215,123,96,204,205,157,105
		];

		<ClassTypeList<T>>::insert(
			ClassType::X1(ProgramType::Age(ProgramOption::Range(Range::LargeThan))),
			age_larger_program_hash,
		);
		<ClassTypeList<T>>::insert(
			ClassType::X1(ProgramType::Country(ProgramOption::Other("eastasia".as_bytes().to_vec()))),
			country_eastasia_program_hash,
        );
		<ClassTypeList<T>>::insert(
			ClassType::X1(ProgramType::Country(ProgramOption::Index(1_u32))),
			country_number_1_program_hash,
		);
		<ClassTypeList<T>>::insert(
			ClassType::X1(ProgramType::Country(ProgramOption::Index(2_u32))),
			country_number_2_program_hash,
		);
        <ClassTypeList<T>>::insert(
			ClassType::X2(ProgramType::Age(ProgramOption::Range(Range::LargeThan)),ProgramType::Country(ProgramOption::Index(1_u32))),
			age_larger_country_number_1_program_hash,
		);
	}
}




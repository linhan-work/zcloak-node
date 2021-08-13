//! # Crowdfunding Module
//! A module for user to transfer dots for Ztokens, before transfer, should automatically check whether KYC is verified onchain,if not should reject.
//! Otherwise ,should make a transaction to tranfer dot from user's account to our funding account, 
//! and transfer according Ztokens later(to make it immediately later).

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
pub use pallet::*;
extern crate alloc;
use frame_support::{log, pallet_prelude::*, transactional, PalletId};
use frame_support::traits::{Currency, ExistenceRequirement, ExistenceRequirement::{AllowDeath}, LockIdentifier};
use frame_support::traits::tokens::fungibles::Transfer;
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::traits::{AccountIdConversion, IdentifyAccount, LookupError, Lookup};

#[cfg(feature = "std")]
use codec::{Codec, Decode, Encode};
use sp_std::{
	cmp::{Eq, PartialEq},
	convert::{TryInto},
    fmt::Debug,
};
use sp_runtime::{traits::{StaticLookup}};

use sp_runtime::RuntimeDebug;
type Class = Vec<u8>;


#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq)]
pub enum CheckError{
    //Not on chain
    ICOVerifyFailedNotOnChain,
    //KYC onchain, but not allowed to do crowdfunding
    ICOVerifyFailedNotAllowed,  
    //KYC onchain, but the corresponding program is not ICOprogram
    ICOVerifyFailedTaskProgramWrong,  

}



#[frame_support::pallet]
pub mod pallet {
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::{ pallet_prelude::*};
    use sp_runtime::{SaturatedConversion};
    use pallet_starks_verifier::VerifyClass;
    use zcloak_support::traits::RegulatedCurrency;
	use super::*;
    extern crate zcloak_support;

    // use zcloak_support::traits::{VerifyClass, RegulatedCurrency};

    #[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

    #[pallet::config]
    #[pallet::disable_frame_system_supertrait_check]
    pub trait Config: frame_system::Config + pallet_assets::Config
    {
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;

        /// After a task is verified, it can still be stored on chain for a `StorePeriod` of time
        #[pallet::constant]
        type StorePeriod: Get<Self::BlockNumber>;
    
        type RegulatedCurrency: zcloak_support::traits::tokens::currency::RegulatedCurrency<Self::AccountId>;
    }

    pub type BalanceOf<T> = <<T as pallet_assets::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;

    pub type RegulatedBalanceOf<T> = <<T as pallet::Config>::RegulatedCurrency as RegulatedCurrency<<T as frame_system::Config>::AccountId>>::Balance;

    
    // #[pallet::storage]
    // #[pallet::getter(fn funding_account)]
    // pub type FundingAccount<T: Config> = StorageDoubleMap<
    //     _, 
    //     Twox64Concat, T::AssetId,
    //     Twox64Concat, T::AccountId,
    //     T::Balance,
    //     ValueQuery
    // >;


    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {

    }

    #[pallet::error]
    #[derive(Clone, PartialEq, Eq)]
    pub enum Error<T>{
        TransferFail,
    }


    #[pallet::call]
    impl<T: Config> Pallet<T> {

        #[pallet::weight(10000)]
        pub fn transfer(
            origin: OriginFor<T>,
            dest: T::AccountId,
            value: T::Balance,
            kyc_verify: VerifyClass,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            let option = ExistenceRequirement::KeepAlive;
            
            let v1 = TryInto::<u128>::try_into(value).ok();
            let v2 = RegulatedBalanceOf::<T>::saturated_from(v1.unwrap());
            
            let res = T::RegulatedCurrency::transfer(&who, &dest, v2, option, kyc_verify);
            ensure!(res.is_ok() == true,
            Error::<T>::TransferFail
            );
            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block: T::BlockNumber) {
        }

    }
}


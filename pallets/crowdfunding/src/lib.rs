//! # Crowdfunding Module
//! A module for user to transfer dots for Ztokens, before transfer, should automatically check whether KYC is verified onchain,if not should reject.
//! Otherwise ,should make a transaction to tranfer dot from user's account to our funding account, 
//! and transfer according Ztokens later(to make it immediately later).

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
use frame_support::pallet;
pub use pallet::*;

#[macro_use]
extern crate alloc;
use alloc::string::String;

use sp_runtime::traits::Convert;
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize},
	DispatchError, DispatchResult,
};
use pallet_starks_verifier::{Check};
use codec::{Codec, Encode, Decode, FullCodec};
use sp_std::{
	cmp::{Eq, PartialEq},
	convert::{TryFrom, TryInto},
	fmt::Debug,
	result,
};
use frame_system::offchain::{
    SendTransactionTypes,
    SubmitTransaction,
};
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
    use frame_support::{
		dispatch::DispatchResult,
		pallet_prelude::*,
	};
	use frame_system::pallet_prelude::*;
	use super::*;

    #[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

    #[pallet::config]
    #[pallet::disable_frame_system_supertrait_check]
    pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
        /// The identifier type for an offchain worker.
        type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord + MaybeSerializeDeserialize;
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// A type for retrieving the validators supposed to be online in a session.
        // type ValidatorSet: ValidatorSetWithIdentification<Self::AccountId>;
    
        /// After a task is verified, it can still be stored on chain for a `StorePeriod` of time
        #[pallet::constant]
        type StorePeriod: Get<Self::BlockNumber>;
    
        /// A configuration for base priority of unsigned transactions.
        ///
        /// This is exposed so that it can be tuned for particular runtime, when
        /// multiple pallets send unsigned transactions.
        #[pallet::constant]
        type UnsignedPriority: Get<TransactionPriority>;
    	
        type Balance: AtLeast32BitUnsigned + FullCodec + Copy + MaybeSerializeDeserialize + Debug + Default;

        type Check: Check<Self::AccountId>;
    }

    #[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    #[pallet::storage]
    #[pallet::getter(fn funding_account)]
    pub type FundingAccount<T: Config> = StorageMap<
        _, 
        Twox64Concat, T::AccountId,
        T::Balance,
        ValueQuery
    >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        //Check KYC onchain, and verified pass
        ICOVerifySuccuss(T::AccountId),

        ICOVerifyFailedNotOnChain,
        // KYC onchain, but not allowed to do crowdfunding
        ICOVerifyFailedNotAllowed,  
        // KYC onchain, but the corresponding program is not ICOprogram
        ICOVerifyFailedTaskProgramWrong,  
        OtherErr,

		//Founding successfly ,with xdots deducted and going to transfer Ztokens to this account.
        FoundingSuccess(T::AccountId, T::Balance, T::Balance),
    }

    #[pallet::error]
    #[derive(Clone, PartialEq, Eq)]
    pub enum Error<T>{
        //Not on chain
        ICOVerifyFailedNotOnChain,
        // KYC onchain, but not allowed to do crowdfunding
        ICOVerifyFailedNotAllowed,  
        // KYC onchain, but the corresponding program is not ICOprogram
        ICOVerifyFailedTaskProgramWrong,  
        OtherErr,
    }


    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(10000)]
        pub fn BuyZtoken(
            origin: OriginFor<T>,
            ZtokenToBuy: T::Balance,
        ) -> DispatchResult{
            let who = ensure_signed(origin)?;
            let IOCProgramString:String = String::from("d0c282c5a418c02ba9c705051e31be89a81daf6ffe6c8af2a1c94c0aee8c610e");
            let IOCProgramhash = IOCProgramString.encode();
            let KYCclass:Class = [22].to_vec();
            log::debug!(target:"buyztoken","IOCProgramhash is {:?}",IOCProgramhash);

            //检查这个proof是否已经在链上存储并检验，如果没存、或者不对应 false，则直接err
            let check_result = T::Check::checkkyc(who.clone(), KYCclass.clone(),IOCProgramhash.clone()); 
            if check_result.is_ok() {
                // return true ，那么就是可以参与buy操作
                Self::deposit_event(Event::ICOVerifySuccuss(who.clone()));
                <FundingAccount<T>>::insert(&who, &ZtokenToBuy);

            }else{
                let CrowdfundingErr = check_result.err();
                match CrowdfundingErr {
                    Some(pallet_starks_verifier::CheckError::ICOVerifyFailedNotAllowed) => {Self::deposit_event(Event::ICOVerifyFailedNotAllowed);return Err(Error::<T>::ICOVerifyFailedNotAllowed.into())},
                    Some(pallet_starks_verifier::CheckError::ICOVerifyFailedTaskProgramWrong) => {Self::deposit_event(Event::ICOVerifyFailedTaskProgramWrong);return Err( Error::<T>::ICOVerifyFailedTaskProgramWrong.into())},
                    Some(pallet_starks_verifier::CheckError::ICOVerifyFailedNotOnChain) => {Self::deposit_event(Event::ICOVerifyFailedNotOnChain);return Err(Error::<T>::ICOVerifyFailedNotOnChain.into())},
                    None => return {Self::deposit_event(Event::OtherErr);Err(Error::<T>::OtherErr.into())},
                }
            }

               //buy操作：转对应的dot到funding 账户，在链上funding account存储应当转的Ztoken数量，之后转


            Ok(())
        }
        
    }
}
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
// use frame_system::RawOrigin as SystemOrigin;
use frame_support::traits::tokens::fungibles::Transfer;
use sp_runtime::traits::Convert;
use sp_application_crypto::RuntimeAppPublic;
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize},
	DispatchError, DispatchResult,
};
use pallet_starks_verifier::{Check};

use codec::{Codec, Encode, Decode, FullCodec, HasCompact};
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
use frame_support::traits::tokens::{ExistenceRequirement, WithdrawReasons, BalanceStatus};
use sp_runtime::RuntimeDebug;
type Class = Vec<u8>;
type AssetId = u32;

#[derive(Encode, Decode, Default, PartialEq, Eq, RuntimeDebug)]
pub struct CrowfundingStatus<AccountId, AssetId, BlockNumber, Balance> {
    // AssetId to get crowdfundation
    pub asset_id: Option<AssetId>,
    // Admin's job is to dispense asset(e.g. Ztoken-1001)
    pub admin: Option<AccountId>,
    // Account to attain dots from customers.
    pub funding_account: Option<AccountId>,
    // Crowdfundation beginning time 
    pub funding_begin: BlockNumber,
    // Crowdfundation expiration
    pub funding_expiration: BlockNumber,
    // Total amount of assets of this crowdfunation
    pub total_funding: Balance,
    // Amount of assets to be dispense 
    pub remain_funding: Balance,
    // Whether the crowdfundation is still going or not 
    pub is_funding_proceed: bool,
    
}

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
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*, traits::fungibles::Inspect};
	use frame_system::{pallet, pallet_prelude::*};
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

        type AssetId: Member + Parameter + Default + Copy + HasCompact; 

        type Inspect: frame_support::traits::fungibles::Inspect<Self::AccountId>;
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

    #[pallet::storage]
    #[pallet::getter(fn crowdfunding_process)]
    pub type CrowdfundingProcess<T: Config> = StorageMap<
        _, 
        Twox64Concat, T::AssetId,
        CrowfundingStatus<T::AccountId, T::AssetId, T::BlockNumber, T::Balance>,
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
        pub fn create_crowdfunding(
            origin: OriginFor<T>,
            asset_id: T::AssetId, 
            admin: T::AccountId,
            funding_account: T::AccountId,
            funding_period: T::BlockNumber,
            total_asset: T::Balance
        ) -> DispatchResult{
            let who = ensure_signed(origin)?;
            // let admin_own = T::Inspect::balance(asset_id,&admin);
            // log::debug!(target:"starks-verifier","own is {:?}",admin_own);
            Ok(())
        }




        #[pallet::weight(10000)]
        pub fn BuyZtoken(
            origin: OriginFor<T>,
            ZtokenToBuy: T::Balance,
        ) -> DispatchResult{
            let who = ensure_signed(origin)?;
            let IOCProgramString=[208, 194, 130, 197, 164, 24, 192, 43, 169, 199, 5, 5, 30, 49, 190, 137, 168, 29, 175, 111, 254, 108, 138, 242, 161, 201, 76, 10, 238, 140, 97, 14];
            let KYCclass:Class = [22].to_vec();

            let check_result = T::Check::checkkyc(who.clone(), KYCclass.clone(),IOCProgramString); 
            if check_result.is_ok() {
                Self::deposit_event(Event::ICOVerifySuccuss(who.clone()));
                <FundingAccount<T>>::insert(&who, &ZtokenToBuy);
                // let asset_id= 2002 ;
                
                let dot_to_buy = ZtokenToBuy;
                // let alice = ;

                // let alice = AccountKeyring::Alice.to_account_id();
                // let res1 = frame_support::traits::Currency::transfer(&who, dest, dot_to_buy, ExistenceRequirement::KeepAlive);

                // let res2 = T::Transfer::transfer(asset_id,alice, &who,ZtokenToBuy,true);
                
            }else{
                let CrowdfundingErr = check_result.err();
                match CrowdfundingErr {
                    Some(pallet_starks_verifier::CheckError::ICOVerifyFailedNotAllowed) => {Self::deposit_event(Event::ICOVerifyFailedNotAllowed);return Err(Error::<T>::ICOVerifyFailedNotAllowed.into())},
                    Some(pallet_starks_verifier::CheckError::ICOVerifyFailedTaskProgramWrong) => {Self::deposit_event(Event::ICOVerifyFailedTaskProgramWrong);return Err( Error::<T>::ICOVerifyFailedTaskProgramWrong.into())},
                    Some(pallet_starks_verifier::CheckError::ICOVerifyFailedNotOnChain) => {Self::deposit_event(Event::ICOVerifyFailedNotOnChain);return Err(Error::<T>::ICOVerifyFailedNotOnChain.into())},
                    None => return {Self::deposit_event(Event::OtherErr);Err(Error::<T>::OtherErr.into())},
                }
            }

            Ok(())
        }


        
    }
}

//! # Crowdfunding Module
//! A module for user to transfer dots for Ztokens, before transfer, should automatically check whether KYC is verified onchain,if not should reject.
//! Otherwise ,should make a transaction to tranfer dot from user's account to our funding account, 
//! and transfer according Ztokens later(to make it immediately later).

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

use sp_std::prelude::*;
pub use pallet::*;

extern crate alloc;

use frame_support::traits::{Currency,ExistenceRequirement::{AllowDeath}};
// use frame_system::RawOrigin as SystemOrigin;
use frame_support::traits::tokens::fungibles::Transfer;
use sp_application_crypto::RuntimeAppPublic;

use pallet_starks_verifier::{Check};

use codec::{ Encode, Decode};
use sp_std::{
	cmp::{Eq, PartialEq},
	convert::{TryInto},
};

use sp_runtime::RuntimeDebug;
type Class = Vec<u8>;


#[derive(Encode, Decode, Default, PartialEq, Eq, RuntimeDebug)]
pub struct CrowfundingStatus<AccountId, BlockNumber, Balance> {
    // AssetId to get crowdfunding
    // pub asset_id: Option<AssetId>,
    // Admin's job is to dispense asset(e.g. Ztoken-1001)
    pub admin: Option<AccountId>,
    // Account to attain dots from customers.
    pub funding_account: Option<AccountId>,
    // crowdfunding beginning time 
    pub funding_begin: BlockNumber,
    // crowdfunding expiration
    pub funding_expiration: BlockNumber,
    // Total amount of assets of this crowdfunation
    pub total_funding: Balance,
    // Amount of assets to be dispense 
    pub remain_funding: Balance,
    // Whether the crowdfunding is still going or not 
    pub is_funding_proceed: Option<bool>,

    // For primitive version ratio stand for 1dot :xZtokens; e.g. ratio = 4, 1dot can buy 4Ztokens.
    pub ratio: Balance,
    
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
    use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::{ pallet_prelude::*};
use sp_runtime::{SaturatedConversion};
	use super::*;

    #[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

    #[pallet::config]
    #[pallet::disable_frame_system_supertrait_check]
    pub trait Config: pallet_assets::Config + frame_system::Config 


    {
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
    	
        type Check: Check<Self::AccountId>;

        // type Currency: Currency<Self::AccountId>;

        type Inspect: frame_support::traits::fungibles::Inspect<Self::AccountId>;

        type CrowdFundingLimit: Get<Self::BlockNumber>;

        type CrowdFundingMetadataDepositBase: Get<<Self as pallet_assets::Config>::Balance>;

        type MinBalance: Get<<Self as pallet_assets::Config>::Balance>;

        // type Transfer: frame_support::traits::tokens::fungibles::Transfer<Self::AccountId>;
        type Transfer: frame_support::traits::tokens::fungibles::Transfer<<Self as frame_system::Config>::AccountId>;
    }

    pub type BalanceOf<T> = <<T as pallet_assets::Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;


    #[pallet::storage]
    #[pallet::getter(fn funding_account)]
    pub type FundingAccount<T: Config> = StorageDoubleMap<
        _, 
        Twox64Concat, T::AssetId,
        Twox64Concat, T::AccountId,
        T::Balance,
        ValueQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn crowdfunding_process)]
    pub type CrowdfundingProcess<T: Config> = StorageMap<
        _, 
        Twox64Concat, T::AssetId,
        CrowfundingStatus<T::AccountId, T::BlockNumber, T::Balance>,
        ValueQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn settled_crowdfunding)]
    pub(super) type Settledcrowdfunding<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat, T::BlockNumber,
        Twox64Concat, (T::AccountId, T::AssetId),
        // Represent whether is going on , false means not finish, true means finish
        Option<bool>,
        ValueQuery,
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
        // A crowdfunding just created,with AssetId,Admin,FundingAccount and total_amount of this crowdfunding
        CrowdfundingCreated(T::AssetId, T::AccountId, T::AccountId, T::Balance),
		//Founding successfly ,with xdots deducted and going to transfer Ztokens to this account.
        FoundingSuccess(T::AccountId, T::Balance, T::Balance),
        // Changing crowdfunding's proceeding status. May stop one or restart one.
        SwitchcrowdfundingStatusTo(T::AssetId, bool),
        // Delete crowdfunding
        AlreadyDeleteCrowdfunding(T::AssetId, T::BlockNumber)
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
        // In creating crowdfunding, admin doesn't have enough asset to dispense
        AdminNotHaveEnoughAsset,
        // Admin Account doesn't have enough ztoken
        AdminNotHavaEnoughZtoken,
        // The crowdfunding period must less than `CrowdFundingLimit`
        CrowdFundingTimeTooLong,
        // A crowdfunding is created shouldn't create another one.
        CrowdFundingAlreadyGoingOn,
        // CrowdFunding shouldn't be zero
        CrowdFundingAmountIsZero,
        // No corresponding Crowdfudation is on-chain
        CrowdfundingNotOnchain,
        // The Crowdfudation is stoped
        CrowdFundingStopped,
        // Already exceed DDL.
        CrowdfundingIsOver,
        // The remainingZtokens is not enought for this transaction.
        NotEnoughZtokensAvailable,
        // The buyer doesn't have enough Dot.
        NotHaveEnoughDotToBuy,
        // Not crowdfunding's admin have no right to do so
        HaveNoRightToModify,
        // Don't have to change ,already in that status.
        AlreadyInThatStatus,
        OtherErr,
    }


    #[pallet::call]
    impl<T: Config> Pallet<T> {

        #[pallet::weight(10000)]
        pub fn create_crowdfunding(
            origin: OriginFor<T>,
            asset_id: T::AssetId, 
            funding_account: T::AccountId,
            funding_period: T::BlockNumber,
            total_asset: T::Balance,
            ratio: T::Balance,
        ) -> DispatchResult{
            let who = ensure_signed(origin)?;
            let CrowfundingStatus{is_funding_proceed, ..} = Self::crowdfunding_process(asset_id);
            ensure!(
                is_funding_proceed == None,
                Error::<T>::CrowdFundingAlreadyGoingOn
            );
            let admin_own = pallet_assets::Pallet::<T>::balance(asset_id, &who);
            ensure!(
				total_asset != T::Balance::default() ,
                Error::<T>::CrowdFundingAmountIsZero
			);
            ensure!(
				total_asset / T::CrowdFundingMetadataDepositBase::get() <= admin_own,
                Error::<T>::AdminNotHaveEnoughAsset
			);
            //Don't know the decimal part of this asset
            //Ensure the crowdfunding period is limited by some timelimitation.
            ensure!(
                funding_period < T::CrowdFundingLimit::get(),
                Error::<T>::CrowdFundingTimeTooLong
            );
  
            let now = <frame_system::Pallet<T>>::block_number();
            let funding_begin = now;
            let funding_expiration = now + funding_period;
            //Insert new CrowdfundingStatus on-chain.
            <CrowdfundingProcess<T>>::insert(&asset_id,CrowfundingStatus{admin: Some(who.clone()), funding_account: Some(funding_account.clone()), funding_begin: funding_begin, funding_expiration: funding_expiration, total_funding: total_asset, remain_funding: total_asset, is_funding_proceed: Some(true), ratio: ratio});
            <Settledcrowdfunding<T>>::insert(&funding_expiration,&(who.clone(), asset_id.clone()),Some(false) );
            Self::deposit_event(Event::CrowdfundingCreated(asset_id, who, funding_account,total_asset));
            
            Ok(())
        }




        #[pallet::weight(10000)]
        pub fn buy_ztoken(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            ztoken_to_buy:  T::Balance,
        ) -> DispatchResult{
            let who = ensure_signed(origin)?;
            ensure!(CrowdfundingProcess::<T>::try_get(&asset_id).is_ok(), Error::<T>::CrowdfundingNotOnchain);
            let CrowfundingStatus{admin, funding_account, funding_begin, funding_expiration, total_funding, remain_funding, is_funding_proceed, ratio} = Self::crowdfunding_process(asset_id);
            ensure!(
                funding_expiration > <frame_system::Pallet<T>>::block_number(),
                Error::<T>::CrowdfundingIsOver
            );
            ensure!(
                is_funding_proceed == Some(true),
                Error::<T>::CrowdFundingStopped
            );
            ensure!(
                remain_funding >= ztoken_to_buy,
                Error::<T>::NotEnoughZtokensAvailable
            );

            // The KYC-Verifying program, to check whether this KYCproof is stored and VerifiedPass on-chain
            let ico_program_string=[208, 194, 130, 197, 164, 24, 192, 43, 169, 199, 5, 5, 30, 49, 190, 137, 168, 29, 175, 111, 254, 108, 138, 242, 161, 201, 76, 10, 238, 140, 97, 14];
            let kyc_class:Class = [22].to_vec();
            let check_result = T::Check::checkkyc(who.clone(), kyc_class.clone(),ico_program_string); 
            // The origin has the access to buy ztokens due to SuccessProved-KYC 
            if check_result.is_ok() {
                Self::deposit_event(Event::ICOVerifySuccuss(who.clone()));

                let dot_to_buy = (ztoken_to_buy / ratio) * T::CrowdFundingMetadataDepositBase::get();
                let dot_in_option = TryInto::<u128>::try_into(dot_to_buy).ok();
                // This `100` is for debug ,will modify next version
                let dot_in_balance = BalanceOf::<T>::saturated_from(dot_in_option.unwrap());
                let origin_own = T::Currency::free_balance(&who);
                ensure!(
                    dot_in_balance < origin_own,
                    Error::<T>::NotHaveEnoughDotToBuy,
                );

                let result = <pallet_assets::Pallet<T> as frame_support::traits::fungibles::Inspect<T::AccountId>>::can_withdraw(asset_id, &admin.clone().unwrap(),ztoken_to_buy / T::CrowdFundingMetadataDepositBase::get() );
                ensure!(
                    result == frame_support::traits::tokens::WithdrawConsequence::Success,
                    Error::<T>::AdminNotHavaEnoughZtoken
                );
                T::Currency::transfer(&who,&funding_account.clone().unwrap(),dot_in_balance, AllowDeath)?;
                <pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(asset_id,&admin.clone().unwrap(), &who,ztoken_to_buy / T::CrowdFundingMetadataDepositBase::get(),true)?;
                <FundingAccount<T>>::insert(&asset_id, &who,  &ztoken_to_buy);
                <CrowdfundingProcess<T>>::insert(&asset_id, CrowfundingStatus{admin: admin.clone(), funding_account, funding_begin, funding_expiration, total_funding, remain_funding: remain_funding - ztoken_to_buy, is_funding_proceed: Some(true), ratio} );

            }else{
                let crowdfunding_err = check_result.err();
                match crowdfunding_err {
                    Some(pallet_starks_verifier::CheckError::ICOVerifyFailedNotAllowed) => {Self::deposit_event(Event::ICOVerifyFailedNotAllowed);return Err(Error::<T>::ICOVerifyFailedNotAllowed.into())},
                    Some(pallet_starks_verifier::CheckError::ICOVerifyFailedTaskProgramWrong) => {Self::deposit_event(Event::ICOVerifyFailedTaskProgramWrong);return Err( Error::<T>::ICOVerifyFailedTaskProgramWrong.into())},
                    Some(pallet_starks_verifier::CheckError::ICOVerifyFailedNotOnChain) => {Self::deposit_event(Event::ICOVerifyFailedNotOnChain);return Err(Error::<T>::ICOVerifyFailedNotOnChain.into())},
                    None => return {Self::deposit_event(Event::OtherErr);Err(Error::<T>::OtherErr.into())},
                }
            }

            Ok(())
        }

        #[pallet::weight(10000)]
        pub fn switch_crowdfunding_status(
            origin: OriginFor<T>,
            asset_id: T::AssetId,
            switch_to: bool,
            delete_crowdfunding: bool,
        ) -> DispatchResult{
            let who = ensure_signed(origin)?;
            let CrowfundingStatus{admin, funding_account, funding_begin, funding_expiration, total_funding, remain_funding, is_funding_proceed, ratio} = Self::crowdfunding_process(asset_id);
            ensure!(
                funding_expiration > <frame_system::Pallet<T>>::block_number(),
                Error::<T>::CrowdfundingIsOver
            );
            ensure!(
                who == admin.clone().unwrap(),
                Error::<T>::HaveNoRightToModify
            );
            ensure!(
                switch_to != is_funding_proceed.unwrap(),
                Error::<T>::AlreadyInThatStatus
            );

            if !delete_crowdfunding{
                <CrowdfundingProcess<T>>::insert(&asset_id, CrowfundingStatus{admin: admin.clone(), funding_account, funding_begin, funding_expiration, total_funding, remain_funding, is_funding_proceed: Some(switch_to), ratio} );
                Self::deposit_event(Event::SwitchcrowdfundingStatusTo(asset_id, is_funding_proceed.unwrap()));         
            }else{
                <CrowdfundingProcess<T>>::remove(asset_id);
                <Settledcrowdfunding<T>>::insert(&funding_expiration,&(admin.clone().unwrap(), asset_id.clone()),Some(true));
                Self::deposit_event(Event::AlreadyDeleteCrowdfunding(asset_id, <frame_system::Pallet<T>>::block_number()));         

            }
            Ok(())

        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block: T::BlockNumber) {
            Settledcrowdfunding::<T>::remove_prefix(block);
        }

    }
}

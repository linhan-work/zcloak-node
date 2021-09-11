//! # Crowdfunding Module
//! A module for user to transfer dots for Ztokens, before transfer, should automatically check whether KYC is verified onchain,if not should reject.
//! Otherwise ,should make a transaction to tranfer dot from user's account to our funding account,
//! and transfer according Ztokens later(to make it immediately later).

// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]

pub use pallet::*;
// use pallet_starks_verifier::Check;
use sp_std::prelude::*;
extern crate alloc;
use frame_support::{
	log,
	pallet_prelude::*,
	traits::{tokens::fungibles::Transfer, Currency, ExistenceRequirement::AllowDeath},
	PalletId,
};
use sp_runtime::traits::AccountIdConversion;

#[cfg(feature = "std")]
use codec::{Decode, Encode};
use primitives_catalog::{inspect::CheckError, regist::ClassTypeRegister, types::{ClassType, ProgramOption, ProgramType, Range}};
use sp_runtime::{traits::StaticLookup, RuntimeDebug};
use sp_std::{
	cmp::{Eq, PartialEq},
	convert::TryInto,
	fmt::Debug,
};

#[cfg(all(feature = "std", test))]
mod mock;

#[cfg(all(feature = "std", test))]
mod tests;


#[derive(Encode, Decode, Default, PartialEq, Eq, RuntimeDebug)]
pub struct CrowdfundingStatus<AccountId, BlockNumber, Balance> {
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
	// ClassType of KYC
	pub class_type: ClassType,
	// The program_hash of this crowdfunding limitaion
	pub program_hash: [u8; 32],
	// ClassType with public inputs
	pub public_inputs: Vec<u128>,
	// For primitive version ratio stand for 1dot :xZtokens; e.g. ratio = 4, 1dot can buy 4Ztokens.
	pub ratio: Balance,
	// For storing logo and text information via IPFS
	pub logo_and_text_information: Vec<u8>,
}

#[derive(Encode, Decode, Default, PartialEq, Eq, RuntimeDebug)]
pub struct CrowdfundingDetail<Balance> {
	// If true, then this account is creator of the asset
	pub is_creator: bool,
	// Amount of assets owned
	pub total_balance: Option<Balance>,
}

// #[derive(Clone, Encode, Decode, PartialEq, Eq, Debug)]
// pub struct IdentityLookup<T>(PhantomData<T>);

// impl<T: Codec + Clone + PartialEq + Debug> StaticLookup for IdentityLookup<T> {
//     type Source = <T::Lookup as StaticLookup>::Source;
//     type Target = T;
//     fn lookup(x: Self::Source) -> Result<Self::Source, LookupError> { Ok(x) }
//     fn unlookup(x: Self::Target) -> Self::Source { x }
// }

// #[derive(Clone, Copy, Encode, Decode, PartialEq, Eq)]
// pub enum CheckError {
// 	//Not on chain
// 	VerifyFailedNotOnChain,
// 	//KYC onchain, but not allowed to do crowdfunding
// 	VerifyFailedNotAllowed,
// 	//KYC onchain, but the corresponding program is not ICOprogram
// 	VerifyFailedTaskProgramWrong,
// }

#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, Debug)]
pub enum CrowdfundingOption {
	// Asset not exist, create now (admin is origin) and mint it to CrowdfundingAccount
	CreateAsset,
	// Asset exist, origin have these asset, tranfer it to CrowdfundingAccount
	TransferAsset,
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::dispatch::DispatchResult;
	use frame_system::pallet_prelude::*;
	use primitives_catalog::{inspect::Inspect, types::ClassType};
	use sp_runtime::SaturatedConversion;
	// extern crate zcloak_support;

	// use zcloak_support::traits::{VerifyClass, RegulatedCurrency};

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

		type Check: primitives_catalog::inspect::Inspect<Self::AccountId>;

		type ClassTypeRegister: primitives_catalog::regist::ClassTypeRegister;

		type Inspect: frame_support::traits::fungibles::Inspect<Self::AccountId>;

		type CrowdFundingLimit: Get<Self::BlockNumber>;

		type CrowdFundingMetadataDepositBase: Get<<Self as pallet_assets::Config>::Balance>;

		type MinBalance: Get<<Self as pallet_assets::Config>::Balance>;

		type Transfer: frame_support::traits::tokens::fungibles::Transfer<
			<Self as frame_system::Config>::AccountId,
		>;

		#[pallet::constant]
		type PalletId: Get<PalletId>;
	}

	pub type BalanceOf<T> = <<T as pallet_assets::Config>::Currency as Currency<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	// impl<T: Config> StaticLookup for IdentityLookup<T> {
	//     type Source = <<T as pallet::Config>::Lookup as StaticLookup>::Source;
	//     type Target = <<T as pallet::Config>::Lookup as StaticLookup>::Target;

	//     fn lookup(s: Self::Source) -> Result<Self::Target, LookupError> {
	//         <<T as pallet::Config>::Lookup as StaticLookup>::lookup(s)
	//     }

	//     fn unlookup(x: Self::Target) -> Self::Source {
	//         <<T as pallet::Config>::Lookup as StaticLookup>::unlookup(x)
	//     }
	// }

	#[pallet::storage]
	#[pallet::getter(fn funding_account)]
	pub type FundingAccount<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AssetId,
		Twox64Concat,
		T::AccountId,
		T::Balance,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn crowdfunding_process)]
	pub type CrowdfundingProcess<T: Config> = StorageMap<
		_,
		Twox64Concat,
		T::AssetId,
		CrowdfundingStatus<T::AccountId, T::BlockNumber, T::Balance>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn crowdfunding_participants)]
	pub type CrowdfundingParticipants<T: Config> = StorageMap<
 		_,
		Twox64Concat,
		T::AssetId,
		Vec<T::AccountId>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn my_crowdfunding)]
	pub type MyCrowdfunding<T: Config> = StorageDoubleMap<
		_,
	Twox64Concat,
	T::AccountId,
	Twox64Concat,
	T::AssetId,
	CrowdfundingDetail<T::Balance>,
	ValueQuery,
>;

	#[pallet::storage]
	#[pallet::getter(fn settled_crowdfunding)]
	pub(super) type Settledcrowdfunding<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::BlockNumber,
		Twox64Concat,
		(T::AccountId, T::AssetId),
		// Represent whether is going on , false means not finish, true means finish
		Option<bool>,
		ValueQuery,
	>;

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
			Pallet::<T>::initialize_pallet_admin();
			Pallet::<T>::initialize_crowdfunding()
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		//Check KYC onchain, and verified pass
		VerifySuccuss(T::AccountId),

		VerifyFailedNotOnChain,
		// KYC onchain, but not allowed to do crowdfunding
		VerifyFailedNotAllowed,
		// KYC onchain, but the corresponding program is not ICOprogram
		VerifyFailedTaskProgramWrong,
		OtherErr,
		// A crowdfunding just created,with AssetId,Admin,FundingAccount and total_amount of this crowdfunding
		CrowdfundingCreated(T::AssetId, T::AccountId, T::AccountId, T::Balance),
		//Founding successfly ,with xdots deducted and going to transfer Ztokens to this account.
		FoundingSuccess(T::AccountId, T::Balance, T::Balance),
		// Changing crowdfunding's proceeding status. May stop one or restart one.
		SwitchcrowdfundingStatusTo(T::AssetId, bool),
		// Delete crowdfunding
		AlreadyDeleteCrowdfunding(T::AssetId, T::BlockNumber),
	}

	#[pallet::error]
	#[derive(Clone, PartialEq, Eq)]
	pub enum Error<T> {
		//Not on chain
		VerifyFailedNotOnChain,
		// KYC onchain, but not allowed to do crowdfunding
		VerifyFailedNotAllowed,
		// KYC onchain, but the corresponding program is not ICOprogram
		VerifyFailedTaskProgramWrong,
		// In creating crowdfunding, admin doesn't have enough asset to dispense
		AdminNotHaveEnoughAsset,
		// Admin Account doesn't have enough ztoken
		CrowdfundingNotHaveEnoughZtoken,
		// The crowdfunding period must less than `CrowdFundingLimit`
		CrowdFundingTimeTooLong,
		// A crowdfunding is created shouldn't create another one.
		CrowdFundingAlreadyGoingOn,
		// CrowdfundingAccountNotHaveEnoughAsset
		CrowdfundingAccountNotHaveEnoughAsset,
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
		// Can't create asset
		CreateAssetFail,
		// Can't mint asset
		MintAssetFail,
		ClassNotRegistOrWrong,
		OtherErr,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		#[pallet::weight(10000)]
		pub fn create_crowdfunding(
			origin: OriginFor<T>,
			crowdfunding_option: CrowdfundingOption,
			asset_id: T::AssetId,
			funding_account: T::AccountId,
			funding_period: T::BlockNumber,
			total_asset: T::Balance,
			ratio: T::Balance,
			class_type: ClassType,
			public_inputs: Vec<u128>,
			logo_and_text_information: Vec<u8>,
		) -> DispatchResult {

			let who = ensure_signed(origin.clone())?;
			let program_hash_result = T::ClassTypeRegister::get(&class_type);
			ensure!(program_hash_result.is_ok(), Error::<T>::ClassNotRegistOrWrong);

						
			let CrowdfundingStatus { is_funding_proceed, .. } = Self::crowdfunding_process(asset_id);
			ensure!(is_funding_proceed == None, Error::<T>::CrowdFundingAlreadyGoingOn);
			if crowdfunding_option == CrowdfundingOption::CreateAsset {
				let res =
					<<T as frame_system::Config>::Lookup as StaticLookup>::unlookup(who.clone());
				// Create Asset First, asset admin is origin, min_balance is set to be 1
				let create_asset_result = <pallet_assets::Pallet<T>>::create(
					origin.clone(),
					asset_id.clone(),
					res.clone(),
					T::MinBalance::get(),
				);
				ensure!(create_asset_result.is_ok() == true, Error::<T>::CreateAssetFail);
				// After create asset, mint this asset to CrowdfundingAccount which is determined by PalletId
				let crowdfunding_account =
					<<T as frame_system::Config>::Lookup as StaticLookup>::unlookup(
						Self::account_id().clone(),
					);
				let mint_asset_result = <pallet_assets::Pallet<T>>::mint(
					origin,
					asset_id,
					crowdfunding_account.clone(),
					total_asset / T::CrowdFundingMetadataDepositBase::get(),
				);
				ensure!(mint_asset_result.is_ok() == true, Error::<T>::MintAssetFail);
			} else if crowdfunding_option == CrowdfundingOption::TransferAsset {
				let origin_own = pallet_assets::Pallet::<T>::balance(asset_id, &who);
				ensure!(
					total_asset / T::CrowdFundingMetadataDepositBase::get() <= origin_own,
					Error::<T>::AdminNotHaveEnoughAsset
				);
				<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
					asset_id,
					&who.clone(),
					&Self::account_id(),
					total_asset / T::CrowdFundingMetadataDepositBase::get(),
					true,
				)?;
			}
			log::debug!(target:"starks-verifier","palletid have {:?}",pallet_assets::Pallet::<T>::balance(asset_id, Self::account_id()));
			let crowdfunding_account_own =
				pallet_assets::Pallet::<T>::balance(asset_id, Self::account_id());
			ensure!(
				total_asset / T::CrowdFundingMetadataDepositBase::get() <= crowdfunding_account_own,
				Error::<T>::CrowdfundingAccountNotHaveEnoughAsset
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
			<CrowdfundingProcess<T>>::insert(
				&asset_id,
				CrowdfundingStatus {
					admin: Some(who.clone()),
					funding_account: Some(funding_account.clone()),
					funding_begin,
					funding_expiration,
					total_funding: total_asset -
						T::MinBalance::get() * T::CrowdFundingMetadataDepositBase::get(),
					remain_funding: total_asset -
						T::MinBalance::get() * T::CrowdFundingMetadataDepositBase::get(),
					is_funding_proceed: Some(true),
					class_type,
					public_inputs,
					ratio,
					logo_and_text_information,
					program_hash: program_hash_result.unwrap(),
				},
			);
			<MyCrowdfunding<T>>::insert(
				who.clone(),
				asset_id.clone(),
				CrowdfundingDetail{
					is_creator: true,
					total_balance: None,
				}
			);
			<CrowdfundingParticipants<T>>::insert(
				asset_id.clone(),
				vec![who.clone()],
			);
			<Settledcrowdfunding<T>>::insert(
				&funding_expiration,
				&(who.clone(), asset_id.clone()),
				Some(false),
			);
			Self::deposit_event(Event::CrowdfundingCreated(
				asset_id,
				who,
				funding_account,
				total_asset,
			));

			Ok(())
		}

		#[pallet::weight(10000)]
		pub fn buy_ztoken(
			origin: OriginFor<T>,
			asset_id: T::AssetId,
			ztoken_to_buy: T::Balance,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(
				CrowdfundingProcess::<T>::try_get(&asset_id).is_ok(),
				Error::<T>::CrowdfundingNotOnchain
			);
			let CrowdfundingStatus {
				admin,
				funding_account,
				funding_begin,
				funding_expiration,
				total_funding,
				remain_funding,
				is_funding_proceed,
				class_type,
				public_inputs,
				ratio,
				logo_and_text_information,
				program_hash,

			} = Self::crowdfunding_process(asset_id);
			ensure!(
				funding_expiration > <frame_system::Pallet<T>>::block_number(),
				Error::<T>::CrowdfundingIsOver
			);
			ensure!(is_funding_proceed == Some(true), Error::<T>::CrowdFundingStopped);
			ensure!(remain_funding >= ztoken_to_buy, Error::<T>::NotEnoughZtokensAvailable);

			let program_hash_result = T::ClassTypeRegister::get(&class_type);
			ensure!(program_hash_result.is_ok(), Error::<T>::ClassNotRegistOrWrong);

			let check_result = T::Check::check(
				&who.clone(),
				program_hash_result.clone().unwrap(),
				public_inputs.clone(),
			);
			// The origin has the access to buy ztokens due to SuccessProved-KYC
			if check_result.is_ok() {
				Self::deposit_event(Event::VerifySuccuss(who.clone()));

				let dot_to_buy =
					(ztoken_to_buy / ratio) * T::CrowdFundingMetadataDepositBase::get();
				let dot_in_option = TryInto::<u128>::try_into(dot_to_buy).ok();
				// This `100` is for debug ,will modify next version
				let dot_in_balance = BalanceOf::<T>::saturated_from(dot_in_option.unwrap());
				let origin_own = T::Currency::free_balance(&who);
				ensure!(dot_in_balance < origin_own, Error::<T>::NotHaveEnoughDotToBuy,);

				let result =
					<pallet_assets::Pallet<T> as frame_support::traits::fungibles::Inspect<
						T::AccountId,
					>>::can_withdraw(
						asset_id,
						&Self::account_id(),
						ztoken_to_buy / T::CrowdFundingMetadataDepositBase::get(),
					);
				ensure!(
					result == frame_support::traits::tokens::WithdrawConsequence::Success,
					Error::<T>::CrowdfundingNotHaveEnoughZtoken
				);

				T::Currency::transfer(
					&who,
					&funding_account.clone().unwrap(),
					dot_in_balance,
					AllowDeath,
				)?;
				<pallet_assets::Pallet<T> as Transfer<T::AccountId>>::transfer(
					asset_id,
					&Self::account_id(),
					&who,
					ztoken_to_buy / T::CrowdFundingMetadataDepositBase::get(),
					false,
				)?;

				<FundingAccount<T>>::insert(&asset_id, &who, &ztoken_to_buy);
				<CrowdfundingProcess<T>>::insert(
					&asset_id,
					CrowdfundingStatus {
						admin: admin.clone(),
						funding_account,
						funding_begin,
						funding_expiration,
						total_funding,
						remain_funding: remain_funding - ztoken_to_buy,
						is_funding_proceed: Some(true),
						class_type,
						public_inputs,
						ratio,
						logo_and_text_information,
						program_hash,
					},
				);
				let res = MyCrowdfunding::<T>::try_get(who.clone(), asset_id.clone());
				if res.is_err(){
				// First time to buy this asset.
				<MyCrowdfunding<T>>::insert(
					who.clone(),
					asset_id.clone(),
					CrowdfundingDetail{
						is_creator: false,
						total_balance: Some(ztoken_to_buy),
					}
				);
				let mut vec_to_store = CrowdfundingParticipants::<T>::get(asset_id);
				vec_to_store.push(who.clone());
				<CrowdfundingParticipants<T>>::insert(
					asset_id.clone(),
					vec_to_store,
				)
			}else{
				let amount_before = res.unwrap().total_balance.unwrap();
				<MyCrowdfunding<T>>::insert(
					who.clone(),
					asset_id.clone(),
					CrowdfundingDetail{
						is_creator: false,
						total_balance: Some(amount_before + ztoken_to_buy),
					}
				)
			};
		} else {
				let crowdfunding_err = check_result.err();
				match crowdfunding_err {
					Some(CheckError::VerifyFailedNotAllowed) => {
						Self::deposit_event(Event::VerifyFailedNotAllowed);
						return Err(Error::<T>::VerifyFailedNotAllowed.into())
					},
					Some(CheckError::VerifyFailedTaskProgramWrong) => {
						Self::deposit_event(Event::VerifyFailedTaskProgramWrong);
						return Err(Error::<T>::VerifyFailedTaskProgramWrong.into())
					},
					Some(CheckError::VerifyFailedNotOnChain) => {
						Self::deposit_event(Event::VerifyFailedNotOnChain);
						return Err(Error::<T>::VerifyFailedNotOnChain.into())
					},
					_ =>
						return {
							Self::deposit_event(Event::OtherErr);
							Err(Error::<T>::OtherErr.into())
						},
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
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let CrowdfundingStatus {
				admin,
				funding_account,
				funding_begin,
				funding_expiration,
				total_funding,
				remain_funding,
				is_funding_proceed,
				class_type,
				public_inputs,
				ratio,
				logo_and_text_information,
				program_hash,
			} = Self::crowdfunding_process(asset_id);
			ensure!(
				funding_expiration > <frame_system::Pallet<T>>::block_number(),
				Error::<T>::CrowdfundingIsOver
			);
			ensure!(who == admin.clone().unwrap(), Error::<T>::HaveNoRightToModify);
			ensure!(switch_to != is_funding_proceed.unwrap(), Error::<T>::AlreadyInThatStatus);

			if !delete_crowdfunding {
				<CrowdfundingProcess<T>>::insert(
					&asset_id,
					CrowdfundingStatus {
						admin: admin.clone(),
						funding_account,
						funding_begin,
						funding_expiration,
						total_funding,
						remain_funding,
						is_funding_proceed: Some(switch_to),
						class_type,
						public_inputs,
						ratio,
						logo_and_text_information,
						program_hash,
					},
				);
				Self::deposit_event(Event::SwitchcrowdfundingStatusTo(
					asset_id,
					switch_to,
				));
			} else {
				<CrowdfundingProcess<T>>::remove(asset_id);
				<Settledcrowdfunding<T>>::insert(
					&funding_expiration,
					&(admin.clone().unwrap(), asset_id.clone()),
					Some(true),
				);
				Self::deposit_event(Event::AlreadyDeleteCrowdfunding(
					asset_id,
					<frame_system::Pallet<T>>::block_number(),
				));
			}
			Ok(())
		}
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(block: T::BlockNumber) {
			let mut res = Settledcrowdfunding::<T>::iter_prefix(block).collect::<Vec<_>>();
			let size = res.len();
			for _i in 0..size{
				let round = res.pop().unwrap();
				let first = round.0;
				let asset_to_del =first.1;
				CrowdfundingProcess::<T>::remove(asset_to_del);
			}
			Settledcrowdfunding::<T>::remove_prefix(block, None);
		}
	}

}
impl<T: Config> Pallet<T> {
	fn initialize_pallet_admin() {
		use sp_runtime::SaturatedConversion;
		let balance2 = TryInto::<u128>::try_into(10_000_000_000_u128).ok();
		// This `100` is for debug ,will modify next version
		let banlance3 = BalanceOf::<T>::saturated_from(balance2.unwrap());
		let _res = T::Currency::deposit_creating(&Pallet::<T>::account_id(), banlance3);
	}
	
	fn initialize_crowdfunding() {
		let a: [u8;32] = hex_literal::hex!["d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"].into();
		let b: [u8;32] = hex_literal::hex!["8eaf04151687736326c9fea17e25fc5287613693c912909cb226aa4794f26a48"].into();
		let c: [u8;32] = hex_literal::hex!["90b5ab205c6974c9ea841be688864633dc9ca8a357843eeacf2314649965fe22"].into();
		let d: [u8;32] = hex_literal::hex!["306721211d5404bd9da88e0204360a1a9ab8b87c66c1bc2fcdd37f3c2222cc20"].into();

		let alice = T::AccountId::decode(&mut &a[..]).unwrap_or_default();
		let bob = T::AccountId::decode(&mut &b[..]).unwrap_or_default();
		let charlie = T::AccountId::decode(&mut &c[..]).unwrap_or_default();
		let dave = T::AccountId::decode(&mut &d[..]).unwrap_or_default();
	
		let ai: [u8; 4] = [233, 3, 0, 0];
		let bi: [u8; 4] = [206, 25, 0, 0];
		let ci: [u8; 4] = [223, 0, 0, 0];
		let di: [u8; 4] = [97, 30, 0, 0];
		let ei: [u8; 4] = [240, 33, 0, 0];
		let fi: [u8; 4] = [104, 226, 13, 0];
		
		let asset_a = T::AssetId::decode(&mut &ai[..]).unwrap_or_default();
		let asset_b = T::AssetId::decode(&mut &bi[..]).unwrap_or_default();
		let asset_c = T::AssetId::decode(&mut &ci[..]).unwrap_or_default();
		let asset_d = T::AssetId::decode(&mut &di[..]).unwrap_or_default();
		let asset_e = T::AssetId::decode(&mut &ei[..]).unwrap_or_default();
		let asset_f = T::AssetId::decode(&mut &fi[..]).unwrap_or_default();

		let a_period: [u8; 4] = [184, 136, 0, 0];
		let b_period: [u8; 4] = [128, 187, 0, 0];
		let c_period: [u8; 4] = [224, 46, 0, 0];
		let d_period: [u8; 4] = [208, 132, 0, 0];
		let e_period: [u8; 4] = [64, 31, 0, 0];
		let f_period: [u8; 4] = [224, 171, 0, 0];

		let a_time = T::BlockNumber::decode(&mut &a_period[..]).unwrap_or_default();
		let b_time = T::BlockNumber::decode(&mut &b_period[..]).unwrap_or_default();
		let c_time = T::BlockNumber::decode(&mut &c_period[..]).unwrap_or_default();
		let d_time = T::BlockNumber::decode(&mut &d_period[..]).unwrap_or_default();
		let e_time = T::BlockNumber::decode(&mut &e_period[..]).unwrap_or_default();
		let f_time = T::BlockNumber::decode(&mut &f_period[..]).unwrap_or_default();

		let a_amount: [u8; 16] = [0, 0, 224, 118, 54, 164, 85, 129, 6, 0, 0, 0, 0, 0, 0, 0];
		let b_amount: [u8; 16] = [0, 128, 31, 137, 246, 46, 117, 35, 0, 0, 0, 0, 0, 0, 0, 0];
		let c_amount: [u8; 16] = [0, 0, 79, 140, 52, 232, 20, 2, 0, 0, 0, 0, 0, 0, 0, 0];
		let d_amount: [u8; 16] = [0, 0, 216, 160, 249, 133, 200, 208, 29, 0, 0, 0, 0, 0, 0, 0];
		let e_amount: [u8; 16] = [0, 0, 60, 164, 136, 163, 56, 1, 0, 0, 0, 0, 0, 0, 0, 0];
		let f_amount: [u8; 16] = [0, 0, 64, 140, 181, 120, 29, 175, 21, 0, 0, 0, 0, 0, 0, 0];

		let a_balance = T::Balance::decode(&mut &a_amount[..]).unwrap_or_default();
		let b_balance = T::Balance::decode(&mut &b_amount[..]).unwrap_or_default();
		let c_balance = T::Balance::decode(&mut &c_amount[..]).unwrap_or_default();
		let d_balance = T::Balance::decode(&mut &d_amount[..]).unwrap_or_default();
		let e_balance = T::Balance::decode(&mut &e_amount[..]).unwrap_or_default();
		let f_balance = T::Balance::decode(&mut &f_amount[..]).unwrap_or_default();

		let v_ratio: [u8; 16] = [0, 16, 165, 212, 232, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
		let ratio = T::Balance::decode(&mut &v_ratio[..]).unwrap_or_default();
		Self::initialize_storage(asset_a, alice.clone(), alice.clone(), a_time, a_balance, ClassType::X1(ProgramType::Age(ProgramOption::Range(Range::LargeThan))), [89, 115, 133, 225, 108, 141, 149, 171, 95, 56, 227, 119, 216, 249, 208, 2, 222, 113,212, 58, 200, 37, 30, 53, 50, 161, 222, 237, 90, 3, 236, 253], vec![20], ratio,b"QmTBSrXDjhogUuVPV4E21YhuXw1FB2hc5rZYoSKCdA21Vx".to_vec());
		Self::initialize_storage(asset_b, bob.clone(), bob.clone(), b_time, b_balance, ClassType::X1(ProgramType::Age(ProgramOption::Range(Range::LargeThan))), [89, 115, 133, 225, 108, 141, 149, 171, 95, 56, 227, 119, 216, 249, 208, 2, 222, 113,212, 58, 200, 37, 30, 53, 50, 161, 222, 237, 90, 3, 236, 253], vec![18], ratio,b"QmZ6tf5gsqjQhTen9DGrizDgQHGSdMcJXvcjHsyuSsJpf4".to_vec());
		Self::initialize_storage(asset_c, charlie.clone(), charlie.clone(), c_time, c_balance, ClassType::X1(ProgramType::Country(ProgramOption::Other("eastasia".as_bytes().to_vec()))), [253,174,66,193,191,97,25,173,194,45,106,148,168,131,36,92,193,54,159,37,13,31,235,147,93,140,136,186,4,250,144,25], vec![], ratio,b"QmUSXLqsmvPx3s47v9zDdZqLmonxWGLFZ4LJzM3ANSiNd8".to_vec());
		Self::initialize_storage(asset_d, dave.clone(), dave.clone(), d_time, d_balance, ClassType::X1(ProgramType::Country(ProgramOption::Index(1_u32))), [131,88,90,167,212,163,30,194,98,142,76,33,216,242,13,208,160,222,41,89,8,71,94,105,52,55,12,122,7,196,50,60], vec![840], ratio,b"QmZ1pqVw92TovaXiK1y7zNrbX1TPzfXnpr6vqUKcfEE9KC".to_vec());
		Self::initialize_storage(asset_e, alice.clone(), alice.clone(), e_time, e_balance, ClassType::X1(ProgramType::Country(ProgramOption::Index(2_u32))), [101,243,206,66,170,13,101,51,133,202,61,213,147,55,61,30,122,105,61,180,30,7,161,145,223,6,103,72,132,175,156,90], vec![826,250], ratio,b"QmXwp6KSBhkYBGqXtfH9aUT1qqAz3321HGpwi9Ef5sJtZ3".to_vec());
		Self::initialize_storage(asset_f, dave.clone(), dave.clone(), f_time, f_balance, ClassType::X2(ProgramType::Age(ProgramOption::Range(Range::LargeThan)),ProgramType::Country(ProgramOption::Index(1_u32))), [96,66,104,233,134,175,69,174,13,195,146,42,132,156,137,53,206,208,189,23,254,48,107,30,245,215,123,96,204,205,157,105], vec![18,840], ratio,b"QmeFJ6FXxY3SeyRjfcfFWNkYhay41X2irueoHNeEZwV5Sb".to_vec());
	}

	fn initialize_storage(
		asset_id: T::AssetId,
		admin: T::AccountId,
		funding_account: T::AccountId,
		funding_expiration: T::BlockNumber,
		total_funding: T::Balance,
		class_type : ClassType,
		program_hash: [u8; 32],
		public_inputs: Vec<u128>,
		ratio: T::Balance,
		logo_and_text_information: Vec<u8>,
		
	){
		<CrowdfundingProcess<T>>::insert(
			&asset_id,
			CrowdfundingStatus {
				admin: Some(admin.clone()),
				funding_account: Some(funding_account.clone()),
				funding_begin: T::BlockNumber::default(),
				funding_expiration,
				total_funding: total_funding,
				remain_funding: total_funding,
				is_funding_proceed: Some(true),
				class_type,
				program_hash: program_hash,
				public_inputs: public_inputs,
				ratio,
				logo_and_text_information,
			},
		);
		<MyCrowdfunding<T>>::insert(
			admin.clone(),
			asset_id.clone(),
			CrowdfundingDetail{
				is_creator: true,
				total_balance: None,
			}
		);
		<CrowdfundingParticipants<T>>::insert(
			asset_id.clone(),
			vec![admin.clone()],
		);
		<Settledcrowdfunding<T>>::insert(
			&funding_expiration,
			&(admin.clone(), asset_id.clone()),
			Some(false),
		);
	}
}

impl<T: Config> Pallet<T> {
	/// Get account of cdp treasury module.
	pub fn account_id() -> T::AccountId {
		T::PalletId::get().into_account()
	}
}


use super::*;
use crate::{mock::*,Error};
use frame_support::assert_noop;
use sp_core::{traits::TaskExecutorExt, testing::TaskExecutor};
use sp_core::offchain::{
	OffchainDbExt,
	OffchainWorkerExt,
	TransactionPoolExt,
	testing::{self as testing, TestOffchainExt, TestTransactionPoolExt},
};
use CrowdfundingOption::{CreateAsset, TransferAsset};
use frame_support::{assert_ok, traits::OnFinalize};
use sp_runtime::{testing::UintAuthorityId};
use frame_support::traits::{OffchainWorker,ExistenceRequirement::AllowDeath};
use sp_runtime::transaction_validity::TransactionSource;
use crate::Error::CreateAssetFail;
use pallet_starks_verifier::{TaskInfo, TaskStatus, Status};

type Balance = u128;

// To build genesis in first place, pallet admin account should init first.
#[test]
fn genesis_pallet_admin_init() {
	new_test_ext().execute_with(|| {
		
		let pallet_admin = StarksCrowdfunding::account_id();
		let balance = Balances::free_balance(&pallet_admin);
		assert_eq!(balance,0);

		StarksCrowdfunding::initialize_pallet_admin();
		let balance = Balances::free_balance(&pallet_admin);
		assert_eq!(balance, 10000000000);
	});
}

#[test]
fn create_asset_and_crowdfunding(){
	new_test_ext().execute_with(|| {
		// Init pallet_admin balance first.
		let pallet_admin = StarksCrowdfunding::account_id();
		StarksCrowdfunding::initialize_pallet_admin();
		let balance = Balances::free_balance(&pallet_admin);
		assert_eq!(balance, 10000000000);

		// let admin_balance = Assets::balance(1001, &1);
		Balances::make_free_balance_be(&1, 10000000);
		Balances::make_free_balance_be(&2, 10000000);



		// println!("admin1 1001 balance is {:?}",admin_balance);
		assert_eq!(Assets::balance(1001, pallet_admin), 0);
		let CrowfundingStatus{ is_funding_proceed, total_funding, ..} = StarksCrowdfunding::crowdfunding_process(1001);
		assert_eq!(is_funding_proceed, None);

		let crowdfunding_result =  StarksCrowdfunding::create_crowdfunding(Origin::signed(1), CreateAsset, 1001, 2, 4000, 4000_000_000_000_000, 2);
		assert_eq!(crowdfunding_result,Ok(()));
		assert_eq!(Assets::balance(1001, pallet_admin), (4000_000_000_000_000 / CrowdFundingMetadataDepositBase::get()));
		
		let CrowfundingStatus{ is_funding_proceed, total_funding, ..} = StarksCrowdfunding::crowdfunding_process(1001);
		assert_eq!(is_funding_proceed, Some(true));
		assert_eq!(total_funding, 4000_000_000_000_000 - MinBalance::get() * CrowdFundingMetadataDepositBase::get());
	});
}

#[test]
fn transfer_asset_and_crowdfunding(){
	new_test_ext().execute_with(|| {
		// Init pallet_admin balance first.
		let pallet_admin = StarksCrowdfunding::account_id();
		StarksCrowdfunding::initialize_pallet_admin();
		let balance = Balances::free_balance(&pallet_admin);
		assert_eq!(balance, 10000000000);
	
		// let admin_balance = Assets::balance(1001, &1);
		Balances::make_free_balance_be(&1, 10000000);
		Balances::make_free_balance_be(&2, 10000000);

		assert_eq!(Assets::balance(1001, pallet_admin), 0);
		let CrowfundingStatus{ is_funding_proceed, total_funding, ..} = StarksCrowdfunding::crowdfunding_process(1001);
		assert_eq!(is_funding_proceed, None);

		let create_asset = Assets::create(Origin::signed(1), 1001, 1, MinBalance::get());
		assert_eq!(create_asset, Ok(()));

		let mint_asset_to_1 = Assets::mint(Origin::signed(1), 1001, 1, (4000_000_000_000_000 / CrowdFundingMetadataDepositBase::get()));
		assert_eq!(mint_asset_to_1, Ok(()));
		assert_eq!((Assets::balance(1001, 1)),(4000_000_000_000_000 / CrowdFundingMetadataDepositBase::get()));

		let crowdfunding_result =  StarksCrowdfunding::create_crowdfunding(Origin::signed(1), TransferAsset, 1001, 2, 4000, 4000_000_000_000_000 - MinBalance::get() * CrowdFundingMetadataDepositBase::get(), 2);

		assert_eq!(crowdfunding_result,Ok(()));
		assert_eq!(Assets::balance(1001, pallet_admin), ((4000_000_000_000_000 - MinBalance::get())/ CrowdFundingMetadataDepositBase::get()));
		
			
	});
}

#[test]
fn create_asset_and_crowdfunding_again(){
	new_test_ext().execute_with(|| {
		// Init pallet_admin balance first.
		let pallet_admin = StarksCrowdfunding::account_id();
		StarksCrowdfunding::initialize_pallet_admin();
		let balance = Balances::free_balance(&pallet_admin);
		assert_eq!(balance, 10000000000);

		// let admin_balance = Assets::balance(1001, &1);
		Balances::make_free_balance_be(&1, 10000000);
		Balances::make_free_balance_be(&2, 10000000);

		// println!("admin1 1001 balance is {:?}",admin_balance);
		assert_eq!(Assets::balance(1001, pallet_admin), 0);
		let CrowfundingStatus{ is_funding_proceed, total_funding, ..} = StarksCrowdfunding::crowdfunding_process(1001);
		assert_eq!(is_funding_proceed, None);

		let crowdfunding_result =  StarksCrowdfunding::create_crowdfunding(Origin::signed(1), CreateAsset, 1001, 2, 4000, 4000_000_000_000_000, 2);
		assert_eq!(crowdfunding_result, Ok(()));

		let crowdfunding_result2 =  StarksCrowdfunding::create_crowdfunding(Origin::signed(2), CreateAsset, 1001, 4, 4000, 4000_000_000_000_000, 2);
		assert_noop!(crowdfunding_result2, Error::<Test>::CrowdFundingAlreadyGoingOn);

	});
}

#[test]
fn transfer_asset_but_not_have_enough_asset(){
	new_test_ext().execute_with(|| {
		// Init pallet_admin balance first.
		let pallet_admin = StarksCrowdfunding::account_id();
		StarksCrowdfunding::initialize_pallet_admin();
		let balance = Balances::free_balance(&pallet_admin);
		assert_eq!(balance, 10000000000);

		// let admin_balance = Assets::balance(1001, &1);
		Balances::make_free_balance_be(&1, 10000000);
		Balances::make_free_balance_be(&2, 10000000);

		// println!("admin1 1001 balance is {:?}",admin_balance);
		assert_eq!(Assets::balance(1001, pallet_admin), 0);
		let CrowfundingStatus{ is_funding_proceed, total_funding, ..} = StarksCrowdfunding::crowdfunding_process(1001);
		assert_eq!(is_funding_proceed, None);

		let create_asset = Assets::create(Origin::signed(1), 1001, 1, MinBalance::get());
		assert_eq!(create_asset, Ok(()));

		let mint_asset_to_1 = Assets::mint(Origin::signed(1), 1001, 1, (2000_000_000_000_000 / CrowdFundingMetadataDepositBase::get()));
		assert_eq!(mint_asset_to_1, Ok(()));
		assert_eq!((Assets::balance(1001, 1)),(2000_000_000_000_000 / CrowdFundingMetadataDepositBase::get()));

		let crowdfunding_result =  StarksCrowdfunding::create_crowdfunding(Origin::signed(1), TransferAsset, 1001, 2, 4000, 4000_000_000_000_000 - MinBalance::get() * CrowdFundingMetadataDepositBase::get(), 2);
		assert_noop!(crowdfunding_result, Error::<Test>::AdminNotHaveEnoughAsset);

	});
}

#[test]
fn crowdfunding_start_buyer_buyZtoken_without_KYC(){
	new_test_ext().execute_with(|| {
			// Init pallet_admin balance first.
			let pallet_admin = StarksCrowdfunding::account_id();
			StarksCrowdfunding::initialize_pallet_admin();
			let balance = Balances::free_balance(&pallet_admin);
			assert_eq!(balance, 10000000000);
	
			// let admin_balance = Assets::balance(1001, &1);
			Balances::make_free_balance_be(&1, 1000_000_000_000_000);
			Balances::make_free_balance_be(&2, 1000_000_000_000_000);
			Balances::make_free_balance_be(&3, 1000_000_000_000_000);

	
			// println!("admin1 1001 balance is {:?}",admin_balance);
			assert_eq!(Assets::balance(1001, pallet_admin), 0);
			let CrowfundingStatus{ is_funding_proceed, total_funding, ..} = StarksCrowdfunding::crowdfunding_process(1001);
			assert_eq!(is_funding_proceed, None);
	
			let crowdfunding_result =  StarksCrowdfunding::create_crowdfunding(Origin::signed(1), CreateAsset, 1001, 2, 4000, 4000_000_000_000_000, 2);
			assert_eq!(crowdfunding_result, Ok(()));

			let buy_result = StarksCrowdfunding::buy_ztoken(Origin::signed(3), 1001, 1000_000_000_000_000);
			assert_noop!(buy_result, Error::<Test>::ICOVerifyFailedNotOnChain);

	});
}
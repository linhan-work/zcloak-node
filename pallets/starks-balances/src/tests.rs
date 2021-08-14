
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
use pallet_starks_verifier::VerifyClass;
use frame_support::{assert_ok, traits::OnFinalize};
use sp_runtime::{testing::UintAuthorityId};
use frame_support::traits::{OffchainWorker,ExistenceRequirement::AllowDeath};
use sp_runtime::transaction_validity::TransactionSource;
use pallet_starks_verifier::{TaskInfo, TaskStatus, Status};

type Balance = u128;

// To build genesis in first place, pallet admin account should init first.
#[test]
fn genesis_pallet_admin_init() {
	new_test_ext().execute_with(|| {
        Balances::make_free_balance_be(&1, 10000000);
		Balances::make_free_balance_be(&2, 10000000);
        
		let transfer_result = StarksBalances::transfer(Origin::signed(1), 2, 200000000000, VerifyClass::Age(20));
        println!("transfer_result is {:?}",transfer_result);
		
	});
}
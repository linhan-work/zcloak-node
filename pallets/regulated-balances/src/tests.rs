use super::*;
use crate::{mock::*, Error};
use frame_support::{
	assert_noop, assert_ok,
	traits::{ExistenceRequirement::AllowDeath, OffchainWorker, OnFinalize},
};
use pallet_starks_verifier::{Status, TaskInfo, TaskStatus, VerifyClass};
use sp_core::{
	offchain::{
		testing::{self as testing, TestOffchainExt, TestTransactionPoolExt},
		OffchainDbExt, OffchainWorkerExt, TransactionPoolExt,
	},
	testing::TaskExecutor,
	traits::TaskExecutorExt,
};
use sp_runtime::{testing::UintAuthorityId, transaction_validity::TransactionSource};

type Balance = u128;

// To build genesis in first place, pallet admin account should init first.
#[test]
fn genesis_pallet_admin_init() {
	new_test_ext().execute_with(|| {
		Balances::make_free_balance_be(&1, 10000000);
		Balances::make_free_balance_be(&2, 10000000);

		let transfer_result =
			StarksBalances::transfer(Origin::signed(1), 2, 200000000000, VerifyClass::Age(20));
		println!("transfer_result is {:?}", transfer_result);
	});
}

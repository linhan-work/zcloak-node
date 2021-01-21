use super::*;
use crate::mock::{Verifier, new_test_ext};
use std::sync::Arc;
use codec::{Encode, Decode};
use frame_support::{
	assert_ok, impl_outer_origin, parameter_types,
};
use sp_core::{
	H256,
	offchain::{OffchainExt, TransactionPoolExt, testing},
	sr25519::{Pair as Sr25519Pair, Signature},
};

use sp_runtime::testing::UintAuthorityId;
use sp_keystore::{
	{KeystoreExt, SyncCryptoStore},
	testing::KeyStore,
};
use sp_runtime::{
	RuntimeAppPublic,
	testing::{Header, TestXt},
	traits::{
		BlakeTwo256, IdentityLookup, Extrinsic as ExtrinsicT,
		IdentifyAccount, Verify,
	},
};



#[test]
fn it_works() {

	let (offchain, offchain_state) = testing::TestOffchainExt::new();
	let (pool, pool_state) = testing::TestTransactionPoolExt::new();
	let keystore = KeyStore::new();
	let public = SyncCryptoStore::sr25519_generate_new(
		&keystore,
		crate::crypto::app_sr25519::Public::ID,
		Some(&format!("{}/hunter1", PHRASE))
	).unwrap();

	let mut t = new_test_ext();
	t.register_extension(OffchainExt::new(offchain));
	t.register_extension(TransactionPoolExt::new(pool));
	t.register_extension(KeystoreExt(Arc::new(keystore)));

	t.execute_with(|| {


	})




}




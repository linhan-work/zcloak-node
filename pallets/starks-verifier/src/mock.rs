use crate::*;
use std::sync::Arc;
use codec::{Encode, Decode};
use frame_support::{
	assert_ok, impl_outer_origin, parameter_types,
};
use sp_core::{
	H256,
	offchain::{OffchainExt, TransactionPoolExt, testing},
	sr25519::{self, Signature},
};

use sp_keystore::{
	{KeystoreExt, SyncCryptoStore},
	testing::KeyStore,
};
use sp_runtime::{
	RuntimeAppPublic,
	testing::{Header, UintAuthorityId, TestXt},
	traits::{
		BlakeTwo256, IdentityLookup, Extrinsic as ExtrinsicT,
		IdentifyAccount, Verify,
	},
};
use std::io::Read;
use frame_system::GenesisConfig;

impl_outer_origin! {
	pub enum Origin for Test where system = frame_system {}
}


// For testing the module, we construct most of a mock runtime. This means
// first constructing a configuration type (`Test`) which `impl`s each of the
// configuration traits of modules we want to use.
#[derive(Clone, Eq, PartialEq, Encode, Decode)]
pub struct Test;
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}
impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Call = ();
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = sp_core::sr25519::Public;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = ();
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = ();
	type AccountData = ();
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
}

type Extrinsic = TestXt<Call<Test>, ()>;
type AccountId = <<Signature as Verify>::Signer as IdentifyAccount>::AccountId;

impl frame_system::offchain::SigningTypes for Test {
	type Public = <Signature as Verify>::Signer;
	type Signature = Signature;
}

impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test where
	Call<Test>: From<LocalCall>,
{
	type OverarchingCall = Call<Test>;
	type Extrinsic = Extrinsic;
}

impl<LocalCall> frame_system::offchain::CreateSignedTransaction<LocalCall> for Test where
	Call<Test>: From<LocalCall>,
{
	fn create_transaction<C: frame_system::offchain::AppCrypto<Self::Public, Self::Signature>>(
		call: Call<Test>,
		_public: <Signature as Verify>::Signer,
		_account: AccountId,
		nonce: u64,
	) -> Option<(Call<Test>, <Extrinsic as ExtrinsicT>::SignaturePayload)> {
		Some((call, (nonce, ())))
	}
}

parameter_types! {
	pub const GracePeriod: u64 = 5;
	pub const UnsignedInterval: u64 = 128;
	pub const UnsignedPriority: u64 = 1 << 20;
}

impl Config for Test {
	type Event = ();
	type AuthorityId = UintAuthorityId;
	type Call = Call<Test>;
}

pub type Verifier = Module<Test>;

pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut t = frame_system::GenesisConfig::default()
		.build_storage::<Test>()
		.unwrap();
	let _ = GenesisConfig::<Test> {
		verifiers: built_in_verifiers(),
		.. Default::default()
	}.assimilate_storage(&mut t).unwrap();
	t.into()
}


pub fn built_in_verifiers() -> Vec<UintAuthorityId> {
	let mut keys = Vec::new();
	for _ in 0..3 {
		keys.push(UintAuthorityId::generate_pair());
	}
	keys
}


pub fn new_proof() -> std::io::Result<Vec<u8>> {
	let mut buf = Vec::new();
	let mut f = std::fs::File::open("../proof.txt")?;
	f.read_to_end(&mut buf)?;
	Ok(buf)
}
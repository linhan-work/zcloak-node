use crate::*;
use std::cell::RefCell;

use crate::Config;
use frame_support::parameter_types;
pub use pallet_starks_verifier::crypto::AuthorityId as VerifierId;
use sp_core::H256;
use sp_runtime::{
	testing::{Header, TestXt, UintAuthorityId},
	traits::{BlakeTwo256, ConvertInto, IdentityLookup},
	Perbill,
};
use sp_staking::SessionIndex;

use crate as starkscrowdfunding;

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;
type Balance = u128;
type BlockNumber = u64;
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Config, Storage, Event<T>},
		Session: pallet_session::{Pallet, Call, Storage, Event, Config<T>},
		Assets: pallet_assets::{Pallet, Call, Storage, Event<T>},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},

		StarksCrowdfunding: starkscrowdfunding::{Pallet, Call, Storage, Event<T>},
		Verifier: pallet_starks_verifier::{Pallet, Call, Storage, Event<T>, ValidateUnsigned},


	}
);

thread_local! {
	pub static VALIDATORS: RefCell<Option<Vec<u64>>> = RefCell::new(Some(vec![
		1,
		2,
		3,
	]));
}
pub struct TestSessionManager;
impl pallet_session::SessionManager<u64> for TestSessionManager {
	fn new_session(_new_index: SessionIndex) -> Option<Vec<u64>> {
		VALIDATORS.with(|l| l.borrow_mut().take())
	}
	fn end_session(_: SessionIndex) {}
	fn start_session(_: SessionIndex) {}
}

/// An extrinsic type used for tests.
pub type Extrinsic = TestXt<Call, ()>;

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const MaxLocks: u32 = 50;
}
parameter_types! {
	pub const StorePeriod: BlockNumber = 10;
	pub const WhiteListPeriod: BlockNumber = 150;

	pub const AssetDeposit: Balance = 100 ; // 100 DOLLARS deposit to create asset

	pub const VerifierPriority: TransactionPriority = TransactionPriority::max_value();
	pub const MetadataDepositBase: Balance = 10 ;
	pub const MetadataDepositPerByte: Balance = 1 ;
	pub const ApprovalDeposit: Balance = 1;
	pub const StringLimit: u32 = 50;
	pub const CrowdFundingLimit: BlockNumber = 4096;
	pub const CrowdFundingMetadataDepositBase: Balance = 1_000_000_000_000;
	pub const MinBalance: Balance = 1;
	pub const CrowdfundingPalletId: PalletId = PalletId(*b"py/crdfg");



}
parameter_types! {
	pub const BlockHashCount: u64 = 250;
	pub BlockWeights: frame_system::limits::BlockWeights =
		frame_system::limits::BlockWeights::simple_max(1024);
}

impl pallet_balances::Config for Test {
	type MaxLocks = MaxLocks;
	/// The type for recording an account's balance.
	type Balance = u128;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
}

impl pallet_assets::Config for Test {
	type Event = Event;
	type Balance = u128;
	type AssetId = u32;
	type Currency = Balances;
	type ForceOrigin = frame_system::EnsureRoot<u64>;
	type AssetDeposit = AssetDeposit;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
}
impl<LocalCall> frame_system::offchain::SendTransactionTypes<LocalCall> for Test
where
	Call: From<LocalCall>,
{
	type OverarchingCall = Call;
	type Extrinsic = Extrinsic;
}

impl pallet_starks_verifier::Config for Test {
	type AuthorityId = VerifierId;
	type Event = Event;
	type StorePeriod = StorePeriod;
	type UnsignedPriority = VerifierPriority;
}

impl frame_system::Config for Test {
	type BaseCallFilter = ();
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type Origin = Origin;
	type Index = u64;
	type BlockNumber = u64;
	type Call = Call;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type Event = Event;
	type BlockHashCount = BlockHashCount;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<u128>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ();
	type OnSetCode = ();
}

parameter_types! {
	pub const Period: u64 = 1;
	pub const Offset: u64 = 0;
}

parameter_types! {
	pub const DisabledValidatorsThreshold: Perbill = Perbill::from_percent(33);
}

impl pallet_session::Config for Test {
	type ShouldEndSession = pallet_session::PeriodicSessions<Period, Offset>;
	type SessionManager = TestSessionManager;
	type SessionHandler = (Verifier,);
	type ValidatorId = u64;
	type ValidatorIdOf = ConvertInto;
	type Keys = UintAuthorityId;
	type Event = Event;
	type DisabledValidatorsThreshold = DisabledValidatorsThreshold;
	type NextSessionRotation = pallet_session::PeriodicSessions<Period, Offset>;
	type WeightInfo = ();
}

parameter_types! {
	pub const UnsignedPriority: u64 = 1 << 20;
	pub const StoragePeriod: u64 = 20;
}

impl Config for Test {
	type AuthorityId = VerifierId;
	type Event = Event;
	type StorePeriod = StorePeriod;
	type UnsignedPriority = VerifierPriority;
	type Check = Verifier;
	type Inspect = Assets;
	type Transfer = Assets;
	type CrowdFundingLimit = CrowdFundingLimit;
	type CrowdFundingMetadataDepositBase = CrowdFundingMetadataDepositBase;
	type MinBalance = MinBalance;
	type PalletId = CrowdfundingPalletId;
}

pub fn new_test_ext() -> sp_io::TestExternalities {
	let t = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();
	t.into()
}

pub fn built_in_verifiers() -> Vec<UintAuthorityId> {
	Session::validators().into_iter().map(UintAuthorityId).collect()
}

pub fn advance_session() {
	let now = System::block_number().max(1);
	System::set_block_number(now + 1);
	Session::rotate_session();
	let keys = built_in_verifiers();
	assert_eq!(Session::current_index(), (now / Period::get()) as u32);
}

use codec::FullCodec;
pub use frame_support::{
	traits::{BalanceStatus, ExistenceRequirement, LockIdentifier},
	transactional,
};
use sp_runtime::{
	traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize},
	DispatchError, DispatchResult,
};
// use sp_inherents::Error;
use frame_support::traits::Currency;
use sp_std::{
	cmp::{PartialEq},
	fmt::Debug,
};

use pallet_starks_verifier::{Check, ClassType};

pub trait RegulatedCurrency<AccountId> {
	/// The balance of an account.
	type Balance: AtLeast32BitUnsigned
		+ FullCodec
		+ Copy
		+ MaybeSerializeDeserialize
		+ Debug
		+ Default;

	/// Transfer some amount from one account to another.
	fn transfer(
		source: &AccountId,
		dest: &AccountId,
		value: Self::Balance,
		existence_requirement: ExistenceRequirement,
		kyc_verify: ClassType,
	) -> DispatchResult;
}

// #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[derive(Debug, Clone, PartialEq)]
pub struct Demostruct<
	T: frame_system::Config,
	C: Currency<AccountId>,
	F: Check<AccountId>,
	AccountId,
> {
	pub _phantom: sp_std::marker::PhantomData<(T, C, F, AccountId)>,
}

impl<T: frame_system::Config, C: Currency<AccountId>, F: Check<AccountId>, AccountId: Debug>
	RegulatedCurrency<AccountId> for Demostruct<T, C, F, AccountId>
{
	type Balance = <C as Currency<AccountId>>::Balance;

	fn transfer(
		source: &AccountId,
		dest: &AccountId,
		value: Self::Balance,
		existence_requirement: ExistenceRequirement,
		kyc_verify: ClassType,
	) -> DispatchResult {
		let kyc_check_result = F::checkkyc_with_verifykyc(source, kyc_verify);
		if kyc_check_result.is_ok() == true {
			if kyc_check_result.unwrap() == true {
				C::transfer(source, dest, value, existence_requirement)?;
			} else {
				return Err(DispatchError::Other("KYCNotPass".into()))
			}
		} else {
			return Err(DispatchError::Other("KYCNotPass".into()))
		}
		Ok(())
	}
}

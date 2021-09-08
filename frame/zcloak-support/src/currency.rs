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
use sp_std::{cmp::PartialEq, fmt::Debug};

// use pallet_starks_verifier::{Check, ClassType};
use primitives_catalog::{
	inspect::Inspect,
	regist::ClassTypeRegister,
	types::{ClassType, ProgramHash, PublicInputs},
};

pub trait RegulatedCurrency<AccountId> {
	type RCurrency: Currency<AccountId>;

	fn transfer(
		source: &AccountId,
		dest: &AccountId,
		value: <Self::RCurrency as Currency<AccountId>>::Balance,
		existence_requirement: ExistenceRequirement,
		class_type: ClassType,
		public_inputs: PublicInputs,
	) -> DispatchResult;
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegulatedCurrencyAdaptor<
	C: Currency<AccountId>,
	I: Inspect<AccountId>,
	R: ClassTypeRegister,
	AccountId,
> {
	pub _phantom: sp_std::marker::PhantomData<(C, I, R, AccountId)>,
}

impl<C: Currency<AccountId>, I: Inspect<AccountId>, R: ClassTypeRegister, AccountId: Debug>
	RegulatedCurrency<AccountId> for RegulatedCurrencyAdaptor<C, I, R, AccountId>
{
	type RCurrency = C;

	fn transfer(
		source: &AccountId,
		dest: &AccountId,
		value: <Self::RCurrency as Currency<AccountId>>::Balance,
		existence_requirement: ExistenceRequirement,
		class_type: ClassType,
		public_inputs: PublicInputs,
	) -> DispatchResult {
		let program_hash_result = R::get(&class_type);

		if program_hash_result.is_ok() {
			let check_result = I::check(source, program_hash_result.unwrap(), public_inputs);
			if check_result.is_ok() == true {
				if check_result.unwrap() == true {
					C::transfer(source, dest, value, existence_requirement)?;
				} else {
					return Err(DispatchError::Other("NotPass".into()))
				}
			} else {
				return Err(DispatchError::Other("NotPass".into()))
			}
		} else {
			return Err(DispatchError::Other("NotPass".into()))
		}
		Ok(())
	}
}

// pub trait RegulatedCurrency<AccountId> {
// 	/// The balance of an account.
// 	type Balance: AtLeast32BitUnsigned
// 		+ FullCodec
// 		+ Copy
// 		+ MaybeSerializeDeserialize
// 		+ Debug
// 		+ Default;

// 	/// Transfer some amount from one account to another.
// 	fn transfer(
// 		source: &AccountId,
// 		dest: &AccountId,
// 		value: Self::Balance,
// 		existence_requirement: ExistenceRequirement,
// 		kyc_verify: ClassType,
// 	) -> DispatchResult;
// }

// // #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// #[derive(Debug, Clone, PartialEq)]
// pub struct Demostruct<
// 	T: frame_system::Config,
// 	C: Currency<AccountId>,
// 	// F: Check<AccountId>,
// 	AccountId,
// > {
// 	pub _phantom: sp_std::marker::PhantomData<(T, C, AccountId)>,
// }

// impl<T: frame_system::Config, C: Currency<AccountId>, F: Check<AccountId>, AccountId: Debug>
// 	RegulatedCurrency<AccountId> for Demostruct<T, C, F, AccountId>
// {
// 	type Balance = <C as Currency<AccountId>>::Balance;

// 	fn transfer(
// 		source: &AccountId,
// 		dest: &AccountId,
// 		value: Self::Balance,
// 		existence_requirement: ExistenceRequirement,
// 		kyc_verify: ClassType,
// 	) -> DispatchResult {
// 		let kyc_check_result = F::checkkyc_with_verifykyc(source, kyc_verify);
// 		if kyc_check_result.is_ok() == true {
// 			if kyc_check_result.unwrap() == true {
// 				C::transfer(source, dest, value, existence_requirement)?;
// 			} else {
// 				return Err(DispatchError::Other("KYCNotPass".into()))
// 			}
// 		} else {
// 			return Err(DispatchError::Other("KYCNotPass".into()))
// 		}
// 		Ok(())
// 	}
// }

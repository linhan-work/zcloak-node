
use codec::{FullCodec};
pub use frame_support::{
	traits::{BalanceStatus, LockIdentifier, ExistenceRequirement},
	transactional,
};
use frame_system::Account;
use sp_runtime::{DispatchError, DispatchResult, traits::{AtLeast32BitUnsigned, MaybeSerializeDeserialize, Verify}};
// use sp_inherents::Error;
use sp_std::{
	cmp::{Eq, PartialEq},
	convert::{TryFrom, TryInto},
	fmt::Debug,
	result,
};
use sp_std::{vec, vec::Vec};
use frame_support::traits::Currency;

// use serde::{Serialize, Deserialize};
use sp_std::marker::PhantomData;
use pallet_starks_verifier::Check;
use codec::{Codec, Decode, Encode};
use pallet_starks_verifier::{ClassType, Junction};

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
pub struct Demostruct<T: frame_system::Config,
C: Currency<AccountId>,
F: Check<AccountId>,
AccountId,
>{
    pub _phantom: sp_std::marker::PhantomData<(T, C, F,AccountId)>
}



impl<T: frame_system::Config,
C: Currency<AccountId>,
F: Check<AccountId>,
AccountId: Debug,
> RegulatedCurrency<AccountId> for Demostruct<T, C, F,AccountId>
where 

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
                if kyc_check_result.unwrap() == true{
                    C::transfer(source, dest, value, existence_requirement)?;
                }else{
                    return Err(DispatchError::Other("KYCNotPass".into()))
                }
            }else{
                return Err(DispatchError::Other("KYCNotPass".into()))
            }
        Ok(())
    }
}
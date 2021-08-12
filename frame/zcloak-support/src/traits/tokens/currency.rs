
use super::arithmetic;
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

#[derive(Eq, PartialEq, Copy, Clone, PartialOrd, Ord, Debug, Encode, Decode)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum VerifyClass {
	Age,
    Country,
}

#[derive(Debug)]
pub enum Error {
    WrongKYCToVerify,
}

pub trait VerifyKyc{
    fn get_program_hash(&self) -> Result<[u8; 32], Error>;
    fn get_kyc_class(&self) -> Result<Vec<u8>, Error>;
}

// Todo
impl VerifyKyc for VerifyClass{
    fn get_program_hash(&self) -> Result<[u8; 32], Error> {
        match *self {
            VerifyClass::Age => Ok([228,150,141,219,97,232,23,59,109,33,136,11,72,175,77,167,38,2,251,107,254,126,91,63,176,46,204,254,90,153,168,40]),
            VerifyClass::Country => Ok([208, 194, 130, 197, 164, 24, 192, 43, 169, 199, 5, 5, 30, 49, 190, 137, 168, 29, 175, 111, 254, 108, 138, 242, 161, 201, 76, 10, 238, 140, 97, 14]),  
            _ => Err(Error::WrongKYCToVerify),
        }
    }
    fn get_kyc_class(&self) -> Result<Vec<u8>, Error>{
        match *self {
            VerifyClass::Age => Ok([21].to_vec()),
            VerifyClass::Country => Ok([22].to_vec()), 
            _ => Err(Error::WrongKYCToVerify),
 
        }   
     }
}

pub trait RegulatedCurrency<AccountId> {
	/// The balance of an account.
	type Balance: AtLeast32BitUnsigned
    + FullCodec
    + Copy
    + MaybeSerializeDeserialize
    + Debug
    + Default;
	
    type VerifyKyc : VerifyKyc;

	/// Transfer some amount from one account to another.
 	fn transfer(
		source: &AccountId,
		dest: &AccountId,
		value: Self::Balance,
		existence_requirement: ExistenceRequirement,
        kyc_verify: VerifyClass,

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

    type VerifyKyc = VerifyClass;


    fn transfer(
		source: &AccountId,
		dest: &AccountId,
		value: Self::Balance,
		existence_requirement: ExistenceRequirement,
        kyc_verify: Self::VerifyKyc,
	) -> DispatchResult {
        let kyc_program = kyc_verify.get_program_hash();
        let kyc_class = kyc_verify.get_kyc_class();
        if kyc_program.is_ok() && kyc_class.is_ok(){
            let kyc_check_result = F::checkkyc(source, kyc_class.unwrap(),kyc_program.unwrap());
            if kyc_check_result.is_ok() == true {
                if kyc_check_result.unwrap() == true{
                    C::transfer(source, dest, value, existence_requirement)?;
                }else{
                    return Err(DispatchError::Other("KYCNotPass".into()))
                }
            }else{
                return Err(DispatchError::Other("KYCNotPass".into()))
            }
        }else{
            return Err(DispatchError::Other("KYCClassWrong".into()))
        }
        Ok(())
    }
}
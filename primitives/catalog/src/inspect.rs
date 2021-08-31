use crate::types::{ProgramHash, PublicInputs};
use codec::{Decode, Encode};

pub trait Inspect<T> {
    fn check(
		who: &T,
		program_hash: ProgramHash,
		public_inputs: PublicInputs,
	) -> Result<bool, CheckError>;

}

#[derive(Clone, Encode, Decode, PartialEq, Eq, Debug)]
pub enum CheckError {
	//Not on chain
	VerifyFailedNotOnChain,
	//Class type onchain, but not allowed to do crowdfunding
	VerifyFailedNotAllowed,
	//Class type onchain, but the corresponding program is not ICOprogram
	VerifyFailedTaskProgramWrong,
	VerifyNotCorrect,
}


use crate::types::{ClassType, ProgramHash};
use codec::{Decode, Encode};


pub trait ClassTypeRegister {

	fn register(class_type: &ClassType, program_hash: &ProgramHash) -> Result<bool, ClassError>;

	fn get(class_type: &ClassType) -> Result<[u8; 32], ClassError>;

	fn remove(class_type: &ClassType) -> Result<bool, ClassError>;
}

#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, Debug)]
pub enum ClassError {
	// Not on chain
	ClassNotExist,
	//  Class Not Fit Program On Chain
	ClassNotFitProgramOnChain,
	//  Class already exist
	ClassAlreadyExist,
}

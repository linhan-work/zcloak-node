#![cfg_attr(not(feature = "std"), no_std)]

use codec::{Decode, Encode};
use sp_std::vec::Vec;

#[derive(PartialEq, Clone, Debug, Encode, Decode, PartialOrd, Eq, Ord)]
pub enum ClassType {
	Null,
	X1(ProgramType),
	X2(ProgramType, ProgramType),
	X3(ProgramType, ProgramType, ProgramType),
	X4(ProgramType, ProgramType, ProgramType, ProgramType),
	X5(ProgramType, ProgramType, ProgramType, ProgramType, ProgramType),
	X6(ProgramType, ProgramType, ProgramType, ProgramType, ProgramType, ProgramType),
}

impl Default for ClassType {
	fn default() -> Self {
		Self::Null
	}
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug)]
pub enum ProgramType {
	Null,
	Age(ProgramOption),
	Country(ProgramOption),
	Other(Vec<u8>),
}

type Index = u32;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug)]
pub enum ProgramOption {
	Null,
	Index(Index),
	Range(Range),
	Other(Vec<u8>),
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Encode, Decode, Debug)]
pub enum Range {
	LargeThan,
	SmallerThan,
	Between,
}

pub type ProgramHash = [u8; 32];

pub type PublicInputs = Vec<u128>;

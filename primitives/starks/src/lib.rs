#![warn(missing_docs)]
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc_error_handler))]
#![cfg_attr(
	feature = "std",
	doc = "Substrate runtime standard library as compiled when linked with Rust's standard library."
)]
#![cfg_attr(
	not(feature = "std"),
	doc = "Substrate's runtime standard library as compiled without Rust's standard library."
)]

use codec::{Decode, Encode};
use sp_runtime_interface::{pass_by::PassByCodec, runtime_interface};

use sp_runtime::RuntimeDebug;

#[derive(PassByCodec, Encode, Decode, RuntimeDebug)]
/// VerifyErr before verification
pub enum VerifyErr {
	/// SerializeErr
	SerializeErr,
	/// DistaffVerifyErr
	DistaffVerifyErr,
	/// NoUTF8
	NoUTF8,
	/// NoHex
	NoHex,
}
/// doc
#[runtime_interface]
pub trait Starks {
	/// To verify with starksVM
	fn verify(
		&mut self,
		program_hash: &[u8; 32],
		public_inputs: &[u128],
		outputs: &[u128],
		proof: &[u8],
	) -> Result<bool, VerifyErr> {
		let body_str = sp_std::str::from_utf8(&proof).map_err(|_| VerifyErr::NoUTF8)?;

		let proof = hex::decode(&body_str[0..body_str.len()]).map_err(|_| VerifyErr::NoHex)?;
		let stark_proof = bincode::deserialize::<starksVM::StarkProof>(&proof)
			.map_err(|_| VerifyErr::SerializeErr)?;

		let res = starksVM::verify(program_hash, public_inputs, outputs, &stark_proof);

		res.map_err(|_| VerifyErr::DistaffVerifyErr)
	}
}

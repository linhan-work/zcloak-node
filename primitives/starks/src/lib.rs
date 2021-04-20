
#![warn(missing_docs)]

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(not(feature = "std"), feature(alloc_error_handler))]

#![cfg_attr(feature = "std",
   doc = "Substrate runtime standard library as compiled when linked with Rust's standard library.")]
#![cfg_attr(not(feature = "std"),
   doc = "Substrate's runtime standard library as compiled without Rust's standard library.")]


use sp_runtime_interface::{runtime_interface, Pointer,
	pass_by::{PassBy, PassByCodec}
};
use codec::{Encode, Decode};
#[cfg(feature = "std")]
use distaff::StarkProof;

#[derive(PassByCodec, Encode, Decode)]
pub enum VerifyErr {
	SerializeErr,
	DistaffVerifyErr
}

#[runtime_interface]
pub trait Starks {
	fn verify(
		&mut self,
		program_hash: &[u8; 32],
		public_inputs: &[u128],
		outputs: &[u128],
		proof: &[u8]) -> Result<bool, VerifyErr>
	{
		log::info!(target: "starks-verifier", "@@@@@@@ Start to stark verify");
		let stark_proof = bincode::deserialize::<StarkProof>(&proof).map_err(|_| VerifyErr::SerializeErr)?;
		log::debug!(target: "starks-verifier", "$$$$$$ StarkProof Parsing is ok");
		let res = distaff::verify(program_hash, public_inputs, outputs, &stark_proof);
		
		log::debug!(target: "starks-verifier", "@@##$$@@##$$@@##$$ THE RES IS {:?}", res);
		
		return Err(VerifyErr::DistaffVerifyErr)
	}
}

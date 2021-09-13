//! # Starks-verifier Module
//! A module for validated offchain-workers to verify tasks.
//!
//!
//! ## Overview
//!
//! This module is used for off-chain workers to verify the tasks stored on chain,
//! including the submission of tasks, the verification of a single off-chain worker on the task,
//! and the `pass or not`results are stored on-chain at the moment the verification is done.
//! When the number of ayes or nays in the verification result exceeds the set threshold,
//! the final verification result will be stored on-chain in the form of SettledTask,
//! and the SettledTask will automatically expire after the set time.
//!
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! * `create_task` - Create a task with program_has h,inputs, outputs, proof_id.
//! * `offchain_worker` - For validated offchain-workers to dispatch only,in order to
//! verify tasks.
//! * `on_finalize` - Remove SettledTask which is expired at this block
//!
//!
//!
// Ensure we're `no_std` when compiling for Wasm.
#![cfg_attr(not(feature = "std"), no_std)]
use codec::{Decode, Encode};
use sp_application_crypto::RuntimeAppPublic;

use frame_support::traits::OneSessionHandler;
use sp_core::crypto::KeyTypeId;
use sp_runtime::{
	offchain::{http, storage::StorageValueRef, Duration},
	RuntimeDebug, SaturatedConversion,
};
use sp_std::{borrow::ToOwned, collections::btree_set::BTreeSet, iter::FromIterator, prelude::*};
// use frame_system::{ensure_signed, ensure_none};
use frame_system::offchain::{SendTransactionTypes, SubmitTransaction};

use sp_runtime::offchain::storage::{MutateStorageError, StorageRetrievalError};

use primitives_catalog::{
	inspect::{CheckError, Inspect},
	regist::ClassTypeRegister,
};

extern crate alloc;

#[cfg(all(feature = "std", test))]
mod mock;

#[cfg(all(feature = "std", test))]
mod tests;

pub use pallet::*;

/// The key type of which to sign the starks verification transactions
pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"zkst");

pub mod crypto {
	pub mod app_sr25519 {
		pub use crate::KEY_TYPE;
		use sp_application_crypto::{app_crypto, sr25519};
		app_crypto!(sr25519, KEY_TYPE);
	}

	sp_application_crypto::with_pair! {
		/// A starks verifier keypair using sr25519 as its crypto.
		pub type AuthorityPair = app_sr25519::Pair;
	}

	/// A starks verifier signature using sr25519 as its crypto.
	pub type AuthoritySignature = app_sr25519::Signature;

	/// A starks verifier identifier using sr25519 as its crypto.
	pub type AuthorityId = app_sr25519::Public;
}

#[derive(Debug)]
pub enum Error {
	WrongClassTypeToVerify,
}

/// The status of a given verification task
#[derive(Encode, Decode, Default, PartialEq, Eq, RuntimeDebug)]
pub struct Status {
	// The verifiers involved so far
	pub verifiers: Vec<u32>,
	// The number of affirmative vote so far
	pub ayes: u32,
	// The number of dissenting vote so far
	pub nays: u32,
}

/// Receipt about any verification occured
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct VerificationReceipt<AccountId, BlockNumber> {
	task_tuple_id: (AccountId, ([u8; 32], Vec<u128>)),
	// The Hash of a certain task to be verified
	program_hash: [u8; 32],
	// The vec<128> of program public input
	public_inputs: Vec<u128>,
	// Whether a task is passed or not
	passed: bool,
	// Block number at the time submission is created.
	submit_at: BlockNumber,
	// Submitted by who
	auth_index: u32,
	// The length of session validator set
	validators_len: u32,
}

#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq)]
pub enum TaskStatus {
	JustCreated,
	Verifying,
	VerifiedTrue,
	VerifiedFalse,
}

impl sp_std::fmt::Debug for TaskStatus {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			TaskStatus::JustCreated => write!(fmt, "JustCreated"),
			TaskStatus::Verifying => write!(fmt, "Verifying"),
			TaskStatus::VerifiedTrue => write!(fmt, "VerifiedTrue"),
			TaskStatus::VerifiedFalse => write!(fmt, "VerifiedFalse"),
		}
	}
}

/// Info of a certain task
#[derive(Encode, Decode, Default, PartialEq, Eq, RuntimeDebug)]
pub struct TaskInfo<BlockNumber> {
	// The id of the proof,combined with a url to fetch the complete proof later
	proof_id: Vec<u8>,
	// Inputs of the task
	public_inputs: Vec<u128>,
	// Outputs of the task
	outputs: Vec<u128>,
	// The hash of the program
	program_hash: [u8; 32],
	// If false,expiration is the time task created;
	// If true ,expiration is the time task expired.
	is_task_finish: Option<TaskStatus>,
	expiration: Option<BlockNumber>,
}

/// Error which may occur while executing the off-chain code.
#[cfg_attr(test, derive(PartialEq))]
pub enum OffchainErr<BlockNumber> {
	NotValidator,
	NoTaskToExecute,
	FailToAcquireLock,
	FailedToFetchProof,
	FailedSigning,
	SubmitTransaction(BlockNumber),
	VerificationFailed,
	NotMyTurn(BlockNumber),
}

impl<BlockNumber: sp_std::fmt::Debug> sp_std::fmt::Debug for OffchainErr<BlockNumber> {
	fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
		match *self {
			OffchainErr::NotValidator => write!(fmt, "Failed to sign heartbeat"),
			OffchainErr::NoTaskToExecute => write!(fmt, "No task to Execute"),
			OffchainErr::FailToAcquireLock => write!(fmt, "Failed to acquire lock"),
			OffchainErr::FailedToFetchProof => write!(fmt, "Failed to fetch proof"),
			OffchainErr::FailedSigning => write!(fmt, "Failed to sign the result"),
			OffchainErr::SubmitTransaction(ref now) =>
				write!(fmt, "Failed to submit transaction at block {:?}", now),
			OffchainErr::VerificationFailed => write!(fmt, "Failed to verify"),
			OffchainErr::NotMyTurn(ref now) => write!(fmt, "Block {:?} is not my turn", now),
		}
	}
}

const DB_PREFIX: &[u8] = b"starksnetwork/verification-tasks/";

pub type OffchainResult<T, A> = Result<A, OffchainErr<<T as frame_system::Config>::BlockNumber>>;

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::{dispatch::DispatchResult, pallet_prelude::*};
	use frame_system::pallet_prelude::*;

	#[pallet::config]
	#[pallet::disable_frame_system_supertrait_check]
	pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
		/// The identifier type for an offchain worker.
		type AuthorityId: Member
			+ Parameter
			+ RuntimeAppPublic
			+ Default
			+ Ord
			+ MaybeSerializeDeserialize;
		/// The overarching event type.
		type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
		/// A type for retrieving the validators supposed to be online in a session.
		// type ValidatorSet: ValidatorSetWithIdentification<Self::AccountId>;

		/// After a task is verified, it can still be stored on chain for a `StorePeriod` of time
		#[pallet::constant]
		type StorePeriod: Get<Self::BlockNumber>;

		/// A configuration for base priority of unsigned transactions.
		///
		/// This is exposed so that it can be tuned for particular runtime, when
		/// multiple pallets send unsigned transactions.
		#[pallet::constant]
		type UnsignedPriority: Get<TransactionPriority>;

		type Register: ClassTypeRegister;
	}

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(PhantomData<T>);

	#[pallet::storage]
	#[pallet::getter(fn keys)]
	/// Current set of keys that are allowed to execute verification tasks
	pub(super) type Keys<T: Config> = StorageValue<_, Vec<T::AuthorityId>, ValueQuery>;

	#[pallet::storage]
	#[pallet::getter(fn task_params)]
	/// Map from the task_params to the TaskInfo(proof_id,inputs,outputs)
	pub(super) type TaskParams<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		([u8; 32], Vec<u128>),
		TaskInfo<T::BlockNumber>,
		ValueQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn ongoing_tasks)]
	/// Record the verification tasks that are about to be verified or under verifying.
	/// The key is program hash
	pub(super) type OngoingTasks<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::AccountId,
		Twox64Concat,
		([u8; 32], Vec<u128>),
		Status,
		OptionQuery,
	>;

	#[pallet::storage]
	#[pallet::getter(fn settled_tasks)]
	/// Completed proof tasks, will be stored onchain for a short period to be challenged
	pub(super) type SettledTasks<T: Config> = StorageDoubleMap<
		_,
		Twox64Concat,
		T::BlockNumber,
		Twox64Concat,
		(T::AccountId, [u8; 32], Vec<u128>),
		Option<bool>,
		ValueQuery,
	>;

	#[pallet::genesis_config]
	pub struct GenesisConfig<T: Config> {
		_phantom: sp_std::marker::PhantomData<T>,
	}

	#[cfg(feature = "std")]
	impl<T: Config> Default for GenesisConfig<T> {
		fn default() -> Self {
			GenesisConfig { _phantom: Default::default() }
		}
	}

	#[pallet::genesis_build]
	impl<T: Config> GenesisBuild<T> for GenesisConfig<T> {
		fn build(&self) {
			// Pallet::<T>::initialize_kyclist()
		}
	}

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	#[pallet::metadata(T::AccountId = "AccountId")]
	pub enum Event<T: Config> {
		/// A new verifier is added with `AccountId`.
		AddVerifier(T::AccountId),
		/// A verifier is removed with `AccountId`.
		RemoveVerifier(T::AccountId),
		/// A new task is created.
		TaskCreated(T::AccountId, [u8; 32], Vec<u128>, Vec<u8>),
		/// A verification submitted on chain
		VerificationSubmitted(T::AccountId, [u8; 32], Vec<u128>, bool, u32, u32, u32),
		/// A verification submitted by a single verifier
		SingleVerification(T::AccountId, [u8; 32], Vec<u128>, bool, u32, u32),
	}

	#[pallet::error]
	pub enum Error<T> {
		/// It's not allowed to recreated an existed task.
		TaskAlreadyExists,
		/// Only permitted verifiers can submit the result
		NotAllowed,
		/// Task does not exist
		TaskNotExists,
		/// Duplicated Submission
		DuplicatedSubmission,
		/// KYCListNotHaveThisOne
		ClassTypeListNotHaveThisOne,
		/// KYCListAlreadyHaveThisOne
		ClassTypeListAlreadyHaveThisOne,
		/// KYCClassIsEmpty
		ClassTypeIsEmpty,
		/// KYCProgramIsEmpty
		ProgramIsEmpty,
		/// class type not store on chain
		ClassTypeNotExsit,
	}

	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// To create a new task for verifiers to verify,make sure that this task hasn't be stored on-chain yet.
		/// If qualified,store the task on-chain ( <TaskParam> & <OngoingTasks> )
		///
		/// The dispatch origin for this call must be _Signed_.
		///
		/// - `program_hash`: The hash of task to be verified.
		/// - `inputs`: Inputs of the task.
		/// - `outputs`: Outputs of the task.
		/// - `proof_id`: The id of the proof,combined with a url to fetch the complete proof later
		///
		/// If the Task created successfully, deposit the `TaskCreated` event.
		#[pallet::weight(10000)]
		pub fn create_task(
			origin: OriginFor<T>,
			program_hash: [u8; 32],
			public_inputs: Vec<u128>,
			outputs: Vec<u128>,
			proof_id: Vec<u8>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			// Ensure task has not been created
			ensure!(
				!TaskParams::<T>::try_get(&who, (&program_hash, &public_inputs)).is_ok(),
				Error::<T>::TaskAlreadyExists
			);
			<TaskParams<T>>::insert(
				&who,
				(&program_hash, &public_inputs),
				TaskInfo {
					proof_id: proof_id.clone(),
					public_inputs: public_inputs.clone(),
					outputs,
					program_hash,
					is_task_finish: Some(TaskStatus::JustCreated),
					expiration: Some(<frame_system::Pallet<T>>::block_number()),
				},
			);
			<OngoingTasks<T>>::insert(&who, (&program_hash, &public_inputs), Status::default());
			Self::deposit_event(Event::TaskCreated(who, program_hash, public_inputs, proof_id));
			Ok(())
		}

		/// Submit a verification with certain receipt.
		///
		/// The dispatch origin for this call must represent an unsigned extrinsin.
		///
		/// - `receipt`: Receipt about a verification occured
		///
		/// Once the number of affirmative vote or dissenting vote above the threshold,store it on-chain(`SettledTask`)
		/// The last parameter of `SettleTask` represents the task if passed or not.
		#[pallet::weight(10000)]
		pub fn submit_verification(
			origin: OriginFor<T>,
			receipt: VerificationReceipt<T::AccountId, T::BlockNumber>,
			_signature: <T::AuthorityId as RuntimeAppPublic>::Signature,
		) -> DispatchResult {
			ensure_none(origin)?;
			let account = receipt.clone().task_tuple_id.0;
			let program_hash = receipt.clone().task_tuple_id.1 .0;
			let public_inputs = receipt.clone().task_tuple_id.1 .1;
			<OngoingTasks<T>>::try_mutate_exists(
				&account.clone(),
				(&program_hash.clone(), &public_inputs.clone()),
				|last_status| -> DispatchResult {
					// Last status must exist.Fetch last status,if not exists return error
					let mut status = last_status.take().ok_or(Error::<T>::TaskNotExists)?;
					// A verifier can not submit more than once
					ensure!(
						!status.verifiers.contains(&receipt.auth_index),
						Error::<T>::DuplicatedSubmission
					);
					// Update the verifier list
					status.verifiers.push(receipt.auth_index);
					// > 50%
					let threshold = (Self::authority_len() + 1) / 2;
					// Adjust ayes or nays according to the receipt.
					if receipt.passed {
						status.ayes += 1;
					} else {
						status.nays += 1;
					}
					// Change expiration.
					let TaskInfo {
						proof_id, public_inputs, outputs, program_hash, expiration, ..
					} = Self::task_params(&account, (&program_hash, &public_inputs));
					Self::deposit_event(Event::SingleVerification(
						account.clone(),
						program_hash.clone(),
						public_inputs.clone(),
						receipt.passed,
						status.ayes.clone(),
						Self::authority_len(),
					));
					<TaskParams<T>>::insert(
						&account,
						(&program_hash, &public_inputs),
						TaskInfo {
							proof_id: proof_id.clone(),
							public_inputs: public_inputs.clone(),
							outputs: outputs.clone(),
							program_hash,
							is_task_finish: Some(TaskStatus::Verifying),
							expiration,
						},
					);
					let expiration = receipt.submit_at + T::StorePeriod::get();
					// If ayes > threshold，pass the task and store it on-chain with a `true`.

					if status.ayes >= threshold {
						// Pass the verification
						SettledTasks::<T>::insert(
							expiration,
							&(account.clone(), program_hash.clone(), public_inputs.clone()),
							Some(true),
						);
						<TaskParams<T>>::insert(
							&account,
							(&program_hash, &public_inputs),
							TaskInfo {
								proof_id,
								public_inputs: public_inputs.clone(),
								outputs,
								program_hash,
								is_task_finish: Some(TaskStatus::VerifiedTrue),
								expiration: Some(expiration),
							},
						);
						*last_status = None;
						Self::deposit_event(Event::VerificationSubmitted(
							account.clone(),
							program_hash.clone(),
							public_inputs.clone(),
							true,
							status.ayes,
							status.nays,
							Self::authority_len(),
						));
					// If nays > threshold，reject the task and store it on-chain with a `false`.
					} else if status.nays >= threshold {
						// fail the verification
						SettledTasks::<T>::insert(
							expiration,
							(account.clone(), program_hash.clone(), public_inputs.clone()),
							Some(false),
						);
						<TaskParams<T>>::insert(
							&account,
							(&program_hash, &public_inputs),
							TaskInfo {
								proof_id,
								public_inputs: public_inputs.clone(),
								outputs,
								program_hash,
								is_task_finish: Some(TaskStatus::VerifiedFalse),
								expiration: Some(expiration),
							},
						);
						*last_status = None;
						Self::deposit_event(Event::VerificationSubmitted(
							account,
							program_hash,
							public_inputs,
							false,
							status.ayes,
							status.nays,
							Self::authority_len(),
						));
					} else {
						// Otherwise, update the task status
						*last_status = Some(status);
					}
					Ok(())
				},
			)
		}
	}

	// Runs after every block.
	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		fn on_finalize(block: T::BlockNumber) {			
			let mut res = SettledTasks::<T>::iter_prefix(block).collect::<Vec<_>>();
			let size = res.len();
			for _i in 0..size{
				let round = res.pop().unwrap();
				let first = round.0;
				let account_id = first.0;
				let program_hash = first.1;
				let public_input = first.2;
				TaskParams::<T>::remove(account_id, (program_hash, public_input));
			}
			SettledTasks::<T>::remove_prefix(block, None);
		}

		fn offchain_worker(now: T::BlockNumber) {
			// Only send messages if we are a potential validator.
			if sp_io::offchain::is_validator() {
				for res in Self::send_verification_output(now).into_iter().flatten() {
					if let Err(e) = res {
						log::warn!(
							target: "starks-verifier",
							"Skipping verifying at {:?}: {:?}",
							now,
							e,
						)
					}
				}
			} else {
				log::trace!(
					target: "starks-verifier",
					"Skipping verifying at {:?}. Not a validator.",
					now,
				)
			}
		}
	}
	/// Invalid transaction custom error. Returned when validators_len field in Receipt is incorrect.
	const INVALID_VALIDATORS_LEN: u8 = 10;
	#[pallet::validate_unsigned]
	impl<T: Config> frame_support::unsigned::ValidateUnsigned for Pallet<T> {
		type Call = Call<T>;

		fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {
			if let Call::submit_verification(receipt, signature) = call {
				if receipt.submit_at > <frame_system::Pallet<T>>::block_number() {
					return InvalidTransaction::Future.into()
				}

				// Verify that the incoming (unverified) pubkey is actually an authority id
				let keys = Keys::<T>::get();
				if keys.len() as u32 != receipt.validators_len {
					return InvalidTransaction::Custom(INVALID_VALIDATORS_LEN).into()
				}

				let authority_id = match keys.get(receipt.auth_index as usize) {
					Some(id) => id,
					None => return InvalidTransaction::BadProof.into(),
				};

				// Check signature (this is expensive so we do it last).
				let signature_valid = receipt.using_encoded(|encoded_receipt| {
					authority_id.verify(&encoded_receipt, &signature)
				});

				if !signature_valid {
					return InvalidTransaction::BadProof.into()
				}

				ValidTransaction::with_tag_prefix("StarksVerifier")
					.priority(T::UnsignedPriority::get())
					.and_provides((receipt.program_hash, authority_id))
					.longevity(5)
					.propagate(true)
					.build()
			} else {
				InvalidTransaction::Call.into()
			}
		}
	}
}

impl<T: Config> Pallet<T> {
	/// The internal entry of offchain worker   
	/// Send verification with index of on-chain authorities and its corresponding local public key
	pub(crate) fn send_verification_output(
		block_number: T::BlockNumber,
	) -> OffchainResult<T, impl Iterator<Item = OffchainResult<T, ()>>> {
		Ok(Self::local_authority_keys().map(move |(auth_index, key)| {
			Self::send_single_result_with_lock(auth_index, key, block_number)
		}))
	}

	fn send_single_result_with_lock(
		auth_index: u32,
		key: T::AuthorityId,
		block_number: T::BlockNumber,
	) -> OffchainResult<T, ()> {
		let res = Self::is_my_turn(auth_index, block_number);
		if res {
			let storage_key = {
				let mut prefix = DB_PREFIX.to_vec();
				prefix.extend(auth_index.encode());
				prefix
			};

			let storage = StorageValueRef::persistent(&storage_key);

			let mut task_id_tuple: (T::AccountId, ([u8; 32], Vec<u128>)) = Default::default();
			let mut initial_local_tasks = BTreeSet::new();

			let res = storage.mutate(
				|tasks: Result<
					Option<Option<BTreeSet<(T::AccountId, ([u8; 32], Vec<u128>))>>>,
					StorageRetrievalError,
				>| {
					// Check if there is already a lock for that particular task.(hash)
					// If there exists a vec of `local_tasks`,we will attempt to find a certian task which is
					// stored on-chain(<OngoingTask>) but not verified locally yet.
					// If such vec doesn't exist ,we will initialize it,with the fist task to be verified on-chain(<OngoingTask>)
					match tasks {
						Ok(Some(Some(mut local_tasks))) => {
							task_id_tuple = Self::task_to_execute(&mut local_tasks)?;
							// TODO: fetch_proof, verify, and submit
							local_tasks.insert(task_id_tuple.clone());
							Ok(Some(local_tasks))
						},
						_ => {
							task_id_tuple = Self::task_to_execute(&mut BTreeSet::new())?;
							initial_local_tasks.insert(task_id_tuple.clone());
							Ok(Some(initial_local_tasks))
						},
					}
				},
			);
			if let Err(MutateStorageError::ValueFunctionFailed(err)) = res {
				return Err(err)
			}

			let local_tasks = res.map_err(|_| OffchainErr::FailToAcquireLock)?;

			// We got the lock, and do the fetch, verify, sign and send
			let res =
				Self::prepare_submission(auth_index, key, block_number, task_id_tuple.clone());

			// Clear the lock in case we have failed to send transaction.
			if res.is_err() {
				storage.set(&local_tasks.unwrap().remove(&task_id_tuple));

				// local_tasks.remove(&task_id_tuple);
				// storage.set(&local_tasks);
			}
			res
		} else {
			return Err(OffchainErr::NotMyTurn(block_number))
		}
	}

	/// To justify if a validator should do verification in this block
	fn is_my_turn(auth_index: u32, block_number: T::BlockNumber) -> bool {
		let number: u32 = block_number.saturated_into();
		return number % 2 == auth_index % 2
	}

	/// Fetch proof, verify, sign and submit the transaction
	fn prepare_submission(
		auth_index: u32,
		key: T::AuthorityId,
		block_number: T::BlockNumber,
		task_tuple_id: (T::AccountId, ([u8; 32], Vec<u128>)),
	) -> OffchainResult<T, ()> {
		let TaskInfo { proof_id, public_inputs, outputs, program_hash, .. } =
			Self::task_params(&task_tuple_id.0, (&task_tuple_id.1 .0, &task_tuple_id.1 .1));

		let mut proof_is_unknow = false;
		// To fetch proof and verify it.
		let proof_result = Self::fetch_proof(&proof_id);

		if let Err(e) = &proof_result {
			match e {
				http::Error::Unknown => {
					proof_is_unknow = true;
				},
				_ => return Err(OffchainErr::FailedToFetchProof),
			}
		}
		let mut res = false;
		if !proof_is_unknow {
			if let Ok(proof) = &proof_result {
				let is_success =
					Self::stark_verify(&program_hash, public_inputs.clone(), outputs, &proof);
				res = if let Ok(r) = is_success { r } else { false };
			}
		}

		let validators_len = Keys::<T>::decode_len().unwrap_or_default() as u32;
		//Create and initialize a verification receipt
		let receipt = VerificationReceipt {
			task_tuple_id,
			program_hash,
			public_inputs,
			passed: res,
			submit_at: block_number,
			auth_index,
			validators_len,
		};

		let signature = key.sign(&receipt.encode()).ok_or(OffchainErr::FailedSigning)?;
		let call = Call::submit_verification(receipt, signature);

		log::info!(
			target: "starks-verifier",
			"[index: {:?} report verification: {:?},  at block: {:?}]",
			auth_index,
			call,
			block_number
		);

		SubmitTransaction::<T, Call<T>>::submit_unsigned_transaction(call.into())
			.map_err(|_| OffchainErr::SubmitTransaction(block_number))?;

		Ok(())
	}

	/// Fetch the proof     
	fn fetch_proof(proof_id: &Vec<u8>) -> Result<Vec<u8>, http::Error> {
		let deadline = sp_io::offchain::timestamp().add(Duration::from_millis(100_000));
		// Combine the the `proof_id` with a certain url
		let url = "https://ipfs.infura.io:5001/api/v0/cat?arg=".to_owned() +
			sp_std::str::from_utf8(proof_id).unwrap();

		let request = http::Request::get(&url);

		let pending = request.deadline(deadline).send().map_err(|_| http::Error::IoError)?;
		let response = pending.try_wait(deadline).map_err(|_| http::Error::DeadlineReached)??;

		// Let's check the status code before we proceed to reading the response.
		if response.code != 200 {
			return Err(http::Error::Unknown)
		}

		// Next we want to fully read the response body and collect it to a vector of bytes.
		// Note that the return object allows you to read the body in chunks as well
		// with a way to control the deadline.
		let body = response.body().collect::<Vec<u8>>();
		// log::warn!(target: "starks-verifier", " body is : {:?}", body.clone());

		Ok(body)
	}

	/// Use Stark_verify to verify every program_hash with proof
	fn stark_verify(
		program_hash: &[u8; 32],
		public_inputs: Vec<u128>,
		outputs: Vec<u128>,
		proof: &[u8],
	) -> OffchainResult<T, bool> {
		sp_starks::starks::verify(program_hash, &public_inputs, &outputs, proof)
			.map_err(|_| OffchainErr::VerificationFailed)
	}

	// Return index of on-chain authorities and its corresponding local public key
	fn local_authority_keys() -> impl Iterator<Item = (u32, T::AuthorityId)> {
		// On-chain storage
		let authorities = Keys::<T>::get();
		// Local keystore
		let mut local_keys = T::AuthorityId::all();
		local_keys.sort();
		authorities.into_iter().enumerate().filter_map(move |(index, authority)| {
			local_keys
				.binary_search(&authority)
				.ok()
				.map(|location| (index as u32, local_keys[location].clone()))
		})
	}

	/// Pick an on-chain tasks to execute which is not included in `local_tasks`
	fn task_to_execute(
		local_tasks: &BTreeSet<(T::AccountId, ([u8; 32], Vec<u128>))>,
	) -> OffchainResult<T, (T::AccountId, ([u8; 32], Vec<u128>))> {
		//On-chain ready-to-verify tasks,put all task_hash of OngoingTasks into a vec.
		let ongoing_tasks_list = BTreeSet::from_iter(OngoingTasks::<T>::iter().map(
			|(account_id, (program_hash, public_inputs), _)| {
				(account_id, (program_hash, public_inputs))
			},
		));

		// Find any one task that is not executed
		// TODO： find all tasks that are not executed
		let mut tasks_not_executed: Vec<_> =
			ongoing_tasks_list.difference(local_tasks).cloned().collect();

		// return the task to execute
		tasks_not_executed.pop().ok_or(OffchainErr::NoTaskToExecute)
	}

	fn authority_len() -> u32 {
		Self::keys().len() as u32
	}

	fn initialize_keys(keys: &[T::AuthorityId]) {
		if !keys.is_empty() {
			assert!(Keys::<T>::get().is_empty(), "Keys are already initialized!");
			Keys::<T>::put(keys);
		}
	}

	#[allow(dead_code)]
	fn set_keys(keys: &Vec<T::AuthorityId>) {
		Keys::<T>::put(&keys)
	}
}

impl<T: Config> Inspect<T::AccountId> for Pallet<T> {
	fn check(
		who: &T::AccountId,
		program_hash: [u8; 32],
		public_inputs: Vec<u128>,
	) -> Result<bool, CheckError> {
		let is_exist = TaskParams::<T>::try_get(who, (&program_hash, &public_inputs)).is_ok();
		if is_exist {
			let TaskInfo { is_task_finish, .. } =
				Self::task_params(who, (&program_hash, &public_inputs));
			match is_task_finish.unwrap() {
				TaskStatus::VerifiedTrue => return Ok(true),
				_ => return Err(CheckError::VerifyFailedNotAllowed),
			}
		} else {
			return Err(CheckError::VerifyFailedNotOnChain)
		}
	}
}

impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
	type Public = T::AuthorityId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
	type Key = T::AuthorityId;

	fn on_genesis_session<'a, I: 'a>(validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
	{
		let keys = validators.map(|x| x.1).collect::<Vec<_>>();
		Self::initialize_keys(&keys);
	}

	fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_validators: I)
	where
		I: Iterator<Item = (&'a T::AccountId, T::AuthorityId)>,
	{
		// Remember who the authorities are for the new session.
		Keys::<T>::put(validators.map(|x| x.1).collect::<Vec<_>>());
	}

	fn on_disabled(_i: usize) {
		// Ignore
	}
}

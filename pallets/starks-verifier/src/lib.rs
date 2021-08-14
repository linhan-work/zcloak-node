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
use sp_application_crypto::RuntimeAppPublic;
use codec::{Encode, Decode};
use codec::{Codec, MaxEncodedLen};

use sp_std::prelude::*;
use sp_std::{
    borrow::ToOwned,
    iter::FromIterator,
    collections::btree_set::BTreeSet,
    convert::From,
};
use sp_runtime::SaturatedConversion;
use sp_runtime::{
    offchain::{http, Duration, storage::StorageValueRef},
    RuntimeDebug
};
use sp_core::crypto::KeyTypeId;
use frame_support::{
    traits::OneSessionHandler, StorageHasher
};
// use frame_system::{ensure_signed, ensure_none};
use frame_system::offchain::{
    SendTransactionTypes,
    SubmitTransaction,
};

use sp_runtime::offchain::storage::{StorageRetrievalError, MutateStorageError};
pub use pallet::*;

extern crate alloc;

#[cfg(all(feature = "std", test))]
mod mock;

#[cfg(all(feature = "std", test))]
mod tests;

pub trait Check<AccountId> {
    fn checkkyc(who: &AccountId, kycclass:Class, ioc_program_hash: [u8; 32]) -> Result<bool, CheckError>;
    fn compare_hash(hash1: Vec<u8>, hash2: Vec<u8>) -> bool;
    fn checkkyc_with_verifykyc(who: &AccountId, verifykyc: VerifyClass) -> Result<bool, CheckError>;
}


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

#[derive(PartialEq, Clone, Debug, Encode, Decode)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum VerifyClass {
	Age(u32),
    Country(u32),
}

#[derive(PartialEq, Clone, Debug, Encode, Decode)]
// #[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
pub enum KYCListOption {
	Add(ADD),
    Delete(DEL),
}

#[derive(PartialEq, Clone, Debug, Encode, Decode)]
pub struct ADD{
    pub kyc_verify_class: VerifyClass,
    pub class: Vec<u8>,
    pub program_hash: [u8; 32],
}

#[derive(PartialEq, Clone, Debug, Encode, Decode)]
pub struct DEL{
    pub kyc_verify_class: VerifyClass,
}





#[derive(Debug)]
pub enum Error {
    WrongKYCToVerify,
}

/// The status of a given verification task
#[derive(Encode, Decode, Default, PartialEq, Eq, RuntimeDebug)]
pub struct Status {
    // The verifiers involved so far
    pub verifiers: Vec<u32>,
    // The number of affirmative vote so far
    pub ayes: u32,
    // The number of dissenting vote so far
    pub nays: u32
}

/// Receipt about any verification occured
#[derive(Encode, Decode, Clone, PartialEq, Eq, RuntimeDebug)]
pub struct VerificationReceipt<AccountId, BlockNumber>{
    task_tuple_id: (AccountId, Class),
    // The Hash of a certain task to be verified
    program_hash: [u8; 32],
    // Whether a task is passed or not
    passed: bool,
    // Block number at the time submission is created.
    submit_at: BlockNumber,
    // Submitted by who
    auth_index: u32,
    // The length of session validator set
    validators_len: u32
}

#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq)]
pub enum TaskStatus {
    JustCreated,
    Verifying,
    VerifiedTrue,
    VerifiedFalse,
}


#[derive(Clone, Copy, Encode, Decode, PartialEq, Eq, Debug)]
pub enum CheckError{
    //Not on chain
    ICOVerifyFailedNotOnChain,
    //KYC onchain, but not allowed to do crowdfunding
    ICOVerifyFailedNotAllowed,  
    //KYC onchain, but the corresponding program is not ICOprogram
    ICOVerifyFailedTaskProgramWrong, 
    VerifyKYCNotCorrect, 

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
pub struct TaskInfo <BlockNumber>{
    // The id of the proof,combined with a url to fetch the complete proof later
    proof_id: Vec<u8>,
    // Inputs of the task 
    inputs: Vec<u128>,
    // Outputs of the task
    outputs: Vec<u128>,
    // The hash of the program
    program_hash: [u8; 32],
    // If false,expiration is the time task created;
    // If true ,expiration is the time task expired.
    is_task_finish : Option<TaskStatus>,
    expiration: Option<BlockNumber>,
}

#[derive(Encode, Decode, Default, PartialEq, Eq, RuntimeDebug)]
pub struct KYCStruct {
    KYCprogram_hash: [u8; 32],
    KYCclass: Vec<u8>,
}

/// Class of the privacy in raw
type Class = Vec<u8>;

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
    use frame_support::{
        dispatch::DispatchResult,
        pallet_prelude::*,
    };
    use frame_system::pallet_prelude::*;
    use super::*;
    #[pallet::config]
    #[pallet::disable_frame_system_supertrait_check]
    pub trait Config: frame_system::Config + SendTransactionTypes<Call<Self>> {
        /// The identifier type for an offchain worker.
        type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord + MaybeSerializeDeserialize;
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
    
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(PhantomData<T>);


    #[pallet::storage]
    #[pallet::getter(fn keys)]
    /// Current set of keys that are allowed to execute verification tasks
    pub(super) type Keys<T: Config> = StorageValue<_, Vec<T::AuthorityId>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn kyc_list)]
	pub(super) type KYCList<T: Config> = StorageMap<
    _,
    Twox64Concat,
    VerifyClass, 
    KYCStruct, 
    ValueQuery
    >;

    #[pallet::storage]
    #[pallet::getter(fn task_params)]
    /// Map from the task_params to the TaskInfo(proof_id,inputs,outputs)
    pub(super) type TaskParams<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat, T::AccountId,
        Twox64Concat, Class,
        TaskInfo<T::BlockNumber>,
        ValueQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn ongoing_tasks)]
    /// Record the verification tasks that are about to be verified or under verifying.
    /// The key is program hash
    pub(super) type OngoingTasks<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat, T::AccountId,
        Twox64Concat, Class,
        Status,
        OptionQuery,
    >;

    #[pallet::storage]
    #[pallet::getter(fn settled_tasks)]
    /// Completed proof tasks, will be stored onchain for a short period to be challenged
    pub(super) type SettledTasks<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat, T::BlockNumber,
        Twox64Concat, (T::AccountId, Class),
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
           Pallet::<T>::initialize_KYCList()
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
        TaskCreated(T::AccountId, Class, Vec<u8>),
        /// A verification submitted on chain
        VerificationSubmitted(T::AccountId, Class, bool, u32, u32, u32),
        /// A verification submitted by a single verifier
        SingleVerification(T::AccountId, Class, bool, u32, u32),

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
        KYCListNotHaveThisOne,
        /// KYCListAlreadyHaveThisOne
        KYCListAlreadyHaveThisOne,
        /// KYCClassIsEmpty
        KYCClassIsEmpty,
        /// KYCProgramIsEmpty
        KYCProgramIsEmpty,
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
            class: Class,
            program_hash: [u8; 32],
            inputs: Vec<u128>,
            outputs: Vec<u128>,
            proof_id: Vec<u8>
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            log::debug!(target:"starks-verifier","class is {:?}",class);

            // Ensure task has not been created
            ensure!(!TaskParams::<T>::try_get(&who, &class).is_ok(), Error::<T>::TaskAlreadyExists);
            <TaskParams<T>>::insert(&who, &class, TaskInfo{proof_id: proof_id.clone(), inputs, outputs, program_hash: program_hash, is_task_finish: Some(TaskStatus::JustCreated), expiration: Some(<frame_system::Pallet<T>>::block_number())});
            <OngoingTasks<T>>::insert(&who, &class, Status::default());
            Self::deposit_event(Event::TaskCreated(who, class, proof_id));
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
            let class = receipt.clone().task_tuple_id.1;
            <OngoingTasks<T>>::try_mutate_exists(
                &account.clone(),
                &class.clone(),
                |last_status| -> DispatchResult {
                    // Last status must exist.Fetch last status,if not exists return error
                    let mut status = last_status.take().ok_or(Error::<T>::TaskNotExists)?;
                    // A verifier can not submit more than once
                    ensure!(!status.verifiers.contains(&receipt.auth_index),
                        Error::<T>::DuplicatedSubmission);
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
                    let TaskInfo {proof_id, inputs, outputs, program_hash, expiration, ..} = Self::task_params(&account, &class);
                    Self::deposit_event(Event::SingleVerification(account.clone(), class.clone(), receipt.passed, status.ayes.clone(), Self::authority_len()));
                    <TaskParams<T>>::insert(&account, &class, TaskInfo{proof_id: proof_id.clone(), inputs: inputs.clone(), outputs: outputs.clone(), program_hash: program_hash, is_task_finish: Some(TaskStatus::Verifying), expiration});
                    let expiration = receipt.submit_at + T::StorePeriod::get();
                    // If ayes > threshold，pass the task and store it on-chain with a `true`.

                    if status.ayes >= threshold {
                        // Pass the verification
                        SettledTasks::<T>::insert(expiration, &(account.clone(),class.clone()), Some(true));
                        <TaskParams<T>>::insert(&account, &class, TaskInfo{proof_id, inputs, outputs, program_hash: program_hash, is_task_finish: Some(TaskStatus::VerifiedTrue), expiration: Some(expiration)});
                        *last_status = None;
                        Self::deposit_event(Event::VerificationSubmitted(account.clone(), class.clone(), true, status.ayes, status.nays, Self::authority_len()));
                        // If nays > threshold，reject the task and store it on-chain with a `false`.
                    } else if status.nays >= threshold {
                        // fail the verification
                        SettledTasks::<T>::insert(expiration, &(account.clone(), class.clone()), Some(false));
                        <TaskParams<T>>::insert(&account, &class, TaskInfo{proof_id, inputs, outputs, program_hash: program_hash, is_task_finish: Some(TaskStatus::VerifiedFalse), expiration: Some(expiration)});
                        *last_status = None;
                        Self::deposit_event(Event::VerificationSubmitted(account, class, false, status.ayes, status.nays, Self::authority_len()));
                    } else {
                        // Otherwise, update the task status
                        *last_status = Some(status);
                    }
                    Ok(())
                })
            }
            #[pallet::weight(10000)]
            pub fn modify_kyc_list(
                origin: OriginFor<T>,
                add_or_delete: KYCListOption,

            ) -> DispatchResult {
                ensure_root(origin)?;
                match add_or_delete {
                    KYCListOption::Add(add_information) => {
                        let ADD{kyc_verify_class, class, program_hash} = add_information;
                        let res_add = <KYCList<T>>::try_get(kyc_verify_class.clone());
                        ensure!(!KYCList::<T>::try_get(kyc_verify_class.clone()).is_ok(),
                        Error::<T>::KYCListAlreadyHaveThisOne
                        );
                        let add_struct = KYCStruct{KYCprogram_hash: program_hash.clone(), KYCclass: class.clone()};
                        ensure!(!program_hash.is_empty(),
                        Error::<T>::KYCProgramIsEmpty
                        );
                        ensure!(!class.is_empty(),
                        Error::<T>::KYCClassIsEmpty
                        );
                        <KYCList<T>>::insert(kyc_verify_class, add_struct);
                    },
                    KYCListOption::Delete(delete_information) => {
                        let DEL{kyc_verify_class} = delete_information;
                        ensure!(KYCList::<T>::try_get(kyc_verify_class.clone()).is_ok(),
                        Error::<T>::KYCListNotHaveThisOne
                        );
                        <KYCList<T>>::remove(kyc_verify_class);
                    }
                }

                Ok(())
            }
        }

    // Runs after every block.  
    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        fn on_finalize(block: T::BlockNumber) {
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

        fn validate_unsigned(
            _source: TransactionSource,
            call: &Self::Call
        ) -> TransactionValidity {

            if let Call::submit_verification(receipt, signature) = call {
                if receipt.submit_at > <frame_system::Pallet<T>>::block_number() {
                    return InvalidTransaction::Future.into();
                }

                // Verify that the incoming (unverified) pubkey is actually an authority id
                let keys = Keys::<T>::get();
                if keys.len() as u32 != receipt.validators_len {
                    return InvalidTransaction::Custom(INVALID_VALIDATORS_LEN).into();
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
                    return InvalidTransaction::BadProof.into();
                }

                ValidTransaction::with_tag_prefix("StarksVerifier")
                    .priority(T::UnsignedPriority::get())
                    .and_provides((receipt.program_hash,authority_id))
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
        block_number: T::BlockNumber
    ) -> OffchainResult<T, impl Iterator<Item=OffchainResult<T, ()>>> {
        Ok(Self::local_authority_keys()
            .map( move |(auth_index, key)|
                Self::send_single_result_with_lock(auth_index, key, block_number)
        ))
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

        let mut task_id_tuple: (T::AccountId, Class) = Default::default();
        let mut initial_local_tasks = BTreeSet::new();

        let res = storage.mutate(
            |tasks: Result<Option<Option<BTreeSet<(T::AccountId, Class)>>>,StorageRetrievalError>| {
            // Check if there is already a lock for that particular task.(hash)
            // If there exists a vec of `local_tasks`,we will attempt to find a certian task which is
            // stored on-chain(<OngoingTask>) but not verified locally yet.
            // If such vec doesn't exist ,we will initialize it,with the fist task to be verified on-chain(<OngoingTask>)
            match tasks {
                Ok(Some(Some(mut local_tasks))) =>  {
                    task_id_tuple = Self::task_to_execute(&mut local_tasks)?;
                    // TODO: fetch_proof, verify, and submit
                    local_tasks.insert(task_id_tuple.clone());
                    Ok(Some(local_tasks))
                },
                _ => {
                    task_id_tuple = Self::task_to_execute(&mut BTreeSet::new())?;
                    initial_local_tasks.insert(task_id_tuple.clone());
                    Ok(Some(initial_local_tasks))
                }
            }
        });
        if let Err(MutateStorageError::ValueFunctionFailed(err)) = res {
			return Err(err)
		}

        let mut local_tasks = res.map_err(|_| OffchainErr::FailToAcquireLock)?;

        // We got the lock, and do the fetch, verify, sign and send
        let res =  Self::prepare_submission(
            auth_index, 
            key, 
            block_number, 
            task_id_tuple.clone());
        
        // Clear the lock in case we have failed to send transaction.
        if res.is_err() {
            storage.set(&local_tasks.unwrap().remove(&task_id_tuple));

            // local_tasks.remove(&task_id_tuple);
            // storage.set(&local_tasks);
        }
        res
    }else{
        return Err(OffchainErr::NotMyTurn(block_number));
    }
}

    /// To justify if a validator should do verification in this block
    fn is_my_turn(
        auth_index: u32,
        block_number: T::BlockNumber
    )->bool{
        let number: u32 = block_number.saturated_into();     
        return number % 2 == auth_index % 2

    }

    /// Fetch proof, verify, sign and submit the transaction
    fn prepare_submission(
        auth_index: u32,
        key: T::AuthorityId,
        block_number: T::BlockNumber,
        task_tuple_id: (T::AccountId, Class)
    ) -> OffchainResult<T, ()> {
        let TaskInfo {proof_id, inputs, outputs, program_hash, ..} = Self::task_params(&task_tuple_id.0, &task_tuple_id.1);
        // To fetch proof and verify it.
        let proof = Self::fetch_proof(&proof_id).map_err(|_| OffchainErr::FailedToFetchProof)?;
        let is_success = Self::stark_verify(&program_hash, inputs,outputs, &proof);
        let res = if let Ok(r) = is_success { r } else {false};
        let validators_len = Keys::<T>::decode_len().unwrap_or_default() as u32;
        //Create and initialize a verification receipt
        let receipt = VerificationReceipt {
            task_tuple_id,
            program_hash: program_hash,
            passed: res,
            submit_at: block_number,
            auth_index: auth_index,
            validators_len
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
        let url = "https://ipfs.infura.io:5001/api/v0/cat?arg=".to_owned() + sp_std::str::from_utf8(proof_id).unwrap();
        let request = http::Request::get(&url);
        let pending = request
            .deadline(deadline)
            .send()
            .map_err(|_| http::Error::IoError)?;

        let response = pending.try_wait(deadline)
        .map_err(|_| http::Error::DeadlineReached)??;

        // Let's check the status code before we proceed to reading the response.
        if response.code != 200 {
            return Err(http::Error::Unknown);
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
        inputs: Vec<u128>,
        outputs: Vec<u128>,
        proof: &[u8]) -> OffchainResult<T, bool> {
        sp_starks::starks::verify(program_hash, &inputs, &outputs, proof)
            .map_err(|_| OffchainErr::VerificationFailed)
    }

    // Return index of on-chain authorities and its corresponding local public key
    fn local_authority_keys() -> impl Iterator<Item=(u32, T::AuthorityId)> {
        // On-chain storage
        let authorities = Keys::<T>::get();
        // Local keystore
        let mut local_keys = T::AuthorityId::all();
        local_keys.sort();
        authorities.into_iter()
            .enumerate()
            .filter_map(move |(index, authority)| {
                local_keys.binary_search(&authority)
                    .ok()
                    .map(|location| (index as u32, local_keys[location].clone()))
            })
    }


    /// Pick an on-chain tasks to execute which is not included in `local_tasks`
    fn task_to_execute(local_tasks: &BTreeSet<(T::AccountId, Class)>) -> OffchainResult<T, (T::AccountId, Class)> {
        //On-chain ready-to-verify tasks,put all task_hash of OngoingTasks into a vec.
        let ongoing_tasks_list = BTreeSet::from_iter(OngoingTasks::<T>::iter()
        .map(|(account_id, class, _)| (account_id, class)));

        // Find any one task that is not executed
        // TODO： find all tasks that are not executed
        let mut tasks_not_executed : Vec<_> = ongoing_tasks_list.difference(local_tasks).cloned().collect();
        
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

impl<T: Config> Check<T::AccountId> for Pallet<T> {
    fn checkkyc(who: &T::AccountId, kycclass:Class, ioc_program_hash: [u8; 32]) -> Result<bool, CheckError>{
        let kyc_is_exist = TaskParams::<T>::try_get(who, &kycclass).is_ok();
        log::debug!(target:"starks-verifier","kyc_is_exist is {:?}.class is {:?}",kyc_is_exist,kycclass);
        if kyc_is_exist {
            let TaskInfo {outputs, program_hash, ..} = Self::task_params(who, &kycclass);
            let ioc_program_vec = ioc_program_hash.encode();
            let program_hash_vec = program_hash.encode();
            let compare_result = Self::compare_hash(ioc_program_vec, program_hash_vec);
            if compare_result == true  && outputs == vec![1] {
                return Ok(true);
            }
            else {
                if  compare_result == true{
                    return Err(CheckError::ICOVerifyFailedNotAllowed);
                }else{
                    return Err(CheckError::ICOVerifyFailedTaskProgramWrong);
                }
            }
        }else {
            return Err(CheckError::ICOVerifyFailedNotOnChain);
        }
    }

    fn compare_hash(mut hash1: Vec<u8>, mut hash2: Vec<u8>) -> bool{
        let len1 = hash1.len();
        let len2 = hash2.len();
        if len1 == len2{
            for _i in 0..len1{
                if hash1.pop() == hash2.pop(){}
                else{return false}
            }
            return true;
        }else{
            return false;
        }
    }

    fn checkkyc_with_verifykyc(who: &T::AccountId, verifykyc: VerifyClass) -> Result<bool, CheckError>{
        let kyc_struct = KYCList::<T>::try_get(verifykyc);
        if kyc_struct.is_ok(){
            let KYCStruct{KYCprogram_hash, KYCclass} = kyc_struct.unwrap();
            return Self::checkkyc(who, KYCclass, KYCprogram_hash);
        }else{
            return Err(CheckError::VerifyKYCNotCorrect);
        }

    }
}



impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
    type Public = T::AuthorityId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
    type Key = T::AuthorityId;

    fn on_genesis_session<'a, I: 'a>(validators: I)
        where I: Iterator<Item=(&'a T::AccountId, T::AuthorityId)>
    {
        let keys = validators.map(|x| x.1).collect::<Vec<_>>();
        Self::initialize_keys(&keys);
    }

    fn on_new_session<'a, I: 'a>(_changed: bool, validators: I, _queued_validators: I)
        where I: Iterator<Item=(&'a T::AccountId, T::AuthorityId)>
    {

        // Remember who the authorities are for the new session.
        Keys::<T>::put(validators.map(|x| x.1).collect::<Vec<_>>());
    }

    fn on_disabled(_i: usize) {
        // Ignore
    }
}


impl<T: Config> Pallet<T> {
    fn initialize_KYCList() {
        let age_program_hash = [228,150,141,219,97,232,23,59,109,33,136,11,72,175,77,167,38,2,251,107,254,126,91,63,176,46,204,254,90,153,168,40];
        let country_program_hash = [208, 194, 130, 197, 164, 24, 192, 43, 169, 199, 5, 5, 30, 49, 190, 137, 168, 29, 175, 111, 254, 108, 138, 242, 161, 201, 76, 10, 238, 140, 97, 14];
        let age_class = [21].to_vec();;
        let country_class = [22].to_vec();
        <KYCList<T>>::insert(VerifyClass::Age(20), KYCStruct{KYCprogram_hash: age_program_hash, KYCclass: age_class});
        <KYCList<T>>::insert(VerifyClass::Country(1), KYCStruct{KYCprogram_hash: country_program_hash, KYCclass: country_class});

    }
}


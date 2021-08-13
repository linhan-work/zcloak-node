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
//! * `create_task` - Create a task with program_has h,inputs, outputs, proof.
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
use sp_std::prelude::*;
use sp_std::{
    convert::From,
};
use sp_runtime::{
    RuntimeDebug
};
use sp_core::crypto::KeyTypeId;
use frame_support::{
    traits::OneSessionHandler
};
// use frame_system::{ensure_signed, ensure_none};
use frame_system::offchain::{
    SendTransactionTypes
};
pub use pallet::*;

#[cfg(all(feature = "std", test))]
mod mock;

#[cfg(all(feature = "std", test))]
mod tests;

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
pub enum UserTaskStatus {
    JustCreated,
    VerifiedTrue,
    VerifiedFalse,
}

impl sp_std::fmt::Debug for UserTaskStatus {
    fn fmt(&self, fmt: &mut sp_std::fmt::Formatter) -> sp_std::fmt::Result {
        match *self {
            UserTaskStatus::JustCreated => write!(fmt, "JustCreated"),
            UserTaskStatus::VerifiedTrue => write!(fmt, "VerifiedTrue"),
            UserTaskStatus::VerifiedFalse => write!(fmt, "VerifiedFalse"),
        }
    }
}

/// Info of a certain task
#[derive(Encode, Decode, Default, PartialEq, Eq, RuntimeDebug)]
pub struct UserTaskInfo {
    // The id of the proof,combined with a url to fetch the complete proof later
    proof: Vec<u8>,
    // Inputs of the task 
    inputs: Vec<u128>,
    // Outputs of the task
    outputs: Vec<u128>,
    // The hash of the program
    program_hash: [u8; 32],
    // If false,expiration is the time task created;
    // If true ,expiration is the time task expired.
    is_task_finish : Option<UserTaskStatus>,
}



/// Class of the privacy in raw
type Class = Vec<u8>;

/// Error which may occur while executing the off-chain code.
#[cfg_attr(test, derive(PartialEq))]
pub enum UserTaskErr {
    FailToVerify
}

pub type UserResult<A> = Result<A, UserTaskErr>;

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
    #[pallet::getter(fn task_params)]
    /// Map from the task_params to the TaskInfo(proof,inputs,outputs)
    pub(super) type TaskParams<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat, T::AccountId,
        Twox64Concat, Class,
        UserTaskInfo,
        ValueQuery,
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

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        /// A new task is created.
        UserTaskCreated(T::AccountId, Class, Vec<u8>),
        /// Whether user's task pass or not
        UserTaskVerification(T::AccountId, Class, bool),
    }

    #[pallet::error]
    pub enum Error<T> {
        /// It's not allowed to recreated an existed task.
        TaskAlreadyExists,
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
        /// - `proof`: The id of the proof,combined with a url to fetch the complete proof later
        /// 
        /// If the Task created successfully, deposit the `TaskCreated` event.
        #[pallet::weight(10000)]
        pub fn user_verify(
            origin: OriginFor<T>,
            class: Class,
            program_hash: [u8; 32],
            inputs: Vec<u128>,
            outputs: Vec<u128>,
            proof: Vec<u8>
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            // Ensure task has not been created
            ensure!(!TaskParams::<T>::try_get(&who, &class).is_ok(), Error::<T>::TaskAlreadyExists);
            <TaskParams<T>>::insert(&who, &class, UserTaskInfo{proof: proof.clone(), inputs:inputs.clone(), outputs:outputs.clone(), program_hash: program_hash.clone(), is_task_finish: Some(UserTaskStatus::JustCreated)});
            Self::deposit_event(Event::UserTaskCreated(who.clone(), class.clone(), proof.clone()));
            
            let expiration = <frame_system::Pallet<T>>::block_number() + T::StorePeriod::get();
            let is_success = Self::stark_verify(&program_hash, inputs.clone(), outputs.clone(), &proof);
            let res = if let Ok(r) = is_success { r } else {false};

            if res {
                <TaskParams<T>>::insert(&who, &class, UserTaskInfo{proof, inputs, outputs, program_hash: program_hash, is_task_finish: Some(UserTaskStatus::VerifiedTrue)});
                SettledTasks::<T>::insert(expiration,&(who.clone(),class.clone()), Some(true));
                Self::deposit_event(Event::UserTaskVerification(who.clone(), class.clone(), true));
            }else{
                <TaskParams<T>>::insert(&who, &class, UserTaskInfo{proof, inputs, outputs, program_hash: program_hash, is_task_finish: Some(UserTaskStatus::VerifiedFalse)});
                SettledTasks::<T>::insert(expiration,&(who.clone(), class.clone()), Some(false));
                Self::deposit_event(Event::UserTaskVerification(who.clone(), class.clone(), false));
            }
            Ok(())
        }
    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        //We don't need to remove any SettledTask
        fn on_finalize(block: T::BlockNumber) {
            SettledTasks::<T>::remove_prefix(block, None);
        }
    }
}

impl<T: Config> Pallet<T> {

    /// Use Stark_verify to verify every program_hash with proof
    fn stark_verify(
        program_hash: &[u8; 32], 
        inputs: Vec<u128>,
        outputs: Vec<u128>,
        proof: &[u8]) -> UserResult<bool> {
        sp_starks::starks::verify(program_hash, &inputs, &outputs, proof)
            .map_err(|_| UserTaskErr::FailToVerify)
        }

}


impl<T: Config> sp_runtime::BoundToRuntimeAppPublic for Pallet<T> {
    type Public = T::AuthorityId;
}

impl<T: Config> OneSessionHandler<T::AccountId> for Pallet<T> {
    type Key = T::AuthorityId;

    fn on_genesis_session<'a, I: 'a>(_validators: I)
        where I: Iterator<Item=(&'a T::AccountId, T::AuthorityId)>
    {}

    fn on_new_session<'a, I: 'a>(_changed: bool, _validators: I, _queued_validators: I)
        where I: Iterator<Item=(&'a T::AccountId, T::AuthorityId)>
    {}

    fn on_disabled(_i: usize) {
        // Ignore
    }
}
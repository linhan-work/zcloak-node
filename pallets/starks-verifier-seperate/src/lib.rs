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
use sp_runtime::{RuntimeDebug};
use sp_core::crypto::KeyTypeId;
use frame_support::{
    traits::OneSessionHandler
};
// use frame_system::{ensure_signed, ensure_none};

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
            TaskStatus::VerifiedTrue => write!(fmt, "VerifiedTrue"),
            TaskStatus::VerifiedFalse => write!(fmt, "VerifiedFalse"),
            TaskStatus::Verifying => write!(fmt, "Verifying"),

        }
    }
}


/// Info of a certain task
#[derive(Encode, Decode, Default, PartialEq, Eq, RuntimeDebug)]
pub struct TaskInfo<BlockNumber> {
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

/// The status of a given verification task
#[derive(Encode, Decode, Default, PartialEq, Eq, RuntimeDebug)]
pub struct SeperateStatus {
    // The verifiers involved so far
    pub verifiers: Vec<u32>,
    // The number of affirmative vote so far
    pub ayes: u32,
    // The number of dissenting vote so far
    pub nays: u32,
    pub come_first: Option<bool>
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
    pub trait Config: frame_system::Config {
        /// The identifier type for an offchain worker.
        type AuthorityId: Member + Parameter + RuntimeAppPublic + Default + Ord + MaybeSerializeDeserialize;
        /// The overarching event type.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        /// A type for retrieving the validators supposed to be online in a session.
        // type ValidatorSet: ValidatorSetWithIdentification<Self::AccountId>;
    
        /// After a task is verified, it can still be stored on chain for a `StorePeriod` of time
        #[pallet::constant]
        type StorePeriod: Get<Self::BlockNumber>;

        #[pallet::constant]
        type WhiteListPeriod: Get<Self::BlockNumber>;
    
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
        TaskInfo<T::BlockNumber>,
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

    #[pallet::storage]
    #[pallet::getter(fn ongoing_tasks)]
    /// Record the verification tasks that are about to be verified or under verifying.
    pub(super) type OngoingTasks<T: Config> = StorageDoubleMap<
        _,
        Twox64Concat, T::AccountId,
        Twox64Concat, Class,
        SeperateStatus,
        OptionQuery,
    >;


    #[pallet::storage]
    #[pallet::getter(fn whitelist_time)]
    /// Store AccountIds who can send single response.
    pub(super) type WhiteListTime<T: Config> = StorageMap<
        _, 
        Twox64Concat, T::BlockNumber,
        Vec<T::AccountId>, 
        ValueQuery
        >;
        

    #[pallet::storage]
    #[pallet::getter(fn whitelist_id)]
    /// Store AccountIds who can send single response.
    pub(super) type WhiteListId<T: Config> = StorageMap<
        _, 
        Twox64Concat, T::AccountId,
        T::BlockNumber,
        ValueQuery
        >;

    #[pallet::event]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    #[pallet::metadata(T::AccountId = "AccountId")]
    pub enum Event<T: Config> {
        /// A new task is created. who, class, programhash, proofID, inputs, outputs
        UserTaskCreated(T::AccountId, Class, [u8; 32], Vec<u8>, Vec<u128>, Vec<u128>),
        // Accept this response. Origin, who, class
        SingleResponseAccept(T::AccountId, T::AccountId, Class),
        // Add new account to WhiteList
        WhiteListAdded(T::AccountId, T::BlockNumber),
        // After force_check, the task is still not finish verificaion
        StillNotFinishVerificaion(T::AccountId, Class),
        // The result of Force Verification
        ForceVerification(T::AccountId, Class, bool),
        // Task verification finished
        TaskVerificationResult(T::AccountId, Class, bool)
    }

    #[pallet::error]
    pub enum Error<T> {
        /// It's not allowed to recreated an existed task.
        TaskAlreadyExists,
        /// Not permitted to send verification result for not a member of whitelist.
        NotInWhitelist,
        // The task is not going on
        TaskNotGoingOn,
        // Member already in whitelist
        AlreadyInWhiteList,
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
        /// This function is only to creat new task.
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
            // Ensure task has not been created
            ensure!(!TaskParams::<T>::try_get(&who, &class).is_ok(), Error::<T>::TaskAlreadyExists);
            <TaskParams<T>>::insert(&who, &class, TaskInfo{proof_id: proof_id.clone(), inputs:inputs.clone(), outputs:outputs.clone(), program_hash: program_hash.clone(), is_task_finish: Some(TaskStatus::JustCreated),expiration: Some(<frame_system::Pallet<T>>::block_number())});
            <OngoingTasks<T>>::insert(&who, &class, SeperateStatus::default());
            Self::deposit_event(Event::UserTaskCreated(who, class, program_hash, proof_id, inputs, outputs));
            Ok(())
        }

        #[pallet::weight(10000)]
        /// Checks that sender is the Sudo `key` before forwarding to `add_whitelist` in the pallet.
        pub fn add_whitelist(
            origin: OriginFor<T>,
            who: T::AccountId,
        ) -> DispatchResult {
            ensure_root(origin)?;
    
            ensure!(!WhiteListId::<T>::try_get(who.clone()).is_ok(),
                Error::<T>::AlreadyInWhiteList
            );
            WhiteListId::<T>::insert(who.clone(), <frame_system::Pallet<T>>::block_number());
            let list = WhiteListTime::<T>::try_get(<frame_system::Pallet<T>>::block_number());
            if list.is_ok(){
                let mut res = list.unwrap();
                res.push(who.clone());
                <WhiteListTime<T>>::insert(<frame_system::Pallet<T>>::block_number(),res);
            }else{
                <WhiteListTime<T>>::insert(<frame_system::Pallet<T>>::block_number(),vec![who.clone()]);
            }
            Self::deposit_event(Event::WhiteListAdded(who, <frame_system::Pallet<T>>::block_number()));

            Ok(())
        }


        #[pallet::weight(10000)]
        pub fn client_single_reponse(
            origin: OriginFor<T>,
            who: T::AccountId, 
            class: Class,
            result: bool,
        ) -> DispatchResult {

            let res = ensure_signed(origin)?;
            ensure!(WhiteListId::<T>::try_get(res.clone()).is_ok(),
            Error::<T>::NotInWhitelist
            );

            ensure!(
                OngoingTasks::<T>::try_get(who.clone(), class.clone()).is_ok(),
                Error::<T>::TaskNotGoingOn
            );
            // Update the late online time , In storage-time delete old one, add new one
            let old_time = WhiteListId::<T>::try_get(res.clone()).unwrap();
            WhiteListId::<T>::insert(res.clone(), <frame_system::Pallet<T>>::block_number());
            let mut list = WhiteListTime::<T>::try_get(old_time).unwrap();
            if list.len() == 1 {
                WhiteListTime::<T>::remove(old_time);
            }else{
            // Change old_time list
                list.retain(|i|i != &res);
                WhiteListTime::<T>::insert(old_time, list);
            }
            // Add new_time list
            let now_list = WhiteListTime::<T>::try_get(<frame_system::Pallet<T>>::block_number());
            if now_list.is_ok(){
                let mut now_list1 = now_list.unwrap();
                now_list1.push(res.clone());
                WhiteListTime::<T>::insert(<frame_system::Pallet<T>>::block_number(), now_list1);
            }else{
                WhiteListTime::<T>::insert(<frame_system::Pallet<T>>::block_number(), vec![res.clone()]);
            }

            let SeperateStatus{verifiers, ayes, nays, ..} = Self::ongoing_tasks(who.clone(), class.clone()).unwrap();
            let threshold = (Self::whitelist_len() + 1) / 2;
            Self::deposit_event(Event::SingleResponseAccept(res, who.clone(), class.clone()));

            if result {
                if ayes + 1  >= threshold{
                    let TaskInfo {proof_id, inputs, outputs, program_hash, ..} = Self::task_params(&who, &class);
                    let expiration = <frame_system::Pallet<T>>::block_number() + T::StorePeriod::get();
                    SettledTasks::<T>::insert(expiration, &(who.clone(),class.clone()), Some(true));
                    <TaskParams<T>>::insert(&who.clone(), &class.clone(), TaskInfo{proof_id, inputs, outputs, program_hash: program_hash, is_task_finish: Some(TaskStatus::VerifiedTrue), expiration: Some(expiration)});
                    OngoingTasks::<T>::remove(who.clone(), class.clone());
                    Self::deposit_event(Event::TaskVerificationResult(who.clone(), class.clone(), true));

                }else{
                    if  ayes + 1 <= nays{
                        OngoingTasks::<T>::insert(who.clone(), class.clone(),SeperateStatus{verifiers, ayes: ayes + 1, nays, come_first: Some(false)});
                    }else{
                        OngoingTasks::<T>::insert(who.clone(), class.clone(),SeperateStatus{verifiers, ayes: ayes + 1, nays, come_first: Some(true)});
                    }
                    let TaskInfo {proof_id, inputs, outputs, program_hash, expiration, ..} = Self::task_params(&who, &class);
                    <TaskParams<T>>::insert(&who.clone(), &class.clone(), TaskInfo{proof_id, inputs, outputs, program_hash: program_hash, is_task_finish: Some(TaskStatus::Verifying), expiration: expiration});

                }
            }else{
                if nays + 1  >= threshold{
                    let TaskInfo {proof_id, inputs, outputs, program_hash, ..} = Self::task_params(&who, &class);
                    let expiration = <frame_system::Pallet<T>>::block_number() + T::StorePeriod::get();
                    SettledTasks::<T>::insert(expiration, &(who.clone(),class.clone()), Some(false));
                    <TaskParams<T>>::insert(&who.clone(), &class.clone(), TaskInfo{proof_id, inputs, outputs, program_hash: program_hash, is_task_finish: Some(TaskStatus::VerifiedFalse), expiration: Some(expiration)});
                    OngoingTasks::<T>::remove(who.clone(), class.clone());
                    Self::deposit_event(Event::TaskVerificationResult(who.clone(), class.clone(), false));

                }else{
                    if  nays + 1 <= ayes{
                        OngoingTasks::<T>::insert(who.clone(), class.clone(), SeperateStatus{verifiers, ayes, nays: nays + 1, come_first: Some(true)});
                    }else{
                        OngoingTasks::<T>::insert(who.clone(), class.clone(), SeperateStatus{verifiers, ayes, nays: nays + 1, come_first: Some(false)});
                    }
                    let TaskInfo {proof_id, inputs, outputs, program_hash, expiration, ..} = Self::task_params(&who, &class);
                    <TaskParams<T>>::insert(&who.clone(), &class.clone(), TaskInfo{proof_id, inputs, outputs, program_hash: program_hash, is_task_finish: Some(TaskStatus::Verifying), expiration: expiration});
                }
            };


            Ok(())
            
        }

        #[pallet::weight(10000)]
        pub fn force_check_task_status(
            origin: OriginFor<T>,
            who: T::AccountId, 
            class: Class,
        ) -> DispatchResult {
            let _res = ensure_signed(origin)?;
            ensure!(
                OngoingTasks::<T>::try_get(who.clone(), class.clone()).is_ok(),
                Error::<T>::TaskNotGoingOn
            );
            let SeperateStatus{verifiers, ayes, nays, come_first} = Self::ongoing_tasks(who.clone(), class.clone()).unwrap();
            let threshold = (Self::whitelist_len() + 1) / 2;
            if ayes >= threshold && nays >= threshold {
                if ayes > nays || (ayes == nays && come_first.unwrap()){
                    let TaskInfo {proof_id, inputs, outputs, program_hash, ..} = Self::task_params(&who, &class);
                    let expiration = <frame_system::Pallet<T>>::block_number() + T::StorePeriod::get();
                    SettledTasks::<T>::insert(expiration, &(who.clone(),class.clone()), Some(true));
                    <TaskParams<T>>::insert(&who.clone(), &class.clone(), TaskInfo{proof_id, inputs, outputs, program_hash: program_hash, is_task_finish: Some(TaskStatus::VerifiedTrue), expiration: Some(expiration)});
                    OngoingTasks::<T>::remove(who.clone(), class.clone());
                    Self::deposit_event(Event::ForceVerification(who, class, true));

                }else if nays > ayes || (ayes == nays && !come_first.unwrap()){
                    let TaskInfo {proof_id, inputs, outputs, program_hash, ..} = Self::task_params(&who, &class);
                    let expiration = <frame_system::Pallet<T>>::block_number() + T::StorePeriod::get();
                    SettledTasks::<T>::insert(expiration, &(who.clone(),class.clone()), Some(false));
                    <TaskParams<T>>::insert(&who.clone(), &class.clone(), TaskInfo{proof_id, inputs, outputs, program_hash: program_hash, is_task_finish: Some(TaskStatus::VerifiedFalse), expiration: Some(expiration)});
                    OngoingTasks::<T>::remove(who.clone(), class.clone());
                    Self::deposit_event(Event::ForceVerification(who, class, false));

                }
            }else if ayes >= threshold{
                    let TaskInfo {proof_id, inputs, outputs, program_hash, ..} = Self::task_params(&who, &class);
                    let expiration = <frame_system::Pallet<T>>::block_number() + T::StorePeriod::get();
                    SettledTasks::<T>::insert(expiration, &(who.clone(),class.clone()), Some(true));
                    <TaskParams<T>>::insert(&who.clone(), &class.clone(), TaskInfo{proof_id, inputs, outputs, program_hash: program_hash, is_task_finish: Some(TaskStatus::VerifiedTrue), expiration: Some(expiration)});
                    OngoingTasks::<T>::remove(who.clone(), class.clone());
                    Self::deposit_event(Event::ForceVerification(who, class, true));

            }else if nays >= threshold{
                    let TaskInfo {proof_id, inputs, outputs, program_hash, ..} = Self::task_params(&who, &class);
                    let expiration = <frame_system::Pallet<T>>::block_number() + T::StorePeriod::get();
                    SettledTasks::<T>::insert(expiration, &(who.clone(),class.clone()), Some(false));
                    <TaskParams<T>>::insert(&who.clone(), &class.clone(), TaskInfo{proof_id, inputs, outputs, program_hash: program_hash, is_task_finish: Some(TaskStatus::VerifiedFalse), expiration: Some(expiration)});
                    OngoingTasks::<T>::remove(who.clone(), class.clone());
                    Self::deposit_event(Event::ForceVerification(who, class, false));

            }else{
                Self::deposit_event(Event::StillNotFinishVerificaion(who, class));
            }
            Ok(())

        }

    }

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
        //We don't need to remove any SettledTask
        fn on_finalize(block: T::BlockNumber) {
            SettledTasks::<T>::remove_prefix(block, None);
            let expire_list = WhiteListTime::<T>::try_get(block - T::WhiteListPeriod::get());
            if expire_list.is_ok() {
                for i in expire_list.unwrap() {
                    WhiteListId::<T>::remove(i);
                }

            }
            WhiteListTime::<T>::remove(block - T::WhiteListPeriod::get());

        }
    }
}

impl<T: Config> Pallet<T> {


    fn whitelist_len() -> u32 {
        WhiteListId::<T>::iter().collect::<Vec<_>>().len()   as u32

        
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

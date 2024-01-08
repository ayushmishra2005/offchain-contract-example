#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;
pub mod weights;
pub use weights::*;

use sp_core::{crypto::KeyTypeId};
use frame_system::offchain::SendSignedTransaction;
// ...

pub const KEY_TYPE: KeyTypeId = KeyTypeId(*b"demo");
// ...

pub mod crypto {
	use super::KEY_TYPE;
	use sp_runtime::{
		app_crypto::{app_crypto, sr25519},
		MultiSignature, MultiSigner
	};
	app_crypto!(sr25519, KEY_TYPE);

	pub struct TestAuthId;

	// implemented for runtime
	impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for TestAuthId {
		type RuntimeAppPublic = Public;
		type GenericSignature = sp_core::sr25519::Signature;
		type GenericPublic = sp_core::sr25519::Public;
	}
}

#[frame_support::pallet]
pub mod pallet {
	use super::*;
	use frame_support::pallet_prelude::*;
	use frame_system::offchain::{AppCrypto, CreateSignedTransaction, Signer};
	use frame_system::pallet_prelude::*;
	use pallet_contracts::{CollectEvents, DebugInfo, Determinism};

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: CreateSignedTransaction<Call<Self>>  + pallet_contracts::Config + frame_system::Config{
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// Type representing the weight of this pallet
		type WeightInfo: WeightInfo;
		/// Authority ID used for offchain worker
		type AuthorityId: AppCrypto<Self::Public, Self::Signature>;
	}

	#[pallet::hooks]
	impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {
		/// Offchain worker entry point.
		///
		/// By implementing `fn offchain_worker` you declare a new offchain worker.
		/// This function will be called when the node is fully synced and a new best block is
		/// successfully imported.
		/// Note that it's not guaranteed for offchain workers to run on EVERY block, there might
		/// be cases where some blocks are skipped, or for some the worker runs twice (re-orgs),
		/// so the code should be able to handle that.
		fn offchain_worker(_block_number: BlockNumberFor<T>) {
			log::info!("Hello from pallet-ocw.");
			let signer = Signer::<T, T::AuthorityId>::all_accounts();
			if !signer.can_sign() {
				log::error!("No local accounts available");
				return
			}

			if let Some(contract) = <ContractAddress<T>>::get() {
				let results = signer.send_signed_transaction(|_account| {
					Call::execute_contract { contract: contract.clone() }
				});

				for (acc, res) in &results {
					match res {
						Ok(()) => log::info!("Contract Executed:  {:?}", acc.id),
						Err(e) => log::error!("[{:?}] Failed to submit transaction: {:?}", acc.id, e),
					}
				}
			} else {
				log::error!("No contract available");
			}

		}

	}

	#[pallet::storage]
	#[pallet::getter(fn contract_address)]
	pub type ContractAddress<T> = StorageValue<_, <T as frame_system::Config>::AccountId>;

	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		ContractStored {  contract: T::AccountId },
	}

	#[pallet::error]
	pub enum Error<T> {
		/// Error names should be descriptive.
		NoneValue,
		/// Errors should have helpful documentation associated with them.
		StorageOverflow,
	}

#[pallet::validate_unsigned]
impl<T: Config> ValidateUnsigned for Pallet<T> {
	type Call = Call<T>;

	/// Validate unsigned call to this module.
	///
	/// By default unsigned transactions are disallowed, but implementing the validator
	/// here we make sure that some particular calls (the ones produced by offchain worker)
	/// are being whitelisted and marked as valid.
	fn validate_unsigned(_source: TransactionSource, call: &Self::Call) -> TransactionValidity {

		match call {
			_ => InvalidTransaction::Call.into(),
		}
	}
}
	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight(10000)]
		pub fn store_contract(origin: OriginFor<T>, contract: T::AccountId) -> DispatchResult {

			ensure_signed(origin)?;
			// Update storage.
			<ContractAddress<T>>::put(contract.clone());

			// Emit an event.
			Self::deposit_event(Event::ContractStored { contract });
			// Return a successful DispatchResultWithPostInfo
			Ok(())
		}

		#[pallet::call_index(1)]
		#[pallet::weight(10000)]
		pub fn execute_contract(origin: OriginFor<T>, contract: T::AccountId) -> DispatchResult {

			let who = ensure_signed(origin)?;
			let  selector = [0x63, 0x3a, 0xa5, 0x51].into();
			let gas_limit: Weight = Weight::from_parts(100_000_000_000, 3 * 1024 * 1024);

			pallet_contracts::Pallet::<T>::bare_call(
				who,
				contract,
				Default::default(),
				gas_limit,
				None,
				selector,
				DebugInfo::Skip,
				CollectEvents::Skip,
				Determinism::Enforced
			).result?;

			Ok(())
		}
	}
}

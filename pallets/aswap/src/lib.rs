#![cfg_attr(not(feature = "std"), no_std)]
/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://docs.substrate.io/reference/frame-pallets/>
pub use pallet::*;

#[cfg(test)]
mod mock;
#[cfg(test)]
mod mock_data;
#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use frame_support::{
		pallet_prelude::{DispatchResult, *},
		sp_io::hashing,
		sp_runtime::traits::{
			AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul,
			One, Zero,
		},
		traits::{
			fungibles::{self, *},
			tokens::WithdrawConsequence,
			Currency, LockableCurrency, ReservableCurrency,
		},
		PalletId,
	};
	use frame_system::pallet_prelude::*;
	pub type BlockNumberOf<T> = <T as frame_system::Config>::BlockNumber;
	pub type BalanceOf<T> =
		<<T as Config>::Currency as Currency<<T as frame_system::Config>::AccountId>>::Balance;
	pub type AssetIdOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<
		<T as frame_system::Config>::AccountId,
	>>::AssetId;
	pub type AssetBalanceOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<
		<T as frame_system::Config>::AccountId,
	>>::Balance;
	pub type AccountIdOf<T> = <T as frame_system::Config>::AccountId;
	pub type AssetPriceOf<T> = <<T as Config>::Fungibles as fungibles::Inspect<
		<T as frame_system::Config>::AccountId,
	>>::Balance;

	#[pallet::pallet]
	#[pallet::generate_store(pub(super) trait Store)]
	pub struct Pallet<T>(_);

	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Pallet ID.
		#[pallet::constant]
		type PalletId: Get<PalletId>;

		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;

		/// Type to access the Balances Pallet.
		type Currency: Currency<Self::AccountId>
			+ ReservableCurrency<Self::AccountId>
			+ LockableCurrency<Self::AccountId>;

		/// Type to access the Assets Pallet.
		type Fungibles: fungibles::Inspect<Self::AccountId>
			+ fungibles::Mutate<Self::AccountId>
			+ fungibles::metadata::Mutate<Self::AccountId>
			+ fungibles::InspectMetadata<Self::AccountId>
			+ fungibles::Transfer<Self::AccountId>
			+ fungibles::Create<Self::AccountId>;
	}

	#[derive(
		Clone, Encode, Decode, Eq, PartialEq, RuntimeDebug, Default, MaxEncodedLen, TypeInfo,
	)]
	/// structure for saving all lock details
	pub struct LockDetails<AssetBalance, AssetId, AccountId, BlockNumber> {
		pub tx_id: [u8; 32],
		pub sender: AccountId,
		pub recipient: AccountId,
		pub asset_id: AssetId,
		pub amount: AssetBalance,
		pub hashlock: [u8; 32],
		pub expiration_block: BlockNumber,
		pub is_withdraw: bool,
		pub is_refunded: bool,
		pub preimage: Option<[u8; 32]>,
	}

	/// type for modeling LockDetails
	pub type LockDetailsOf<T> =
		LockDetails<AssetBalanceOf<T>, AssetIdOf<T>, AccountIdOf<T>, BlockNumberOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn lock_transactions)]
	/// Data storage for keeping all lock transactions
	pub(super) type LockTransactions<T: Config> =
		StorageMap<_, Blake2_128Concat, [u8; 32], LockDetailsOf<T>, OptionQuery>;

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/main-docs/build/events-errors/
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Notify about new lock transaction
		Locked {
			tx_id: [u8; 32],
			recipient: AccountIdOf<T>,
			hashlock: [u8; 32],
			expiration_block: BlockNumberOf<T>,
			asset_id: AssetIdOf<T>,
			asset_amount: AssetBalanceOf<T>,
		},
		/// Notify about unlock transaction
		Unlocked { tx_id: [u8; 32] },
		/// Notify about canceled transaction
		Canceled { tx_id: [u8; 32] },
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		/// Token doesn't exist
		TokenNotExists,
		/// Insufficient funds to perform operation
		LowBalance,
		/// invalid amount or below minimum
		InvalidAmount,
		/// Operation not inplemented
		NotImplemented,
		/// Overflow or Underflow error
		OverflowOrUnderflow,
		/// deadline block has passed
		Expired,
		/// Invalid Timelock. Timelock + current block must be in the future
		InvalidTimelock,
		/// hash does not match
		InvalidPreimage,
		/// transaction doesn't exists
		TransactionNotExists,
		/// lock with transaction id already exists
		TransactionIdExists,
		/// already withdrawn
		AlreadyWithdrawn,
		/// already refunded
		AlreadyRefunded,
		/// Invalid receiver to unlock
		InvalidReceiver,
		/// Timelock has not expired
		TimeLockNotExpired,
	}

	impl<T: Config> Pallet<T> {
		/// account associated to this pallet
		pub fn account_id() -> T::AccountId {
			T::PalletId::get().into_account_truncating()
		}
	}
	/// helpers functions to perform validations related to assets and perfom actions storage
	/// related in relation to exchanges.
	pub trait PalletHelpers: Config {
		///	checks if a tx_id exists in the storage
		fn lockDetailsExists(tx_id: [u8; 32]) -> bool;
		///	ensure that tx_id exists in the storage and who equals to recipient or throws error
		fn ensureLockDetailsValidToUnlock(
			who: &AccountIdOf<Self>,
			tx_id: [u8; 32],
		) -> Result<(), Error<Self>>;
		///	ensure that tx_id's hash and preimage's hash matches or throws error
		fn ensureHashlockMatches(tx_id: [u8; 32], preimage: [u8; 32]) -> Result<(), Error<Self>>;
		///	ensure that tx_id's expiration block is in the past and it's refundable or Error
		fn ensureRefundable(tx_id: [u8; 32]) -> Result<(), Error<Self>>;
		///	ensure that tx_id's expiration block is valid and withdrawable or Error
		fn ensureWithdrawable(tx_id: [u8; 32]) -> Result<(), Error<Self>>;
		/// ensures that provided amount is above zero or throws an Error
		fn ensure_is_not_zero(amount: AssetBalanceOf<Self>) -> Result<(), Error<Self>>;
		/// checks if account has the amount expected to withdraw for the specific asset
		fn ensure_has_balance(
			who: &AccountIdOf<Self>,
			asset_id: AssetIdOf<Self>,
			amount: AssetBalanceOf<Self>,
		) -> Result<(), Error<Self>>;
		/// asset exists or dispatchs an Error
		fn ensure_asset_exists(asset_id: AssetIdOf<Self>) -> Result<(), Error<Self>>;
		/// checks current block number with the deadline provided. Error if current block number is above
		fn ensure_deadline(expiration_block: &Self::BlockNumber) -> Result<(), Error<Self>>;
		/// checks current block number with the deadline provided. Error if block number has not expired
		fn ensure_expired(expiration_block: &Self::BlockNumber) -> Result<(), Error<Self>>;
	}

	/// Helpers implementation
	impl<T: Config> PalletHelpers for T {
		
		fn lockDetailsExists(tx_id: [u8; 32]) -> bool {
			LockTransactions::<T>::contains_key(tx_id)
		}
		
		fn ensureLockDetailsValidToUnlock(
			who: &AccountIdOf<Self>,
			tx_id: [u8; 32],
		) -> Result<(), Error<Self>> {
			let lockDetails =
				LockTransactions::<T>::get(tx_id).ok_or(Error::TransactionNotExists)?;
			ensure!(lockDetails.recipient == who.clone(), Error::<T>::InvalidReceiver);
			Ok(())
		}

		fn ensureHashlockMatches(tx_id: [u8; 32], preimage: [u8; 32]) -> Result<(), Error<Self>> {
			let lockDetails =
				LockTransactions::<T>::get(tx_id).ok_or(Error::TransactionNotExists)?;
			let secretHash: [u8; 32] = hashing::sha2_256(&preimage.clone());
			ensure!(lockDetails.hashlock == secretHash, Error::<T>::InvalidPreimage);
			Ok(())
		}
		fn ensureRefundable(tx_id: [u8; 32]) -> Result<(), Error<Self>> {
			let lockDetails =
				LockTransactions::<T>::get(tx_id).ok_or(Error::TransactionNotExists)?;
			
			ensure!(lockDetails.is_refunded == false, Error::<T>::AlreadyRefunded);
			ensure!(lockDetails.is_withdraw == false, Error::<T>::AlreadyWithdrawn);
			Self::ensure_expired(&lockDetails.expiration_block);
			Ok(())
		}
		fn ensureWithdrawable(tx_id: [u8; 32]) -> Result<(), Error<Self>> {
			let lockDetails =
				LockTransactions::<T>::get(tx_id).ok_or(Error::TransactionNotExists)?;
			
			ensure!(lockDetails.is_withdraw == false, Error::<T>::AlreadyWithdrawn);
			ensure!(lockDetails.is_refunded == false, Error::<T>::AlreadyRefunded);
			// if we want to disallow claim to be made after the timeout, uncomment the following line
			// Self::ensure_deadline(&lockDetails.expiration_block);
			Ok(())
		}

		/// ensures that provided amount is above zero or throws an Error
		fn ensure_is_not_zero(amount: AssetBalanceOf<Self>) -> Result<(), Error<Self>> {
			ensure!(!amount.is_zero(), Error::InvalidAmount);
			Ok(())
		}

		/// checks if account has the amount expected to withdraw for the specific asset
		fn ensure_has_balance(
			who: &AccountIdOf<Self>,
			asset_id: AssetIdOf<Self>,
			amount: AssetBalanceOf<Self>,
		) -> Result<(), Error<Self>> {
			match Self::Fungibles::can_withdraw(asset_id.clone(), who, amount) {
				WithdrawConsequence::Success => Ok(()),
				WithdrawConsequence::ReducedToZero(_) => Ok(()),
				_ => Err(Error::LowBalance),
			}
		}

		/// asset exists or dispatchs an Error
		fn ensure_asset_exists(asset_id: AssetIdOf<Self>) -> Result<(), Error<Self>> {
			ensure!(Self::Fungibles::asset_exists(asset_id), Error::TokenNotExists);
			Ok(())
		}

		/// checks current block number with the deadline provided. Error if block number is above
		fn ensure_deadline(expiration_block: &Self::BlockNumber) -> Result<(), Error<Self>> {
			ensure!(
				expiration_block > &<frame_system::Pallet<Self>>::block_number(),
				Error::Expired
			);
			Ok(())
		}

		/// checks current block number with the deadline provided. Error if block number has not expired
		fn ensure_expired(expiration_block: &Self::BlockNumber) -> Result<(), Error<Self>> {
			ensure!(
				expiration_block <= &<frame_system::Pallet<Self>>::block_number(),
				Error::Expired
			);
			Ok(())
		}
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {	
		/// Locks funds for a given time ( current block + timelock )
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn lock(
			origin: OriginFor<T>,
			tx_id: [u8; 32],
			recipient: AccountIdOf<T>,
			hashlock: [u8; 32],
			timelock: BlockNumberOf<T>,
			asset_id: AssetIdOf<T>,
			asset_amount: AssetBalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(T::lockDetailsExists(tx_id) == false, Error::<T>::TransactionIdExists);
			let now = <frame_system::Pallet<T>>::block_number();
			let expiration_block = now + timelock;
			T::ensure_deadline(&expiration_block)?;
			T::ensure_asset_exists(asset_id)?;
			T::ensure_has_balance(&who, asset_id, asset_amount)?;
			// tokens transfered to pallet account.
			T::Fungibles::transfer(asset_id, &who, &Self::account_id(), asset_amount, true)?;
			<LockTransactions<T>>::insert(
				tx_id,
				LockDetails { 
					tx_id,
					sender: who,
					recipient: recipient.clone(),
					asset_id,
					amount: asset_amount,
					hashlock,
					expiration_block,
					is_withdraw: false,
					is_refunded: false,
					preimage: None
			    }
			);

			Self::deposit_event(Event::Locked {
				tx_id,
				recipient,
				hashlock,
				expiration_block,
				asset_id,
				asset_amount 
			});

			Ok(())
		}

		/// Unlocks funds if preimage is correct and timelock  has not expired
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn unlock(origin: OriginFor<T>, tx_id: [u8; 32], preimage: [u8; 32]) -> DispatchResult {
			let who = ensure_signed(origin)?;
			T::ensureLockDetailsValidToUnlock(&who, tx_id);
			T::ensureHashlockMatches(tx_id, preimage);
			T::ensureWithdrawable(tx_id);
			let mut lockDetails = LockTransactions::<T>::get(tx_id).ok_or(Error::<T>::TransactionNotExists)?;
			lockDetails.preimage = Some(preimage);
			lockDetails.is_withdraw = true;
			T::Fungibles::transfer(lockDetails.asset_id, &Self::account_id(), &lockDetails.recipient, lockDetails.amount, true)?;
			<LockTransactions<T>>::insert(
				tx_id,
				lockDetails.clone()
			);
			Self::deposit_event(Event::Unlocked {
				tx_id
			});
			Ok(())
		}

		/// Called by the sender if there was no withdraw and the time lock has expired.
		/// This will restore ownership of the tokens to the sender.
		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn cancel(origin: OriginFor<T>, tx_id: [u8; 32]) -> DispatchResult {
			let who = ensure_signed(origin)?;
			ensure!(T::lockDetailsExists(tx_id) == true, Error::<T>::TransactionIdExists);
			T::ensureRefundable(tx_id);
			let mut lockDetails = LockTransactions::<T>::get(tx_id).ok_or(Error::<T>::TransactionNotExists)?;
			lockDetails.is_refunded = true;
			T::Fungibles::transfer(lockDetails.asset_id, &Self::account_id(), &lockDetails.sender, lockDetails.amount, true)?;
			Self::deposit_event(Event::Canceled {
				tx_id
			});
			Ok(())
		}
	}
}

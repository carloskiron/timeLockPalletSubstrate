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
			AccountIdConversion, CheckedAdd, CheckedDiv, CheckedMul, CheckedSub, IntegerSquareRoot,
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
		pub preimage: [u8; 32],
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
			recipient: AccountIdOf<T>,
			hashlock: [u8; 32],
			timelock: BlockNumberOf<T>,
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
			who: AccountIdOf<Self>,
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
	}

	/// Helpers implementation
	impl<T: Config> PalletHelpers for T {
		fn lockDetailsExists(tx_id: [u8; 32]) -> bool {
			LockTransactions::<T>::contains_key(tx_id)
		}
		fn ensureLockDetailsValidToUnlock(
			who: AccountIdOf<Self>,
			tx_id: [u8; 32],
		) -> Result<(), Error<Self>> {
			let lockDetails =
				LockTransactions::<T>::get(tx_id).ok_or(Error::TransactionNotExists)?;
			ensure!(lockDetails.recipient == who, Error::<T>::InvalidReceiver);
			Ok(())
		}
		fn ensureHashlockMatches(tx_id: [u8; 32], preimage: [u8; 32]) -> Result<(), Error<Self>> {
			//let lockDetails = LockDetails::<T>::get(tx_id);
			let secretHash: [u8; 32] = hashing::sha2_256(&preimage.clone());
			//ensure!(lockDetails.hashlock == secretHash, Error::<T>::InvalidPreimage);
			Ok(())
		}
		fn ensureRefundable(tx_id: [u8; 32]) -> Result<(), Error<Self>> {
			Ok(())
		}
		fn ensureWithdrawable(tx_id: [u8; 32]) -> Result<(), Error<Self>> {
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

		/// checks current block number with the deadline provided. Error if block numer is above
		fn ensure_deadline(expiration_block: &Self::BlockNumber) -> Result<(), Error<Self>> {
			ensure!(
				expiration_block >= &<frame_system::Pallet<Self>>::block_number(),
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
		/// Calls to handle pairs defitnions

		/// Create a new exchange pair with given tokens and tager lp asset id.
		/// lp_asset_id is used to create a new asset to represent senders' positions in the
		/// pool as they add liquidity.
		#[pallet::call_index(0)]
		#[pallet::weight(0)]
		pub fn new_exchange(
			origin: OriginFor<T>,
			asset_a_id: AssetIdOf<T>,
			asset_b_id: AssetIdOf<T>,
			asset_lp_id: AssetIdOf<T>,
		) -> DispatchResult {
			/* // check valid origin
			ensure_signed(origin)?;
			// check if asset exists before creating a exchange
			T::ensure_asset_exists(asset_a_id)?;
			T::ensure_asset_exists(asset_b_id)?;
			T::ensure_liquidity_pool_and_token_not_exists(asset_a_id, asset_b_id, asset_lp_id)?;
			//lp token creation
			T::Fungibles::create(
				asset_lp_id,
				Self::account_id(),
				false,
				<AssetBalanceOf<T>>::one(),
			)?;
			//lp created in storage
			T::setup_exchange(asset_a_id, asset_b_id, asset_lp_id);
			// Notify exchange/pair creation.
			Self::deposit_event(Event::ExchangeCreated { asset_a_id, asset_b_id, asset_lp_id }); */
			Ok(())
			//TODO should we include some sort of fees customtization to define insentives during
			// exchange creation?
		}

		/// Provides liquidity to existent pool and generates LP tokens to user
		#[pallet::call_index(1)]
		#[pallet::weight(0)]
		pub fn add_liquidity(
			origin: OriginFor<T>,
			asset_a_id: AssetIdOf<T>,
			asset_a_amount: AssetBalanceOf<T>,
			asset_b_id: AssetIdOf<T>,
			asset_b_amount: AssetBalanceOf<T>,
		) -> DispatchResult {
			/* let who = ensure_signed(origin)?;
			// check if assets exists
			T::ensure_asset_exists(asset_a_id)?;
			T::ensure_asset_exists(asset_b_id)?;
			// check if not zero
			T::ensure_is_not_zero(asset_a_amount)?;
			T::ensure_is_not_zero(asset_b_amount)?;
			// check who has balance
			T::ensure_has_balance(&who, asset_a_id, asset_a_amount)?;
			T::ensure_has_balance(&who, asset_b_id, asset_b_amount)?;
			// check that pool exists in storaged reserves
			T::ensure_liquidity_pool_exists(asset_a_id, asset_b_id)?;
			// token a moved to pallet account.
			T::Fungibles::transfer(asset_a_id, &who, &Self::account_id(), asset_a_amount, true)?;
			// token b moved to pallet account.
			T::Fungibles::transfer(asset_b_id, &who, &Self::account_id(), asset_b_amount, true)?;
			// update tokens reserves.
			let lp_asset_amount =
				T::add_exchange_liquidity(asset_a_id, asset_a_amount, asset_b_id, asset_b_amount)?;
			// provided and current pool size
			T::Fungibles::mint_into(
				T::get_liquidity_pool_asset_id(asset_a_id, asset_b_id)?,
				&who,
				lp_asset_amount,
			)?;
			// emit liquidity added
			Self::deposit_event(Event::LiquidityAdded {
				asset_a_id,
				asset_a_amount,
				asset_b_id,
				asset_b_amount,
			}); */

			Ok(())
		}

		/// Removes position from pool, returns funds + fees to user
		#[pallet::call_index(2)]
		#[pallet::weight(0)]
		pub fn remove_liquidity(
			origin: OriginFor<T>,
			lp_asset_id: AssetIdOf<T>,
			lp_asset_amount: AssetBalanceOf<T>,
		) -> DispatchResult {
			/* let who = ensure_signed(origin)?;
			// ensure that everything-inputs exists / are valid
			T::ensure_asset_exists(lp_asset_id)?;
			// Check if not zero
			T::ensure_is_not_zero(lp_asset_amount)?;
			// check if has this lp token and the amount >= in balance
			T::ensure_has_balance(&who, lp_asset_id, lp_asset_amount)?;
			// calculate the amount of token a and token b that must be transfered to origin from
			let (asset_a_id, asset_a_amount, asset_b_id, asset_b_amount) =
				T::calculate_amounts_to_return(lp_asset_id, lp_asset_amount)?;
			// burn lp tokens from origin
			T::Fungibles::burn_from(lp_asset_id, &who, lp_asset_amount)?;
			// transfer tokens A and B to origin
			T::Fungibles::transfer(asset_a_id, &Self::account_id(), &who, asset_a_amount, true)?;
			T::Fungibles::transfer(asset_b_id, &Self::account_id(), &who, asset_b_amount, true)?;
			// update reserves in storage. Do they need to be rebalance?
			T::remove_exchange_liquidity(
				asset_a_id,
				asset_a_amount,
				asset_b_id,
				asset_b_amount,
				lp_asset_amount,
				true,
			)?;
			// emit liquidity removed
			Self::deposit_event(Event::LiquidityRemoved {
				asset_a_id,
				asset_a_amount,
				asset_b_id,
				asset_b_amount,
				lp_asset_amount_burned: lp_asset_amount,
			}); */
			Ok(())
		}

		/// Swaps from token A to token B
		/// As part of the process fees are taken and distributed based on actual positions in pool
		#[pallet::call_index(3)]
		#[pallet::weight(0)]
		pub fn swap(
			origin: OriginFor<T>,
			asset_a_id: AssetIdOf<T>,
			asset_a_amount: AssetBalanceOf<T>,
			asset_b_id: AssetIdOf<T>,
			min_asset_b_amount: AssetBalanceOf<T>,
			execution_deadline: Option<T::BlockNumber>,
			to_account: Option<AccountIdOf<T>>,
		) -> DispatchResult {
			/* let who = ensure_signed(origin)?;
			// deadline validation
			if execution_deadline.is_some() {
				T::ensure_deadline(&execution_deadline.unwrap())?;
			}
			// ensure that everything-inputs exists / are valid
			T::ensure_asset_exists(asset_a_id)?;
			T::ensure_asset_exists(asset_b_id)?;
			T::ensure_has_balance(&who, asset_a_id, asset_a_amount)?;
			// check that pool exists in storaged reserves
			T::ensure_liquidity_pool_exists(asset_a_id, asset_b_id)?;
			// calculates amount of token b to get based on current pool and fees.
			let amount_out = T::get_amount_out(asset_a_id, asset_a_amount, asset_b_id)?;
			ensure!(amount_out >= min_asset_b_amount, Error::<T>::SwapOutBelowMin);
			// Transfer a from origin to pallet account
			T::Fungibles::transfer(asset_a_id, &who, &Self::account_id(), asset_a_amount, true)?;
			// Transfer b from pallet account to origin or to_account if provided
			T::Fungibles::transfer(
				asset_b_id,
				&Self::account_id(),
				&to_account.unwrap_or(who),
				amount_out,
				true,
			)?;
			// update reserves in storage
			T::take_amount_out_from_reserves(asset_a_id, asset_a_amount, asset_b_id, amount_out)?;
			// emit new swap
			Self::deposit_event(Event::SwapeExecuted {
				asset_a_id,
				asset_a_amount,
				asset_b_id,
				asset_b_amount: amount_out,
			}); */
			Ok(())
		}

		/// Locks funds for a given time ( current block + timelock )
		#[pallet::call_index(4)]
		#[pallet::weight(0)]
		pub fn lock(
			origin: OriginFor<T>,
			recipient: AccountIdOf<T>,
			hashlock: [u8; 32],
			timelock: BlockNumberOf<T>,
			asset_id: AssetIdOf<T>,
			asset_amount: AssetBalanceOf<T>,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Ok(())
		}

		/// Unlocks funds if preimage is correct and timelock  has not expired
		#[pallet::call_index(5)]
		#[pallet::weight(0)]
		pub fn unlock(origin: OriginFor<T>, tx_id: [u8; 32], preimage: [u8; 32]) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Ok(())
		}

		/// Called by the sender if there was no withdraw AND the time lock has expired.
		/// This will restore ownership of the tokens to the sender.
		#[pallet::call_index(6)]
		#[pallet::weight(0)]
		pub fn cancel(origin: OriginFor<T>, tx_id: [u8; 32]) -> DispatchResult {
			let who = ensure_signed(origin)?;
			Ok(())
		}
	}
}

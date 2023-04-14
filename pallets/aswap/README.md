# Atomic Swaps Pallet

The idea is to swap tokens between two chains with the help of a Hash Time Lock Contract/Pallet on each chain. The swap must be executed quickly otherwise the transaction will expire and the locked tokens will go back to their owners’ wallets. This pallet provides a way to perform an atomic swap between assets.

In atomic swaps, participant A creates a secret, gets its hash, and calls lock in the pallet to start the swap (providing: amount, hash, duration, and target). Funds are transferred to the pallet. Participant B can now do the same using the same hash created by Participant A. Participant A can now call unlock to get the tokens, revealing the secret to participant B. Participant B can now unlock their tokens.

Inspiration from: https://github.com/chatch/hashed-timelock-contract-ethereum/blob/master/test/htlcERC20.js

### 1. Lock

```rust
	assert_ok!(Aswap::lock(
			RuntimeOrigin::signed(ACCOUNT_A),
			tx_id,
			ACCOUNT_B,
			hash,
			timelock,
			ASSET_A,
			asset_amount
	));	
```

### 2. Unlock

```rust
    assert_ok!(Aswap::unlock(RuntimeOrigin::signed(ACCOUNT_B), tx_id, secret.to_vec()));
```

### 3. Cancel

```rust
	assert_ok!(Aswap::cancel(RuntimeOrigin::signed(ACCOUNT_A), tx_id));
```

## Technical/Design notes:

### Storage Design
 
```rust
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
	}	
	/// type for modeling LockDetails
	pub type LockDetailsOf<T> =
		LockDetails<AssetBalanceOf<T>, AssetIdOf<T>, AccountIdOf<T>, BlockNumberOf<T>>;
	/// Data storage for keeping all lock transactions
	pub(super) type LockTransactions<T: Config> =
		StorageMap<_, Blake2_128Concat, [u8; 32], LockDetailsOf<T>, OptionQuery>;
	/// Data storage for keeping all lock transactions
	pub(super) type KnownSecrets<T: Config> =
		StorageMap<_, Blake2_128Concat, [u8; 32], Vec<u8>, OptionQuery>;
```

### Pallet helpers
Some helpers where created as part of the pallet code to allow extrinsics to perform certain actions and validations. On the other hand, they were created to reuse logic common to several extrincs and therefore keep extrinsics' code cleanear. 

```rust
	pub trait PalletHelpers: Config {
		///	checks if a tx_id exists in the storage
		fn lock_details_exists(tx_id: [u8; 32]) -> bool;
		///	ensure that tx_id exists in the storage and who equals to recipient or throws error
		fn ensure_lock_details_valid_to_unlock(
			who: &AccountIdOf<Self>,
			tx_id: [u8; 32],
		) -> Result<(), Error<Self>>;
		///	ensure that tx_id's hash and preimage's hash matches or throws error
		fn ensure_hashlock_matches(tx_id: [u8; 32], preimage: Vec<u8>) -> Result<(), Error<Self>>;
		///	ensure that tx_id's expiration block is in the past and it's refundable or Error
		fn ensure_refundable(tx_id: [u8; 32]) -> Result<(), Error<Self>>;
		///	ensure that tx_id's expiration block is valid and withdrawable or Error
		fn ensure_withdrawable(tx_id: [u8; 32]) -> Result<(), Error<Self>>;
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
		/// checks current block number with the deadline provided. Error if current block number is
		/// above
		fn ensure_valid_deadline(expiration_block: &Self::BlockNumber) -> Result<(), Error<Self>>;
		/// checks current block number with the deadline provided. Error if current block number is
		/// above
		fn ensure_deadline(expiration_block: &Self::BlockNumber) -> Result<(), Error<Self>>;
		/// checks current block number with the deadline provided. Error if block number has not
		/// expired
		fn ensure_expired(expiration_block: &Self::BlockNumber) -> Result<(), Error<Self>>;
	}
```

### Errors and events
Custom errors and especific events where created to handle validations and emit notifications during extrinsics' execution.
##### Events:
```rust
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
		Canceled { tx_id: [u8; 32] }
```
##### Errors:
```rust
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
		TimeLockNotExpired
```

## Unit tests and mock data

A set of unit tests were created to validate extrinsics' results under happy and unexpected conditions. Mock.rs was created to have a runtime for testing and to include additional logic like creating and funding some accounts and creating some initial tokens to play with, along with some helpers. All mock data like accounts, initial balances, and tokens that are currenty in use as part of the tests can be changed through mock_data.rs.

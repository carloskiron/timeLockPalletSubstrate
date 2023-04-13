# DeX Pallet
Based on uniswap v2
https://docs.uniswap.org/contracts/v2/overview

Uniswap V2 was definitly a game changer in the Ethereum ecosystem. Previusly, in v1, if you wanted to swap between token A and token B the protol had to first convert token A to the underline exchange currenty (ETH) to buy token B. In V2 the protocol saves reserves of both tokens through independent liquidity pools representing the specific pair. Swaps between them are executed through the constant product formula ( x * y = k) where x and y are the reserves of A and B respectively. The constant product formula is the automated market algorithm behind the protocol. When ever a trader wants to trate A per B, the constant product formula uses the amount of A to determine the amount of B that can be expected.

This pallet is an implementation of the same uniswap V2 model. It's a work a in progress but this initial version brings the possibility to:

### 1. Create a exchange
Initial step. It's the way of creating a tredable pair. The process creates a new asset representing the liqudity pool.

```rust

        assert_ok!(Dex::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		let pool_a = Dex::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Dex::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 0u128.into());
		assert_eq!(pool_b.reserve, 0u128.into());
		assert_eq!(pool_a.asset_lp_id, LIQ_TOKEN_AB);
		assert_eq!(pool_b.asset_lp_id, LIQ_TOKEN_AB);

```

### 2. Add Liquidity
Once you have a exchage you can proceed to add liquidity. When adding liquidity you will get liquidity pool tokens representing your position in the pool.

```rust

        //adding liquidity
		let asset_a_amount = 500;
		let asset_b_amount = 1_500;
		assert_ok!(Dex::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			asset_a_amount,
			ASSET_B,
			asset_b_amount
		));
		assert_eq!(get_account_balance(ACCOUNT_A, LIQ_TOKEN_AB), 866);

```

### 3. Remove liquidity
You can burn your lp tokens to get back A and B. You can remove partially or you whole position.

```rust
assert_ok!(Dex::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), LIQ_TOKEN_AB, 600));
//amount to return asset x = (amount of lp asset to burn_withdraw * reserves of a) / total
```

### 4. Swap
Swapping is the main goal of our dex. The goal is facilitating the trade between token A and token B. In addition to the basic functionally, it's possible to provide an additional account to receive the output of the trade, indicate the minimum amount of tokens out to execute the trade, and a block deadline for making the trade expire if it's not executed before it.

```rust
        assert_ok!(Dex::swap(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_B,
			amount_to_swap,
			ASSET_A,
			min_to_get,
			None,
			ACCOUNT_C.into()
		));

        //swapping some tokens
		assert_noop!(
			Dex::swap(
				RuntimeOrigin::signed(ACCOUNT_A),
				ASSET_B,
				amount_to_swap,
				ASSET_A,
				min_to_get,
				0.into(),
				None
			),
			Error::<Test>::Expired
		);

        //swapping some tokens
		assert_noop!(
			Dex::swap(
				RuntimeOrigin::signed(ACCOUNT_A),
				ASSET_B,
				amount_to_swap,
				ASSET_A,
				min_to_get,
				None,
				None
			),
			Error::<Test>::SwapOutBelowMin
		);
```

### 5. Scheduled Dca Swa
Not implemented yet but the idea is to take advantage of the possibility to schedule extrinsics to fully automate a crypto purchase dca strategy. The stratregy will execute swaps based on the specific details like frequency, quantity, price, etc.

## Technical/Design notes:

#### Storage Design
Liquidity pools details and reserves for tokens involved where handled using custom structs, tuples, and the StorageMap. ReserveDetails entity was created to save the reserves of a token linked to pair. For a pair (A,B) two elements are saved in LiquidityPoolsReserves, one to get A reserves with key (A,B), and the other to get B reserves with key (B, A). In addition, LiquidityPoolDetails entity was created to represent a liquidity pool asset, it saves the lp asset id, assets a, and b ids, and current lp asset supply (important to perform calculations). 
```rust
	/// structure for modeling token reserve within a lp
	pub struct ReserveDetails<AssetBalance, AssetId> {
		pub reserve: AssetBalance,
		pub asset_lp_id: AssetId,
	}
	/// structure for modeling liquidity pool details
	pub struct LiquidityPoolDetails<AssetBalance, AssetId> {
		pub supply: AssetBalance,
		pub asset_a_id: AssetId,
		pub asset_b_id: AssetId,
	}
	/// type for modeling exchange reserves items
	pub type ReserveDetailsOf<T> = ReserveDetails<AssetBalanceOf<T>, AssetIdOf<T>>;
	/// type for modeling exchange reserves keys
	pub type LiquidityPoolKeyOf<T> = (AssetIdOf<T>, AssetIdOf<T>);
	/// type for modeling liquidity pool information
	pub type LiquidityPoolOf<T> = LiquidityPoolDetails<AssetBalanceOf<T>, AssetIdOf<T>>;

	#[pallet::storage]
	#[pallet::getter(fn liquidity_pools_reserves)]
	/// Data storage for keeping exchanges' reserves
	pub(super) type LiquidityPoolsReserves<T: Config> =
		StorageMap<_, Blake2_128Concat, LiquidityPoolKeyOf<T>, ReserveDetailsOf<T>, OptionQuery>;

	#[pallet::storage]
	#[pallet::getter(fn liquidity_pools)]
	/// Data storage for saving luquidity pools details
	pub(super) type LiquidityPools<T: Config> =
		StorageMap<_, Blake2_128Concat, AssetIdOf<T>, LiquidityPoolOf<T>, OptionQuery>;

```

#### Pallet helpers
Some helpers where created as part of the pallet code to allow extrinsics to perform certain actions and validations. On the other hand, they were created to reuse logic common to several extrincs and therefore keep extrinsics' code cleanear. In particular, helpers like setup_exchange, add_exchange_liquidity, remove_exchange_liquidity, get_amount_out, and calculate_amounts_to_return are very important for the core functionally of the protocol and the implementation of the constant product formula.

```rust
    pub trait PalletHelpers: Config {
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
		fn ensure_liquidity_pool_exists(
			asset_a_id: AssetIdOf<Self>,
			asset_b_id: AssetIdOf<Self>,
		) -> Result<(), Error<Self>>;
		/// returns the asset id of the lp token
		fn get_liquidity_pool_asset_id(
			asset_a_id: AssetIdOf<Self>,
			asset_b_id: AssetIdOf<Self>,
		) -> Result<AssetIdOf<Self>, Error<Self>>;
		fn ensure_liquidity_pool_and_token_not_exists(
			asset_a_id: AssetIdOf<Self>,
			asset_b_id: AssetIdOf<Self>,
			asset_lp_id: AssetIdOf<Self>,
		) -> Result<(), Error<Self>>;
		/// Adds new entries to exchanges storaged with zero balance
		/// Two entries are created (A,B) -> {A reserves, lp token id} and (B, A) -> {B reserves, lp
		/// token id
		fn setup_exchange(
			asset_a_id: AssetIdOf<Self>,
			asset_b_id: AssetIdOf<Self>,
			asset_lp_id: AssetIdOf<Self>,
		);
		/// Adds liquidity taking care of overflow. Updates reserves of the pair
		fn add_exchange_liquidity(
			asset_a_id: AssetIdOf<Self>,
			asset_a_amount: AssetBalanceOf<Self>,
			asset_b_id: AssetIdOf<Self>,
			asset_b_amount: AssetBalanceOf<Self>,
		) -> Result<AssetBalanceOf<Self>, Error<Self>>;
		/// Removes liquidity taking care of underflow. Updates reserves of the pair
		fn remove_exchange_liquidity(
			asset_a_id: AssetIdOf<Self>,
			asset_a_amount: AssetBalanceOf<Self>,
			asset_b_id: AssetIdOf<Self>,
			asset_b_amount: AssetBalanceOf<Self>,
			asset_lp_amount: AssetBalanceOf<Self>,
			with_supply_update: bool,
		) -> Result<(), Error<Self>>;
		/// determines the amount ot tokens of B that can get based on the amount of A
		fn get_amount_out(
			asset_a_id: AssetIdOf<Self>,
			amount_a: AssetBalanceOf<Self>,
			asset_b_id: AssetIdOf<Self>,
		) -> Result<AssetBalanceOf<Self>, Error<Self>>;
		/// updates reserves of A and B removing the specific amounts provided
		fn take_amount_out_from_reserves(
			asset_a_id: AssetIdOf<Self>,
			amount_a: AssetBalanceOf<Self>,
			asset_b_id: AssetIdOf<Self>,
			amount_b: AssetBalanceOf<Self>,
		) -> Result<(), Error<Self>>;

		/// calculates the amount of tokens to return to the user when buring some lp tokens
		fn calculate_amounts_to_return(
			asset_lp_id: AssetIdOf<Self>,
			asset_lp_amount: AssetBalanceOf<Self>,
		) -> Result<
			(AssetIdOf<Self>, AssetBalanceOf<Self>, AssetIdOf<Self>, AssetBalanceOf<Self>),
			Error<Self>,
		>;
		/// checks current block number with the deadline provided. Error if block numer is above
		fn ensure_deadline(execution_deadline: &Self::BlockNumber) -> Result<(), Error<Self>>;
		/// returns current reserves of assets a and b
		fn get_pool_reserves(
			asset_a_id: AssetIdOf<Self>,
			asset_b_id: AssetIdOf<Self>,
		) -> Result<(AssetBalanceOf<Self>, AssetBalanceOf<Self>), Error<Self>>;
	}
```

#### Errors and events
Custom errors and especific events where created to handle validations and emit notifications during extrinsics' execution.
##### Events:
```rust
       
		ExchangeCreated 
		/// Notify about liquidity added to exchange
		/// parameters. [asset_a_id, asset_a_amount, asset_b_id, asset_b_amount]
		LiquidityAdded 
		/// Notify about liquidity removed from exchange
		/// parameters. [asset_a_id, asset_a_amount, asset_b_id, asset_b_amount]
		LiquidityRemoved 
		/// Notify about new swap performed
		/// parameters. [asset_a_id, asset_a_amount, asset_b_id, asset_b_amount]
		SwapeExecuted 
```
##### Errors:
```rust

        /// Token doesn't exist
		TokenNotExists,
		/// Pair doesn't exist.
		PairNotExists,
		/// Insufficient funds to perform operation
		LowBalance,
		/// invalid amount or below minimum
		InvalidAmount,
		/// No position found in pool
		NoPositionInPool,
		/// error occurred while swapping
		SwapError,
		/// B tokens (tokens out) below min expected by user
		SwapOutBelowMin,
		/// Invalid schedule details
		InvalidScheduleDetails,
		/// Operation not inplemented
		NotImplemented,
		/// LP token or exchange already exists
		LpTokenOrExchangeAlreadyExists,
		/// Overflow or Underflow error
		OverflowOrUnderflow,
		/// deadline block has passed
		Expired,
		/// Insuficient liquidity
		InsuficientLiquidity,
```


##### Price Oracle

```rust
    /// Price Oracle
	/// Using constant product formula.
	/// It can be implemented by any pallet that provides price information
	/// basic implementation is provided in this pallet but it should be expanded
	/// to fully implement uniswap 2 oracles apporach https://docs.uniswap.org/contracts/v2/concepts/core-concepts/oracles
	pub trait Oracle: Config {
		fn get_price(asset_a: AssetIdOf<Self>, asset_b: AssetIdOf<Self>) -> AssetPriceOf<Self>;
	}
```

## Unit tests and mock data

A set of unit tests were created to validate extrinsics' results under happy and unexpected conditions. Operations like adding a new exchange, adding new liqudity, removing liquidity, and swapping, have specific tests with different scenarios. For handling different cases in an easier way mock.rs was updated to include additional logic like creating and funding some accounts and creating some initial tokens to play with, along with some helpers. All mock data like accounts, initial balances, and tokens that are currenty in use as part of the tests can be changed through mock_data.rs.

## About Fees
This first version, like in the old times of uniswap, users don't get charged any fees for adding liq, removing liq, or swapping. This is an initial insentive. Ahead, it could be possible to implement a mechanism that allows fees configuration and more important, make it something that can be defined by som sort of governance process.

## Final comments
The process of building this dex pallet has been really fun and rewarding. In the beginning it was a bit complicated to get used to FRAME model to build all the pieces required. Over time I found myself enjoying a lot the process of thinking how to solve the problem / business logic becuase I was confortable already with my pallet code base, and the code from substrate's repo, where I found a bunch of useful stuff to learn how to solve specific coding challenges. I am looking forward to adding additional features, tests and documentation to this pallet, and to build additional ones to support other use cases.

## Resources:
1. https://docs.uniswap.org/contracts/v2/overview

2. https://betterprogramming.pub/uniswap-v2-in-depth-98075c826254

3. https://github.com/Uniswap/v2-periphery

4. https://github.com/Uniswap/v2-core
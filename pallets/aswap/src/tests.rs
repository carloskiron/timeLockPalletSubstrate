use core::ops::Add;

use crate::{mock::*, mock_data::*, Error, Event};
use frame_support::{assert_noop, assert_ok};
use sp_runtime::traits::IntegerSquareRoot;

#[test]
fn create_new_exchange_ok() {
	new_test_ext().execute_with(|| {
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		let liquidity_pool_details = Aswap::liquidity_pools(LIQ_TOKEN_AB).unwrap();
		assert_eq!(pool_a.reserve, 0u128.into());
		assert_eq!(pool_b.reserve, 0u128.into());
		assert_eq!(pool_a.asset_lp_id, LIQ_TOKEN_AB);
		assert_eq!(pool_b.asset_lp_id, LIQ_TOKEN_AB);
		assert_eq!(liquidity_pool_details.asset_a_id, ASSET_A);
		assert_eq!(liquidity_pool_details.asset_b_id, ASSET_B);
		assert_eq!(liquidity_pool_details.supply, 0u128.into());
	});
}

#[test]
fn create_new_exchange_err_pool_exists() {
	new_test_ext().execute_with(|| {
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_B,
			ASSET_C,
			LIQ_TOKEN_BC
		));
		let mut pool = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		assert_eq!(pool.asset_lp_id, LIQ_TOKEN_AB);
		pool = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_C)).unwrap();
		assert_eq!(pool.asset_lp_id, LIQ_TOKEN_BC);
		//trying to add same token with different pool id
		assert_noop!(
			Aswap::new_exchange(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, ASSET_B, LIQ_TOKEN_AC),
			Error::<Test>::LpTokenOrExchangeAlreadyExists
		);
		//trying to add different token with an existent pool id
		assert_noop!(
			Aswap::new_exchange(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, ASSET_C, LIQ_TOKEN_AB),
			Error::<Test>::LpTokenOrExchangeAlreadyExists
		);
		//trying to add same tokens in different order
		assert_noop!(
			Aswap::new_exchange(RuntimeOrigin::signed(ACCOUNT_A), ASSET_B, ASSET_A, LIQ_TOKEN_AB),
			Error::<Test>::LpTokenOrExchangeAlreadyExists
		);
	});
}

#[test]
fn create_new_exchange_err_token_doesnt_exists() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Aswap::new_exchange(
				RuntimeOrigin::signed(ACCOUNT_A),
				ASSET_NOT_EXIST,
				ASSET_C,
				LIQ_TOKEN_AB
			),
			Error::<Test>::TokenNotExists
		);
		assert_noop!(
			Aswap::new_exchange(
				RuntimeOrigin::signed(ACCOUNT_A),
				ASSET_A,
				ASSET_NOT_EXIST,
				LIQ_TOKEN_AB
			),
			Error::<Test>::TokenNotExists
		);
	});
}

#[test]
fn swap_assets_ok() {
	new_test_ext().execute_with(|| {
		//exchange creation
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 0u128.into());
		assert_eq!(pool_b.reserve, 0u128.into());
		assert_eq!(pool_a.asset_lp_id, LIQ_TOKEN_AB);
		assert_eq!(pool_b.asset_lp_id, LIQ_TOKEN_AB);
		//adding liquidity
		let asset_a_amount = 500;
		let asset_b_amount = 1_500;
		assert_ok!(Aswap::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			asset_a_amount,
			ASSET_B,
			asset_b_amount
		));
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 500);
		assert_eq!(pool_b.reserve, 1_500);
		let liquidity_pool_details = Aswap::liquidity_pools(LIQ_TOKEN_AB).unwrap();
		let mul_result = asset_a_amount.checked_mul(asset_b_amount).unwrap();
		let liquidity_to_mint = mul_result.integer_sqrt_checked().unwrap();
		assert_eq!(liquidity_pool_details.supply, liquidity_to_mint);

		// balances before swap
		let pallet_balance_a = get_pallet_balance(ASSET_A);
		let pallet_balance_b = get_pallet_balance(ASSET_B);
		let account_a_balance_a = get_account_balance(ACCOUNT_A, ASSET_A);
		let account_b_balance_b = get_account_balance(ACCOUNT_A, ASSET_B);
		let amount_to_swap = 16u128;
		let min_to_get = 5u128;
		let tokens_out_result = 5;
		//swapping some tokens
		assert_ok!(Aswap::swap(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_B,
			amount_to_swap,
			ASSET_A,
			min_to_get,
			None,
			None
		));
		// checking accounts' balances after swapping
		assert_eq!(
			get_account_balance(ACCOUNT_A, ASSET_B),
			account_b_balance_b.checked_sub(amount_to_swap).unwrap()
		);
		assert_eq!(
			get_account_balance(ACCOUNT_A, ASSET_A),
			account_a_balance_a.checked_add(tokens_out_result).unwrap()
		);
		assert_eq!(
			get_pallet_balance(ASSET_B),
			pallet_balance_b.checked_add(amount_to_swap).unwrap()
		);
		assert_eq!(
			get_pallet_balance(ASSET_A),
			pallet_balance_a.checked_sub(tokens_out_result).unwrap()
		);
		// checing reserves after swapping
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 495);
		assert_eq!(pool_b.reserve, 1484);
	});
}

#[test]
fn swap_sending_to_other_account_ok() {
	new_test_ext().execute_with(|| {
		//exchange creation
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 0u128.into());
		assert_eq!(pool_b.reserve, 0u128.into());
		assert_eq!(pool_a.asset_lp_id, LIQ_TOKEN_AB);
		assert_eq!(pool_b.asset_lp_id, LIQ_TOKEN_AB);
		//adding liquidity
		let asset_a_amount = 500;
		let asset_b_amount = 1_500;
		assert_ok!(Aswap::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			asset_a_amount,
			ASSET_B,
			asset_b_amount
		));
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 500);
		assert_eq!(pool_b.reserve, 1_500);
		let liquidity_pool_details = Aswap::liquidity_pools(LIQ_TOKEN_AB).unwrap();
		let mul_result = asset_a_amount.checked_mul(asset_b_amount).unwrap();
		let liquidity_to_mint = mul_result.integer_sqrt_checked().unwrap();
		assert_eq!(liquidity_pool_details.supply, liquidity_to_mint);

		// balances before swap
		let pallet_balance_a = get_pallet_balance(ASSET_A);
		let pallet_balance_b = get_pallet_balance(ASSET_B);
		let account_a_balance_a = get_account_balance(ACCOUNT_A, ASSET_A);
		let account_b_balance_b = get_account_balance(ACCOUNT_A, ASSET_B);
		let amount_to_swap = 16u128;
		let min_to_get = 5u128;
		let tokens_out_result = 5;
		//swapping some tokens
		assert_ok!(Aswap::swap(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_B,
			amount_to_swap,
			ASSET_A,
			min_to_get,
			None,
			ACCOUNT_C.into()
		));
		// checking accounts' balances after swapping
		assert_eq!(
			get_account_balance(ACCOUNT_A, ASSET_B),
			account_b_balance_b.checked_sub(amount_to_swap).unwrap()
		);
		assert_eq!(get_account_balance(ACCOUNT_A, ASSET_A), account_a_balance_a);
		// tokens swapped are now part of ACCOUNT_C
		assert_eq!(
			get_account_balance(ACCOUNT_C, ASSET_A),
			ACCOUNTS_START_BALANCE.checked_add(5).unwrap()
		);
		assert_eq!(
			get_pallet_balance(ASSET_B),
			pallet_balance_b.checked_add(amount_to_swap).unwrap()
		);
		assert_eq!(
			get_pallet_balance(ASSET_A),
			pallet_balance_a.checked_sub(tokens_out_result).unwrap()
		);
		// checing reserves after swapping
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 495);
		assert_eq!(pool_b.reserve, 1484);
	});
}

#[test]
fn swap_not_executed_expired() {
	new_test_ext().execute_with(|| {
		//exchange creation
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		assert_ok!(Aswap::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			500,
			ASSET_B,
			1500
		));
		// balances before swap
		let amount_to_swap = 16u128;
		let min_to_get = 8u128;
		//swapping some tokens
		assert_noop!(
			Aswap::swap(
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
	});
}
#[test]
fn swap_not_executed_min_tokens_than_expected() {
	new_test_ext().execute_with(|| {
		//exchange creation
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 0u128.into());
		assert_eq!(pool_b.reserve, 0u128.into());
		assert_eq!(pool_a.asset_lp_id, LIQ_TOKEN_AB);
		assert_eq!(pool_b.asset_lp_id, LIQ_TOKEN_AB);
		//adding liquidity
		let asset_a_amount = 500;
		let asset_b_amount = 1_500;
		assert_ok!(Aswap::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			asset_a_amount,
			ASSET_B,
			asset_b_amount
		));
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 500);
		assert_eq!(pool_b.reserve, 1_500);
		let liquidity_pool_details = Aswap::liquidity_pools(LIQ_TOKEN_AB).unwrap();
		let mul_result = asset_a_amount.checked_mul(asset_b_amount).unwrap();
		let liquidity_to_mint = mul_result.integer_sqrt_checked().unwrap();
		assert_eq!(liquidity_pool_details.supply, liquidity_to_mint);

		// balances before swap
		let amount_to_swap = 16u128;
		let min_to_get = 8u128;
		//swapping some tokens
		assert_noop!(
			Aswap::swap(
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
	});
}

#[test]
fn swap_assets_pool_not_exists() {
	new_test_ext().execute_with(|| {
		//exchange creation
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		//adding liquidity
		let asset_a_amount = 500;
		let asset_b_amount = 1_500;
		assert_ok!(Aswap::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			asset_a_amount,
			ASSET_B,
			asset_b_amount
		));
		assert_eq!(get_account_balance(ACCOUNT_A, LIQ_TOKEN_AB), 866);
		//swapping some tokens
		assert_noop!(
			Aswap::swap(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 10, ASSET_C, 15, None, None),
			Error::<Test>::PairNotExists
		);
	});
}

#[test]
fn remove_liquidity_ok() {
	new_test_ext().execute_with(|| {
		//exchange creation
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		//adding liquidity
		assert_ok!(Aswap::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			500,
			ASSET_B,
			1500
		));
		assert_eq!(get_account_balance(ACCOUNT_A, LIQ_TOKEN_AB), 866);
		assert_eq!(get_pallet_balance(ASSET_A), 500);
		assert_eq!(get_pallet_balance(ASSET_B), 1500);
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 500);
		assert_eq!(pool_b.reserve, 1500);
		assert_eq!(
			get_account_balance(ACCOUNT_A, ASSET_A),
			ACCOUNTS_START_BALANCE.checked_sub(500).unwrap()
		);
		assert_eq!(
			get_account_balance(ACCOUNT_A, ASSET_B),
			ACCOUNTS_START_BALANCE.checked_sub(1500).unwrap()
		);
		assert_ok!(Aswap::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), LIQ_TOKEN_AB, 600));
		//amount to return asset x = (amount of lp asset to burn_withdraw * reserves of a) / total
		// lp asset supply
		assert_eq!(
			get_account_balance(ACCOUNT_A, ASSET_A),
			ACCOUNTS_START_BALANCE.checked_sub(500).unwrap().checked_add(346).unwrap()
		);
		assert_eq!(
			get_account_balance(ACCOUNT_A, ASSET_B),
			ACCOUNTS_START_BALANCE.checked_sub(1500).unwrap().checked_add(1039).unwrap()
		);
	});
}

#[test]
fn remove_liquidity_lp_not_exists() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Aswap::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), LIQ_TOKEN_AB, 600),
			Error::<Test>::TokenNotExists
		);
	});
}

#[test]
fn remove_liquidity_cannot_above_balance() {
	new_test_ext().execute_with(|| {
		//exchange creation
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		//adding liquidity
		assert_ok!(Aswap::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			500,
			ASSET_B,
			1500
		));
		assert_eq!(get_account_balance(ACCOUNT_A, LIQ_TOKEN_AB), 866);
		assert_noop!(
			Aswap::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), LIQ_TOKEN_AB, 900),
			Error::<Test>::LowBalance
		);
	});
}

#[test]
fn remove_liquidity_mustbe_above_zero() {
	new_test_ext().execute_with(|| {
		//exchange creation
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		//adding liquidity
		assert_ok!(Aswap::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			500,
			ASSET_B,
			1500
		));
		assert_eq!(get_account_balance(ACCOUNT_A, LIQ_TOKEN_AB), 866);
		assert_noop!(
			Aswap::remove_liquidity(RuntimeOrigin::signed(ACCOUNT_A), LIQ_TOKEN_AB, 0),
			Error::<Test>::InvalidAmount
		);
	});
}

#[test]
fn add_liquidity_ok() {
	new_test_ext().execute_with(|| {
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 0u128.into());
		assert_eq!(pool_b.reserve, 0u128.into());
		assert_eq!(pool_a.asset_lp_id, LIQ_TOKEN_AB);
		assert_eq!(pool_b.asset_lp_id, LIQ_TOKEN_AB);
		// assert the exchange has the correct asset balance
		let pallet_balance_a = get_pallet_balance(ASSET_A);
		let pallet_balance_b = get_pallet_balance(ASSET_B);
		assert_eq!(pallet_balance_a, 0);
		assert_eq!(pallet_balance_b, 0);
		// very first time adding liquidity
		let asset_a_amount = 500;
		let asset_b_amount = 1_500;
		assert_ok!(Aswap::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			asset_a_amount,
			ASSET_B,
			asset_b_amount
		));
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 500);
		assert_eq!(pool_b.reserve, 1_500);
		let liquidity_pool_details = Aswap::liquidity_pools(LIQ_TOKEN_AB).unwrap();
		let mul_result = asset_a_amount.checked_mul(asset_b_amount).unwrap();
		let liquidity_to_mint = mul_result.integer_sqrt_checked(); //expected 866 lp tokens
		assert_eq!(liquidity_pool_details.supply, liquidity_to_mint.unwrap());
		//checking pallet account balances
		let pallet_balance_a = get_pallet_balance(ASSET_A);
		let pallet_balance_b = get_pallet_balance(ASSET_B);
		assert_eq!(pallet_balance_a, 500);
		assert_eq!(pallet_balance_b, 1_500);
		assert_eq!(get_account_balance(ACCOUNT_A, ASSET_A), 999_999_500);
		assert_eq!(get_account_balance(ACCOUNT_A, ASSET_B), 999_998_500);
		// adding more liquidity
		let asset_a_amount_2 = 1_000;
		let asset_b_amount_2 = 3_000;
		assert_ok!(Aswap::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_B),
			ASSET_A,
			asset_a_amount_2,
			ASSET_B,
			asset_b_amount_2
		));
		//lp tokens to mint = min ( (amount asset a * total lp asset supply ) / a asset reserves,
		// (amount asset b * total lp asset supply ) / b asset reserves)
		let value_1 = asset_a_amount_2
			.checked_mul(liquidity_to_mint.unwrap())
			.unwrap()
			.checked_div(500)
			.unwrap();
		let value_2 = asset_b_amount_2
			.checked_mul(liquidity_to_mint.unwrap())
			.unwrap()
			.checked_div(1500)
			.unwrap();
		let new_supply = if value_1 < value_2 { value_1 } else { value_2 };

		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		let liquidity_pool_details = Aswap::liquidity_pools(LIQ_TOKEN_AB).unwrap();
		assert_eq!(pool_a.reserve, 1_500);
		assert_eq!(pool_b.reserve, 4_500);
		assert_eq!(liquidity_pool_details.supply, new_supply);
		//checking pallet account balances
		let pallet_balance_a = get_pallet_balance(ASSET_A);
		let pallet_balance_b = get_pallet_balance(ASSET_B);
		assert_eq!(pallet_balance_a, 1_500);
		assert_eq!(pallet_balance_b, 4_500);
		assert_eq!(get_account_balance(ACCOUNT_B, ASSET_A), 999_999_000);
		assert_eq!(get_account_balance(ACCOUNT_B, ASSET_B), 999_997_000);
		assert_eq!(get_account_balance(ACCOUNT_A, ASSET_A), 999_999_500);
		assert_eq!(get_account_balance(ACCOUNT_A, ASSET_B), 999_998_500);
	});
}

#[test]
fn add_liquidity_pool_not_exists() {
	new_test_ext().execute_with(|| {
		let asset_a_amount = 500;
		let asset_b_amount = 1_500;
		assert_noop!(
			Aswap::add_liquidity(
				RuntimeOrigin::signed(ACCOUNT_A),
				ASSET_B,
				asset_a_amount,
				ASSET_B,
				asset_b_amount
			),
			Error::<Test>::PairNotExists
		);
		assert_noop!(
			Aswap::add_liquidity(
				RuntimeOrigin::signed(ACCOUNT_A),
				ASSET_NOT_EXIST,
				asset_a_amount,
				ASSET_B,
				asset_b_amount
			),
			Error::<Test>::TokenNotExists
		);
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 0u128.into());
		assert_eq!(pool_b.reserve, 0u128.into());
		assert_eq!(pool_a.asset_lp_id, LIQ_TOKEN_AB);
		assert_eq!(pool_b.asset_lp_id, LIQ_TOKEN_AB);
		assert_noop!(
			Aswap::add_liquidity(
				RuntimeOrigin::signed(ACCOUNT_A),
				ASSET_A,
				asset_a_amount,
				ASSET_C,
				asset_b_amount
			),
			Error::<Test>::PairNotExists
		);
	});
}

#[test]
fn add_liquidity_pool_zero_no_allowed() {
	new_test_ext().execute_with(|| {
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		let pool_a = Aswap::liquidity_pools_reserves((ASSET_A, ASSET_B)).unwrap();
		let pool_b = Aswap::liquidity_pools_reserves((ASSET_B, ASSET_A)).unwrap();
		assert_eq!(pool_a.reserve, 0u128.into());
		assert_eq!(pool_b.reserve, 0u128.into());
		assert_eq!(pool_a.asset_lp_id, LIQ_TOKEN_AB);
		assert_eq!(pool_b.asset_lp_id, LIQ_TOKEN_AB);
		assert_noop!(
			Aswap::add_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 0, ASSET_B, 0),
			Error::<Test>::InvalidAmount
		);
		assert_noop!(
			Aswap::add_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 1, ASSET_B, 0),
			Error::<Test>::InvalidAmount
		);
		assert_noop!(
			Aswap::add_liquidity(RuntimeOrigin::signed(ACCOUNT_A), ASSET_A, 0, ASSET_B, 1),
			Error::<Test>::InvalidAmount
		);
	});
}

#[test]
fn add_liquidity_pool_account_with_low_balance_or_without_balance() {
	new_test_ext().execute_with(|| {
		assert_ok!(Aswap::new_exchange(
			RuntimeOrigin::signed(ACCOUNT_A),
			ASSET_A,
			ASSET_B,
			LIQ_TOKEN_AB
		));
		assert_noop!(
			Aswap::add_liquidity(
				RuntimeOrigin::signed(ACCOUNT_D_LOW_BALANCES),
				ASSET_A,
				15,
				ASSET_B,
				1
			),
			Error::<Test>::LowBalance
		);
		assert_ok!(Aswap::add_liquidity(
			RuntimeOrigin::signed(ACCOUNT_D_LOW_BALANCES),
			ASSET_A,
			1,
			ASSET_B,
			1
		));
		assert_noop!(
			Aswap::add_liquidity(
				RuntimeOrigin::signed(ACCOUNT_D_LOW_BALANCES),
				ASSET_A,
				1,
				ASSET_C,
				1
			),
			Error::<Test>::LowBalance
		);
	});
}

#[test]
fn unsigned_cannot_execute_actions() {
	new_test_ext().execute_with(|| {
		assert_noop!(
			Aswap::new_exchange(RuntimeOrigin::none(), ASSET_A, ASSET_B, LIQ_TOKEN_AB),
			frame_support::error::BadOrigin
		);
		assert_noop!(
			Aswap::add_liquidity(RuntimeOrigin::none(), ASSET_A, 15, ASSET_B, 1),
			frame_support::error::BadOrigin
		);
		assert_noop!(
			Aswap::remove_liquidity(RuntimeOrigin::none(), LIQ_TOKEN_AB, 10),
			frame_support::error::BadOrigin
		);
		assert_noop!(
			Aswap::swap(RuntimeOrigin::none(), ASSET_B, 10, ASSET_A, 2, None, None),
			frame_support::error::BadOrigin
		);
	});
}

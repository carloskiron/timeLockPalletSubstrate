use crate::{mock::*, mock_data::*, Error};
use codec::Encode;
use frame_support::{assert_noop, assert_ok, sp_io::hashing};

/// Account A wants to swap 1_000 units of A per 1_000 units of B with Account B
#[test]
fn lock_ok() {
	new_test_ext().execute_with(|| {
		let secret = b"Something between us 2023";
		let hash = hashing::sha2_256(secret);
		let asset_amount = 1_000;
		//5 blocks
		let timelock = 5;
		let tx_id_elements = (ACCOUNT_A, ACCOUNT_B, hash, timelock, ASSET_A, asset_amount).encode();
		let tx_id = hashing::sha2_256(&tx_id_elements.as_slice());

		assert_eq!(get_pallet_balance(ASSET_A), PALLET_START_BALANCE);
		assert_ok!(Aswap::lock(
			RuntimeOrigin::signed(ACCOUNT_A),
			tx_id,
			ACCOUNT_B,
			hash,
			timelock,
			ASSET_A,
			asset_amount
		));
		assert_eq!(get_pallet_balance(ASSET_A), PALLET_START_BALANCE + asset_amount);

		let lock_details = Aswap::lock_transactions(tx_id).unwrap();
		assert_eq!(lock_details.amount, asset_amount);
		assert_eq!(lock_details.sender, ACCOUNT_A);
		assert_eq!(lock_details.recipient, ACCOUNT_B);
		assert_eq!(lock_details.asset_id, ASSET_A);
		assert_eq!(lock_details.hashlock, hash);
		assert_eq!(lock_details.expiration_block, timelock + 1);
		assert_eq!(lock_details.is_withdraw, false);
		assert_eq!(lock_details.is_refunded, false);
	});
}

#[test]
fn lock_validations_ok() {
	new_test_ext().execute_with(|| {
		let secret = b"Something between us 2023";
		let hash = hashing::sha2_256(secret);
		let asset_amount = 1_000;
		//5 blocks
		let timelock = 5;
		let tx_id_elements = (ACCOUNT_A, ACCOUNT_B, hash, timelock, ASSET_A, asset_amount).encode();
		let tx_id = hashing::sha2_256(&tx_id_elements.as_slice());

		assert_eq!(get_pallet_balance(ASSET_A), PALLET_START_BALANCE);
		assert_ok!(Aswap::lock(
			RuntimeOrigin::signed(ACCOUNT_A),
			tx_id,
			ACCOUNT_B,
			hash,
			timelock,
			ASSET_A,
			asset_amount
		));
		assert_eq!(get_pallet_balance(ASSET_A), PALLET_START_BALANCE + asset_amount);

		let lock_details = Aswap::lock_transactions(tx_id).unwrap();
		assert_eq!(lock_details.amount, asset_amount);
		assert_eq!(lock_details.sender, ACCOUNT_A);
		assert_eq!(lock_details.recipient, ACCOUNT_B);
		assert_eq!(lock_details.asset_id, ASSET_A);
		assert_eq!(lock_details.hashlock, hash);
		assert_eq!(lock_details.expiration_block, timelock + 1);
		assert_eq!(lock_details.is_withdraw, false);
		assert_eq!(lock_details.is_refunded, false);

		//trying to add unsigned
		assert_noop!(
			Aswap::lock(
				RuntimeOrigin::none(),
				tx_id,
				ACCOUNT_B,
				hash,
				timelock,
				ASSET_A,
				asset_amount
			),
			frame_support::error::BadOrigin
		);

		//trying to add a new lock with same tx_id
		assert_noop!(
			Aswap::lock(
				RuntimeOrigin::signed(ACCOUNT_B),
				tx_id,
				ACCOUNT_C,
				hash,
				timelock,
				ASSET_A,
				asset_amount
			),
			Error::<Test>::TransactionIdExists
		);

		let new_tx_id_elements =
			(ACCOUNT_B, ACCOUNT_C, hash, timelock, ASSET_A, asset_amount).encode();
		let new_tx_id = hashing::sha2_256(&new_tx_id_elements.as_slice());

		//trying to add wrong deadline
		assert_noop!(
			Aswap::lock(
				RuntimeOrigin::signed(ACCOUNT_B),
				new_tx_id,
				ACCOUNT_C,
				hash,
				0,
				ASSET_A,
				asset_amount
			),
			Error::<Test>::InvalidTimelock
		);

		//trying to add non existent asset
		assert_noop!(
			Aswap::lock(
				RuntimeOrigin::signed(ACCOUNT_B),
				new_tx_id,
				ACCOUNT_C,
				hash,
				timelock,
				ASSET_NOT_EXIST,
				asset_amount
			),
			Error::<Test>::TokenNotExists
		);

		//trying with origin with low balance
		assert_noop!(
			Aswap::lock(
				RuntimeOrigin::signed(ACCOUNT_D_LOW_BALANCES),
				new_tx_id,
				ACCOUNT_C,
				hash,
				timelock,
				ASSET_A,
				asset_amount
			),
			Error::<Test>::LowBalance
		);
	});
}

#[test]
fn unlock_ok() {
	new_test_ext().execute_with(|| {
		let secret = b"Something between us 2023";
		let hash = hashing::sha2_256(secret);
		let asset_amount = 1_000;
		//5 blocks
		let timelock = 5;
		let tx_id_elements = (ACCOUNT_A, ACCOUNT_B, hash, timelock, ASSET_A, asset_amount).encode();
		let tx_id = hashing::sha2_256(&tx_id_elements.as_slice());

		assert_eq!(get_pallet_balance(ASSET_A), PALLET_START_BALANCE);
		assert_ok!(Aswap::lock(
			RuntimeOrigin::signed(ACCOUNT_A),
			tx_id,
			ACCOUNT_B,
			hash,
			timelock,
			ASSET_A,
			asset_amount
		));
		assert_eq!(get_pallet_balance(ASSET_A), PALLET_START_BALANCE + asset_amount);

		let lock_details = Aswap::lock_transactions(tx_id).unwrap();
		assert_eq!(lock_details.amount, asset_amount);
		assert_eq!(lock_details.sender, ACCOUNT_A);
		assert_eq!(lock_details.recipient, ACCOUNT_B);
		assert_eq!(lock_details.asset_id, ASSET_A);
		assert_eq!(lock_details.hashlock, hash);
		assert_eq!(lock_details.expiration_block, timelock + 1);
		assert_eq!(lock_details.is_withdraw, false);
		assert_eq!(lock_details.is_refunded, false);

		//Account b unlocking
		assert_eq!(get_account_balance(ACCOUNT_B, ASSET_A), ACCOUNTS_START_BALANCE);
		assert_ok!(Aswap::unlock(RuntimeOrigin::signed(ACCOUNT_B), tx_id, secret.to_vec()));
		assert_eq!(get_account_balance(ACCOUNT_B, ASSET_A), ACCOUNTS_START_BALANCE + asset_amount);
		assert_eq!(get_pallet_balance(ASSET_A), PALLET_START_BALANCE);

		//reveled secret
		let known_secret = Aswap::known_secrets(tx_id).unwrap();
		assert_eq!(known_secret, secret.to_vec());
	});
}

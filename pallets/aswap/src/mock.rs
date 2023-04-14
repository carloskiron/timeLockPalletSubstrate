use crate as pallet_aswap;
use crate::mock_data::*;
use frame_support::{
	parameter_types,
	traits::{
		fungibles, AsEnsureOriginWithArg, ConstU128, ConstU16, ConstU32, ConstU64, GenesisBuild,
	},
	PalletId,
};
use frame_system::{EnsureRoot, EnsureSigned};
use sp_core::H256;
use sp_runtime::{
	testing::Header,
	traits::{BlakeTwo256, IdentityLookup},
};

type UncheckedExtrinsic = frame_system::mocking::MockUncheckedExtrinsic<Test>;
type Block = frame_system::mocking::MockBlock<Test>;

type Balance = u128;

// Configure a mock runtime to test the pallet.
frame_support::construct_runtime!(
	pub enum Test where
		Block = Block,
		NodeBlock = Block,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system,
		Balances: pallet_balances,
		Assets: pallet_assets,
		Aswap: pallet_aswap,
	}
);

impl frame_system::Config for Test {
	type BaseCallFilter = frame_support::traits::Everything;
	type BlockWeights = ();
	type BlockLength = ();
	type DbWeight = ();
	type RuntimeOrigin = RuntimeOrigin;
	type RuntimeCall = RuntimeCall;
	type Index = u64;
	type BlockNumber = u64;
	type Hash = H256;
	type Hashing = BlakeTwo256;
	type AccountId = u64;
	type Lookup = IdentityLookup<Self::AccountId>;
	type Header = Header;
	type RuntimeEvent = RuntimeEvent;
	type BlockHashCount = ConstU64<250>;
	type Version = ();
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type SystemWeightInfo = ();
	type SS58Prefix = ConstU16<42>;
	type OnSetCode = ();
	type MaxConsumers = frame_support::traits::ConstU32<16>;
}

parameter_types! {
	pub const ExistentialDeposit: u64 = 1;
	pub const MaxLocks: u32 = 10;
}
impl pallet_balances::Config for Test {
	type Balance = u128;
	type DustRemoval = ();
	type RuntimeEvent = RuntimeEvent;
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = MaxLocks;
	type MaxReserves = ();
	type ReserveIdentifier = [u8; 8];
}

parameter_types! {
	pub const AssetDeposit: Balance = 100;
	pub const ApprovalDeposit: Balance = 1;
	pub const StringLimit: u32 = 50;
	pub const MetadataDepositBase: Balance = 10;
	pub const MetadataDepositPerByte: Balance = 1;
}

impl pallet_assets::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Balance = u128;
	type AssetId = u32;
	type AssetIdParameter = codec::Compact<u32>;
	type Currency = Balances;
	type CreateOrigin = AsEnsureOriginWithArg<EnsureSigned<Self::AccountId>>;
	type ForceOrigin = EnsureRoot<Self::AccountId>;
	type AssetDeposit = AssetDeposit;
	type AssetAccountDeposit = ConstU128<1>;
	type MetadataDepositBase = MetadataDepositBase;
	type MetadataDepositPerByte = MetadataDepositPerByte;
	type ApprovalDeposit = ApprovalDeposit;
	type StringLimit = StringLimit;
	type Freezer = ();
	type Extra = ();
	type WeightInfo = ();
	type RemoveItemsLimit = ConstU32<1000>;
}

parameter_types! {
	pub const AswapPalletId: PalletId = PalletId(*b"aswapjur");
}

impl pallet_aswap::Config for Test {
	type RuntimeEvent = RuntimeEvent;
	type Currency = Balances;
	type Fungibles = Assets;
	type PalletId = AswapPalletId;
}

pub fn get_pallet_balance(asset_id: u32) -> Balance {
	let pallet_account = Aswap::account_id();
	<<Test as crate::Config>::Fungibles as fungibles::Inspect<_>>::balance(
		asset_id,
		&pallet_account,
	)
}

pub fn get_account_balance(account_id: u64, asset_id: u32) -> Balance {
	<<Test as crate::Config>::Fungibles as fungibles::Inspect<_>>::balance(asset_id, &account_id)
}

// Build genesis storage according to the mock runtime.
pub fn new_test_ext() -> sp_io::TestExternalities {
	let mut storage = frame_system::GenesisConfig::default().build_storage::<Test>().unwrap();

	pallet_balances::GenesisConfig::<Test> {
		balances: vec![
			(ACCOUNT_A, ACCOUNTS_START_BALANCE),
			(ACCOUNT_B, ACCOUNTS_START_BALANCE),
			(ACCOUNT_C, ACCOUNTS_START_BALANCE),
			(ACCOUNT_D_LOW_BALANCES, ACCOUNTS_START_LOW_BALANCE),
			(Aswap::account_id(), ACCOUNTS_START_BALANCE),
		],
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	pallet_assets::GenesisConfig::<Test> {
		assets: vec![
			(ASSET_A, ACCOUNT_A, true, 1),
			(ASSET_B, ACCOUNT_B, true, 1),
			(ASSET_C, ACCOUNT_C, true, 1),
		],
		metadata: vec![],
		accounts: vec![
			(ASSET_A, Aswap::account_id(), PALLET_START_BALANCE),
			(ASSET_B, Aswap::account_id(), PALLET_START_BALANCE),
			(ASSET_C, Aswap::account_id(), PALLET_START_BALANCE),
			(ASSET_A, ACCOUNT_A, ACCOUNTS_START_BALANCE),
			(ASSET_A, ACCOUNT_B, ACCOUNTS_START_BALANCE),
			(ASSET_A, ACCOUNT_C, ACCOUNTS_START_BALANCE),
			(ASSET_A, ACCOUNT_D_LOW_BALANCES, ACCOUNTS_START_LOW_BALANCE),
			(ASSET_B, ACCOUNT_A, ACCOUNTS_START_BALANCE),
			(ASSET_B, ACCOUNT_B, ACCOUNTS_START_BALANCE),
			(ASSET_B, ACCOUNT_C, ACCOUNTS_START_BALANCE),
			(ASSET_B, ACCOUNT_D_LOW_BALANCES, ACCOUNTS_START_LOW_BALANCE),
			(ASSET_C, ACCOUNT_A, ACCOUNTS_START_BALANCE),
			(ASSET_C, ACCOUNT_B, ACCOUNTS_START_BALANCE),
			(ASSET_C, ACCOUNT_C, ACCOUNTS_START_BALANCE),
		],
	}
	.assimilate_storage(&mut storage)
	.unwrap();

	let mut test_ext: sp_io::TestExternalities = storage.into();
	test_ext.execute_with(|| System::set_block_number(1));
	test_ext
}

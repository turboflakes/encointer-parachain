// Copyright (c) 2019 Alain Brenzikofer
// This file is part of Encointer
//
// Encointer is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Encointer is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Encointer.  If not, see <http://www.gnu.org/licenses/>.

#![cfg_attr(not(feature = "std"), no_std)]
// `construct_runtime!` does a lot of recursion and requires us to increase the limit to 256.
#![recursion_limit = "256"]

// Make the WASM binary available.
#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use sp_api::impl_runtime_apis;
use sp_core::OpaqueMetadata;
use sp_runtime::{
	create_runtime_str, generic, impl_opaque_keys,
	traits::{AccountIdLookup, BlakeTwo256, Block as BlockT, Verify},
	transaction_validity::{TransactionSource, TransactionValidity},
	ApplyExtrinsicResult, MultiSignature,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

// A few exports that help ease life for downstream crates.
pub use frame_support::{
	construct_runtime, parameter_types, match_type,
	traits::{Randomness, All, IsInVec},
	weights::{
		constants::{BlockExecutionWeight, ExtrinsicBaseWeight, RocksDbWeight, WEIGHT_PER_SECOND},
		DispatchClass, IdentityFee, Weight,
	},
	StorageValue,
};
use frame_system::limits::{BlockLength, BlockWeights};
#[cfg(any(feature = "std", test))]
pub use sp_runtime::BuildStorage;
pub use sp_runtime::{Perbill, Permill};

pub use pallet_balances::Call as BalancesCall;
pub use pallet_timestamp::Call as TimestampCall;

// A few exports that help ease life for downstream crates.
pub use encointer_balances::Call as EncointerBalancesCall;
pub use encointer_bazaar::Call as EncointerBazaarCall;
pub use encointer_ceremonies::Call as EncointerCeremoniesCall;
pub use encointer_communities::Call as EncointerCommunitiesCall;
pub use encointer_scheduler::Call as EncointerSchedulerCall;

pub use encointer_primitives::balances::{BalanceEntry, BalanceType};
pub use encointer_primitives::scheduler::CeremonyPhaseType;

// XCM imports
use xcm::v0::{Junction::*, MultiLocation, MultiLocation::*, NetworkId};
use xcm_builder::{
	LocationInverter, ParentIsDefault, FixedWeightBounds, AllowUnpaidExecutionFrom,
	ParentAsSuperuser, SovereignSignedViaLocation,
};
use xcm_executor::{Config, XcmExecutor};

/// This runtime version.
#[sp_version::runtime_version]
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("encointer-parachain"),
	impl_name: create_runtime_str!("encointer-parachain"),
	authoring_version: 1,
	spec_version: 16,
	impl_version: 0,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
};

/// A type to hold UTC unix epoch [ms]
pub type Moment = u64;
pub const ONE_DAY: Moment = 86_400_000;

/// The version information used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion {
		runtime_version: VERSION,
		can_author_with: Default::default(),
	}
}

/// We assume that ~10% of the block weight is consumed by `on_initialize` handlers.
/// This is used to limit the maximal weight of a single extrinsic.
const AVERAGE_ON_INITIALIZE_RATIO: Perbill = Perbill::from_percent(10);
/// We allow `Normal` extrinsics to fill up the block up to 75%, the rest can be used
/// by  Operational  extrinsics.
const NORMAL_DISPATCH_RATIO: Perbill = Perbill::from_percent(75);
/// We allow for 2 seconds of compute with a 6 second average block time.
const MAXIMUM_BLOCK_WEIGHT: Weight = 2 * WEIGHT_PER_SECOND;

parameter_types! {
	pub const BlockHashCount: BlockNumber = 250;
	pub const Version: RuntimeVersion = VERSION;
	pub RuntimeBlockLength: BlockLength =
		BlockLength::max_with_normal_ratio(5 * 1024 * 1024, NORMAL_DISPATCH_RATIO);
	pub RuntimeBlockWeights: BlockWeights = BlockWeights::builder()
		.base_block(BlockExecutionWeight::get())
		.for_class(DispatchClass::all(), |weights| {
			weights.base_extrinsic = ExtrinsicBaseWeight::get();
		})
		.for_class(DispatchClass::Normal, |weights| {
			weights.max_total = Some(NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT);
		})
		.for_class(DispatchClass::Operational, |weights| {
			weights.max_total = Some(MAXIMUM_BLOCK_WEIGHT);
			// Operational transactions have some extra reserved space, so that they
			// are included even if block reached `MAXIMUM_BLOCK_WEIGHT`.
			weights.reserved = Some(
				MAXIMUM_BLOCK_WEIGHT - NORMAL_DISPATCH_RATIO * MAXIMUM_BLOCK_WEIGHT
			);
		})
		.avg_block_initialization(AVERAGE_ON_INITIALIZE_RATIO)
		.build_or_panic();
	pub const SS58Prefix: u8 = 42;
}

impl frame_system::Config for Runtime {
	/// The identifier used to distinguish between accounts.
	type AccountId = AccountId;
	/// The aggregated dispatch type that is available for extrinsics.
	type Call = Call;
	/// The lookup mechanism to get account ID from whatever is passed in dispatchers.
	type Lookup = AccountIdLookup<AccountId, ()>;
	/// The index type for storing how many extrinsics an account has signed.
	type Index = Index;
	/// The index type for blocks.
	type BlockNumber = BlockNumber;
	/// The type for hashing blocks and tries.
	type Hash = Hash;
	/// The hashing algorithm used.
	type Hashing = BlakeTwo256;
	/// The header type.
	type Header = generic::Header<BlockNumber, BlakeTwo256>;
	/// The ubiquitous event type.
	type Event = Event;
	/// The ubiquitous origin type.
	type Origin = Origin;
	/// Maximum number of block number to block hash mappings to keep (oldest pruned first).
	type BlockHashCount = BlockHashCount;
	/// Runtime version.
	type Version = Version;
	/// Converts a module to an index of this module in the runtime.
	type PalletInfo = PalletInfo;
	type AccountData = pallet_balances::AccountData<Balance>;
	type OnNewAccount = ();
	type OnKilledAccount = ();
	type DbWeight = ();
	type BaseCallFilter = ();
	type SystemWeightInfo = ();
	type BlockWeights = RuntimeBlockWeights;
	type BlockLength = RuntimeBlockLength;
	type SS58Prefix = SS58Prefix;
	type OnSetCode = cumulus_pallet_parachain_system::ParachainSetCode<Self>;
}

parameter_types! {
	pub const MinimumPeriod: u64 = SLOT_DURATION / 2;
}

impl pallet_timestamp::Config for Runtime {
	/// A timestamp: milliseconds since the unix epoch.
	type Moment = u64;
	type OnTimestampSet = EncointerScheduler;
	type MinimumPeriod = MinimumPeriod;
	type WeightInfo = ();
}

parameter_types! {
	// We do anything the parent chain tells us in this runtime.
	pub const ReservedDmpWeight: Weight = MAXIMUM_BLOCK_WEIGHT / 2;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type Event = Event;
	type OnValidationData = ();
	type SelfParaId = parachain_info::Pallet<Runtime>;
	type OutboundXcmpMessageSource = ();
	type DmpMessageHandler = cumulus_pallet_xcm::UnlimitedDmpExecution<Runtime>;
	type ReservedDmpWeight = ReservedDmpWeight;
	type XcmpMessageHandler = ();
	type ReservedXcmpWeight = ();
}

parameter_types! {
	pub const ExistentialDeposit: u128 = 500;
	pub const TransferFee: u128 = 0;
	pub const CreationFee: u128 = 0;
	pub const TransactionByteFee: u128 = 1;
	pub const MaxLocks: u32 = 50;
}

impl pallet_balances::Config for Runtime {
	/// The type for recording an account's balance.
	type Balance = Balance;
	/// The ubiquitous event type.
	type Event = Event;
	type DustRemoval = ();
	type ExistentialDeposit = ExistentialDeposit;
	type AccountStore = System;
	type WeightInfo = ();
	type MaxLocks = MaxLocks;
}

impl pallet_transaction_payment::Config for Runtime {
	type OnChargeTransaction = pallet_transaction_payment::CurrencyAdapter<Balances, ()>;
	type TransactionByteFee = TransactionByteFee;
	type WeightToFee = IdentityFee<Balance>;
	type FeeMultiplierUpdate = ();
}

impl pallet_sudo::Config for Runtime {
	type Call = Call;
	type Event = Event;
}

impl cumulus_pallet_parachain_system::Config for Runtime {
	type Event = Event;
	type OnValidationData = ();
	type SelfParaId = parachain_info::Module<Runtime>;
	type DownwardMessageHandlers = XcmHandler;
	type XcmpMessageHandlers = XcmHandler;
}

impl parachain_info::Config for Runtime {}

parameter_types! {
	pub const RococoLocation: MultiLocation = X1(Parent);
	pub const RococoNetwork: NetworkId = NetworkId::Polkadot;
	pub Ancestry: MultiLocation = Parachain(ParachainInfo::parachain_id().into()).into();
}

/// This is the type we use to convert an (incoming) XCM origin into a local `Origin` instance,
/// ready for dispatching a transaction with Xcm's `Transact`. There is an `OriginKind` which can
/// bias the kind of local `Origin` it will become.
pub type XcmOriginToTransactDispatchOrigin = (
	// Sovereign account converter; this attempts to derive an `AccountId` from the origin location
	// using `LocationToAccountId` and then turn that into the usual `Signed` origin. Useful for
	// foreign chains who want to have a local sovereign account on this chain which they control.
	SovereignSignedViaLocation<ParentIsDefault<AccountId>, Origin>,
	// Superuser converter for the Relay-chain (Parent) location. This will allow it to issue a
	// transaction from the Root origin.
	ParentAsSuperuser<Origin>,
);

match_type! {
	pub type JustTheParent: impl Contains<MultiLocation> = { X1(Parent) };
}

parameter_types! {
	// One XCM operation is 1_000_000 weight - almost certainly a conservative estimate.
	pub UnitWeightCost: Weight = 1_000_000;
}

pub struct XcmConfig;
impl Config for XcmConfig {
	type Call = Call;
	type XcmSender = ();	// sending XCM not supported
	type AssetTransactor = ();	// balances not supported
	type OriginConverter = XcmOriginToTransactDispatchOrigin;
	type IsReserve = ();	// balances not supported
	type IsTeleporter = ();	// balances not supported
	type LocationInverter = LocationInverter<Ancestry>;
	type Barrier = AllowUnpaidExecutionFrom<JustTheParent>;
	type Weigher = FixedWeightBounds<UnitWeightCost, Call>;	// balances not supported
	type Trader = ();	// balances not supported
	type ResponseHandler = ();	// Don't handle responses for now.
}

impl cumulus_pallet_xcm::Config for Runtime {
	type Event = Event;
	type XcmExecutor = XcmExecutor<XcmConfig>;
}

parameter_types! {
	pub const MomentsPerDay: Moment = 86_400_000; // [ms/d]
}
impl encointer_scheduler::Config for Runtime {
	type Event = Event;
	type OnCeremonyPhaseChange = encointer_ceremonies::Module<Runtime>;
	type MomentsPerDay = MomentsPerDay;
}

impl encointer_ceremonies::Config for Runtime {
	type Event = Event;
	type Public = <MultiSignature as Verify>::Signer;
	type Signature = MultiSignature;
}

impl encointer_communities::Config for Runtime {
	type Event = Event;
}

impl encointer_balances::Config for Runtime {
	type Event = Event;
}

impl encointer_bazaar::Config for Runtime {
	type Event = Event;
}

/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;

construct_runtime! {
	pub enum Runtime where
		Block = Block,
		NodeBlock = generic::Block<Header, sp_runtime::OpaqueExtrinsic>,
		UncheckedExtrinsic = UncheckedExtrinsic,
	{
		System: frame_system::{Pallet, Call, Storage, Config, Event<T>},
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event<T>, ValidateUnsigned},
		Timestamp: pallet_timestamp::{Pallet, Call, Storage, Inherent},
		Balances: pallet_balances::{Pallet, Call, Storage, Config<T>, Event<T>},
		Sudo: pallet_sudo::{Pallet, Call, Storage, Config<T>, Event<T>},
		RandomnessCollectiveFlip: pallet_randomness_collective_flip::{Pallet, Call, Storage},
		ParachainSystem: cumulus_pallet_parachain_system::{Pallet, Call, Storage, Inherent, Event},
		TransactionPayment: pallet_transaction_payment::{Pallet, Storage},
		ParachainInfo: parachain_info::{Pallet, Storage, Config},
		EncointerScheduler: encointer_scheduler::{Pallet, Call, Storage, Config<T>, Event},
		EncointerCeremonies: encointer_ceremonies::{Pallet, Call, Storage, Config<T>, Event<T>},
		EncointerCommunities: encointer_communities::{Pallet, Call, Storage, Config<T>, Event<T>},
		EncointerBalances: encointer_balances::{Pallet, Call, Storage, Event<T>},
		EncointerBazaar: encointer_bazaar::{Pallet, Call, Storage, Event<T>},

		// DMP handler.
		CumulusXcm: cumulus_pallet_xcm::{Pallet, Call, Storage, Event<T>, Origin},
	}
}

/// Simple implementation which fails any transaction which is signed.
#[derive(Eq, PartialEq, Clone, Default, sp_core::RuntimeDebug, codec::Encode, codec::Decode)]
pub struct DisallowSigned;
impl sp_runtime::traits::SignedExtension for DisallowSigned {
	const IDENTIFIER: &'static str = "DisallowSigned";
	type AccountId = AccountId;
	type Call = Call;
	type AdditionalSigned = ();
	type Pre = ();
	fn additional_signed(&self)
		-> sp_std::result::Result<(), sp_runtime::transaction_validity::TransactionValidityError>
	{
		Ok(())
	}
	fn validate(
		&self,
		_who: &Self::AccountId,
		_call: &Self::Call,
		_info: &sp_runtime::traits::DispatchInfoOf<Self::Call>,
		_len: usize,
	) -> TransactionValidity {
		let i = sp_runtime::transaction_validity::InvalidTransaction::BadProof;
		Err(sp_runtime::transaction_validity::TransactionValidityError::Invalid(i))
	}
}

/// Alias to 512-bit hash when used in the context of a transaction signature on the chain.
pub type Signature = sp_runtime::MultiSignature;
/// Some way of identifying an account on the chain. We intentionally make it equivalent
/// to the public key of our transaction signing scheme.
pub type AccountId = <<Signature as sp_runtime::traits::Verify>::Signer as sp_runtime::traits::IdentifyAccount>::AccountId;
/// Index of a transaction in the chain.
pub type Index = u32;
/// A hash of some data used by the chain.
pub type Hash = sp_core::H256;
/// An index to a block.
pub type BlockNumber = u32;
/// The address format for describing accounts.
pub type Address = sp_runtime::MultiAddress<AccountId, ()>;
/// Block header type as expected by this runtime.
pub type Header = generic::Header<BlockNumber, BlakeTwo256>;
/// Block type as expected by this runtime.
pub type Block = generic::Block<Header, UncheckedExtrinsic>;
/// A Block signed with a Justification
pub type SignedBlock = generic::SignedBlock<Block>;
/// BlockId type as expected by this runtime.
pub type BlockId = generic::BlockId<Block>;
/// The SignedExtension to the basic transaction logic.
pub type SignedExtra = DisallowSigned;
/// Unchecked extrinsic type as expected by this runtime.
pub type UncheckedExtrinsic = generic::UncheckedExtrinsic<Address, Call, Signature, SignedExtra>;
/// Extrinsic type that has already been checked.
pub type CheckedExtrinsic = generic::CheckedExtrinsic<AccountId, Call, SignedExtra>;
/// Executive: handles dispatch to the various modules.
pub type Executive = frame_executive::Executive<
	Runtime,
	Block,
	frame_system::ChainContext<Runtime>,
	Runtime,
	AllPallets,
>;

impl_runtime_apis! {
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Executive::execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Executive::initialize_block(header)
		}
	}

	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			Runtime::metadata().into()
		}
	}

	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(
			extrinsic: <Block as BlockT>::Extrinsic,
		) -> ApplyExtrinsicResult {
			Executive::apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Executive::finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			data.create_extrinsics()
		}

		fn check_inherents(block: Block, data: sp_inherents::InherentData) -> sp_inherents::CheckInherentsResult {
			data.check_extrinsics(&block)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
		) -> TransactionValidity {
			Executive::validate_transaction(source, tx)
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(header: &<Block as BlockT>::Header) {
			Executive::offchain_worker(header)
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn decode_session_keys(_: Vec<u8>) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			Some(Vec::new())
		}

		fn generate_session_keys(_: Option<Vec<u8>>) -> Vec<u8> {
			Vec::new()
		}
	}

	impl cumulus_primitives_core::CollectCollationInfo<Block> for Runtime {
		fn collect_collation_info() -> cumulus_primitives_core::CollationInfo {
			ParachainSystem::collect_collation_info()
		}
	}
}

cumulus_pallet_parachain_system::register_validate_block!(Runtime, Executive);
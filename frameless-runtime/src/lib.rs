#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
include!(concat!(env!("OUT_DIR"), "/wasm_binary.rs"));

use parity_scale_codec::{Decode, Encode};
use sp_consensus_aura::sr25519::AuthorityId as AuraId;

use log::{debug, info, trace};

use sp_api::{impl_runtime_apis, HashT};
use sp_runtime::{
	create_runtime_str,
	impl_opaque_keys,
	traits::{BlakeTwo256, Block as BlockT, Extrinsic},
	transaction_validity::{
		InvalidTransaction, TransactionSource, TransactionValidity, TransactionValidityError,
		ValidTransaction,
	},
	ApplyExtrinsicResult, BoundToRuntimeAppPublic,
};
use sp_std::prelude::*;
#[cfg(feature = "std")]
use sp_storage::well_known_keys;

#[cfg(any(feature = "std", test))]
use sp_runtime::{BuildStorage, Storage};

use sp_core::{hexdisplay::HexDisplay, OpaqueMetadata, H256, sr25519};
use sp_io::hashing::blake2_256;

#[cfg(feature = "std")]
use sp_version::NativeVersion;
use sp_version::RuntimeVersion;

#[cfg(feature = "std")]
use serde::{Deserialize, Serialize};
use sp_core::sr25519::{Public, Signature};

use sp_io::crypto::sr25519_verify;
use crate::Call::Transfer;

/// Opaque types. These are used by the CLI to instantiate machinery that don't need to know
/// the specifics of the runtime. They can then be made to be agnostic over specific formats
/// of data like extrinsics, allowing for them to continue syncing the network through upgrades
/// to even the core datas-tructures.
pub mod opaque {
	use super::*;
	// TODO: eventually you will have to change this.
	type OpaqueExtrinsic = BasicExtrinsic;
	// type OpaqueExtrinsic = Vec<u8>;

	/// Opaque block header type.
	pub type Header = sp_runtime::generic::Header<BlockNumber, BlakeTwo256>;
	/// Opaque block type.
	pub type Block = sp_runtime::generic::Block<Header, OpaqueExtrinsic>;

	// This part is necessary for generating session keys in the runtime
	impl_opaque_keys! {
		pub struct SessionKeys {
			pub aura: AuraAppPublic,
			pub grandpa: GrandpaAppPublic,
		}
	}

	// Typically these are not implemented manually, but rather for the pallet associated with the
	// keys. Here we are not using the pallets, and these implementations are trivial, so we just
	// re-write them.
	pub struct AuraAppPublic;
	impl BoundToRuntimeAppPublic for AuraAppPublic {
		type Public = AuraId;
	}

	pub struct GrandpaAppPublic;
	impl BoundToRuntimeAppPublic for GrandpaAppPublic {
		type Public = sp_finality_grandpa::AuthorityId;
	}
}

/// This runtime version.
pub const VERSION: RuntimeVersion = RuntimeVersion {
	spec_name: create_runtime_str!("frameless-runtime"),
	impl_name: create_runtime_str!("frameless-runtime"),
	authoring_version: 1,
	spec_version: 1,
	impl_version: 1,
	apis: RUNTIME_API_VERSIONS,
	transaction_version: 1,
	state_version: 0,
};

/// The version infromation used to identify this runtime when compiled natively.
#[cfg(feature = "std")]
pub fn native_version() -> NativeVersion {
	NativeVersion { runtime_version: VERSION, can_author_with: Default::default() }
}

/// The type that provides the genesis storage values for a new chain
#[cfg_attr(feature = "std", derive(Serialize, Deserialize, Default))]
pub struct GenesisConfig;

#[cfg(feature = "std")]
impl BuildStorage for GenesisConfig {
	fn assimilate_storage(&self, storage: &mut Storage) -> Result<(), String> {
		// we have nothing to put into storage in genesis, except this:
		storage.top.insert(
			well_known_keys::CODE.into(),
			WASM_BINARY.unwrap().to_vec()
		);
		Ok(())
	}
}

pub type BlockNumber = u32;
pub type Header = sp_runtime::generic::Header<BlockNumber, BlakeTwo256>;
pub type Block = sp_runtime::generic::Block<Header, BasicExtrinsic>;


pub struct Account([u8; 32]);
// pub type Address = [u8; 32];
pub type Amount = u32;

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Clone, Copy, Decode, PartialEq, Eq, Debug)]
pub struct Transaction {
	pub from: Public,
	pub to: Public,
	pub send_amount: Amount,
	pub fee: Amount
}

#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Encode, Decode, Debug, PartialEq, Eq, Clone)]
pub enum Call {
	SetValue(u32),
	Transfer(Transaction),
	Mint(Public, Amount),
	Upgrade(Vec<u8>),
}

// this extrinsic type does nothing other than fulfill the compiler.
#[cfg_attr(feature = "std", derive(Serialize, Deserialize))]
#[derive(Debug, Encode, Decode, PartialEq, Eq, Clone)]
pub struct BasicExtrinsic{
	call: Call,
	signature: Option<Signature>
}

impl BasicExtrinsic {
	pub fn new_unsigned(call: Call) -> Self {
		<Self as Extrinsic>::new(call, None).unwrap()
	}
	pub fn signed(call: Call, signature: Option<<Self as Extrinsic>::SignaturePayload>) -> Self {
		<Self as Extrinsic>::new(call, signature).unwrap()
	}
}

impl sp_runtime::traits::Extrinsic for BasicExtrinsic {
	type Call = Call;
	type SignaturePayload = Signature;

	fn new(data: Self::Call, signature: Option<Self::SignaturePayload>) -> Option<Self> {
		Some(Self{ call: data, signature: signature})
	}
}

impl sp_runtime::traits::GetNodeBlockType for Runtime {
	type NodeBlock = opaque::Block;
}

impl sp_runtime::traits::GetRuntimeBlockType for Runtime {
	type RuntimeBlock = Block;
}

const LOG_TARGET: &'static str = "frameless";
const BLOCK_TIME: u64 = 3000;

const HEADER_KEY: &[u8] = b"header"; // 686561646572
const EXTRINSIC_KEY: &[u8] = b"extrinsics";
const VALUE_KEY: &[u8] = b"VALUE_KEY";
const BLOCK_REWARD: u32 = 10;

// just FYI:
// :code => 3a636f6465

/// The main struct in this module. In frame this comes from `construct_runtime!`
pub struct Runtime;

type DispatchResult = Result<(), ()>;

impl Runtime {
	fn print_state() {
		let mut key = vec![];
		while let Some(next) = sp_io::storage::next_key(&key) {
			let val = sp_io::storage::get(&next).unwrap().to_vec();
			log::trace!(
				target: LOG_TARGET,
				"{} <=> {}",
				HexDisplay::from(&next),
				HexDisplay::from(&val)
			);
			key = next;
		}
	}

	fn get_state<T: Decode>(key: &[u8]) -> Option<T> {
		sp_io::storage::get(key).and_then(|d| T::decode(&mut &*d).ok())
	}

	fn mutate_state<T: Decode + Encode + Default>(key: &[u8], update: impl FnOnce(&mut T)) {
		let mut value = Self::get_state(key).unwrap_or_default();
		update(&mut value);
		sp_io::storage::set(key, &value.encode());
	}

	fn dispatch_extrinsic(ext: BasicExtrinsic) -> DispatchResult {
		log::debug!(target: LOG_TARGET, "I am dispatching {:?}", ext);

		Self::mutate_state::<Vec<Vec<u8>>>(EXTRINSIC_KEY, |s| s.push(ext.encode()));

		// execute it
		match ext.call {
			Call::SetValue(v) => {
				sp_io::storage::set(VALUE_KEY, &v.encode());
			},
			Call::Transfer(transaction) => {
				// Fetch Balances
				let origin_balance = Self::get_state::<Amount>(&transaction.from).unwrap_or(0);
				let target_balance = Self::get_state::<Amount>(&transaction.to).unwrap_or(0);

				let message_hash = blake2_256(&transaction.clone().encode());

				// Transfer Extrinsic needs a Signature
				if ext.signature == None {
					info!(target: LOG_TARGET,"👍❌NO SIGNATURE PROVIDED, DROPPING EXTRINSIC .... ");
					return Err::<(), ()>(());
				}

				// Extrinsic Signature is invalid => Not signed by sender
				if !sr25519_verify(&ext.signature.unwrap(), &message_hash, &transaction.from) {
					info!(target: LOG_TARGET,"👍❌INVALID SIGNATURE PROVIDED, DROPPING EXTRINSIC .... ");
					return Err::<(), ()>(());
				}

				info!(target: LOG_TARGET,"👍✅SIGNATURE STATUS .... ");

				// Validate Funds Transferred then Set State
				if origin_balance >= (transaction.send_amount + transaction.fee) {
					let new_origin_balance = origin_balance - (transaction.send_amount + transaction.fee);
					info!(target: LOG_TARGET,"✅SETTING ORIGIN BALANCE FROM {:?} TO {:?} .... ", origin_balance, new_origin_balance);
					sp_io::storage::set(&transaction.from, &new_origin_balance.encode());
					let new_target_balance = target_balance + transaction.send_amount;
					info!(target: LOG_TARGET,"✅SETTING TARGET BALANCE FROM {:?} TO {:?} .... ", target_balance, new_target_balance);
					sp_io::storage::set(&transaction.to, &new_target_balance.encode());
				} else{
					info!(target: LOG_TARGET,"❌ ORIGIN BALANCE DOESNT HAVE ENOUGH AMOUNT TO SEND {:?} WITH FEE AMOUNT {:?} .... ", transaction.send_amount, transaction.fee);
					return Err(());
				}



			},
			Call::Upgrade(new_wasm_code) => {
				// NOTE: make sure to upgrade your spec-version!
				sp_io::storage::set(sp_storage::well_known_keys::CODE, &new_wasm_code);
			},

			Call::Mint(account_address, amount) => {
				let current_account_balance = Self::get_state::<Amount>(&account_address).unwrap_or(0);

				if current_account_balance > 0 {
					let new_contract_balance = current_account_balance + amount;
					sp_io::storage::set(&account_address, &new_contract_balance.encode());
					info!(target: LOG_TARGET,"🚀ADDRESS {:?} TOKENS INCREMENTED FROM {:?} TO {:?} TOKENS  ", account_address, current_account_balance, new_contract_balance);
				} else{
					// initialize mint amount
					sp_io::storage::set(&account_address, &amount.encode());
					info!(target: LOG_TARGET,"💰MINTING STEP- ADDRESS {:?} INITIALIZED WITH {:?} TOKENS ", account_address,amount);
				}
			}
		}
		Ok(())
	}

	fn do_initialize_block(header: &<Block as BlockT>::Header) {
		info!(
			target: LOG_TARGET,
			"Entering initialize_block. header: {:?} / version: {:?}", header, VERSION.spec_version
		);
		sp_io::storage::set(&HEADER_KEY, &header.encode());

		// BLOCK_REWARDs for Alice (Could not get to work on finalize block)
		let alice_pubkey: Public = Public::from_raw(hex_literal::hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d"));
		let author_balance = Self::get_state::<Amount>(&alice_pubkey).unwrap_or(0);
		info!(target: LOG_TARGET, "🤑ALICE BALANCE: {:?}", author_balance);
		let new_reward = author_balance + BLOCK_REWARD;
		sp_io::storage::set(&alice_pubkey, &new_reward.encode());
		info!(target: LOG_TARGET, "🤑REWARDING ALICE for AUTHORING BLOCK, ALICE TOTAL REWARDS: {:?}", new_reward);
	}

	fn do_finalize_block() -> <Block as BlockT>::Header {
		let mut header = Self::get_state::<<Block as BlockT>::Header>(HEADER_KEY)
			.expect("We initialized with header, it never got mutated, qed");

		// the header itself contains the state root, so it cannot be inside the state (circular
		// dependency..). Make sure in execute block path we have the same rule.
		sp_io::storage::clear(&HEADER_KEY);

		let extrinsics = Self::get_state::<Vec<Vec<u8>>>(EXTRINSIC_KEY).unwrap_or_default();
		let extrinsics_root =
			BlakeTwo256::ordered_trie_root(extrinsics, sp_runtime::StateVersion::V0);
		sp_io::storage::clear(&EXTRINSIC_KEY);
		header.extrinsics_root = extrinsics_root;

		let raw_state_root = &sp_io::storage::root(VERSION.state_version())[..];
		header.state_root = sp_core::H256::decode(&mut &raw_state_root[..]).unwrap();

		info!(target: LOG_TARGET, "finalizing block {:?}", header);
		header

	}

	fn do_execute_block(block: Block) {
		info!(target: LOG_TARGET, "Entering execute_block. block: {:?}", block);

		for extrinsic in block.clone().extrinsics {
			// block import cannot fail.
			Runtime::dispatch_extrinsic(extrinsic).unwrap();
		}

		// check state root
		let raw_state_root = &sp_io::storage::root(VERSION.state_version())[..];
		let state_root = H256::decode(&mut &raw_state_root[..]).unwrap();
		Self::print_state();
		assert_eq!(block.header.state_root, state_root);

		// check extrinsics root.
		let extrinsics =
			block.extrinsics.into_iter().map(|x| x.encode()).collect::<Vec<_>>();
		let extrinsics_root =
			BlakeTwo256::ordered_trie_root(extrinsics, sp_core::storage::StateVersion::V0);
		assert_eq!(block.header.extrinsics_root, extrinsics_root);
	}

	fn do_apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
		info!(target: LOG_TARGET, "Entering apply_extrinsic: {:?}", extrinsic);

		Self::dispatch_extrinsic(extrinsic)
			.map_err(|_| TransactionValidityError::Invalid(InvalidTransaction::Custom(0)))?;

		Ok(Ok(()))
	}

	fn do_validate_transaction(
		source: TransactionSource,
		tx: <Block as BlockT>::Extrinsic,
		block_hash: <Block as BlockT>::Hash,
	) -> TransactionValidity {
		log::info!(
			target: LOG_TARGET,
			"Entering validate_transaction. source: {:?}, tx: {:?}, block hash: {:?}",
			source,
			tx,
			block_hash
		);

		let data = tx.call;

		Ok(ValidTransaction { provides: vec![data.encode()], ..Default::default() })
	}

	fn do_inherent_extrinsics(_: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
		log::debug!(target: LOG_TARGET, "Entering do_inherent_extrinsics");
		Default::default()
	}

	fn do_check_inherents(
		_: Block,
		_: sp_inherents::InherentData,
	) -> sp_inherents::CheckInherentsResult {
		log::debug!(target: LOG_TARGET, "Entering do_check_inherents");
		Default::default()
	}
}

impl_runtime_apis! {
	// https://substrate.dev/rustdocs/master/sp_api/trait.Core.html
	impl sp_api::Core<Block> for Runtime {
		fn version() -> RuntimeVersion {
			VERSION
		}

		fn execute_block(block: Block) {
			Self::do_execute_block(block)
		}

		fn initialize_block(header: &<Block as BlockT>::Header) {
			Self::do_initialize_block(header)
		}
	}

	// https://substrate.dev/rustdocs/master/sc_block_builder/trait.BlockBuilderApi.html
	impl sp_block_builder::BlockBuilder<Block> for Runtime {
		fn apply_extrinsic(extrinsic: <Block as BlockT>::Extrinsic) -> ApplyExtrinsicResult {
			Self::do_apply_extrinsic(extrinsic)
		}

		fn finalize_block() -> <Block as BlockT>::Header {
			Self::do_finalize_block()
		}

		fn inherent_extrinsics(data: sp_inherents::InherentData) -> Vec<<Block as BlockT>::Extrinsic> {
			Self::do_inherent_extrinsics(data)
		}

		fn check_inherents(
			block: Block,
			data: sp_inherents::InherentData
		) -> sp_inherents::CheckInherentsResult {
			Self::do_check_inherents(block, data)
		}
	}

	impl sp_transaction_pool::runtime_api::TaggedTransactionQueue<Block> for Runtime {
		fn validate_transaction(
			source: TransactionSource,
			tx: <Block as BlockT>::Extrinsic,
			block_hash: <Block as BlockT>::Hash,
		) -> TransactionValidity {
			Self::do_validate_transaction(source, tx, block_hash)
		}
	}

	// Ignore everything after this.
	impl sp_api::Metadata<Block> for Runtime {
		fn metadata() -> OpaqueMetadata {
			OpaqueMetadata::new(Default::default())
		}
	}

	impl sp_offchain::OffchainWorkerApi<Block> for Runtime {
		fn offchain_worker(_header: &<Block as BlockT>::Header) {
			// we do not do anything.
		}
	}

	impl sp_session::SessionKeys<Block> for Runtime {
		fn generate_session_keys(seed: Option<Vec<u8>>) -> Vec<u8> {
			info!(target: "frameless", "🖼️ Entering generate_session_keys. seed: {:?}", seed);
			opaque::SessionKeys::generate(seed)
		}

		fn decode_session_keys(
			encoded: Vec<u8>,
		) -> Option<Vec<(Vec<u8>, sp_core::crypto::KeyTypeId)>> {
			opaque::SessionKeys::decode_into_raw_public_keys(&encoded)
		}
	}

	impl sp_consensus_aura::AuraApi<Block, AuraId> for Runtime {
		fn slot_duration() -> sp_consensus_aura::SlotDuration {
			sp_consensus_aura::SlotDuration::from_millis(BLOCK_TIME)
		}

		fn authorities() -> Vec<AuraId> {
			// The only authority is Alice. This makes things work nicely in `--dev` mode
			use sp_application_crypto::ByteArray;

			vec![
				AuraId::from_slice(
					&hex_literal::hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d").to_vec()
				).unwrap()
			]
		}
	}

	impl sp_finality_grandpa::GrandpaApi<Block> for Runtime {
		fn grandpa_authorities() -> sp_finality_grandpa::AuthorityList {
			use sp_application_crypto::ByteArray;
			vec![
				(
					sp_finality_grandpa::AuthorityId::from_slice(
						&hex_literal::hex!("88dc3417d5058ec4b4503e0c12ea1a0a89be200fe98922423d4334014fa6b0ee").to_vec()
					).unwrap(),
					1
				)
			]
		}

		fn current_set_id() -> sp_finality_grandpa::SetId {
			0u64
		}

		fn submit_report_equivocation_unsigned_extrinsic(
			_equivocation_proof: sp_finality_grandpa::EquivocationProof<
				<Block as BlockT>::Hash,
				sp_runtime::traits::NumberFor<Block>,
			>,
			_key_owner_proof: sp_finality_grandpa::OpaqueKeyOwnershipProof,
		) -> Option<()> {
			None
		}

		fn generate_key_ownership_proof(
			_set_id: sp_finality_grandpa::SetId,
			_authority_id: sp_finality_grandpa::AuthorityId,
		) -> Option<sp_finality_grandpa::OpaqueKeyOwnershipProof> {
			None
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use parity_scale_codec::{ Encode, Decode };
	use sp_core::hexdisplay::HexDisplay;
	use sp_core::crypto::Pair;
	use sp_core::{blake2_256, sr25519};

	fn new_accounts() -> (sr25519::Pair, sr25519::Pair){
		let p1_mnemonic = "grass rib slab system month zoo observe render display shallow clay venture";
		let p2_mnemonic = "scan true window staff cloth loud accuse retreat tent rack glimpse genuine";

		let keypair: sr25519::Pair = Pair::from_string(p1_mnemonic, None).unwrap();
		let keypair_p2: sr25519::Pair = Pair::from_string(p2_mnemonic, None).unwrap();
		// println!("P1 Public Key: 0x{:?}", keypair.to_owned().public());
		// println!("P2 Public Key: 0x{:?}", keypair_p2.to_owned().public());

		(keypair, keypair_p2)
	}

	#[test]
	fn mint_balance_on_account() {
		let balance = 100;
		let (p1, p2) = new_accounts();
		let extrinsic = BasicExtrinsic::new_unsigned(Call::Mint(p1.public(), balance));

		let mint_more_ext = BasicExtrinsic::new_unsigned(Call::Mint(p1.public(), 100));

		sp_io::TestExternalities::new_empty().execute_with(|| {
			// initial balance on startup is None
			let initial_balance: Option<u32> = Runtime::get_state(&p1.public());
			assert_eq!(None, initial_balance);

			// Mint 100 Tokens
			Runtime::do_apply_extrinsic(extrinsic.clone()).unwrap();
			let account_balance: u32 = Runtime::get_state(&p1.public()).unwrap();
			assert_eq!(100, account_balance);

			// Mint 100 more == 200
			Runtime::do_apply_extrinsic(mint_more_ext.clone()).unwrap();
			let new_balance: u32 = Runtime::get_state(&p1.public()).unwrap();
			assert_eq!(200, new_balance);
		});


	}

	#[test]
	fn test_unsuccessful_transfer_with_no_signature() {
		let balance = 100;
		let (p1, p2) = new_accounts();
		let mint_ext = BasicExtrinsic::new_unsigned(Call::Mint(p1.public(), balance));

		let transaction = Transaction {
			from: p1.public(),
			to: p2.public(),
			send_amount: 10,
			fee: 0
		};

		let transfer_extrinsic = BasicExtrinsic::new_unsigned(Call::Transfer(transaction));

		sp_io::TestExternalities::new_empty().execute_with(|| {
			// sp_tracing::try_init_simple();

			Runtime::do_apply_extrinsic(mint_ext.clone()).unwrap();
			let result = Runtime::do_apply_extrinsic(transfer_extrinsic.clone()).expect_err("Unsigned transaction");
			let end_balance: u32 = Runtime::get_state::<u32>(&transaction.from).unwrap();

			// unchanged balance
			assert_eq!(100, end_balance);
			assert_eq!(result, TransactionValidityError::Invalid(InvalidTransaction::Custom(0)));
		})
	}

	#[test]
	fn test_unsuccessful_transfer_with_invalid_signature() {
		let balance = 100;
		let (p1, p2) = new_accounts();
		let mint_ext = BasicExtrinsic::new_unsigned(Call::Mint(p1.public(), balance));

		let transaction = Transaction {
			from: p1.public(),
			to: p2.public(),
			send_amount: 10,
			fee: 0
		};

		let message_hash = blake2_256(&*transaction.clone().encode());

		// Signing Here with the recipients pub key instead of sender
		let signature = p2.sign(&message_hash);

		let transfer_extrinsic = BasicExtrinsic::signed(Call::Transfer(transaction), Some(signature));

		sp_io::TestExternalities::new_empty().execute_with(|| {
			// sp_tracing::try_init_simple();
			Runtime::do_apply_extrinsic(mint_ext.clone()).unwrap();
			let result = Runtime::do_apply_extrinsic(transfer_extrinsic.clone()).expect_err("Invalid Signature");
			let end_balance: u32 = Runtime::get_state::<u32>(&transaction.from).unwrap();

			// unchanged balance
			assert_eq!(100, end_balance);
			assert_eq!(result, TransactionValidityError::Invalid(InvalidTransaction::Custom(0)));
		})
	}


	#[test]
	fn successful_transfer_with_signed_extrinsic() {
		let balance = 100;
		let (p1, p2) = new_accounts();
		let mint_ext = BasicExtrinsic::new_unsigned(Call::Mint(p1.public(), balance));

		let transaction = Transaction {
			from: p1.public(),
			to: p2.public(),
			send_amount: 10,
			fee: 0
		};

		let message_hash = blake2_256(&*transaction.clone().encode());
		let signature = p1.sign(&message_hash);

		let transfer_extrinsic = BasicExtrinsic::signed(Call::Transfer(transaction), Some(signature));

		sp_io::TestExternalities::new_empty().execute_with(|| {
			Runtime::do_apply_extrinsic(mint_ext.clone()).unwrap();
			let result = Runtime::do_apply_extrinsic(transfer_extrinsic.clone()).unwrap();
			let end_balance: u32 = Runtime::get_state::<u32>(&transaction.from).unwrap();

			// changed balance
			assert_eq!(90, end_balance);
			assert_eq!(result, Ok(()));
		})
	}

	#[test]
	fn call_signed_extrinsic_with_not_enough_funds() {
		let balance = 100;
		let (p1, p2) = new_accounts();
		let mint_ext = BasicExtrinsic::new_unsigned(Call::Mint(p1.public(), balance));

		let transaction = Transaction {
			from: p1.public(),
			to: p2.public(),
			send_amount: 101, //sending more than account balance => Err()
			fee: 0
		};

		let message_hash = blake2_256(&*transaction.clone().encode());
		let signature = p1.sign(&message_hash);

		let transfer_extrinsic = BasicExtrinsic::signed(Call::Transfer(transaction), Some(signature));

		sp_io::TestExternalities::new_empty().execute_with(|| {
			// sp_tracing::try_init_simple();
			Runtime::do_apply_extrinsic(mint_ext.clone()).unwrap();
			let result = Runtime::do_apply_extrinsic(transfer_extrinsic.clone()).expect_err("Not enough Tokens to send");
			let end_balance: u32 = Runtime::get_state::<u32>(&transaction.from).unwrap();

			// unchanged balance
			assert_eq!(100, end_balance);
			assert_eq!(result, TransactionValidityError::Invalid(InvalidTransaction::Custom(0)));
		})
	}

	#[test]
	fn call_signed_extrinsic_with_amount_and_fees_higher_than_balance() {
		let balance = 100;
		let (p1, p2) = new_accounts();
		let mint_ext = BasicExtrinsic::new_unsigned(Call::Mint(p1.public(), balance));

		let transaction = Transaction {
			from: p1.public(),
			to: p2.public(),
			send_amount: 100, // Account Balance < Fee + Send Amount
			fee: 10
		};

		let message_hash = blake2_256(&*transaction.clone().encode());
		let signature = p1.sign(&message_hash);

		let transfer_extrinsic = BasicExtrinsic::signed(Call::Transfer(transaction), Some(signature));

		sp_io::TestExternalities::new_empty().execute_with(|| {
			// sp_tracing::try_init_simple();
			Runtime::do_apply_extrinsic(mint_ext.clone()).unwrap();
			let result = Runtime::do_apply_extrinsic(transfer_extrinsic.clone()).expect_err("Not enough Tokens to send");
			let end_balance: u32 = Runtime::get_state::<u32>(&transaction.from).unwrap();

			// unchanged balance
			assert_eq!(100, end_balance);
			assert_eq!(result, TransactionValidityError::Invalid(InvalidTransaction::Custom(0)));
		})
	}

	#[test]
	fn call_signed_extrinsic_with_amount_and_fees_less_than_balance() {
		let balance = 100;
		let (p1, p2) = new_accounts();
		let mint_ext = BasicExtrinsic::new_unsigned(Call::Mint(p1.public(), balance));

		let transaction = Transaction {
			from: p1.public(),
			to: p2.public(),
			send_amount: 80,
			fee: 10
		};

		let message_hash = blake2_256(&*transaction.clone().encode());
		let signature = p1.sign(&message_hash);

		let transfer_extrinsic = BasicExtrinsic::signed(Call::Transfer(transaction), Some(signature));

		sp_io::TestExternalities::new_empty().execute_with(|| {
			// sp_tracing::try_init_simple();
			Runtime::do_apply_extrinsic(mint_ext.clone()).unwrap();
			let result = Runtime::do_apply_extrinsic(transfer_extrinsic.clone()).unwrap();
			let end_balance: u32 = Runtime::get_state::<u32>(&transaction.from).unwrap();

			// unchanged balance
			assert_eq!(10, end_balance);
			assert_eq!(Ok(()), result);
		})
	}

}

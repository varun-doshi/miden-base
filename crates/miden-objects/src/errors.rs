use alloc::{boxed::Box, string::String, vec::Vec};
use core::error::Error;

use assembly::{diagnostics::reporting::PrintDiagnostic, Report};
use miden_crypto::utils::HexParseError;
use thiserror::Error;
use vm_core::{mast::MastForestError, Felt, FieldElement};
use vm_processor::DeserializationError;

use super::{
    account::AccountId,
    asset::{FungibleAsset, NonFungibleAsset},
    crypto::merkle::MerkleError,
    note::NoteId,
    Digest, Word, MAX_BATCHES_PER_BLOCK, MAX_OUTPUT_NOTES_PER_BATCH,
};
use crate::{
    account::{
        AccountCode, AccountIdPrefix, AccountStorage, AccountType, StorageValueName,
        StorageValueNameError, TemplateTypeError,
    },
    batch::BatchId,
    block::BlockNumber,
    note::{NoteAssets, NoteExecutionHint, NoteTag, NoteType, Nullifier},
    transaction::TransactionId,
    ACCOUNT_UPDATE_MAX_SIZE, MAX_ACCOUNTS_PER_BATCH, MAX_INPUTS_PER_NOTE,
    MAX_INPUT_NOTES_PER_BATCH, MAX_INPUT_NOTES_PER_TX, MAX_OUTPUT_NOTES_PER_TX,
};

// ACCOUNT COMPONENT TEMPLATE ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum AccountComponentTemplateError {
    #[cfg(feature = "std")]
    #[error("error trying to deserialize from toml")]
    TomlDeserializationError(#[source] toml::de::Error),
    #[error("slot {0} is defined multiple times")]
    DuplicateSlot(u8),
    #[error("storage value name is incorrect: {0}")]
    IncorrectStorageValueName(#[source] StorageValueNameError),
    #[error("type `{0}` is not valid for `{1}` slots")]
    InvalidType(String, String),
    #[error("multi-slot entry should contain as many values as storage slots indices")]
    MultiSlotArityMismatch,
    #[error("error deserializing component metadata: {0}")]
    MetadataDeserializationError(String),
    #[error("component storage slots are not contiguous ({0} is followed by {1})")]
    NonContiguousSlots(u8, u8),
    #[error("storage value for placeholder `{0}` was not provided in the init storage data")]
    PlaceholderValueNotProvided(StorageValueName),
    #[cfg(feature = "std")]
    #[error("error trying to deserialize from toml")]
    TomlSerializationError(#[source] toml::ser::Error),
    #[error("error converting value into expected type: ")]
    StorageValueParsingError(#[source] TemplateTypeError),
    #[error("storage map contains duplicate keys")]
    StorageMapHasDuplicateKeys(#[source] Box<dyn Error + Send + Sync + 'static>),
    #[error("component storage slots have to start at 0, but they start at {0}")]
    StorageSlotsDoNotStartAtZero(u8),
}

// ACCOUNT ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum AccountError {
    #[error("failed to deserialize account code")]
    AccountCodeDeserializationError(#[source] DeserializationError),
    #[error("account code does not contain procedures but must contain at least one procedure")]
    AccountCodeNoProcedures,
    #[error("account code contains {0} procedures but it may contain at most {max} procedures", max = AccountCode::MAX_NUM_PROCEDURES)]
    AccountCodeTooManyProcedures(usize),
    #[error("account procedure {0}'s storage offset {1} does not fit into u8")]
    AccountCodeProcedureStorageOffsetTooLarge(Digest, Felt),
    #[error("account procedure {0}'s storage size {1} does not fit into u8")]
    AccountCodeProcedureStorageSizeTooLarge(Digest, Felt),
    #[error("account procedure {0}'s final two elements must be Felt::ZERO")]
    AccountCodeProcedureInvalidPadding(Digest),
    #[error("failed to assemble account component:\n{}", PrintDiagnostic::new(.0))]
    AccountComponentAssemblyError(Report),
    #[error("failed to merge components into one account code mast forest")]
    AccountComponentMastForestMergeError(#[source] MastForestError),
    #[error("procedure with MAST root {0} is present in multiple account components")]
    AccountComponentDuplicateProcedureRoot(Digest),
    #[error("failed to create account component")]
    AccountComponentTemplateInstantiationError(#[source] AccountComponentTemplateError),
    #[error("failed to update asset vault")]
    AssetVaultUpdateError(#[source] AssetVaultError),
    #[error("account build error: {0}")]
    BuildError(String, #[source] Option<Box<AccountError>>),
    #[error("faucet metadata decimals is {actual} which exceeds max value of {max}")]
    FungibleFaucetTooManyDecimals { actual: u8, max: u8 },
    #[error("faucet metadata max supply is {actual} which exceeds max value of {max}")]
    FungibleFaucetMaxSupplyTooLarge { actual: u64, max: u64 },
    #[error("account header data has length {actual} but it must be of length {expected}")]
    HeaderDataIncorrectLength { actual: usize, expected: usize },
    #[error("new account nonce {new} is less than the current nonce {current}")]
    NonceNotMonotonicallyIncreasing { current: u64, new: u64 },
    #[error("digest of the seed has {actual} trailing zeroes but must have at least {expected} trailing zeroes")]
    SeedDigestTooFewTrailingZeros { expected: u32, actual: u32 },
    #[error("storage slot at index {0} is not of type map")]
    StorageSlotNotMap(u8),
    #[error("storage slot at index {0} is not of type value")]
    StorageSlotNotValue(u8),
    #[error("storage slot index is {index} but the slots length is {slots_len}")]
    StorageIndexOutOfBounds { slots_len: u8, index: u8 },
    #[error("number of storage slots is {0} but max possible number is {max}", max = AccountStorage::MAX_NUM_STORAGE_SLOTS)]
    StorageTooManySlots(u64),
    #[error("procedure storage offset + size is {0} which exceeds the maximum value of {max}",
      max = AccountStorage::MAX_NUM_STORAGE_SLOTS
    )]
    StorageOffsetPlusSizeOutOfBounds(u16),
    #[error(
        "procedure which does not access storage (storage size = 0) has non-zero storage offset"
    )]
    PureProcedureWithStorageOffset,
    #[error("account component at index {component_index} is incompatible with account of type {account_type}")]
    UnsupportedComponentForAccountType {
        account_type: AccountType,
        component_index: usize,
    },
    #[error("failed to parse account ID from final account header")]
    FinalAccountHeaderIdParsingFailed(#[source] AccountIdError),
    /// This variant can be used by methods that are not inherent to the account but want to return
    /// this error type.
    #[error("assumption violated: {0}")]
    AssumptionViolated(String),
}

// ACCOUNT ID ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum AccountIdError {
    #[error("failed to convert bytes into account ID prefix field element")]
    AccountIdInvalidPrefixFieldElement(#[source] DeserializationError),
    #[error("failed to convert bytes into account ID suffix field element")]
    AccountIdInvalidSuffixFieldElement(#[source] DeserializationError),
    #[error("`{0}` is not a known account storage mode")]
    UnknownAccountStorageMode(Box<str>),
    #[error(r#"`{0}` is not a known account type, expected one of "FungibleFaucet", "NonFungibleFaucet", "RegularAccountImmutableCode" or "RegularAccountUpdatableCode""#)]
    UnknownAccountType(Box<str>),
    #[error("failed to parse hex string into account ID")]
    AccountIdHexParseError(#[source] HexParseError),
    #[error("`{0}` is not a known account ID version")]
    UnknownAccountIdVersion(u8),
    #[error("anchor epoch in account ID must not be u16::MAX ({})", u16::MAX)]
    AnchorEpochMustNotBeU16Max,
    #[error("least significant byte of account ID suffix must be zero")]
    AccountIdSuffixLeastSignificantByteMustBeZero,
    #[error(
        "anchor block must be an epoch block, that is, its block number must be a multiple of 2^{}",
        BlockNumber::EPOCH_LENGTH_EXPONENT
    )]
    AnchorBlockMustBeEpochBlock,
}

// ACCOUNT DELTA ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum AccountDeltaError {
    #[error("storage slot {0} was updated as a value and as a map")]
    StorageSlotUsedAsDifferentTypes(u8),
    #[error("non fungible vault can neither be added nor removed twice")]
    DuplicateNonFungibleVaultUpdate(NonFungibleAsset),
    #[error("fungible asset issued by faucet {faucet_id} has delta {delta} which overflows when added to current value {current}")]
    FungibleAssetDeltaOverflow {
        faucet_id: AccountId,
        current: i64,
        delta: i64,
    },
    #[error("account update of type `{left_update_type}` cannot be merged with account update of type `{right_update_type}`")]
    IncompatibleAccountUpdates {
        left_update_type: &'static str,
        right_update_type: &'static str,
    },
    #[error("account delta could not be applied to account {account_id}")]
    AccountDeltaApplicationFailed {
        account_id: AccountId,
        source: AccountError,
    },
    #[error("inconsistent nonce update: {0}")]
    InconsistentNonceUpdate(String),
    #[error("account ID {0} in fungible asset delta is not of type fungible faucet")]
    NotAFungibleFaucetId(AccountId),
}

// BATCH ACCOUNT UPDATE ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum BatchAccountUpdateError {
    #[error("account update for account {expected_account_id} cannot be merged with update from transaction {transaction} which was executed against account {actual_account_id}")]
    AccountUpdateIdMismatch {
        transaction: TransactionId,
        expected_account_id: AccountId,
        actual_account_id: AccountId,
    },
    #[error("final state commitment in account update from transaction {0} does not match initial state of current update")]
    AccountUpdateInitialStateMismatch(TransactionId),
    #[error("failed to merge account delta from transaction {0}")]
    TransactionUpdateMergeError(TransactionId, #[source] AccountDeltaError),
}

// ASSET ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum AssetError {
    #[error(
      "fungible asset amount {0} exceeds the max allowed amount of {max_amount}",
      max_amount = FungibleAsset::MAX_AMOUNT
    )]
    FungibleAssetAmountTooBig(u64),
    #[error("subtracting {subtrahend} from fungible asset amount {minuend} would overflow")]
    FungibleAssetAmountNotSufficient { minuend: u64, subtrahend: u64 },
    #[error("fungible asset word {hex} does not contain expected ZERO at word index 1",
      hex = vm_core::utils::to_hex(Felt::elements_as_bytes(.0))
    )]
    FungibleAssetExpectedZero(Word),
    #[error("cannot add fungible asset with issuer {other_issuer} to fungible asset with issuer {original_issuer}")]
    FungibleAssetInconsistentFaucetIds {
        original_issuer: AccountId,
        other_issuer: AccountId,
    },
    #[error("faucet account ID in asset is invalid")]
    InvalidFaucetAccountId(#[source] Box<dyn Error + Send + Sync + 'static>),
    #[error(
      "faucet id {0} of type {id_type} must be of type {expected_ty} for fungible assets",
      id_type = .0.account_type(),
      expected_ty = AccountType::FungibleFaucet
    )]
    FungibleFaucetIdTypeMismatch(AccountId),
    #[error(
      "faucet id {0} of type {id_type} must be of type {expected_ty} for non fungible assets",
      id_type = .0.account_type(),
      expected_ty = AccountType::NonFungibleFaucet
    )]
    NonFungibleFaucetIdTypeMismatch(AccountIdPrefix),
    #[error("{0}")]
    TokenSymbolError(String),
}

// ASSET VAULT ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum AssetVaultError {
    #[error("adding fungible asset amounts would exceed maximum allowed amount")]
    AddFungibleAssetBalanceError(#[source] AssetError),
    #[error("provided assets contain duplicates")]
    DuplicateAsset(#[source] MerkleError),
    #[error("non fungible asset {0} already exists in the vault")]
    DuplicateNonFungibleAsset(NonFungibleAsset),
    #[error("fungible asset {0} does not exist in the vault")]
    FungibleAssetNotFound(FungibleAsset),
    #[error("faucet id {0} is not a fungible faucet id")]
    NotAFungibleFaucetId(AccountId),
    #[error("non fungible asset {0} does not exist in the vault")]
    NonFungibleAssetNotFound(NonFungibleAsset),
    #[error("subtracting fungible asset amounts would underflow")]
    SubtractFungibleAssetBalanceError(#[source] AssetError),
}

// NOTE ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum NoteError {
    #[error("duplicate fungible asset from issuer {0} in note")]
    DuplicateFungibleAsset(AccountId),
    #[error("duplicate non fungible asset {0} in note")]
    DuplicateNonFungibleAsset(NonFungibleAsset),
    #[error("note type {0:?} is inconsistent with note tag {1}")]
    InconsistentNoteTag(NoteType, u64),
    #[error("adding fungible asset amounts would exceed maximum allowed amount")]
    AddFungibleAssetBalanceError(#[source] AssetError),
    #[error("note sender is not a valid account ID")]
    NoteSenderInvalidAccountId(#[source] AccountIdError),
    #[error("note tag use case {0} must be less than 2^{exp}", exp = NoteTag::MAX_USE_CASE_ID_EXPONENT)]
    NoteTagUseCaseTooLarge(u16),
    #[error(
        "note execution hint tag {0} must be in range {from}..={to}",
        from = NoteExecutionHint::NONE_TAG,
        to = NoteExecutionHint::ON_BLOCK_SLOT_TAG,
    )]
    NoteExecutionHintTagOutOfRange(u8),
    #[error("note execution hint after block variant cannot contain u32::MAX")]
    NoteExecutionHintAfterBlockCannotBeU32Max,
    #[error("invalid note execution hint payload {1} for tag {0}")]
    InvalidNoteExecutionHintPayload(u8, u32),
    #[error("note type {0:b} does not match any of the valid note types {public}, {private} or {encrypted}",
      public = NoteType::Public as u8,
      private = NoteType::Private as u8,
      encrypted = NoteType::Encrypted as u8,
    )]
    InvalidNoteType(u64),
    #[error("note location index {node_index_in_block} is out of bounds 0..={highest_index}")]
    NoteLocationIndexOutOfBounds {
        node_index_in_block: u16,
        highest_index: usize,
    },
    #[error("note network execution requires account stored on chain")]
    NetworkExecutionRequiresOnChainAccount,
    #[error("note network execution requires a public note but note is of type {0:?}")]
    NetworkExecutionRequiresPublicNote(NoteType),
    #[error("failed to assemble note script:\n{}", PrintDiagnostic::new(.0))]
    NoteScriptAssemblyError(Report),
    #[error("failed to deserialize note script")]
    NoteScriptDeserializationError(#[source] DeserializationError),
    #[error("public use case requires a public note but note is of type {0:?}")]
    PublicUseCaseRequiresPublicNote(NoteType),
    #[error("note contains {0} assets which exceeds the maximum of {max}", max = NoteAssets::MAX_NUM_ASSETS)]
    TooManyAssets(usize),
    #[error("note contains {0} inputs which exceeds the maximum of {max}", max = MAX_INPUTS_PER_NOTE)]
    TooManyInputs(usize),
}

// CHAIN MMR ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum ChainMmrError {
    #[error("block num {block_num} exceeds chain length {chain_length} implied by the chain MMR")]
    BlockNumTooBig {
        chain_length: usize,
        block_num: BlockNumber,
    },
    #[error("duplicate block {block_num} in chain MMR")]
    DuplicateBlock { block_num: BlockNumber },
    #[error("chain MMR does not track authentication paths for block {block_num}")]
    UntrackedBlock { block_num: BlockNumber },
}

impl ChainMmrError {
    pub fn block_num_too_big(chain_length: usize, block_num: BlockNumber) -> Self {
        Self::BlockNumTooBig { chain_length, block_num }
    }

    pub fn duplicate_block(block_num: BlockNumber) -> Self {
        Self::DuplicateBlock { block_num }
    }

    pub fn untracked_block(block_num: BlockNumber) -> Self {
        Self::UntrackedBlock { block_num }
    }
}

// TRANSACTION SCRIPT ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum TransactionScriptError {
    #[error("failed to assemble transaction script:\n{}", PrintDiagnostic::new(.0))]
    AssemblyError(Report),
}

// TRANSACTION INPUT ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum TransactionInputError {
    #[error("account seed must be provided for new accounts")]
    AccountSeedNotProvidedForNewAccount,
    #[error("account seed must not be provided for existing accounts")]
    AccountSeedProvidedForExistingAccount,
    #[error(
      "anchor block header for epoch {0} (block number = {block_number}) must be provided in the chain mmr for the new account",
      block_number = BlockNumber::from_epoch(*.0),
    )]
    AnchorBlockHeaderNotProvidedForNewAccount(u16),
    #[error("transaction input note with nullifier {0} is a duplicate")]
    DuplicateInputNote(Nullifier),
    #[error("ID {expected} of the new account does not match the ID {actual} computed from the provided seed")]
    InconsistentAccountSeed { expected: AccountId, actual: AccountId },
    #[error("chain mmr has length {actual} which does not match block number {expected}")]
    InconsistentChainLength {
        expected: BlockNumber,
        actual: BlockNumber,
    },
    #[error("chain mmr has root {actual} which does not match block header's root {expected}")]
    InconsistentChainRoot { expected: Digest, actual: Digest },
    #[error("block in which input note with id {0} was created is not in chain mmr")]
    InputNoteBlockNotInChainMmr(NoteId),
    #[error("input note with id {0} was not created in block {1}")]
    InputNoteNotInBlock(NoteId, BlockNumber),
    #[error("account ID computed from seed is invalid")]
    InvalidAccountIdSeed(#[source] AccountIdError),
    #[error(
        "total number of input notes is {0} which exceeds the maximum of {MAX_INPUT_NOTES_PER_TX}"
    )]
    TooManyInputNotes(usize),
}

// TRANSACTION OUTPUT ERROR
// ===============================================================================================

#[derive(Debug, Error)]
pub enum TransactionOutputError {
    #[error("transaction output note with id {0} is a duplicate")]
    DuplicateOutputNote(NoteId),
    #[error("final account hash is not in the advice map")]
    FinalAccountHashMissingInAdviceMap,
    #[error("failed to parse final account header")]
    FinalAccountHeaderParseFailure(#[source] AccountError),
    #[error("output notes commitment {expected} from kernel does not match computed commitment {actual}")]
    OutputNotesCommitmentInconsistent { expected: Digest, actual: Digest },
    #[error("transaction kernel output stack is invalid: {0}")]
    OutputStackInvalid(String),
    #[error("total number of output notes is {0} which exceeds the maximum of {MAX_OUTPUT_NOTES_PER_TX}")]
    TooManyOutputNotes(usize),
}

// PROVEN TRANSACTION ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum ProvenTransactionError {
    #[error("proven transaction's final account hash {tx_final_hash} and account details hash {details_hash} must match")]
    AccountFinalHashMismatch {
        tx_final_hash: Digest,
        details_hash: Digest,
    },
    #[error("proven transaction's final account ID {tx_account_id} and account details id {details_account_id} must match")]
    AccountIdMismatch {
        tx_account_id: AccountId,
        details_account_id: AccountId,
    },
    #[error("failed to construct input notes for proven transaction")]
    InputNotesError(TransactionInputError),
    #[error("off-chain account {0} should not have account details")]
    OffChainAccountWithDetails(AccountId),
    #[error("on-chain account {0} is missing its account details")]
    OnChainAccountMissingDetails(AccountId),
    #[error("new on-chain account {0} is missing its account details")]
    NewOnChainAccountRequiresFullDetails(AccountId),
    #[error(
        "existing on-chain account {0} should only provide delta updates instead of full details"
    )]
    ExistingOnChainAccountRequiresDeltaDetails(AccountId),
    #[error("failed to construct output notes for proven transaction")]
    OutputNotesError(TransactionOutputError),
    #[error(
      "account update of size {update_size} for account {account_id} exceeds maximum update size of {ACCOUNT_UPDATE_MAX_SIZE}",
    )]
    AccountUpdateSizeLimitExceeded {
        account_id: AccountId,
        update_size: usize,
    },
}

// PROPOSED BATCH ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum ProposedBatchError {
    #[error(
        "transaction batch has {0} input notes but at most {MAX_INPUT_NOTES_PER_BATCH} are allowed"
    )]
    TooManyInputNotes(usize),

    #[error(
        "transaction batch has {0} output notes but at most {MAX_OUTPUT_NOTES_PER_BATCH} are allowed"
    )]
    TooManyOutputNotes(usize),

    #[error(
      "transaction batch has {0} account updates but at most {MAX_ACCOUNTS_PER_BATCH} are allowed"
    )]
    TooManyAccountUpdates(usize),

    #[error("transaction {transaction_id} expires at block number {transaction_expiration_num} which is not greater than the number of the batch's reference block {reference_block_num}")]
    ExpiredTransaction {
        transaction_id: TransactionId,
        transaction_expiration_num: BlockNumber,
        reference_block_num: BlockNumber,
    },

    #[error("transaction batch must contain at least one transaction")]
    EmptyTransactionBatch,

    #[error("transaction {transaction_id} appears twice in the proposed batch input")]
    DuplicateTransaction { transaction_id: TransactionId },

    #[error("transaction {second_transaction_id} consumes the note with nullifier {note_nullifier} that is also consumed by another transaction {first_transaction_id} in the batch")]
    DuplicateInputNote {
        note_nullifier: Nullifier,
        first_transaction_id: TransactionId,
        second_transaction_id: TransactionId,
    },

    #[error("transaction {second_transaction_id} creates the note with id {note_id} that is also created by another transaction {first_transaction_id} in the batch")]
    DuplicateOutputNote {
        note_id: NoteId,
        first_transaction_id: TransactionId,
        second_transaction_id: TransactionId,
    },

    #[error("note hashes mismatch for note {id}: (input: {input_hash}, output: {output_hash})")]
    NoteHashesMismatch {
        id: NoteId,
        input_hash: Digest,
        output_hash: Digest,
    },

    #[error("failed to merge transaction delta into account {account_id}")]
    AccountUpdateError {
        account_id: AccountId,
        source: BatchAccountUpdateError,
    },

    #[error("unable to prove unauthenticated note inclusion because block {block_number} in which note with id {note_id} was created is not in chain mmr")]
    UnauthenticatedInputNoteBlockNotInChainMmr {
        block_number: BlockNumber,
        note_id: NoteId,
    },

    #[error(
        "unable to prove unauthenticated note inclusion of note {note_id} in block {block_num}"
    )]
    UnauthenticatedNoteAuthenticationFailed {
        note_id: NoteId,
        block_num: BlockNumber,
        source: MerkleError,
    },

    #[error("chain mmr has length {actual} which does not match block number {expected}")]
    InconsistentChainLength {
        expected: BlockNumber,
        actual: BlockNumber,
    },

    #[error("chain mmr has root {actual} which does not match block header's root {expected}")]
    InconsistentChainRoot { expected: Digest, actual: Digest },

    #[error("block {block_reference} referenced by transaction {transaction_id} is not in the chain mmr")]
    MissingTransactionBlockReference {
        block_reference: Digest,
        transaction_id: TransactionId,
    },
}

// PROPOSED BLOCK ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum ProposedBlockError {
    #[error("block must contain at least one transaction batch")]
    EmptyBlock,

    #[error("block must contain at most {MAX_BATCHES_PER_BLOCK} transaction batches")]
    TooManyBatches,

    #[error("batch {batch_id} expired at block {batch_expiration_block_num} but the current block number is {current_block_num}")]
    ExpiredBatch {
        batch_id: BatchId,
        batch_expiration_block_num: BlockNumber,
        current_block_num: BlockNumber,
    },

    #[error("batch {batch_id} appears twice in the block inputs")]
    DuplicateBatch { batch_id: BatchId },

    #[error("batch {second_batch_id} consumes the note with nullifier {note_nullifier} that is also consumed by another batch {first_batch_id} in the block")]
    DuplicateInputNote {
        note_nullifier: Nullifier,
        first_batch_id: BatchId,
        second_batch_id: BatchId,
    },

    #[error("batch {second_batch_id} creates the note with ID {note_id} that is also created by another batch {first_batch_id} in the block")]
    DuplicateOutputNote {
        note_id: NoteId,
        first_batch_id: BatchId,
        second_batch_id: BatchId,
    },

    #[error("timestamp {provided_timestamp} does not increase monotonically compared to timestamp {previous_timestamp} from the previous block header")]
    TimestampDoesNotIncreaseMonotonically {
        provided_timestamp: u32,
        previous_timestamp: u32,
    },

    #[error("account {account_id} is updated from the same initial state commitment {initial_state_commitment} by multiple conflicting batches with IDs {first_batch_id} and {second_batch_id}")]
    ConflictingBatchesUpdateSameAccount {
        account_id: AccountId,
        initial_state_commitment: Digest,
        first_batch_id: BatchId,
        second_batch_id: BatchId,
    },

    #[error("chain mmr has length {chain_length} which does not match the block number {prev_block_num} of the previous block referenced by the to-be-built block")]
    ChainLengthNotEqualToPreviousBlockNumber {
        chain_length: BlockNumber,
        prev_block_num: BlockNumber,
    },

    #[error("chain mmr has root {chain_root} which does not match the chain root {prev_block_chain_root} of the previous block {prev_block_num}")]
    ChainRootNotEqualToPreviousBlockChainRoot {
        chain_root: Digest,
        prev_block_chain_root: Digest,
        prev_block_num: BlockNumber,
    },

    #[error("chain mmr is missing block {reference_block_num} referenced by batch {batch_id} in the block")]
    BatchReferenceBlockMissingFromChain {
        reference_block_num: BlockNumber,
        batch_id: BatchId,
    },

    #[error("note hashes mismatch for note {id}: (input: {input_hash}, output: {output_hash})")]
    NoteHashesMismatch {
        id: NoteId,
        input_hash: Digest,
        output_hash: Digest,
    },

    #[error("failed to prove unauthenticated note inclusion because block {block_number} in which note with id {note_id} was created is not in chain mmr")]
    UnauthenticatedInputNoteBlockNotInChainMmr {
        block_number: BlockNumber,
        note_id: NoteId,
    },

    #[error(
        "failed to prove unauthenticated note inclusion of note {note_id} in block {block_num}"
    )]
    UnauthenticatedNoteAuthenticationFailed {
        note_id: NoteId,
        block_num: BlockNumber,
        source: MerkleError,
    },

    #[error("unauthenticated note with nullifier {nullifier} was not created in the same block and no inclusion proof to authenticate it was provided")]
    UnauthenticatedNoteConsumed { nullifier: Nullifier },

    #[error("block inputs do not contain a proof of inclusion for account {0}")]
    MissingAccountWitness(AccountId),

    #[error("account {account_id} with state {state_commitment} cannot transition to any of the remaining states {remaining_state_commitments:?}")]
    InconsistentAccountStateTransition {
        account_id: AccountId,
        state_commitment: Digest,
        remaining_state_commitments: Vec<Digest>,
    },

    #[error("no proof for nullifier {0} was provided")]
    NullifierProofMissing(Nullifier),

    #[error("note with nullifier {0} is already spent")]
    NullifierSpent(Nullifier),

    #[error("failed to merge transaction delta into account {account_id}")]
    AccountUpdateError {
        account_id: AccountId,
        source: AccountDeltaError,
    },
}

// NULLIFIER TREE ERROR
// ================================================================================================

#[derive(Debug, Error)]
pub enum NullifierTreeError {
    #[error("attempt to mark nullifier {0} as spent but it is already spent")]
    NullifierAlreadySpent(Nullifier),
    #[error("nullifier {nullifier} is not tracked by the partial nullifier tree")]
    UntrackedNullifier {
        nullifier: Nullifier,
        source: MerkleError,
    },
    #[error("new tree root after nullifier witness insertion does not match previous tree root")]
    TreeRootConflict(#[source] MerkleError),
}

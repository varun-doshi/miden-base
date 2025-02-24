use alloc::{collections::BTreeMap, vec::Vec};

use crate::{
    account::AccountId,
    batch::{BatchAccountUpdate, BatchId},
    block::BlockNumber,
    note::Nullifier,
    transaction::{InputNoteCommitment, InputNotes, OutputNote},
    utils::{ByteReader, ByteWriter, Deserializable, DeserializationError, Serializable},
    Digest,
};

/// A transaction batch with an execution proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProvenBatch {
    id: BatchId,
    reference_block_commitment: Digest,
    reference_block_num: BlockNumber,
    account_updates: BTreeMap<AccountId, BatchAccountUpdate>,
    input_notes: InputNotes<InputNoteCommitment>,
    output_notes: Vec<OutputNote>,
    batch_expiration_block_num: BlockNumber,
}

impl ProvenBatch {
    // CONSTRUCTORS
    // --------------------------------------------------------------------------------------------

    /// Creates a new [`ProvenBatch`] from the provided parts.
    #[allow(clippy::too_many_arguments)]
    pub fn new_unchecked(
        id: BatchId,
        reference_block_commitment: Digest,
        reference_block_num: BlockNumber,
        account_updates: BTreeMap<AccountId, BatchAccountUpdate>,
        input_notes: InputNotes<InputNoteCommitment>,
        output_notes: Vec<OutputNote>,
        batch_expiration_block_num: BlockNumber,
    ) -> Self {
        Self {
            id,
            reference_block_commitment,
            reference_block_num,
            account_updates,
            input_notes,
            output_notes,
            batch_expiration_block_num,
        }
    }

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// The ID of this batch. See [`BatchId`] for details on how it is computed.
    pub fn id(&self) -> BatchId {
        self.id
    }

    /// Returns the commitment to the reference block of the batch.
    pub fn reference_block_commitment(&self) -> Digest {
        self.reference_block_commitment
    }

    /// Returns the number of the reference block of the batch.
    pub fn reference_block_num(&self) -> BlockNumber {
        self.reference_block_num
    }

    /// Returns the block number at which the batch will expire.
    pub fn batch_expiration_block_num(&self) -> BlockNumber {
        self.batch_expiration_block_num
    }

    /// Returns an iterator over the IDs of all accounts updated in this batch.
    pub fn updated_accounts(&self) -> impl Iterator<Item = AccountId> + use<'_> {
        self.account_updates.keys().copied()
    }

    /// Returns the map of account IDs mapped to their [`BatchAccountUpdate`]s.
    ///
    /// If an account was updated by multiple transactions, the [`BatchAccountUpdate`] is the result
    /// of merging the individual updates.
    ///
    /// For example, suppose an account's state before this batch is `A` and the batch contains two
    /// transactions that updated it. Applying the first transaction results in intermediate state
    /// `B`, and applying the second one results in state `C`. Then the returned update represents
    /// the state transition from `A` to `C`.
    pub fn account_updates(&self) -> &BTreeMap<AccountId, BatchAccountUpdate> {
        &self.account_updates
    }

    /// Returns the [`InputNotes`] of this batch.
    pub fn input_notes(&self) -> &InputNotes<InputNoteCommitment> {
        &self.input_notes
    }

    /// Returns an iterator over the nullifiers created in this batch.
    pub fn created_nullifiers(&self) -> impl Iterator<Item = Nullifier> + use<'_> {
        self.input_notes.iter().map(InputNoteCommitment::nullifier)
    }

    /// Returns the output notes of the batch.
    ///
    /// This is the aggregation of all output notes by the transactions in the batch, except the
    /// ones that were consumed within the batch itself.
    pub fn output_notes(&self) -> &[OutputNote] {
        &self.output_notes
    }
}

// SERIALIZATION
// ================================================================================================

impl Serializable for ProvenBatch {
    fn write_into<W: ByteWriter>(&self, target: &mut W) {
        self.id.write_into(target);
        self.reference_block_commitment.write_into(target);
        self.reference_block_num.write_into(target);
        self.account_updates.write_into(target);
        self.input_notes.write_into(target);
        self.output_notes.write_into(target);
        self.batch_expiration_block_num.write_into(target);
    }
}

impl Deserializable for ProvenBatch {
    fn read_from<R: ByteReader>(source: &mut R) -> Result<Self, DeserializationError> {
        let id = BatchId::read_from(source)?;
        let reference_block_commitment = Digest::read_from(source)?;
        let reference_block_num = BlockNumber::read_from(source)?;
        let account_updates = BTreeMap::read_from(source)?;
        let input_notes = InputNotes::<InputNoteCommitment>::read_from(source)?;
        let output_notes = Vec::<OutputNote>::read_from(source)?;
        let batch_expiration_block_num = BlockNumber::read_from(source)?;

        Ok(Self::new_unchecked(
            id,
            reference_block_commitment,
            reference_block_num,
            account_updates,
            input_notes,
            output_notes,
            batch_expiration_block_num,
        ))
    }
}

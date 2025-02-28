use alloc::sync::Arc;

use miden_lib::notes::create_p2idr_note;
use miden_objects::{
    accounts::{
        account_id::testing::{
            ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN,
            ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2, ACCOUNT_ID_SENDER,
        },
        Account, AccountId,
    },
    assets::{Asset, AssetVault, FungibleAsset},
    crypto::rand::RpoRandomCoin,
    notes::NoteType,
    transaction::TransactionArgs,
    Felt,
};
use miden_tx::{testing::TransactionContextBuilder, TransactionExecutor};

use crate::{
    build_default_auth_script, get_account_with_basic_authenticated_wallet,
    get_new_pk_and_authenticator,
};

// P2IDR TESTS
// ===============================================================================================
// We want to test the Pay to ID Reclaim script, which is a script that allows the user
// to provide a block height to the P2ID script. Before the block height is reached,
// the note can only be consumed by the target account. After the block height is reached,
// the note can also be consumed (reclaimed) by the sender account.
#[test]
fn p2idr_script() {
    // Create assets
    let faucet_id = AccountId::try_from(ACCOUNT_ID_FUNGIBLE_FAUCET_ON_CHAIN).unwrap();
    let fungible_asset: Asset = FungibleAsset::new(faucet_id, 100).unwrap().into();

    // Create sender and target and malicious account
    let sender_account_id = AccountId::try_from(ACCOUNT_ID_SENDER).unwrap();
    let (sender_pub_key, sender_falcon_auth) = get_new_pk_and_authenticator();
    let sender_account =
        get_account_with_basic_authenticated_wallet(sender_account_id, sender_pub_key, None);

    // Now create the target account
    let target_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN).unwrap();
    let (target_pub_key, target_falcon_auth) = get_new_pk_and_authenticator();
    let target_account =
        get_account_with_basic_authenticated_wallet(target_account_id, target_pub_key, None);

    // Now create the malicious account
    let malicious_account_id =
        AccountId::try_from(ACCOUNT_ID_REGULAR_ACCOUNT_UPDATABLE_CODE_ON_CHAIN_2).unwrap();
    let (malicious_pub_key, malicious_falcon_auth) = get_new_pk_and_authenticator();
    let malicious_account =
        get_account_with_basic_authenticated_wallet(malicious_account_id, malicious_pub_key, None);

    // --------------------------------------------------------------------------------------------
    // Create notes
    // Create the reclaim block height (Note: Current block height is 4)
    let reclaim_block_height_in_time = 5_u32;
    let reclaim_block_height_reclaimable = 3_u32;

    // Create the notes with the P2IDR script
    // Create the note_in_time
    let note_in_time = create_p2idr_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset],
        NoteType::Public,
        Felt::new(0),
        reclaim_block_height_in_time,
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // Create the reclaimable_note
    let note_reclaimable = create_p2idr_note(
        sender_account_id,
        target_account_id,
        vec![fungible_asset],
        NoteType::Public,
        Felt::new(0),
        reclaim_block_height_reclaimable,
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // --------------------------------------------------------------------------------------------
    // We have two cases.
    //
    // Case "in time": block height is 4, reclaim block height is 5. Only the target account can
    // consume the note.
    //
    // Case "reclaimable": block height is 4, reclaim block height is 3. Target and sender account
    // can consume the note. The malicious account should never be able to consume the note.
    // --------------------------------------------------------------------------------------------
    // CONSTRUCT AND EXECUTE TX (Case "in time" - Target Account Execution Success)
    // --------------------------------------------------------------------------------------------
    let tx_context_1 = TransactionContextBuilder::new(target_account.clone())
        .input_notes(vec![note_in_time.clone()])
        .build();
    let executor_1 =
        TransactionExecutor::new(Arc::new(tx_context_1.clone()), Some(target_falcon_auth.clone()));

    let block_ref_1 = tx_context_1.tx_inputs().block_header().block_num();
    let note_ids = tx_context_1.input_notes().iter().map(|note| note.id()).collect::<Vec<_>>();

    let tx_script_target = build_default_auth_script();
    let tx_args_target = TransactionArgs::with_tx_script(tx_script_target);

    // Execute the transaction and get the witness
    let executed_transaction_1 = executor_1
        .execute_transaction(target_account_id, block_ref_1, &note_ids, tx_args_target.clone())
        .unwrap();

    // Assert that the target_account received the funds and the nonce increased by 1
    let target_account_after: Account = Account::from_parts(
        target_account_id,
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert_eq!(executed_transaction_1.final_account().hash(), target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "in time" - Sender Account Execution Failure)
    // --------------------------------------------------------------------------------------------
    let tx_context_2 = TransactionContextBuilder::new(sender_account.clone())
        .input_notes(vec![note_in_time.clone()])
        .build();
    let executor_2 =
        TransactionExecutor::new(Arc::new(tx_context_2.clone()), Some(sender_falcon_auth.clone()));
    let tx_script_sender = build_default_auth_script();
    let tx_args_sender = TransactionArgs::with_tx_script(tx_script_sender);

    let block_ref_2 = tx_context_2.tx_inputs().block_header().block_num();
    let note_ids_2 = tx_context_2.input_notes().iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_2 = executor_2.execute_transaction(
        sender_account_id,
        block_ref_2,
        &note_ids_2,
        tx_args_sender.clone(),
    );

    // Check that we got the expected result - TransactionExecutorError and not ExecutedTransaction
    // Second transaction should not work (sender consumes too early), we expect an error
    assert!(executed_transaction_2.is_err());

    // CONSTRUCT AND EXECUTE TX (Case "in time" - Malicious Target Account Failure)
    // --------------------------------------------------------------------------------------------
    let tx_context_3 = TransactionContextBuilder::new(malicious_account.clone())
        .input_notes(vec![note_in_time.clone()])
        .build();
    let executor_3 = TransactionExecutor::new(
        Arc::new(tx_context_3.clone()),
        Some(malicious_falcon_auth.clone()),
    );

    let tx_script_malicious = build_default_auth_script();
    let tx_args_malicious = TransactionArgs::with_tx_script(tx_script_malicious);

    let block_ref_3 = tx_context_3.tx_inputs().block_header().block_num();
    let note_ids_3 = tx_context_3.input_notes().iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_3 = executor_3.execute_transaction(
        malicious_account_id,
        block_ref_3,
        &note_ids_3,
        tx_args_malicious.clone(),
    );

    // Check that we got the expected result - TransactionExecutorError and not ExecutedTransaction
    // Third transaction should not work (malicious account can never consume), we expect an error
    assert!(executed_transaction_3.is_err());

    // CONSTRUCT AND EXECUTE TX (Case "reclaimable" - Execution Target Account Success)
    // --------------------------------------------------------------------------------------------
    let tx_context_4 = TransactionContextBuilder::new(target_account.clone())
        .input_notes(vec![note_reclaimable.clone()])
        .build();
    let executor_4 =
        TransactionExecutor::new(Arc::new(tx_context_4.clone()), Some(target_falcon_auth));

    let block_ref_4 = tx_context_4.tx_inputs().block_header().block_num();
    let note_ids_4 = tx_context_4.input_notes().iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_4 = executor_4
        .execute_transaction(target_account_id, block_ref_4, &note_ids_4, tx_args_target)
        .unwrap();

    // Check that we got the expected result - ExecutedTransaction
    // Assert that the target_account received the funds and the nonce increased by 1
    // Nonce delta
    assert_eq!(executed_transaction_4.account_delta().nonce(), Some(Felt::new(2)));

    // Vault delta
    let target_account_after: Account = Account::from_parts(
        target_account_id,
        AssetVault::new(&[fungible_asset]).unwrap(),
        target_account.storage().clone(),
        target_account.code().clone(),
        Felt::new(2),
    );
    assert_eq!(executed_transaction_4.final_account().hash(), target_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "too late" - Execution Sender Account Success)
    // --------------------------------------------------------------------------------------------
    let tx_context_5 = TransactionContextBuilder::new(sender_account.clone())
        .input_notes(vec![note_reclaimable.clone()])
        .build();
    let executor_5 =
        TransactionExecutor::new(Arc::new(tx_context_5.clone()), Some(sender_falcon_auth));

    let block_ref_5 = tx_context_5.tx_inputs().block_header().block_num();
    let note_ids_5 = tx_context_5.input_notes().iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_5 = executor_5
        .execute_transaction(sender_account_id, block_ref_5, &note_ids_5, tx_args_sender)
        .unwrap();

    // Assert that the sender_account received the funds and the nonce increased by 1
    // Nonce delta
    assert_eq!(executed_transaction_5.account_delta().nonce(), Some(Felt::new(2)));

    // Vault delta (Note: vault was empty before)
    let sender_account_after: Account = Account::from_parts(
        sender_account_id,
        AssetVault::new(&[fungible_asset]).unwrap(),
        sender_account.storage().clone(),
        sender_account.code().clone(),
        Felt::new(2),
    );
    assert_eq!(executed_transaction_5.final_account().hash(), sender_account_after.hash());

    // CONSTRUCT AND EXECUTE TX (Case "too late" - Malicious Account Failure)
    // --------------------------------------------------------------------------------------------
    let tx_context_6 = TransactionContextBuilder::new(malicious_account.clone())
        .input_notes(vec![note_reclaimable.clone()])
        .build();

    let executor_6 =
        TransactionExecutor::new(Arc::new(tx_context_6.clone()), Some(malicious_falcon_auth));

    let block_ref_6 = tx_context_6.tx_inputs().block_header().block_num();
    let note_ids_6 = tx_context_6.input_notes().iter().map(|note| note.id()).collect::<Vec<_>>();

    // Execute the transaction and get the witness
    let executed_transaction_6 = executor_6.execute_transaction(
        malicious_account_id,
        block_ref_6,
        &note_ids_6,
        tx_args_malicious,
    );

    // Check that we got the expected result - TransactionExecutorError and not ExecutedTransaction
    // Sixth transaction should not work (malicious account can never consume), we expect an error
    assert!(executed_transaction_6.is_err())
}

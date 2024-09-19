use alloc::vec::Vec;

use miden_objects::{digest, Digest, Felt, Hasher};

use super::TransactionKernel;

// TRANSACTION KERNEL
// ================================================================================================

impl TransactionKernel {
    // CONSTANTS
    // --------------------------------------------------------------------------------------------

    /// Number of currently used kernel versions.
    pub const NUM_VERSIONS: usize = 1;

    /// Array of all available kernels.
    pub const PROCEDURES: [&'static [Digest]; Self::NUM_VERSIONS] = [&KERNEL0_PROCEDURES];

    // PUBLIC ACCESSORS
    // --------------------------------------------------------------------------------------------

    /// Returns procedures of the kernel specified by the `kernel_version` as vector of Felts.
    pub fn procedures_as_elements(kernel_version: u8) -> Vec<Felt> {
        Digest::digests_as_elements(
            Self::PROCEDURES
                .get(kernel_version as usize)
                .expect("provided kernel index is out of bounds")
                .iter(),
        )
        .cloned()
        .collect::<Vec<Felt>>()
    }

    /// Computes the accumulative hash of all procedures of the kernel specified by the
    /// `kernel_version`.
    pub fn kernel_hash(kernel_version: u8) -> Digest {
        Hasher::hash_elements(&Self::procedures_as_elements(kernel_version))
    }

    /// Computes a hash from all kernel hashes.
    pub fn kernel_root() -> Digest {
        Hasher::hash_elements(&[Self::kernel_hash(0).as_elements()].concat())
    }
}

// KERNEL V0 PROCEDURES
// ================================================================================================

/// Hashes of all dynamically executed procedures from the kernel 0.
const KERNEL0_PROCEDURES: [Digest; 28] = [
    // account_vault_add_asset
    digest!(
        117074302502728688,
        11439878644778514598,
        16324818132154524894,
        6489512630979919440
    ),
    // account_vault_get_balance
    digest!(
        7035484340365940230,
        17797159859808856495,
        10586583242494928923,
        9763511907089065699
    ),
    // account_vault_has_non_fungible_asset
    digest!(
        3461454265989980777,
        16222005807253493271,
        5019331476826215138,
        8747291997159999285
    ),
    // account_vault_remove_asset
    digest!(
        2235246958022854005,
        5794405659267712135,
        12598697568377601936,
        10963092377629893642
    ),
    // get_account_id
    digest!(
        8040261465733444704,
        11111141085375373880,
        7423929485586361344,
        4119214601469502087
    ),
    // get_account_item
    digest!(
        18206004789224066622,
        4233449336812475978,
        6804658891075571436,
        3940070286581972689
    ),
    // get_account_map_item
    digest!(
        9209967448327341770,
        8988024763842561887,
        12632818454415758249,
        8233400257714804605
    ),
    // get_account_nonce
    digest!(
        7949369589472998218,
        13470489034885204869,
        7657993556512253706,
        4189240183103072865
    ),
    // get_account_vault_commitment
    digest!(
        15827173769627914405,
        8397707743192029429,
        7205844492194182641,
        1677433344562532693
    ),
    // get_current_account_hash
    digest!(
        18067387847945059633,
        4630780713348682492,
        16252299253975780120,
        12604901563870135002
    ),
    // get_initial_account_hash
    digest!(
        16301123123708038227,
        8835228777116955671,
        1233594748884564040,
        17497683909577038473
    ),
    // incr_account_nonce
    digest!(
        14589349829020905629,
        1412999498410091194,
        17301618149076423693,
        2638573156781761162
    ),
    // set_account_code
    digest!(
        13397042012380537032,
        174474080566637302,
        1465955330516409718,
        13427241200626333441
    ),
    // set_account_item
    digest!(
        7028525769329264650,
        7531398982722010851,
        3695061772051382659,
        2998651828779176432
    ),
    // set_account_map_item
    digest!(
        7037030220885902605,
        1540995878644451898,
        11995286967942035929,
        11976243733826929886
    ),
    // burn_asset
    digest!(
        10812504956203964835,
        17035791932747451701,
        8886876315554082935,
        6015659628759368174
    ),
    // get_fungible_faucet_total_issuance
    digest!(
        1872004623160272764,
        3364880498288329522,
        9154945937727211188,
        2334132046349758621
    ),
    // mint_asset
    digest!(
        17329749049914215544,
        5633414059905366308,
        2519432440213570275,
        8693308573092701498
    ),
    // add_asset_to_note
    digest!(
        16660224074633768406,
        3681728837439485251,
        11007804027515511275,
        7127888127578457912
    ),
    // create_note
    digest!(
        386212833718199205,
        11471520476317876635,
        15232296418503481248,
        574740517948464248
    ),
    // get_input_notes_commitment
    digest!(
        2019728671844693749,
        18222437788741437389,
        12821100448410084889,
        17418670035031233675
    ),
    // get_note_assets_info
    digest!(
        12346411220238036656,
        18027533406091104744,
        14723639276543495147,
        11542458885879781389
    ),
    // get_note_inputs_hash
    digest!(
        17186028199923932877,
        2563818256742276816,
        8351223767950877211,
        11379249881600223287
    ),
    // get_note_sender
    digest!(
        15233821980580537524,
        8874650687593596380,
        14910554371357890324,
        11945045801206913876
    ),
    // get_note_serial_number
    digest!(
        203467101694736292,
        1871816977533069235,
        11026610821411620572,
        8345006103126977916
    ),
    // get_output_notes_hash
    digest!(
        4412523757021344747,
        8883378993868597671,
        16885133168375194469,
        15472424727696440458
    ),
    // get_block_hash
    digest!(
        15575368355470837910,
        13483490255982391120,
        5407999307430887046,
        13895912493177462699
    ),
    // get_block_number
    digest!(
        957081505105679725,
        18012382143736246386,
        13337406348155951825,
        4537613255382865554
    ),
];
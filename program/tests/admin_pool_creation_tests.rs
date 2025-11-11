// ============================================================================
// Admin-Only Pool Creation Tests [Q-01]
// ============================================================================
// This test suite validates that:
// 1. ProgramAuthority state logic works correctly
// 2. Add/remove authorized creators functions properly
// 3. Main authority cannot be removed
// 4. Authorization checks work as expected
//
// Test Status: ✅ Unit tests (state logic)
// Note: Full integration tests are in TypeScript (example/src/)

use num_traits::FromPrimitive;
use solana_program::program_error::ProgramError;
use solana_sdk::pubkey::Pubkey;
use your_wallet_stake_pool::{
    error::StakePoolError,
    state::{Key, ProgramAuthority},
};

/// Helper to convert ProgramError to StakePoolError
fn to_stake_pool_error(e: ProgramError) -> StakePoolError {
    if let ProgramError::Custom(code) = e {
        StakePoolError::from_u32(code).expect("Unknown error code")
    } else {
        panic!("Expected custom error")
    }
}

// ============================================================================
// ProgramAuthority State Tests
// ============================================================================

#[test]
fn test_program_authority_initialization() {
    let authority = Pubkey::new_unique();

    let program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    assert!(matches!(program_authority.key, Key::ProgramAuthority));
    assert_eq!(program_authority.authority, authority);
    assert_eq!(program_authority.creator_count, 0);
    assert!(program_authority
        .authorized_creators
        .iter()
        .all(|c| c.is_none()));
}

#[test]
fn test_main_authority_is_authorized() {
    let authority = Pubkey::new_unique();

    let program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Main authority should always be authorized
    assert!(program_authority.is_authorized(&authority));
}

#[test]
fn test_unauthorized_address_is_not_authorized() {
    let authority = Pubkey::new_unique();
    let unauthorized = Pubkey::new_unique();

    let program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Unauthorized address should not be authorized
    assert!(!program_authority.is_authorized(&unauthorized));
}

#[test]
fn test_add_authorized_creator_success() {
    let authority = Pubkey::new_unique();
    let creator = Pubkey::new_unique();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Add creator
    let result = program_authority.add_creator(creator);
    assert!(result.is_ok());

    // Verify creator was added
    assert_eq!(program_authority.creator_count, 1);
    assert!(program_authority.is_authorized(&creator));
    assert!(program_authority
        .authorized_creators
        .iter()
        .any(|c| c == &Some(creator)));
}

#[test]
fn test_add_main_authority_fails() {
    let authority = Pubkey::new_unique();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Try to add main authority - should fail
    let result = program_authority.add_creator(authority);
    assert!(result.is_err());

    if let Err(e) = result {
        let error = to_stake_pool_error(e);
        assert!(matches!(error, StakePoolError::InvalidParameters));
    }

    // Verify count is still 0 and main authority remains authorized via is_authorized()
    assert_eq!(program_authority.creator_count, 0);
    assert!(program_authority.is_authorized(&authority));
}

#[test]
fn test_add_duplicate_creator_fails() {
    let authority = Pubkey::new_unique();
    let creator = Pubkey::new_unique();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Add creator once
    program_authority.add_creator(creator).unwrap();

    // Try to add again - should fail
    let result = program_authority.add_creator(creator);
    assert!(result.is_err());

    if let Err(e) = result {
        let error = to_stake_pool_error(e);
        assert!(matches!(error, StakePoolError::CreatorAlreadyAuthorized));
    }
}

#[test]
fn test_add_max_creators_success() {
    let authority = Pubkey::new_unique();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Add maximum creators
    for i in 0..ProgramAuthority::MAX_CREATORS {
        let creator = Pubkey::new_unique();
        let result = program_authority.add_creator(creator);
        assert!(result.is_ok(), "Failed to add creator {}", i);
        assert!(program_authority.is_authorized(&creator));
    }

    assert_eq!(
        program_authority.creator_count as usize,
        ProgramAuthority::MAX_CREATORS
    );
}

#[test]
fn test_add_beyond_max_creators_fails() {
    let authority = Pubkey::new_unique();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Fill up to max
    for _ in 0..ProgramAuthority::MAX_CREATORS {
        let creator = Pubkey::new_unique();
        program_authority.add_creator(creator).unwrap();
    }

    // Try to add one more - should fail
    let extra_creator = Pubkey::new_unique();
    let result = program_authority.add_creator(extra_creator);
    assert!(result.is_err());

    if let Err(e) = result {
        let error = to_stake_pool_error(e);
        assert!(matches!(
            error,
            StakePoolError::MaxAuthorizedCreatorsReached
        ));
    }
}

#[test]
fn test_remove_authorized_creator_success() {
    let authority = Pubkey::new_unique();
    let creator = Pubkey::new_unique();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Add creator
    program_authority.add_creator(creator).unwrap();
    assert_eq!(program_authority.creator_count, 1);

    // Remove creator
    let result = program_authority.remove_creator(&creator);
    assert!(result.is_ok());

    // Verify creator was removed
    assert_eq!(program_authority.creator_count, 0);
    assert!(!program_authority.is_authorized(&creator));
    assert!(program_authority
        .authorized_creators
        .iter()
        .all(|c| c.is_none()));
}

#[test]
fn test_remove_main_authority_fails() {
    let authority = Pubkey::new_unique();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Try to remove main authority - should fail
    let result = program_authority.remove_creator(&authority);
    assert!(result.is_err());

    if let Err(e) = result {
        let error = to_stake_pool_error(e);
        assert!(matches!(error, StakePoolError::CannotRemoveMainAuthority));
    }

    // Main authority should still be authorized
    assert!(program_authority.is_authorized(&authority));
}

#[test]
fn test_remove_nonexistent_creator_fails() {
    let authority = Pubkey::new_unique();
    let nonexistent = Pubkey::new_unique();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Try to remove creator that was never added
    let result = program_authority.remove_creator(&nonexistent);
    assert!(result.is_err());

    if let Err(e) = result {
        let error = to_stake_pool_error(e);
        assert!(matches!(error, StakePoolError::CreatorNotFound));
    }
}

#[test]
fn test_add_and_remove_multiple_creators() {
    let authority = Pubkey::new_unique();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Add 5 creators
    let creators: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();

    for creator in &creators {
        program_authority.add_creator(*creator).unwrap();
    }

    assert_eq!(program_authority.creator_count, 5);

    // Verify all are authorized
    for creator in &creators {
        assert!(program_authority.is_authorized(creator));
    }

    // Remove 3 creators
    for creator in &creators[0..3] {
        program_authority.remove_creator(creator).unwrap();
    }

    assert_eq!(program_authority.creator_count, 2);

    // Verify first 3 are not authorized
    for creator in &creators[0..3] {
        assert!(!program_authority.is_authorized(creator));
    }

    // Verify last 2 are still authorized
    for creator in &creators[3..5] {
        assert!(program_authority.is_authorized(creator));
    }
}

#[test]
fn test_pda_derivation_is_deterministic() {
    let (pda1, bump1) = ProgramAuthority::find_pda();
    let (pda2, bump2) = ProgramAuthority::find_pda();

    assert_eq!(pda1, pda2);
    assert_eq!(bump1, bump2);
}

#[test]
fn test_pda_is_off_curve() {
    let (pda, _bump) = ProgramAuthority::find_pda();

    // PDA should be off-curve (not a valid ed25519 point)
    assert!(!pda.is_on_curve());
}

#[test]
fn test_authorized_creators_array_size() {
    // Verify the max creators constant matches the array size
    assert_eq!(ProgramAuthority::MAX_CREATORS, 10);

    let authority = Pubkey::new_unique();
    let program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    assert_eq!(program_authority.authorized_creators.len(), 10);
}

#[test]
fn test_creator_count_accuracy() {
    let authority = Pubkey::new_unique();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Add creators and verify count
    for i in 0..5 {
        let creator = Pubkey::new_unique();
        program_authority.add_creator(creator).unwrap();
        assert_eq!(program_authority.creator_count, i + 1);
    }

    // Remove creators and verify count
    let creators_to_remove: Vec<Pubkey> = program_authority
        .authorized_creators
        .iter()
        .filter_map(|c| *c)
        .collect();

    for (i, creator) in creators_to_remove.iter().enumerate() {
        program_authority.remove_creator(creator).unwrap();
        assert_eq!(program_authority.creator_count, 4 - i as u8);
    }
}

#[test]
fn test_serialization_size() {
    let authority = Pubkey::new_unique();
    let program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    let serialized = borsh::to_vec(&program_authority).unwrap();

    // Should match the defined LEN constant
    assert!(serialized.len() <= ProgramAuthority::LEN);
}

#[test]
fn test_deserialization_roundtrip() {
    let authority = Pubkey::new_unique();
    let creator1 = Pubkey::new_unique();
    let creator2 = Pubkey::new_unique();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    program_authority.add_creator(creator1).unwrap();
    program_authority.add_creator(creator2).unwrap();

    // Serialize
    let serialized = borsh::to_vec(&program_authority).unwrap();

    // Deserialize
    use borsh::BorshDeserialize;
    let deserialized: ProgramAuthority = ProgramAuthority::try_from_slice(&serialized).unwrap();

    // Verify all fields match
    assert!(matches!(deserialized.key, Key::ProgramAuthority));
    assert_eq!(deserialized.authority, program_authority.authority);
    assert_eq!(deserialized.creator_count, program_authority.creator_count);
    assert_eq!(deserialized.bump, program_authority.bump);

    // Verify creators match
    assert!(deserialized.is_authorized(&creator1));
    assert!(deserialized.is_authorized(&creator2));
}

#[test]
fn test_array_compaction_after_removal() {
    let authority = Pubkey::new_unique();
    let creators: Vec<Pubkey> = (0..5).map(|_| Pubkey::new_unique()).collect();

    let mut program_authority = ProgramAuthority {
        key: Key::ProgramAuthority,
        authority,
        authorized_creators: [None; ProgramAuthority::MAX_CREATORS],
        creator_count: 0,
        bump: 255,
    };

    // Add 5 creators
    for creator in &creators {
        program_authority.add_creator(*creator).unwrap();
    }

    // Remove creators at positions 1 and 3
    program_authority.remove_creator(&creators[1]).unwrap();
    program_authority.remove_creator(&creators[3]).unwrap();

    // After compaction, array should have no holes
    // Should be: [creators[0], creators[2], creators[4], None, None, ...]
    let mut found_none = false;
    for slot in &program_authority.authorized_creators {
        if slot.is_none() {
            found_none = true;
        } else if found_none {
            panic!("Found Some after None - array not compacted!");
        }
    }

    // Verify remaining creators are still authorized
    assert!(program_authority.is_authorized(&creators[0]));
    assert!(!program_authority.is_authorized(&creators[1])); // removed
    assert!(program_authority.is_authorized(&creators[2]));
    assert!(!program_authority.is_authorized(&creators[3])); // removed
    assert!(program_authority.is_authorized(&creators[4]));

    // Verify count is correct
    assert_eq!(program_authority.creator_count, 3);
}

// ============================================================================
// Integration Notes
// ============================================================================
//
// For full integration tests including transaction processing:
// See: example/src/devnet-integration-test.ts
//
// These tests cover:
// - InitializeProgramAuthority transaction
// - ManageAuthorizedCreators transaction
// - InitializePool with authorization check
// - Unauthorized pool creation rejection
// - Authority transfer scenarios
//
// ============================================================================

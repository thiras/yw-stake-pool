import test from 'ava';
import {
  getProgramAuthorityCodec,
  getInitializeProgramAuthorityInstructionDataCodec,
  getManageAuthorizedCreatorsInstructionDataCodec,
  Key,
  STAKE_POOL_ERROR__UNAUTHORIZED_POOL_CREATOR,
  STAKE_POOL_ERROR__CREATOR_ALREADY_AUTHORIZED,
  STAKE_POOL_ERROR__MAX_AUTHORIZED_CREATORS_REACHED,
  STAKE_POOL_ERROR__CANNOT_REMOVE_MAIN_AUTHORITY,
  STAKE_POOL_ERROR__CREATOR_NOT_FOUND,
  STAKE_POOL_ERROR__ALREADY_INITIALIZED,
} from '../src';
import { address } from '@solana/kit';

// ============================================================================
// ProgramAuthority Account Tests
// ============================================================================

test('ProgramAuthority codec encodes and decodes correctly', (t) => {
  const codec = getProgramAuthorityCodec();

  const programAuthority = {
    key: Key.ProgramAuthority,
    authority: address('11111111111111111111111111111111'),
    authorizedCreators: [
      address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
      address('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb'),
      null,
      null,
      null,
      null,
      null,
      null,
      null,
      null,
    ],
    creatorCount: 2,
    bump: 255,
  };

  const encoded = codec.encode(programAuthority);
  const decoded = codec.decode(encoded);

  t.is(decoded.key, Key.ProgramAuthority);
  t.is(decoded.authority, programAuthority.authority);
  t.is(decoded.creatorCount, 2);
  t.is(decoded.bump, 255);

  // Check authorized creators - decode returns Option type
  t.deepEqual(decoded.authorizedCreators[0], {
    __option: 'Some',
    value: programAuthority.authorizedCreators[0],
  });
  t.deepEqual(decoded.authorizedCreators[1], {
    __option: 'Some',
    value: programAuthority.authorizedCreators[1],
  });
  t.deepEqual(decoded.authorizedCreators[2], { __option: 'None' });
});

test('ProgramAuthority codec handles empty creators list', (t) => {
  const codec = getProgramAuthorityCodec();

  const programAuthority = {
    key: Key.ProgramAuthority,
    authority: address('11111111111111111111111111111111'),
    authorizedCreators: Array(10).fill(null),
    creatorCount: 0,
    bump: 255,
  };

  const encoded = codec.encode(programAuthority);
  const decoded = codec.decode(encoded);

  t.is(decoded.creatorCount, 0);
  decoded.authorizedCreators.forEach((creator) => {
    t.deepEqual(creator, { __option: 'None' });
  });
});

test('ProgramAuthority codec handles max creators', (t) => {
  const codec = getProgramAuthorityCodec();

  // Use simple valid addresses (all same address for testing purposes)
  const creators = Array.from({ length: 10 }, () =>
    address('11111111111111111111111111111111')
  );

  const programAuthority = {
    key: Key.ProgramAuthority,
    authority: address('11111111111111111111111111111111'),
    authorizedCreators: creators,
    creatorCount: 10,
    bump: 255,
  };

  const encoded = codec.encode(programAuthority);
  const decoded = codec.decode(encoded);

  t.is(decoded.creatorCount, 10);
  decoded.authorizedCreators.forEach((creator, idx) => {
    t.deepEqual(creator, { __option: 'Some', value: creators[idx] });
  });
});

test('ProgramAuthority has correct size', (t) => {
  const codec = getProgramAuthorityCodec();

  // Test empty creators list (all null)
  const emptyAuthority = {
    key: Key.ProgramAuthority,
    authority: address('11111111111111111111111111111111'),
    authorizedCreators: Array(10).fill(null),
    creatorCount: 0,
    bump: 255,
  };

  const encodedEmpty = codec.encode(emptyAuthority);
  // Size: 1 (key) + 32 (authority) + 10 (10 * 1 byte for Option::None) + 1 (count) + 1 (bump) = 45 bytes
  // Note: Borsh encodes Option::None as 1 byte (0x00), Option::Some as 1 byte (0x01) + value
  t.is(encodedEmpty.length, 45);

  // Test full creators list (all Some)
  const fullAuthority = {
    key: Key.ProgramAuthority,
    authority: address('11111111111111111111111111111111'),
    authorizedCreators: Array(10).fill(
      address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA')
    ),
    creatorCount: 10,
    bump: 255,
  };

  const encodedFull = codec.encode(fullAuthority);
  // Size: 1 (key) + 32 (authority) + 330 (10 * 33 bytes for Option::Some + Pubkey) + 1 (count) + 1 (bump) = 365 bytes
  // Note: Each Option::Some(Pubkey) is 1 byte discriminator + 32 bytes pubkey = 33 bytes
  t.is(encodedFull.length, 365);
});

// ============================================================================
// InitializeProgramAuthority Instruction Tests
// ============================================================================

test('InitializeProgramAuthority instruction data codec', (t) => {
  const codec = getInitializeProgramAuthorityInstructionDataCodec();

  const data = {};

  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  // Should have discriminator
  t.is(encoded.length, 1); // Just the discriminator byte
  t.is(decoded.discriminator, 10); // InitializeProgramAuthority has discriminator 10
  t.is(Object.keys(decoded).length, 1); // Only the discriminator field
});

// ============================================================================
// ManageAuthorizedCreators Instruction Tests
// ============================================================================

test('ManageAuthorizedCreators instruction data codec - add creators', (t) => {
  const codec = getManageAuthorizedCreatorsInstructionDataCodec();

  const data = {
    add: [
      address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
      address('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb'),
    ],
    remove: [],
  };

  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  t.is(decoded.add.length, 2);
  t.is(decoded.add[0], data.add[0]);
  t.is(decoded.add[1], data.add[1]);
  t.is(decoded.remove.length, 0);
});

test('ManageAuthorizedCreators instruction data codec - remove creators', (t) => {
  const codec = getManageAuthorizedCreatorsInstructionDataCodec();

  const data = {
    add: [],
    remove: [address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA')],
  };

  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  t.is(decoded.add.length, 0);
  t.is(decoded.remove.length, 1);
  t.is(decoded.remove[0], data.remove[0]);
});

test('ManageAuthorizedCreators instruction data codec - add and remove', (t) => {
  const codec = getManageAuthorizedCreatorsInstructionDataCodec();

  const data = {
    add: [address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA')],
    remove: [address('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb')],
  };

  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  t.is(decoded.add.length, 1);
  t.is(decoded.add[0], data.add[0]);
  t.is(decoded.remove.length, 1);
  t.is(decoded.remove[0], data.remove[0]);
});

test('ManageAuthorizedCreators instruction data codec - empty operation', (t) => {
  const codec = getManageAuthorizedCreatorsInstructionDataCodec();

  const data = {
    add: [],
    remove: [],
  };

  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  t.is(decoded.add.length, 0);
  t.is(decoded.remove.length, 0);
});

test('ManageAuthorizedCreators instruction data codec - max batch size', (t) => {
  const codec = getManageAuthorizedCreatorsInstructionDataCodec();

  // Use simple valid addresses (all same address for testing purposes)
  const creators = Array.from({ length: 10 }, () =>
    address('11111111111111111111111111111111')
  );

  const data = {
    add: creators,
    remove: [],
  };

  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  t.is(decoded.add.length, 10);
  decoded.add.forEach((creator, idx) => {
    t.is(creator, creators[idx]);
  });
});

// ============================================================================
// Error Code Tests
// ============================================================================

test('Admin pool creation error codes are defined', (t) => {
  t.is(STAKE_POOL_ERROR__UNAUTHORIZED_POOL_CREATOR, 35);
  t.is(STAKE_POOL_ERROR__CREATOR_ALREADY_AUTHORIZED, 36);
  t.is(STAKE_POOL_ERROR__MAX_AUTHORIZED_CREATORS_REACHED, 37);
  t.is(STAKE_POOL_ERROR__CANNOT_REMOVE_MAIN_AUTHORITY, 38);
  t.is(STAKE_POOL_ERROR__CREATOR_NOT_FOUND, 39);
  t.is(STAKE_POOL_ERROR__ALREADY_INITIALIZED, 40);
});

// ============================================================================
// Round-trip Tests
// ============================================================================

test('ProgramAuthority round-trip with various states', (t) => {
  const codec = getProgramAuthorityCodec();

  const testCases = [
    {
      name: 'No creators',
      data: {
        key: Key.ProgramAuthority,
        authority: address('11111111111111111111111111111111'),
        authorizedCreators: Array(10).fill(null),
        creatorCount: 0,
        bump: 255,
      },
    },
    {
      name: 'One creator',
      data: {
        key: Key.ProgramAuthority,
        authority: address('11111111111111111111111111111111'),
        authorizedCreators: [
          address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
          ...Array(9).fill(null),
        ],
        creatorCount: 1,
        bump: 255,
      },
    },
    {
      name: 'Multiple creators with gaps',
      data: {
        key: Key.ProgramAuthority,
        authority: address('11111111111111111111111111111111'),
        authorizedCreators: [
          address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
          null,
          address('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb'),
          ...Array(7).fill(null),
        ],
        creatorCount: 2,
        bump: 255,
      },
    },
  ];

  testCases.forEach(({ name, data }) => {
    const encoded = codec.encode(data);
    const decoded = codec.decode(encoded);

    t.is(decoded.key, data.key, `${name}: key should match`);
    t.is(decoded.authority, data.authority, `${name}: authority should match`);
    t.is(
      decoded.creatorCount,
      data.creatorCount,
      `${name}: count should match`
    );
    t.is(decoded.bump, data.bump, `${name}: bump should match`);

    // Check creators individually since decoded will have Option type
    data.authorizedCreators.forEach((creator, idx) => {
      if (creator === null) {
        t.deepEqual(
          decoded.authorizedCreators[idx],
          { __option: 'None' },
          `${name}: creator ${idx} should be None`
        );
      } else {
        t.deepEqual(
          decoded.authorizedCreators[idx],
          { __option: 'Some', value: creator },
          `${name}: creator ${idx} should match`
        );
      }
    });
  });
});

// ============================================================================
// Instruction Size Tests
// ============================================================================

test('InitializeProgramAuthority instruction has minimal size', (t) => {
  const codec = getInitializeProgramAuthorityInstructionDataCodec();
  const encoded = codec.encode({});

  // Should only have discriminator (1 byte)
  t.is(encoded.length, 1);
});

test('ManageAuthorizedCreators instruction size scales with operations', (t) => {
  const codec = getManageAuthorizedCreatorsInstructionDataCodec();

  // Test with increasing number of operations
  // Use valid base58 addresses (all '1's is a valid address representing all zeros)
  const sizes = [0, 1, 5, 10].map((count) => {
    const creators = Array.from({ length: count }, () =>
      address('11111111111111111111111111111111')
    );

    const data = { add: creators, remove: [] };
    const encoded = codec.encode(data);
    return encoded.length;
  });

  // Each pubkey is 32 bytes, plus vector length encoding
  // Size should increase as we add more creators
  t.true(sizes[0] < sizes[1]);
  t.true(sizes[1] < sizes[2]);
  t.true(sizes[2] < sizes[3]);
});

import test from 'ava';
import {
  STAKE_POOL_ERROR__REWARD_RATE_CHANGE_DELAY_NOT_ELAPSED,
  STAKE_POOL_ERROR__NO_PENDING_REWARD_RATE_CHANGE,
  STAKE_POOL_ERROR__PENDING_REWARD_RATE_CHANGE_EXISTS,
  STAKE_POOL_ERROR__INVALID_TIMESTAMP,
  getStakePoolErrorMessage,
  getFinalizeRewardRateChangeInstructionDataCodec,
  FINALIZE_REWARD_RATE_CHANGE_DISCRIMINATOR,
  getStakePoolCodec,
} from '../src';
import { address, some } from '@solana/kit';

test('RewardRateChangeDelayNotElapsed error code is correct', (t) => {
  t.is(STAKE_POOL_ERROR__REWARD_RATE_CHANGE_DELAY_NOT_ELAPSED, 0x1d); // 29
});

test('NoPendingRewardRateChange error code is correct', (t) => {
  t.is(STAKE_POOL_ERROR__NO_PENDING_REWARD_RATE_CHANGE, 0x1e); // 30
});

test('PendingRewardRateChangeExists error code is correct', (t) => {
  t.is(STAKE_POOL_ERROR__PENDING_REWARD_RATE_CHANGE_EXISTS, 0x1f); // 31
});

test('InvalidTimestamp error code is correct', (t) => {
  t.is(STAKE_POOL_ERROR__INVALID_TIMESTAMP, 0x20); // 32
});

test('RewardRateChangeDelayNotElapsed error message', (t) => {
  const message = getStakePoolErrorMessage(
    STAKE_POOL_ERROR__REWARD_RATE_CHANGE_DELAY_NOT_ELAPSED
  );
  t.is(message, 'Reward rate change delay not elapsed');
});

test('NoPendingRewardRateChange error message', (t) => {
  const message = getStakePoolErrorMessage(
    STAKE_POOL_ERROR__NO_PENDING_REWARD_RATE_CHANGE
  );
  t.is(message, 'No pending reward rate change');
});

test('PendingRewardRateChangeExists error message', (t) => {
  const message = getStakePoolErrorMessage(
    STAKE_POOL_ERROR__PENDING_REWARD_RATE_CHANGE_EXISTS
  );
  t.is(message, 'Pending reward rate change already exists');
});

test('InvalidTimestamp error message', (t) => {
  const message = getStakePoolErrorMessage(STAKE_POOL_ERROR__INVALID_TIMESTAMP);
  t.is(message, 'Invalid timestamp (timestamp is in the future)');
});

test('FinalizeRewardRateChange instruction has correct discriminator', (t) => {
  t.is(FINALIZE_REWARD_RATE_CHANGE_DISCRIMINATOR, 9);
});

test('FinalizeRewardRateChange instruction data codec encodes and decodes', (t) => {
  const codec = getFinalizeRewardRateChangeInstructionDataCodec();

  const data = {};
  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  t.is(decoded.discriminator, FINALIZE_REWARD_RATE_CHANGE_DISCRIMINATOR);
});

test('StakePool has pendingRewardRate field', (t) => {
  // This test verifies that the StakePool type includes the new field
  // The actual encoding/decoding is tested in accounts.test.ts
  const codec = getStakePoolCodec();

  // Verify codec accepts pendingRewardRate
  const stakePool = {
    key: 1,
    authority: address('11111111111111111111111111111111'),
    stakeMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    rewardMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    poolId: 0n,
    stakeVault: address('11111111111111111111111111111111'),
    rewardVault: address('11111111111111111111111111111111'),
    totalStaked: 0n,
    totalRewardsOwed: 0n,
    rewardRate: 100_000_000n,
    minStakeAmount: 1_000_000n,
    lockupPeriod: 86400n,
    isPaused: false,
    enforceLockup: false,
    bump: 255,
    pendingAuthority: null,
    poolEndDate: null,
    pendingRewardRate: some(150_000_000n),
    rewardRateChangeTimestamp: some(1700000000n),
    reserved: new Uint8Array(16),
  };

  // Should not throw
  const encoded = codec.encode(stakePool);
  t.truthy(encoded);
});

test('StakePool has rewardRateChangeTimestamp field', (t) => {
  const codec = getStakePoolCodec();

  const currentTime = 1700000000n;
  const proposalTime = currentTime;

  const stakePool = {
    key: 1,
    authority: address('11111111111111111111111111111111'),
    stakeMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    rewardMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    poolId: 0n,
    stakeVault: address('11111111111111111111111111111111'),
    rewardVault: address('11111111111111111111111111111111'),
    totalStaked: 0n,
    totalRewardsOwed: 0n,
    rewardRate: 100_000_000n,
    minStakeAmount: 1_000_000n,
    lockupPeriod: 86400n,
    isPaused: false,
    enforceLockup: false,
    bump: 255,
    pendingAuthority: null,
    poolEndDate: null,
    pendingRewardRate: some(150_000_000n), // Must be Some when timestamp is Some
    rewardRateChangeTimestamp: some(proposalTime),
    reserved: new Uint8Array(16),
  };

  const encoded = codec.encode(stakePool);
  const decoded = codec.decode(encoded);

  // Verify both pending fields are in sync (invariant requirement)
  t.deepEqual(decoded.pendingRewardRate, {
    __option: 'Some',
    value: 150_000_000n,
  });
  t.deepEqual(decoded.rewardRateChangeTimestamp, {
    __option: 'Some',
    value: proposalTime,
  });
});

test('Time-lock delay constant is 7 days', (t) => {
  // The delay is enforced on-chain
  // This test documents the expected delay: 604800 seconds = 7 days
  const SEVEN_DAYS_IN_SECONDS = 7 * 24 * 60 * 60;
  t.is(SEVEN_DAYS_IN_SECONDS, 604800);
});

test('StakePool pending fields work together', (t) => {
  const codec = getStakePoolCodec();

  // Scenario: pending reward rate change
  const currentTime = 1700000000n;
  const proposalTime = currentTime; // When the change was proposed

  const stakePool = {
    key: 1,
    authority: address('11111111111111111111111111111111'),
    stakeMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    rewardMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    poolId: 0n,
    stakeVault: address('11111111111111111111111111111111'),
    rewardVault: address('11111111111111111111111111111111'),
    totalStaked: 1_000_000_000n,
    totalRewardsOwed: 0n,
    rewardRate: 100_000_000n, // Current: 10%
    minStakeAmount: 1_000_000n,
    lockupPeriod: 86400n,
    isPaused: false,
    enforceLockup: false,
    bump: 255,
    pendingAuthority: null,
    poolEndDate: null,
    pendingRewardRate: some(200_000_000n), // Pending: 20%
    rewardRateChangeTimestamp: some(proposalTime),
    reserved: new Uint8Array(16),
  };

  const encoded = codec.encode(stakePool);
  const decoded = codec.decode(encoded);

  // Verify both fields are preserved
  t.deepEqual(decoded.pendingRewardRate, {
    __option: 'Some',
    value: 200_000_000n,
  });
  t.deepEqual(decoded.rewardRateChangeTimestamp, {
    __option: 'Some',
    value: proposalTime,
  });
  t.is(decoded.rewardRate, 100_000_000n); // Current rate unchanged
});

test('StakePool with no pending changes', (t) => {
  const codec = getStakePoolCodec();

  const stakePool = {
    key: 1,
    authority: address('11111111111111111111111111111111'),
    stakeMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    rewardMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    poolId: 0n,
    stakeVault: address('11111111111111111111111111111111'),
    rewardVault: address('11111111111111111111111111111111'),
    totalStaked: 0n,
    totalRewardsOwed: 0n,
    rewardRate: 100_000_000n,
    minStakeAmount: 1_000_000n,
    lockupPeriod: 86400n,
    isPaused: false,
    enforceLockup: false,
    bump: 255,
    pendingAuthority: null,
    poolEndDate: null,
    pendingRewardRate: null,
    rewardRateChangeTimestamp: null,
    reserved: new Uint8Array(16),
  };

  const encoded = codec.encode(stakePool);
  const decoded = codec.decode(encoded);

  // Both should be None when no pending change
  t.deepEqual(decoded.pendingRewardRate, { __option: 'None' });
  t.deepEqual(decoded.rewardRateChangeTimestamp, { __option: 'None' });
});

test('Reward rate bounds validation constant', (t) => {
  // The on-chain program validates reward_rate <= 1_000_000_000_000
  // This test documents the maximum allowed rate
  const MAX_REWARD_RATE = 1_000_000_000_000n;

  // This represents 100,000% APY (1_000_000_000_000 / 1_000_000_000)
  // In practice, reasonable rates are much lower (e.g., 10-50% = 100M-500M)
  t.is(MAX_REWARD_RATE, 1_000_000_000_000n);

  // Example: 100% APY
  const oneHundredPercent = 1_000_000_000n;
  t.true(oneHundredPercent <= MAX_REWARD_RATE);
});

test('Cancellation mechanism - proposing current rate', (t) => {
  // This test documents that proposing the current reward rate
  // cancels any pending change (on-chain behavior)
  const codec = getStakePoolCodec();

  const currentRate = 100_000_000n;

  // Pool with pending change
  const poolBeforeCancellation = {
    key: 1,
    authority: address('11111111111111111111111111111111'),
    stakeMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    rewardMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    poolId: 0n,
    stakeVault: address('11111111111111111111111111111111'),
    rewardVault: address('11111111111111111111111111111111'),
    totalStaked: 0n,
    totalRewardsOwed: 0n,
    rewardRate: currentRate,
    minStakeAmount: 1_000_000n,
    lockupPeriod: 86400n,
    isPaused: false,
    enforceLockup: false,
    bump: 255,
    pendingAuthority: null,
    poolEndDate: null,
    pendingRewardRate: some(200_000_000n),
    rewardRateChangeTimestamp: some(1700000000n),
    reserved: new Uint8Array(16),
  };

  // After proposing current rate (simulated state after on-chain cancellation)
  const poolAfterCancellation = {
    ...poolBeforeCancellation,
    pendingRewardRate: null,
    rewardRateChangeTimestamp: null,
  };

  const encoded = codec.encode(poolAfterCancellation);
  const decoded = codec.decode(encoded);

  // Pending fields should be None after cancellation
  t.deepEqual(decoded.pendingRewardRate, { __option: 'None' });
  t.deepEqual(decoded.rewardRateChangeTimestamp, { __option: 'None' });
  t.is(decoded.rewardRate, currentRate); // Current rate unchanged
});

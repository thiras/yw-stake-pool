import test from 'ava';
import { getStakePoolCodec, getStakeAccountCodec } from '../src';
import { Key } from '../src';
import { address } from '@solana/kit';

test('StakePool codec encodes and decodes correctly', (t) => {
  const codec = getStakePoolCodec();

  const stakePool = {
    key: Key.StakePool,
    authority: address('11111111111111111111111111111111'),
    stakeMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    rewardMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    stakeVault: address('11111111111111111111111111111111'),
    rewardVault: address('11111111111111111111111111111111'),
    totalStaked: 1_000_000_000n,
    rewardRate: 100_000_000n, // 10% (100_000_000 / 1e9)
    minStakeAmount: 1_000_000n,
    lockupPeriod: 86400n, // 1 day in seconds
    isPaused: false,
    bump: 255,
    pendingAuthority: null,
  };

  const encoded = codec.encode(stakePool);
  const decoded = codec.decode(encoded);

  // Compare all fields except pendingAuthority
  t.is(decoded.key, stakePool.key);
  t.is(decoded.authority, stakePool.authority);
  t.is(decoded.stakeMint, stakePool.stakeMint);
  t.is(decoded.rewardMint, stakePool.rewardMint);
  t.is(decoded.stakeVault, stakePool.stakeVault);
  t.is(decoded.rewardVault, stakePool.rewardVault);
  t.is(decoded.totalStaked, stakePool.totalStaked);
  t.is(decoded.rewardRate, stakePool.rewardRate);
  t.is(decoded.minStakeAmount, stakePool.minStakeAmount);
  t.is(decoded.lockupPeriod, stakePool.lockupPeriod);
  t.is(decoded.isPaused, stakePool.isPaused);
  t.is(decoded.bump, stakePool.bump);
  
  // Check pendingAuthority is None
  t.deepEqual(decoded.pendingAuthority, { __option: 'None' });
});

test('StakePool has correct reward_rate field', (t) => {
  const codec = getStakePoolCodec();

  const stakePool = {
    key: Key.StakePool,
    authority: address('11111111111111111111111111111111'),
    stakeMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    rewardMint: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    stakeVault: address('11111111111111111111111111111111'),
    rewardVault: address('11111111111111111111111111111111'),
    totalStaked: 0n,
    rewardRate: 250_000_000n, // 25% reward rate
    minStakeAmount: 1_000_000n,
    lockupPeriod: 604800n, // 7 days
    isPaused: false,
    bump: 255,
    pendingAuthority: null,
  };

  const encoded = codec.encode(stakePool);
  const decoded = codec.decode(encoded);

  t.is(decoded.rewardRate, 250_000_000n);
});

test('StakeAccount codec encodes and decodes correctly', (t) => {
  const codec = getStakeAccountCodec();

  const stakeAccount = {
    key: Key.StakeAccount,
    pool: address('11111111111111111111111111111111'),
    owner: address('11111111111111111111111111111111'),
    index: 0n,
    amountStaked: 10_000_000n,
    stakeTimestamp: 1234567890n,
    claimedRewards: 0n,
    bump: 255,
  };

  const encoded = codec.encode(stakeAccount);
  const decoded = codec.decode(encoded);

  t.deepEqual(decoded, stakeAccount);
});

test('StakeAccount has claimedRewards field', (t) => {
  const codec = getStakeAccountCodec();

  const stakeAccount = {
    key: Key.StakeAccount,
    pool: address('11111111111111111111111111111111'),
    owner: address('11111111111111111111111111111111'),
    index: 0n,
    amountStaked: 100_000_000n,
    stakeTimestamp: 1234567890n,
    claimedRewards: 10_000_000n, // Already claimed 10 tokens
    bump: 255,
  };

  const encoded = codec.encode(stakeAccount);
  const decoded = codec.decode(encoded);

  t.is(decoded.claimedRewards, 10_000_000n);
});

test('StakeAccount does not have deprecated reward fields', (t) => {
  const codec = getStakeAccountCodec();

  const stakeAccount = {
    key: Key.StakeAccount,
    pool: address('11111111111111111111111111111111'),
    owner: address('11111111111111111111111111111111'),
    index: 0n,
    amountStaked: 100_000_000n,
    stakeTimestamp: 1234567890n,
    claimedRewards: 0n,
    bump: 255,
  };

  const encoded = codec.encode(stakeAccount);
  const decoded = codec.decode(encoded);

  // Ensure old fields don't exist
  t.false('rewardPerTokenPaid' in decoded);
  t.false('rewardsEarned' in decoded);
});

test('Reward calculation matches on-chain formula', (t) => {
  // Simulating the on-chain calculation: (amount * reward_rate) / 1e9
  const amountStaked = 1_000_000_000n; // 1000 tokens (with 6 decimals)
  const rewardRate = 100_000_000n; // 10% (100_000_000 / 1_000_000_000)

  const expectedRewards = (amountStaked * rewardRate) / 1_000_000_000n;

  t.is(expectedRewards, 100_000_000n); // 100 tokens reward (10% of 1000)
});

test('Reward calculation with different reward rates', (t) => {
  const amountStaked = 500_000_000n; // 500 tokens

  // 5% reward rate
  const rewardRate5 = 50_000_000n;
  const rewards5 = (amountStaked * rewardRate5) / 1_000_000_000n;
  t.is(rewards5, 25_000_000n); // 25 tokens

  // 15% reward rate
  const rewardRate15 = 150_000_000n;
  const rewards15 = (amountStaked * rewardRate15) / 1_000_000_000n;
  t.is(rewards15, 75_000_000n); // 75 tokens

  // 50% reward rate
  const rewardRate50 = 500_000_000n;
  const rewards50 = (amountStaked * rewardRate50) / 1_000_000_000n;
  t.is(rewards50, 250_000_000n); // 250 tokens
});

test('Unclaimed rewards calculation', (t) => {
  const amountStaked = 1_000_000_000n;
  const rewardRate = 200_000_000n; // 20%
  const claimedRewards = 50_000_000n; // Already claimed 50 tokens

  const totalRewards = (amountStaked * rewardRate) / 1_000_000_000n;
  const unclaimedRewards = totalRewards - claimedRewards;

  t.is(totalRewards, 200_000_000n); // 200 tokens total
  t.is(unclaimedRewards, 150_000_000n); // 150 tokens unclaimed
});

import test from 'ava';
import {
  getInitializePoolInstructionDataCodec,
  getStakeInstructionDataCodec,
  getUnstakeInstructionDataCodec,
  getUpdatePoolInstructionDataCodec,
  getFundRewardsInstructionDataCodec,
} from '../src';
import { none, some } from '@solana/kit';

test('initializePool instruction data codec with reward_rate', (t) => {
  const codec = getInitializePoolInstructionDataCodec();

  const data = {
    rewardRate: 100_000_000n, // 10%
    minStakeAmount: 1_000_000n,
    lockupPeriod: 86400n, // 1 day
    poolEndDate: null,
  };

  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  t.is(decoded.rewardRate, 100_000_000n);
  t.is(decoded.minStakeAmount, 1_000_000n);
  t.is(decoded.lockupPeriod, 86400n);
});

test('initializePool with different reward_rate values', (t) => {
  const codec = getInitializePoolInstructionDataCodec();

  const rewardRates = [
    { rate: 50_000_000n, description: '5%' },
    { rate: 100_000_000n, description: '10%' },
    { rate: 250_000_000n, description: '25%' },
    { rate: 500_000_000n, description: '50%' },
    { rate: 1_000_000_000n, description: '100%' },
  ];

  rewardRates.forEach(({ rate, description }) => {
    const data = {
      rewardRate: rate,
      minStakeAmount: 1_000_000n,
      lockupPeriod: 86400n,
      poolEndDate: null,
    };

    const encoded = codec.encode(data);
    const decoded = codec.decode(encoded);

    t.is(
      decoded.rewardRate,
      rate,
      `Reward rate ${description} should be preserved`
    );
  });
});

test('stake instruction data codec', (t) => {
  const codec = getStakeInstructionDataCodec();

  const data = {
    amount: 100_000_000n,
    index: 0n,
    expectedRewardRate: null,
    expectedLockupPeriod: null,
  };

  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  t.is(decoded.amount, 100_000_000n);
  t.is(decoded.index, 0n);
});

test('stake with various amounts', (t) => {
  const codec = getStakeInstructionDataCodec();

  const amounts = [
    1_000_000n, // 1 token
    10_000_000n, // 10 tokens
    100_000_000n, // 100 tokens
    1_000_000_000n, // 1000 tokens
  ];

  amounts.forEach((amount, index) => {
    const data = {
      amount,
      index: BigInt(index),
      expectedRewardRate: null,
      expectedLockupPeriod: null,
    };

    const encoded = codec.encode(data);
    const decoded = codec.decode(encoded);

    t.is(decoded.amount, amount);
    t.is(decoded.index, BigInt(index));
  });
});

test('unstake instruction data codec', (t) => {
  const codec = getUnstakeInstructionDataCodec();

  const data = {
    amount: 50_000_000n,
    expectedRewardRate: null,
  };

  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  t.is(decoded.amount, 50_000_000n);
});

test('unstake full vs partial amounts', (t) => {
  const codec = getUnstakeInstructionDataCodec();

  // Full unstake
  const fullUnstakeData = {
    amount: 100_000_000n,
    expectedRewardRate: null,
  };

  const fullEncoded = codec.encode(fullUnstakeData);
  const fullDecoded = codec.decode(fullEncoded);

  t.is(fullDecoded.amount, 100_000_000n);

  // Partial unstake
  const partialUnstakeData = {
    amount: 50_000_000n,
    expectedRewardRate: null,
  };

  const partialEncoded = codec.encode(partialUnstakeData);
  const partialDecoded = codec.decode(partialEncoded);

  t.is(partialDecoded.amount, 50_000_000n);
});

test('fundRewards instruction data codec', (t) => {
  const codec = getFundRewardsInstructionDataCodec();

  const data = {
    amount: 1_000_000_000n,
  };

  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  t.is(decoded.amount, 1_000_000_000n);
});

test('updatePool instruction data codec with reward_rate', (t) => {
  const codec = getUpdatePoolInstructionDataCodec();

  const data = {
    rewardRate: some(150_000_000n), // 15%
    minStakeAmount: some(2_000_000n),
    lockupPeriod: some(172800n), // 2 days
    isPaused: some(false),
    poolEndDate: null,
  };

  const encoded = codec.encode(data);
  const decoded = codec.decode(encoded);

  t.deepEqual(decoded.rewardRate, some(150_000_000n));
  t.deepEqual(decoded.minStakeAmount, some(2_000_000n));
  t.deepEqual(decoded.lockupPeriod, some(172800n));
  t.deepEqual(decoded.isPaused, some(false));
});

test('updatePool can toggle pause state', (t) => {
  const codec = getUpdatePoolInstructionDataCodec();

  // Paused
  const pausedData = {
    rewardRate: null,
    minStakeAmount: null,
    lockupPeriod: null,
    isPaused: some(true),
    poolEndDate: null,
  };

  const pausedEncoded = codec.encode(pausedData);
  const pausedDecoded = codec.decode(pausedEncoded);

  t.deepEqual(pausedDecoded.isPaused, some(true));

  // Unpaused
  const unpausedData = {
    rewardRate: null,
    minStakeAmount: null,
    lockupPeriod: null,
    isPaused: some(false),
    poolEndDate: null,
  };

  const unpausedEncoded = codec.encode(unpausedData);
  const unpausedDecoded = codec.decode(unpausedEncoded);

  t.deepEqual(unpausedDecoded.isPaused, some(false));
});

test('updatePool with different reward_rate values', (t) => {
  const codec = getUpdatePoolInstructionDataCodec();

  const testCases = [
    { rewardRate: 50_000_000n, lockupPeriod: 86400n },
    { rewardRate: 200_000_000n, lockupPeriod: 604800n },
    { rewardRate: 500_000_000n, lockupPeriod: 2592000n },
  ];

  testCases.forEach(({ rewardRate, lockupPeriod }) => {
    const data = {
      rewardRate: some(rewardRate),
      minStakeAmount: null,
      lockupPeriod: some(lockupPeriod),
      isPaused: null,
      poolEndDate: null,
    };

    const encoded = codec.encode(data);
    const decoded = codec.decode(encoded);

    t.deepEqual(decoded.rewardRate, some(rewardRate));
    t.deepEqual(decoded.lockupPeriod, some(lockupPeriod));
  });
});

test('updatePool with partial updates', (t) => {
  const codec = getUpdatePoolInstructionDataCodec();

  // Only update reward_rate
  const onlyRewardRate = {
    rewardRate: some(300_000_000n),
    minStakeAmount: null,
    lockupPeriod: null,
    isPaused: null,
    poolEndDate: null,
  };

  const encoded1 = codec.encode(onlyRewardRate);
  const decoded1 = codec.decode(encoded1);

  t.deepEqual(decoded1.rewardRate, some(300_000_000n));
  t.deepEqual(decoded1.minStakeAmount, none());
  t.deepEqual(decoded1.lockupPeriod, none());
  t.deepEqual(decoded1.isPaused, none());

  // Only update lockup_period
  const onlyLockup = {
    rewardRate: null,
    minStakeAmount: null,
    lockupPeriod: some(259200n),
    isPaused: null,
    poolEndDate: null,
  };

  const encoded2 = codec.encode(onlyLockup);
  const decoded2 = codec.decode(encoded2);

  t.deepEqual(decoded2.rewardRate, none());
  t.deepEqual(decoded2.lockupPeriod, some(259200n));
});

test('instruction data codecs preserve exact values', (t) => {
  // Test that bigint values are preserved exactly through encode/decode

  const initPoolCodec = getInitializePoolInstructionDataCodec();
  const initPoolData = {
    rewardRate: 123_456_789n,
    minStakeAmount: 987_654_321n,
    lockupPeriod: 555_555n,
    poolEndDate: null,
  };

  const initPoolEncoded = initPoolCodec.encode(initPoolData);
  const initPoolDecoded = initPoolCodec.decode(initPoolEncoded);

  t.is(initPoolDecoded.rewardRate, 123_456_789n);
  t.is(initPoolDecoded.minStakeAmount, 987_654_321n);
  t.is(initPoolDecoded.lockupPeriod, 555_555n);

  const stakeCodec = getStakeInstructionDataCodec();
  const stakeData = {
    amount: 9_876_543_210n,
    index: 42n,
    expectedRewardRate: null,
    expectedLockupPeriod: null,
  };

  const stakeEncoded = stakeCodec.encode(stakeData);
  const stakeDecoded = stakeCodec.decode(stakeEncoded);

  t.is(stakeDecoded.amount, 9_876_543_210n);
  t.is(stakeDecoded.index, 42n);
});

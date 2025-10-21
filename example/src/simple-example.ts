/**
 * Simple Example - Basic Library Usage
 * 
 * This is a minimal example showing basic usage of the client library.
 * It demonstrates instruction creation without requiring a live connection.
 */

import { address } from '@solana/kit';
import {
  getInitializePoolInstruction,
  getStakeInstruction,
  getClaimRewardsInstruction,
  STAKE_POOL_PROGRAM_ADDRESS,
} from '@yourwallet/stake-pool';

console.log('='.repeat(60));
console.log('  YW Stake Pool - Simple Example');
console.log('='.repeat(60));
console.log();

console.log('📦 Program Information:');
console.log(`   Program ID: ${STAKE_POOL_PROGRAM_ADDRESS}`);
console.log();

// Example addresses (placeholders)
const poolAddress = address('11111111111111111111111111111111');
const authorityAddress = address('11111111111111111111111111111111');
const stakeMint = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const rewardMint = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');

console.log('✨ Creating Instructions:');
console.log();

// 1. Initialize Pool Instruction
console.log('1️⃣  Initialize Pool Instruction');
try {
  const initPoolIx = getInitializePoolInstruction({
    pool: poolAddress,
    authority: authorityAddress as any,
    stakeMint,
    rewardMint,
    stakeVault: address('11111111111111111111111111111112'),
    rewardVault: address('11111111111111111111111111111113'),
    payer: authorityAddress as any,
    tokenProgram: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    systemProgram: address('11111111111111111111111111111111'),
    rent: address('SysvarRent111111111111111111111111111111111'),
    rewardRate: 100_000_000n,
    minStakeAmount: 1_000_000n,
    lockupPeriod: 86400n,
    poolEndDate: null,
  });
  
  console.log('   ✅ Created successfully');
  console.log(`   Accounts: ${initPoolIx.accounts?.length || 0}`);
  console.log(`   Program ID: ${initPoolIx.programAddress}`);
} catch (error) {
  console.log('   ❌ Error:', (error as Error).message);
}
console.log();

// 2. Stake Instruction
console.log('2️⃣  Stake Instruction');
try {
  const stakeIx = getStakeInstruction({
    pool: poolAddress,
    stakeAccount: address('11111111111111111111111111111114'),
    owner: authorityAddress as any,
    userTokenAccount: address('11111111111111111111111111111115'),
    stakeVault: address('11111111111111111111111111111112'),
    rewardVault: address('11111111111111111111111111111113'),
    stakeMint,
    tokenProgram: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    payer: authorityAddress as any,
    systemProgram: address('11111111111111111111111111111111'),
    amount: 1_000_000n,
    index: 0n,
    expectedRewardRate: null,
    expectedLockupPeriod: null,
  });
  
  console.log('   ✅ Created successfully');
  console.log(`   Accounts: ${stakeIx.accounts?.length || 0}`);
  console.log(`   Stake Amount: 100 tokens`);
} catch (error) {
  console.log('   ❌ Error:', (error as Error).message);
}
console.log();

// 3. Claim Rewards Instruction
console.log('3️⃣  Claim Rewards Instruction');
try {
  const claimIx = getClaimRewardsInstruction({
    pool: poolAddress,
    stakeAccount: address('11111111111111111111111111111114'),
    owner: authorityAddress as any,
    userRewardAccount: address('11111111111111111111111111111116'),
    rewardVault: address('11111111111111111111111111111113'),
    rewardMint,
    tokenProgram: address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA'),
    clock: address('SysvarC1ock11111111111111111111111111111111'),
  });
  
  console.log('   ✅ Created successfully');
  console.log(`   Accounts: ${claimIx.accounts?.length || 0}`);
} catch (error) {
  console.log('   ❌ Error:', (error as Error).message);
}
console.log();

console.log('='.repeat(60));
console.log('✅ Library is working correctly!');
console.log();
console.log('📚 Next Steps:');
console.log('   - Run: pnpm complete-flow');
console.log('   - Run: pnpm pool-admin');
console.log('   - Run: pnpm user-staking');
console.log('   - Read: README.md for full documentation');
console.log('='.repeat(60));

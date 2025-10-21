/**
 * User Staking Example
 * 
 * Demonstrates typical user staking operations:
 * - Initialize stake account
 * - Stake tokens
 * - Check stake status
 * - Claim rewards
 * - Partial and full unstaking
 */

import { address } from '@solana/kit';
import {
  getInitializeStakeAccountInstruction,
  getStakeInstruction,
  getClaimRewardsInstruction,
  getUnstakeInstruction,
} from '@yourwallet/stake-pool';

import { config, formatAmount, formatRewardRate } from './config.js';
import {
  createRpc,
  createFundedKeypair,
  findPoolPda,
  findStakeAccountPda,
  logSection,
  logStep,
  waitForRateLimit,
  handleError,
  calculateRewards,
  sleep,
} from './utils.js';

async function main() {
  try {
    logSection('User Staking Examples');

    const rpc = createRpc();
    console.log(`🌐 Connected to: ${config.rpcUrl}\n`);

    // Create user keypair (uses local keypair by default)
    const user = await createFundedKeypair(rpc, 'User', config.useLocalKeypair);
    await waitForRateLimit();

    // Assume pool already exists (created by admin)
    // Generate a separate authority keypair for pool reference
    const poolAuthority = await createFundedKeypair(rpc, 'Pool Authority', false);
    const stakeMint = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
    const rewardMint = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
    
    const [poolAddress] = await findPoolPda(poolAuthority.address, stakeMint);
    console.log(`📍 Using Pool: ${poolAddress}`);

    // Token accounts (placeholders)
    const userStakeTokenAccount = address('11111111111111111111111111111114');
    const userRewardTokenAccount = address('11111111111111111111111111111115');
    const stakeVault = address('11111111111111111111111111111112');
    const rewardVault = address('11111111111111111111111111111113');

    // ========================================================================
    // Example 1: Initialize Stake Account
    // ========================================================================
    logStep(1, 'Initialize Stake Account');

    const stakeIndex = 0n; // First stake account for this user
    const [stakeAccountAddress] = await findStakeAccountPda(
      poolAddress,
      user.address,
      stakeIndex
    );

    console.log(`📍 Stake Account PDA: ${stakeAccountAddress}`);
    console.log(`   Index: ${stakeIndex}`);

    const initStakeIx = getInitializeStakeAccountInstruction({
      stakeAccount: stakeAccountAddress,
      pool: poolAddress,
      owner: user,
      payer: user,
      systemProgram: config.systemProgramId,
      index: stakeIndex,
    });

    console.log('✅ Initialize stake account instruction created');
    console.log('   💡 Users can have multiple stake accounts with different indices');
    await waitForRateLimit();

    // ========================================================================
    // Example 2: Stake Tokens
    // ========================================================================
    logStep(2, 'Stake Tokens');

    const stakeAmount = 100_000_000n; // 100 tokens
    console.log(`🔒 Staking ${formatAmount(stakeAmount)} tokens`);

    const stakeIx = getStakeInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress,
      owner: user,
      userTokenAccount: userStakeTokenAccount,
      stakeVault,
      rewardVault,
      tokenProgram: config.tokenProgramId,
      payer: user,
      systemProgram: config.systemProgramId,
      amount: stakeAmount,
      index: stakeIndex,
      expectedRewardRate: null,
      expectedLockupPeriod: null,
    });

    console.log('✅ Stake instruction created');
    
    // Calculate expected rewards
    const rewardRate = config.defaultPoolConfig.rewardRate;
    const expectedRewards = calculateRewards(stakeAmount, rewardRate, true);
    
    console.log(`\n📊 Stake Details:`);
    console.log(`   Amount: ${formatAmount(stakeAmount)} tokens`);
    console.log(`   Reward Rate: ${formatRewardRate(rewardRate)}`);
    console.log(`   Expected Rewards: ${formatAmount(expectedRewards)} tokens`);
    await waitForRateLimit();

    // ========================================================================
    // Example 3: Stake with Frontrunning Protection
    // ========================================================================
    logStep(3, 'Stake with Frontrunning Protection');

    console.log('🛡️  Using expected parameters to prevent frontrunning');
    console.log('   If pool params change before tx lands, it will fail');

    const safeStakeIx = getStakeInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress,
      owner: user,
      userTokenAccount: userStakeTokenAccount,
      stakeVault,
      rewardVault,
      tokenProgram: config.tokenProgramId,
      payer: user,
      systemProgram: config.systemProgramId,
      amount: stakeAmount,
      index: stakeIndex,
      expectedRewardRate: rewardRate, // Verify rate hasn't changed
      expectedLockupPeriod: config.defaultPoolConfig.lockupPeriod, // Verify lockup hasn't changed
    });

    console.log('✅ Protected stake instruction created');
    console.log('   ✓ Expected reward rate will be verified');
    console.log('   ✓ Expected lockup period will be verified');
    await waitForRateLimit();

    // ========================================================================
    // Example 4: Wait for Lockup & Claim Rewards
    // ========================================================================
    logStep(4, 'Wait for Lockup & Claim Rewards');

    console.log('⏳ In production, wait for lockup period to pass');
    console.log(`   Lockup: ${config.defaultPoolConfig.lockupPeriod}s`);
    await sleep(2000); // Simulate wait

    console.log('\n💎 Claiming rewards...');

    const claimIx = getClaimRewardsInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress,
      owner: user,
      userRewardAccount: userRewardTokenAccount,
      rewardVault,
      tokenProgram: config.tokenProgramId,
      clock: address('SysvarC1ock11111111111111111111111111111111'),
    });

    console.log('✅ Claim rewards instruction created');
    console.log(`   Expected rewards: ${formatAmount(expectedRewards)} tokens`);
    await waitForRateLimit();

    // ========================================================================
    // Example 5: Partial Unstake
    // ========================================================================
    logStep(5, 'Partial Unstake');

    const partialAmount = 40_000_000n; // 40 tokens
    console.log(`🔓 Partial unstake: ${formatAmount(partialAmount)} tokens`);
    console.log(`   Remaining staked: ${formatAmount(stakeAmount - partialAmount)} tokens`);

    const partialUnstakeIx = getUnstakeInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress,
      owner: user,
      userTokenAccount: userStakeTokenAccount,
      stakeVault,
      tokenProgram: config.tokenProgramId,
      clock: address('SysvarC1ock11111111111111111111111111111111'),
      amount: partialAmount,
      expectedRewardRate: null,
    });

    console.log('✅ Partial unstake instruction created');
    await waitForRateLimit();

    // ========================================================================
    // Example 6: Full Unstake
    // ========================================================================
    logStep(6, 'Full Unstake');

    const remainingAmount = stakeAmount - partialAmount;
    console.log(`🔓 Full unstake: ${formatAmount(remainingAmount)} tokens`);

    const fullUnstakeIx = getUnstakeInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress,
      owner: user,
      userTokenAccount: userStakeTokenAccount,
      stakeVault,
      tokenProgram: config.tokenProgramId,
      clock: address('SysvarC1ock11111111111111111111111111111111'),
      amount: remainingAmount,
      expectedRewardRate: null,
    });

    console.log('✅ Full unstake instruction created');
    console.log('   After this, stake account will have 0 tokens staked');
    await waitForRateLimit();

    // ========================================================================
    // Example 7: Multiple Stake Accounts
    // ========================================================================
    logStep(7, 'Multiple Stake Accounts (Advanced)');

    console.log('💡 Users can create multiple stake accounts with different indices');
    console.log('   This allows for separate stakes with different strategies\n');

    // Second stake account
    const stakeIndex2 = 1n;
    const [stakeAccountAddress2] = await findStakeAccountPda(
      poolAddress,
      user.address,
      stakeIndex2
    );

    console.log(`📍 Stake Account #2: ${stakeAccountAddress2}`);

    const initStake2Ix = getInitializeStakeAccountInstruction({
      stakeAccount: stakeAccountAddress2,
      pool: poolAddress,
      owner: user,
      payer: user,
      systemProgram: config.systemProgramId,
      index: stakeIndex2,
    });

    console.log('✅ Second stake account instruction created');

    // Stake in second account
    const stake2Amount = 200_000_000n; // 200 tokens
    const stake2Ix = getStakeInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress2,
      owner: user,
      userTokenAccount: userStakeTokenAccount,
      stakeVault,
      rewardVault,
      tokenProgram: config.tokenProgramId,
      payer: user,
      systemProgram: config.systemProgramId,
      amount: stake2Amount,
      index: stakeIndex2,
      expectedRewardRate: null,
      expectedLockupPeriod: null,
    });

    console.log(`✅ Stake to second account: ${formatAmount(stake2Amount)} tokens`);

    // ========================================================================
    // Summary
    // ========================================================================
    logSection('User Staking Examples Complete!');

    console.log('📋 Operations Demonstrated:');
    console.log('   1. ✅ Initialize stake account');
    console.log('   2. ✅ Stake tokens (basic)');
    console.log('   3. ✅ Stake with frontrunning protection');
    console.log('   4. ✅ Claim rewards after lockup');
    console.log('   5. ✅ Partial unstake');
    console.log('   6. ✅ Full unstake');
    console.log('   7. ✅ Multiple stake accounts\n');

    console.log('💡 Tips for Users:');
    console.log('   - Wait for lockup period before claiming rewards');
    console.log('   - Use frontrunning protection for added security');
    console.log('   - Partial unstake lets you access some funds early');
    console.log('   - Multiple stake accounts enable diverse strategies');
    console.log('   - Claim rewards regularly to compound earnings\n');

    console.log('⚠️  Important Notes:');
    console.log('   - Rewards accrue after lockup period');
    console.log('   - Early unstaking may forfeit unclaimed rewards');
    console.log('   - Check pool status before staking');
    console.log('   - Ensure pool has sufficient reward funds\n');

  } catch (error) {
    handleError(error, 'User Staking');
  }
}

main();

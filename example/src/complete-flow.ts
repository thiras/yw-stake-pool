/**
 * Complete Stake Pool Flow Example
 * 
 * This example demonstrates the entire lifecycle of a stake pool:
 * 1. Setup - Create test tokens and fund accounts
 * 2. Pool initialization
 * 3. User staking
 * 4. Reward claiming
 * 5. Unstaking
 * 6. Pool management (updates, authority transfer)
 */

import { address } from '@solana/kit';
import {
  getInitializePoolInstruction,
  getInitializeStakeAccountInstruction,
  getStakeInstruction,
  getClaimRewardsInstruction,
  getUnstakeInstruction,
  getFundRewardsInstruction,
  getUpdatePoolInstruction,
  getNominateNewAuthorityInstruction,
  getAcceptAuthorityInstruction,
} from '@yourwallet/stake-pool';
import { some, none } from '@solana/kit';

import { config, formatAmount, formatRewardRate, formatDuration } from './config.js';
import {
  createRpc,
  createFundedKeypair,
  findPoolPda,
  findStakeAccountPda,
  buildAndSendTransaction,
  logSection,
  logStep,
  logTransaction,
  sleep,
  handleError,
  calculateRewards,
} from './utils.js';

/**
 * Main function to run the complete flow
 */
async function main() {
  try {
    logSection('YW Stake Pool - Complete Flow Example');

    // Initialize RPC client
    const rpc = createRpc();
    console.log(`🌐 Connected to: ${config.rpcUrl}\n`);

    // ========================================================================
    // Step 1: Setup - Create keypairs and test tokens
    // ========================================================================
    logStep(1, 'Setup - Create Keypairs and Test Tokens');

    // Create authority keypair (pool operator)
    const authority = await createFundedKeypair(rpc, 'Authority');

    // Create user keypair (staker)
    const user = await createFundedKeypair(rpc, 'User');

    // For this example, we'll use placeholder addresses for mints and token accounts
    // In a real scenario, you would create actual SPL tokens
    console.log('\n📝 Note: Using placeholder addresses for tokens');
    console.log('   In production, create actual SPL tokens using @solana/spl-token');

    const stakeMint = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
    const rewardMint = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
    const stakeVault = address('11111111111111111111111111111112');
    const rewardVault = address('11111111111111111111111111111113');
    const userStakeAccount = address('11111111111111111111111111111114');
    const userRewardAccount = address('11111111111111111111111111111115');

    // ========================================================================
    // Step 2: Initialize Stake Pool
    // ========================================================================
    logStep(2, 'Initialize Stake Pool');

    console.log('Pool Configuration:');
    console.log(`  Reward Rate: ${formatRewardRate(config.defaultPoolConfig.rewardRate)}`);
    console.log(`  Min Stake: ${formatAmount(config.defaultPoolConfig.minStakeAmount)} tokens`);
    console.log(`  Lockup Period: ${formatDuration(config.defaultPoolConfig.lockupPeriod)}`);

    // Derive pool PDA
    const [poolAddress] = await findPoolPda(authority.address, stakeMint);
    console.log(`\n📍 Pool Address (PDA): ${poolAddress}`);

    // Create initialize pool instruction
    const initPoolIx = getInitializePoolInstruction({
      pool: poolAddress,
      authority: authority,
      stakeMint,
      rewardMint,
      stakeVault,
      rewardVault,
      payer: authority,
      tokenProgram: config.tokenProgramId,
      systemProgram: config.systemProgramId,
      rent: address('SysvarRent111111111111111111111111111111111'),
      rewardRate: config.defaultPoolConfig.rewardRate,
      minStakeAmount: config.defaultPoolConfig.minStakeAmount,
      lockupPeriod: config.defaultPoolConfig.lockupPeriod,
      poolEndDate: null,
    });

    console.log('\n📝 Note: Transaction simulation - actual on-chain execution requires proper setup');
    console.log('   Instructions created successfully!');

    // ========================================================================
    // Step 3: Fund Reward Vault
    // ========================================================================
    logStep(3, 'Fund Reward Vault');

    const fundAmount = config.exampleAmounts.fund;
    console.log(`💰 Funding reward vault with ${formatAmount(fundAmount)} tokens`);

    const fundIx = getFundRewardsInstruction({
      pool: poolAddress,
      funder: authority,
      funderTokenAccount: userRewardAccount,
      rewardVault,
      tokenProgram: config.tokenProgramId,
      amount: fundAmount,
    });

    console.log('✅ Fund rewards instruction created');

    // ========================================================================
    // Step 4: Initialize User Stake Account
    // ========================================================================
    logStep(4, 'Initialize User Stake Account');

    const stakeIndex = 0n;
    const [stakeAccountAddress] = await findStakeAccountPda(
      poolAddress,
      user.address,
      stakeIndex
    );

    console.log(`📍 Stake Account Address (PDA): ${stakeAccountAddress}`);

    const initStakeAccountIx = getInitializeStakeAccountInstruction({
      stakeAccount: stakeAccountAddress,
      pool: poolAddress,
      owner: user,
      payer: user,
      systemProgram: config.systemProgramId,
      index: stakeIndex,
    });

    console.log('✅ Initialize stake account instruction created');

    // ========================================================================
    // Step 5: Stake Tokens
    // ========================================================================
    logStep(5, 'Stake Tokens');

    const stakeAmount = config.exampleAmounts.stake;
    console.log(`🔒 Staking ${formatAmount(stakeAmount)} tokens`);

    const stakeIx = getStakeInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress,
      owner: user,
      userTokenAccount: userStakeAccount,
      stakeVault,
      rewardVault,
      tokenProgram: config.tokenProgramId,
      payer: user,
      systemProgram: config.systemProgramId,
      amount: stakeAmount,
      index: stakeIndex,
      expectedRewardRate: null, // Optional: set to prevent frontrunning
      expectedLockupPeriod: null, // Optional: set to prevent frontrunning
    });

    console.log('✅ Stake instruction created');

    // Calculate expected rewards
    const expectedRewards = calculateRewards(
      stakeAmount,
      config.defaultPoolConfig.rewardRate,
      true
    );
    console.log(`\n📊 Expected rewards after lockup: ${formatAmount(expectedRewards)} tokens`);

    // ========================================================================
    // Step 6: Wait for Lockup Period (simulated)
    // ========================================================================
    logStep(6, 'Wait for Lockup Period');

    console.log(`⏳ Lockup period: ${formatDuration(config.defaultPoolConfig.lockupPeriod)}`);
    console.log('   (In this example, we simulate the wait)');
    console.log('   In production, users must wait before claiming rewards');

    // Simulate wait (in real scenario, this would be actual time passage)
    await sleep(2000);

    // ========================================================================
    // Step 7: Claim Rewards
    // ========================================================================
    logStep(7, 'Claim Rewards');

    console.log('💎 Claiming accrued rewards');

    const claimIx = getClaimRewardsInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress,
      owner: user,
      userRewardAccount,
      rewardVault,
      tokenProgram: config.tokenProgramId,
      clock: address('SysvarC1ock11111111111111111111111111111111'),
    });

    console.log('✅ Claim rewards instruction created');
    console.log(`   User should receive ~${formatAmount(expectedRewards)} tokens`);

    // ========================================================================
    // Step 8: Partial Unstake
    // ========================================================================
    logStep(8, 'Partial Unstake');

    const unstakeAmount = config.exampleAmounts.unstake;
    console.log(`🔓 Unstaking ${formatAmount(unstakeAmount)} tokens (partial)`);

    const unstakeIx = getUnstakeInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress,
      owner: user,
      userTokenAccount: userStakeAccount,
      stakeVault,
      tokenProgram: config.tokenProgramId,
      clock: address('SysvarC1ock11111111111111111111111111111111'),
      amount: unstakeAmount,
      expectedRewardRate: null, // Optional: set to prevent frontrunning
    });

    console.log('✅ Unstake instruction created');
    console.log(`   Remaining staked: ${formatAmount(stakeAmount - unstakeAmount)} tokens`);

    // ========================================================================
    // Step 9: Update Pool Parameters
    // ========================================================================
    logStep(9, 'Update Pool Parameters');

    const newRewardRate = 150_000_000n; // 15%
    console.log(`⚙️  Updating pool reward rate to ${formatRewardRate(newRewardRate)}`);

    const updatePoolIx = getUpdatePoolInstruction({
      pool: poolAddress,
      authority: authority,
      rewardRate: some(newRewardRate),
      minStakeAmount: none(), // Don't change
      lockupPeriod: none(), // Don't change
      isPaused: none(), // Don't change
      poolEndDate: null, // Don't change
    });

    console.log('✅ Update pool instruction created');

    // ========================================================================
    // Step 10: Transfer Pool Authority (Two-Step)
    // ========================================================================
    logStep(10, 'Transfer Pool Authority (Two-Step Process)');

    const newAuthority = await createFundedKeypair(rpc, 'New Authority');

    console.log('Step 10a: Nominate new authority');
    const nominateIx = getNominateNewAuthorityInstruction({
      pool: poolAddress,
      currentAuthority: authority,
      newAuthority: newAuthority.address,
    });
    console.log('✅ Nominate authority instruction created');

    console.log('\nStep 10b: Accept authority (must be called by new authority)');
    const acceptIx = getAcceptAuthorityInstruction({
      pool: poolAddress,
      pendingAuthority: newAuthority,
    });
    console.log('✅ Accept authority instruction created');

    // ========================================================================
    // Summary
    // ========================================================================
    logSection('Example Complete!');

    console.log('✨ All instructions created successfully!\n');
    console.log('📋 Summary of Operations:');
    console.log('   1. ✅ Initialized stake pool');
    console.log('   2. ✅ Funded reward vault');
    console.log('   3. ✅ Created user stake account');
    console.log('   4. ✅ Staked tokens');
    console.log('   5. ✅ Claimed rewards');
    console.log('   6. ✅ Partial unstake');
    console.log('   7. ✅ Updated pool parameters');
    console.log('   8. ✅ Transferred authority\n');

    console.log('📝 Next Steps:');
    console.log('   - Deploy the program to devnet or mainnet');
    console.log('   - Create actual SPL tokens for staking and rewards');
    console.log('   - Send these instructions on-chain');
    console.log('   - Integrate with your frontend application\n');

    console.log('📚 Resources:');
    console.log('   - Program README: ../README.md');
    console.log('   - Client Docs: ../clients/js/README.md');
    console.log('   - Security Audit: ../SECURITY_AUDIT.md\n');

  } catch (error) {
    handleError(error, 'Complete Flow');
  }
}

// Run the example
main();

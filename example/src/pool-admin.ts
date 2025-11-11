/**
 * Pool Administration Example
 * 
 * Demonstrates pool operator/admin operations:
 * - Initialize pools
 * - Update pool settings
 * - Pause/unpause
 * - Fund rewards
 * - Transfer authority
 */

import { address, some, none } from '@solana/kit';
import {
  getInitializePoolInstruction,
  getUpdatePoolInstruction,
  getFundRewardsInstruction,
  getNominateNewAuthorityInstruction,
} from '@yourwallet/stake-pool';

import { config, formatAmount, formatRewardRate, formatDuration } from './config.js';
import {
  createRpc,
  createFundedKeypair,
  findPoolPda,
  logSection,
  logStep,
  handleError,
} from './utils.js';

async function main() {
  try {
    logSection('Pool Administration Examples');

    const rpc = createRpc();
    console.log(`🌐 Connected to: ${config.rpcUrl}\n`);

    // Create admin keypair (uses local keypair by default)
    const admin = await createFundedKeypair(rpc, 'Pool Admin', config.useLocalKeypair);

    // Placeholder addresses (in production, use real SPL tokens)
    const stakeMint = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
    const rewardMint = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
    const stakeVault = address('11111111111111111111111111111112');
    const rewardVault = address('11111111111111111111111111111113');

    // ========================================================================
    // Example 1: Initialize Pool
    // ========================================================================
    logStep(1, 'Initialize New Stake Pool');

    const [poolAddress] = await findPoolPda(stakeMint);
    console.log(`📍 Pool PDA: ${poolAddress}`);

    const initIx = getInitializePoolInstruction({
      pool: poolAddress,
      authority: admin,
      stakeMint,
      rewardMint,
      stakeVault,
      rewardVault,
      payer: admin,
      tokenProgram: config.tokenProgramId,
      systemProgram: config.systemProgramId,
      rent: address('SysvarRent111111111111111111111111111111111'),
      poolId: 0n, // First pool for this authority + stake_mint
      rewardRate: config.defaultPoolConfig.rewardRate,
      minStakeAmount: config.defaultPoolConfig.minStakeAmount,
      lockupPeriod: config.defaultPoolConfig.lockupPeriod,
      enforceLockup: false,
      poolEndDate: null,
    });

    console.log('✅ Initialize pool instruction created');
    console.log(`   Reward Rate: ${formatRewardRate(config.defaultPoolConfig.rewardRate)}`);
    console.log(`   Min Stake: ${formatAmount(config.defaultPoolConfig.minStakeAmount)}`);
    console.log(`   Lockup: ${formatDuration(config.defaultPoolConfig.lockupPeriod)}`);

    // ========================================================================
    // Example 2: Fund Reward Vault
    // ========================================================================
    logStep(2, 'Fund Reward Vault');

    const fundAmount = 10_000_000_000n; // 10,000 tokens
    console.log(`💰 Funding with ${formatAmount(fundAmount)} reward tokens`);

    const fundIx = getFundRewardsInstruction({
      pool: poolAddress,
      funder: admin,
      funderTokenAccount: rewardVault,
      rewardVault,
      rewardMint,
      tokenProgram: config.tokenProgramId,
      amount: fundAmount,
    });

    console.log('✅ Fund rewards instruction created');

    // ========================================================================
    // Example 3: Update Pool - Change Reward Rate
    // ========================================================================
    logStep(3, 'Update Pool - Change Reward Rate');

    const newRate = 200_000_000n; // 20%
    console.log(`⚙️  New reward rate: ${formatRewardRate(newRate)}`);

    const updateRateIx = getUpdatePoolInstruction({
      pool: poolAddress,
      authority: admin,
      rewardRate: some(newRate),
      minStakeAmount: none(),
      lockupPeriod: none(),
      isPaused: none(),
      enforceLockup: none(),
      poolEndDate: null,
    });

    console.log('✅ Update reward rate instruction created');

    // ========================================================================
    // Example 4: Update Pool - Pause
    // ========================================================================
    logStep(4, 'Update Pool - Pause');

    console.log('⏸️  Pausing pool (no new stakes allowed)');

    const pauseIx = getUpdatePoolInstruction({
      pool: poolAddress,
      authority: admin,
      rewardRate: none(),
      minStakeAmount: none(),
      lockupPeriod: none(),
      isPaused: some(true),
      enforceLockup: none(),
      poolEndDate: null,
    });

    console.log('✅ Pause pool instruction created');

    // ========================================================================
    // Example 5: Update Pool - Multiple Parameters
    // ========================================================================
    logStep(5, 'Update Pool - Multiple Parameters');

    console.log('⚙️  Updating multiple settings at once:');
    console.log('   - Unpause pool');
    console.log('   - Set minimum stake to 5 tokens');
    console.log('   - Extend lockup to 7 days');

    const multiUpdateIx = getUpdatePoolInstruction({
      pool: poolAddress,
      authority: admin,
      rewardRate: none(),
      minStakeAmount: some(5_000_000n), // 5 tokens
      lockupPeriod: some(604800n), // 7 days
      isPaused: some(false),
      enforceLockup: none(),
      poolEndDate: null,
    });

    console.log('✅ Multi-parameter update instruction created');

    // ========================================================================
    // Example 6: Set Pool End Date
    // ========================================================================
    logStep(6, 'Set Pool End Date');

    // Set pool to end in 30 days
    const endDate = BigInt(Math.floor(Date.now() / 1000) + 30 * 24 * 60 * 60);
    console.log(`📅 Setting pool end date: ${new Date(Number(endDate) * 1000).toLocaleString()}`);

    const setEndDateIx = getUpdatePoolInstruction({
      pool: poolAddress,
      authority: admin,
      rewardRate: none(),
      minStakeAmount: none(),
      lockupPeriod: none(),
      isPaused: none(),
      enforceLockup: none(),
      poolEndDate: some(some(endDate)),
    });

    console.log('✅ Set end date instruction created');

    // ========================================================================
    // Example 7: Transfer Authority (Two-Step)
    // ========================================================================
    logStep(7, 'Transfer Authority (Two-Step Process)');

    console.log('⚠️  Note: Authority transfer requires a funded new authority account');
    console.log('   Skipping keypair generation to avoid devnet airdrop issues');
    console.log('   In production, generate a new keypair and fund it before transfer\n');

    // Using a placeholder address for demonstration
    const newAdminAddress = address('11111111111111111111111111111119');

    console.log('\n7a. Current admin nominates new authority');
    const nominateIx = getNominateNewAuthorityInstruction({
      pool: poolAddress,
      currentAuthority: admin,
      newAuthority: newAdminAddress,
    });
    console.log('✅ Nominate instruction created');
    console.log(`   New authority: ${newAdminAddress}`);

    console.log('\n7b. New authority accepts the nomination');
    console.log('   (In production, this would be signed by the new authority keypair)');
    console.log('   🔐 Two-step process prevents accidental authority loss');

    // ========================================================================
    // Summary
    // ========================================================================
    logSection('Pool Admin Examples Complete!');

    console.log('📋 Operations Demonstrated:');
    console.log('   1. ✅ Initialize new pool');
    console.log('   2. ✅ Fund reward vault');
    console.log('   3. ✅ Update reward rate');
    console.log('   4. ✅ Pause/unpause pool');
    console.log('   5. ✅ Update multiple parameters');
    console.log('   6. ✅ Set pool end date');
    console.log('   7. ✅ Transfer authority (two-step)\n');

    console.log('💡 Tips for Pool Operators:');
    console.log('   - Always ensure reward vault has sufficient funds');
    console.log('   - Use pause feature for emergency situations');
    console.log('   - Update parameters carefully to maintain user trust');
    console.log('   - Two-step authority transfer prevents mistakes');
    console.log('   - Monitor pool metrics regularly\n');

  } catch (error) {
    handleError(error, 'Pool Administration');
  }
}

main();

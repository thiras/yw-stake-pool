/**
 * Devnet Integration Test
 *
 * This example demonstrates the entire lifecycle of a stake pool on Solana devnet:
 * 1. Setup - Create test tokens and fund accounts
 * 2. Pool initialization
 * 3. User staking
 * 4. Reward claiming
 * 5. Unstaking
 * 6. Pool management (updates)
 *
 * This is a LIVE TEST that executes real transactions on devnet.
 */

import { address, generateKeyPairSigner } from '@solana/kit';
import {
  getInitializePoolInstruction,
  getStakeInstruction,
  getClaimRewardsInstruction,
  getUnstakeInstruction,
  getFundRewardsInstruction,
  getUpdatePoolInstruction,
} from '@yourwallet/stake-pool';
import { some, none } from '@solana/kit';

import {
  config,
  formatAmount,
  formatRewardRate,
  formatDuration,
} from './config.js';
import {
  createRpc,
  createFundedKeypair,
  findPoolPda,
  findProgramAuthorityPda,
  findStakeAccountPda,
  buildAndSendTransaction,
  logSection,
  logStep,
  logTransaction,
  sleep,
  waitForRateLimit,
  handleError,
  calculateRewards,
} from './utils.js';
import { setupTestTokens } from './setup-tokens.js';

/**
 * Main function to run the complete flow
 */
async function main() {
  try {
    logSection('YW Stake Pool - Devnet Integration Test');

    // Initialize RPC client
    const rpc = createRpc();
    console.log(`🌐 Connected to: ${config.rpcUrl}\n`);

    // ========================================================================
    // Step 1: Setup - Create keypairs and test tokens
    // ========================================================================
    logStep(1, 'Setup - Create Keypairs and Test Tokens');

    // Create authority keypair (pool operator)
    // By default, uses your local Solana keypair unless config.useLocalKeypair is false
    const authority = await createFundedKeypair(
      rpc,
      'Authority',
      config.useLocalKeypair
    );

    // For this example, we'll use the same keypair for both authority and user
    // This avoids needing to airdrop SOL to a new user account on devnet
    const user = authority;
    console.log('📝 Using authority keypair as User for simplified testing');
    console.log(`   User Address: ${user.address}\n`);

    await waitForRateLimit();

    // ========================================================================
    // Step 1b: Create Real SPL Tokens and Derive Pool PDA
    // ========================================================================
    console.log('\n📝 Creating real SPL tokens for testing...');
    console.log('   This will create mints, vaults, and user token accounts');

    // First, we need to derive the pool PDA to use as the vault owner
    const stakeMintKeypair = await generateKeyPairSigner();
    const stakeMint = stakeMintKeypair.address;

    // Derive pool PDA with pool_id = 0 (first pool for this authority + mint)
    // NOTE: The pool_id used here MUST match the one passed to initializePool
    // Otherwise the transaction will fail with "address mismatch"
    const [poolAddress] = await findPoolPda(stakeMint, 0n);
    console.log(`\n📍 Derived Pool PDA: ${poolAddress}`);
    console.log('   (Will be used as vault owner)\n');

    const tokens = await setupTestTokens(
      authority,
      user,
      poolAddress,
      stakeMintKeypair
    );

    const rewardMint = tokens.rewardMint;
    const stakeVault = tokens.stakeVault;
    const rewardVault = tokens.rewardVault;
    const authorityRewardAccount = tokens.authorityRewardAccount;
    const userStakeAccount = tokens.userStakeAccount;
    const userRewardAccount = tokens.userRewardAccount;

    console.log('\n✅ Token setup complete!');
    console.log(`   Stake Mint: ${stakeMint}`);
    console.log(`   Reward Mint: ${rewardMint}`);
    console.log(`   Stake Vault: ${stakeVault} (owner: pool PDA)`);
    console.log(`   Reward Vault: ${rewardVault} (owner: pool PDA)`);

    // Save deployment info for reference
    console.log('\n💾 Deployment Information (save this!):');
    console.log('   ═'.repeat(50));
    console.log(`   Authority: ${authority.address}`);
    console.log(`   Stake Mint: ${stakeMint}`);
    console.log(`   Pool Address: ${poolAddress}`);
    console.log('   ═'.repeat(50));

    // ========================================================================
    // Step 2: Initialize Stake Pool
    // ========================================================================
    logStep(2, 'Initialize Stake Pool');

    console.log('Pool Configuration:');
    console.log(
      `  Reward Rate: ${formatRewardRate(config.defaultPoolConfig.rewardRate)}`
    );
    console.log(
      `  Min Stake: ${formatAmount(config.defaultPoolConfig.minStakeAmount)} tokens`
    );
    console.log(
      `  Lockup Period: ${formatDuration(config.defaultPoolConfig.lockupPeriod)}`
    );

    console.log(`\n📍 Pool Address (PDA): ${poolAddress}`);

    // Get the program authority PDA (required for pool initialization)
    const [programAuthority] = await findProgramAuthorityPda();
    console.log(`📍 Program Authority PDA: ${programAuthority}`);

    // Create and send initialize pool transaction
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
      programAuthority, // Required: validates creator permission
      poolId: 0n, // First pool for this stake_mint
      rewardRate: config.defaultPoolConfig.rewardRate,
      minStakeAmount: config.defaultPoolConfig.minStakeAmount,
      lockupPeriod: config.defaultPoolConfig.lockupPeriod,
      enforceLockup: false,
      poolEndDate: null,
    });

    const initPoolSig = await buildAndSendTransaction(
      rpc,
      [initPoolIx],
      authority
    );
    logTransaction(initPoolSig, 'Pool Initialized');

    await waitForRateLimit();

    // ========================================================================
    // Step 3: Fund Reward Vault
    // ========================================================================
    logStep(3, 'Fund Reward Vault');

    const fundAmount = config.exampleAmounts.fund;
    console.log(
      `💰 Funding reward vault with ${formatAmount(fundAmount)} tokens`
    );

    const fundIx = getFundRewardsInstruction({
      pool: poolAddress,
      funder: authority,
      funderTokenAccount: authorityRewardAccount, // Authority's reward account that has the tokens
      rewardVault,
      rewardMint,
      tokenProgram: config.tokenProgramId,
      amount: fundAmount,
    });

    const fundSig = await buildAndSendTransaction(rpc, [fundIx], authority);
    logTransaction(fundSig, 'Reward Vault Funded');

    await waitForRateLimit();

    // ========================================================================
    // Step 4: Stake Tokens
    // ========================================================================
    logStep(4, 'Stake Tokens');

    const stakeIndex = 0n;
    const [stakeAccountAddress] = await findStakeAccountPda(
      poolAddress,
      user.address,
      stakeIndex
    );

    console.log(`📍 Stake Account Address (PDA): ${stakeAccountAddress}`);

    const stakeAmount = config.exampleAmounts.stake;
    console.log(`🔒 Staking ${formatAmount(stakeAmount)} tokens`);
    console.log(
      `   (Note: Stake instruction creates the stake account automatically)\n`
    );

    const stakeIx = getStakeInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress,
      owner: user,
      userTokenAccount: userStakeAccount,
      stakeVault,
      stakeMint,
      rewardVault,
      tokenProgram: config.tokenProgramId,
      payer: user,
      systemProgram: config.systemProgramId,
      amount: stakeAmount,
      index: stakeIndex,
      expectedRewardRate: null, // Optional: set to prevent frontrunning
      expectedLockupPeriod: null, // Optional: set to prevent frontrunning
    });

    // Calculate expected rewards
    const expectedRewards = calculateRewards(
      stakeAmount,
      config.defaultPoolConfig.rewardRate,
      true
    );
    console.log(
      `\n📊 Expected rewards after lockup: ${formatAmount(expectedRewards)} tokens`
    );

    const stakeSig = await buildAndSendTransaction(rpc, [stakeIx], user);
    logTransaction(stakeSig, 'Tokens Staked');

    await waitForRateLimit();

    // ========================================================================
    // Step 5: Wait for Lockup Period (simulated)
    // ========================================================================
    logStep(5, 'Wait for Lockup Period');

    console.log(
      `⏳ Lockup period: ${formatDuration(config.defaultPoolConfig.lockupPeriod)}`
    );
    console.log('   (In this example, we simulate the wait)');
    console.log('   In production, users must wait before claiming rewards');

    // Simulate wait (in real scenario, this would be actual time passage)
    await sleep(2000);

    await waitForRateLimit();

    // ========================================================================
    // Step 6: Claim Rewards
    // ========================================================================
    logStep(6, 'Claim Rewards');

    console.log('💎 Claiming accrued rewards');

    const claimIx = getClaimRewardsInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress,
      owner: user,
      userRewardAccount,
      rewardVault,
      rewardMint,
      tokenProgram: config.tokenProgramId,
      clock: address('SysvarC1ock11111111111111111111111111111111'),
    });

    console.log(
      `   User should receive ~${formatAmount(expectedRewards)} tokens`
    );
    const claimSig = await buildAndSendTransaction(rpc, [claimIx], user);
    logTransaction(claimSig, 'Rewards Claimed');

    await waitForRateLimit();

    // ========================================================================
    // Step 7: Partial Unstake
    // ========================================================================
    logStep(7, 'Partial Unstake');

    const unstakeAmount = config.exampleAmounts.unstake;
    console.log(`🔓 Unstaking ${formatAmount(unstakeAmount)} tokens (partial)`);

    const unstakeIx = getUnstakeInstruction({
      pool: poolAddress,
      stakeAccount: stakeAccountAddress,
      owner: user,
      userTokenAccount: userStakeAccount,
      stakeVault,
      stakeMint,
      tokenProgram: config.tokenProgramId,
      clock: address('SysvarC1ock11111111111111111111111111111111'),
      amount: unstakeAmount,
      expectedRewardRate: null, // Optional: set to prevent frontrunning
    });

    console.log(
      `   Remaining staked: ${formatAmount(stakeAmount - unstakeAmount)} tokens`
    );
    const unstakeSig = await buildAndSendTransaction(rpc, [unstakeIx], user);
    logTransaction(unstakeSig, 'Tokens Unstaked');

    await waitForRateLimit();

    // ========================================================================
    // Step 8: Update Pool Parameters
    // ========================================================================
    logStep(8, 'Update Pool Parameters');

    const newRewardRate = 150_000_000n; // 15%
    console.log(
      `⚙️  Updating pool reward rate to ${formatRewardRate(newRewardRate)}`
    );

    const updatePoolIx = getUpdatePoolInstruction({
      pool: poolAddress,
      authority: authority,
      rewardRate: some(newRewardRate),
      minStakeAmount: none(), // Don't change
      lockupPeriod: none(), // Don't change
      isPaused: none(), // Don't change
      enforceLockup: none(), // Don't change
      poolEndDate: null, // Don't change
    });

    const updateSig = await buildAndSendTransaction(
      rpc,
      [updatePoolIx],
      authority
    );
    logTransaction(updateSig, 'Pool Parameters Updated');

    await waitForRateLimit();

    // ========================================================================
    // Step 9: Transfer Pool Authority (Two-Step)
    // ========================================================================
    logStep(9, 'Transfer Pool Authority (Two-Step Process)');

    console.log(
      '⚠️  Note: Authority transfer requires a funded new authority account'
    );
    console.log('   Skipping this step to avoid devnet airdrop issues');
    console.log(
      '   In production, generate a new keypair and fund it before transfer\n'
    );

    // Uncomment below to test authority transfer with a new keypair
    // const newAuthority = await createFundedKeypair(rpc, 'New Authority', false);
    //
    // console.log('Step 10a: Nominate new authority');
    // const nominateIx = getNominateNewAuthorityInstruction({
    //   pool: poolAddress,
    //   currentAuthority: authority,
    //   newAuthority: newAuthority.address,
    // });
    //
    // const nominateSig = await buildAndSendTransaction(rpc, [nominateIx], authority);
    // logTransaction(nominateSig, 'Authority Nominated');
    //
    // console.log('\nStep 10b: Accept authority (must be called by new authority)');
    // const acceptIx = getAcceptAuthorityInstruction({
    //   pool: poolAddress,
    //   pendingAuthority: newAuthority,
    // });
    //
    // const acceptSig = await buildAndSendTransaction(rpc, [acceptIx], newAuthority);
    // logTransaction(acceptSig, 'Authority Transfer Accepted');

    // ========================================================================
    // Summary
    // ========================================================================
    logSection('Devnet Integration Test Complete!');

    console.log('✨ All transactions executed successfully on devnet!\n');
    console.log('📋 Transaction Summary:');
    console.log(`   1. ✅ Pool Initialized - ${initPoolSig}`);
    console.log(`   2. ✅ Rewards Funded - ${fundSig}`);
    console.log(`   3. ✅ Tokens Staked - ${stakeSig}`);
    console.log(`   4. ✅ Rewards Claimed - ${claimSig}`);
    console.log(`   5. ✅ Tokens Unstaked - ${unstakeSig}`);
    console.log(`   6. ✅ Pool Updated - ${updateSig}`);
    console.log(`   (Authority transfer skipped - see Step 9 for details)\n`);

    console.log('💾 Save This Information:');
    console.log('   ═'.repeat(60));
    console.log(`   Authority: ${authority.address}`);
    console.log(`   Stake Mint: ${stakeMint}`);
    console.log(`   Reward Mint: ${rewardMint}`);
    console.log(`   Pool Address: ${poolAddress}`);
    console.log(`   Stake Account: ${stakeAccountAddress}`);
    console.log('   ═'.repeat(60));

    console.log('\n🔍 View transactions on Solana Explorer:');
    console.log(
      `   https://explorer.solana.com/address/${poolAddress}?cluster=custom\n`
    );

    console.log('📝 Next Steps:');
    console.log('   - Create actual SPL tokens for staking and rewards');
    console.log('   - Update token mint addresses in config');
    console.log('   - Test with real token transfers');
    console.log('   - Integrate with your frontend application\n');

    console.log('📚 Resources:');
    console.log('   - Program README: ../README.md');
    console.log('   - Client Docs: ../clients/js/README.md');
    console.log('   - Security Audit: ../SECURITY_AUDIT.md\n');
  } catch (error) {
    handleError(error, 'Devnet Integration Test');
  }
}

// Run the integration test
main();

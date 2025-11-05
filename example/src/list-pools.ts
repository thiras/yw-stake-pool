/**
 * List Pool Script
 *
 * This script checks if a stake pool exists for a given authority and stake mint.
 * In the current implementation, each authority + stake_mint combination can have only ONE pool.
 *
 * Usage:
 *   tsx src/list-pools.ts <authority> <stake_mint>
 *
 * Example:
 *   tsx src/list-pools.ts 7xYz3... TokenkegQf...
 */

import { address, Address } from '@solana/kit';
import { getStakePoolCodec } from '@yourwallet/stake-pool';
import { createRpc, findPoolPda } from './utils';

// Helper to fetch and decode pool account
async function fetchPoolAccount(rpc: any, poolAddress: Address) {
  try {
    const accountInfo = await rpc
      .getAccountInfo(poolAddress, { encoding: 'base64' })
      .send();

    if (!accountInfo || !accountInfo.value) {
      return null;
    }

    return {
      exists: true,
      lamports: accountInfo.value.lamports,
      owner: accountInfo.value.owner,
      data: accountInfo.value.data,
    };
  } catch (error) {
    return null;
  }
}

// Helper to decode pool data
function decodePoolData(data: [string, string]): any {
  try {
    const codec = getStakePoolCodec();
    const buffer = Buffer.from(data[0], data[1] as BufferEncoding);
    const decoded = codec.decode(buffer);
    return decoded;
  } catch (error) {
    console.error('Error decoding pool data:', error);
    return null;
  }
}

async function checkPool(
  authorityAddress: Address,
  stakeMintAddress: Address
) {
  console.log('\n🔍 Checking for Stake Pool');
  console.log('═'.repeat(70));
  console.log(`Authority: ${authorityAddress}`);
  console.log(`Stake Mint: ${stakeMintAddress}`);
  console.log('═'.repeat(70));

  const rpc = createRpc();

  // Find the pool PDA
  const [poolAddress, bump] = await findPoolPda(
    authorityAddress,
    stakeMintAddress
  );

  console.log(`\n📍 Pool PDA: ${poolAddress}`);
  console.log(`   Bump: ${bump}`);

  console.log('\n🔎 Fetching pool account...');

  const accountInfo = await fetchPoolAccount(rpc, poolAddress);

  if (!accountInfo) {
    console.log('\n' + '═'.repeat(70));
    console.log('❌ Pool Not Found');
    console.log('═'.repeat(70));
    console.log('\nNo stake pool exists for this authority + stake mint combination.');
    console.log('💡 You can create one using the initialize pool instruction.\n');
    return;
  }

  console.log('✅ Pool Found!\n');

  // Try to decode the pool data
  const poolData = decodePoolData(accountInfo.data);

  console.log('═'.repeat(70));
  console.log('📊 Pool Information');
  console.log('═'.repeat(70));
  console.log(`Address: ${poolAddress}`);
  console.log(`Bump: ${bump}`);
  console.log(`Lamports: ${accountInfo.lamports}`);
  console.log(`Owner: ${accountInfo.owner}`);

  if (poolData) {
    console.log('\n📋 Pool Configuration:');
    console.log('─'.repeat(70));
    console.log(`Authority: ${poolData.authority}`);
    console.log(`Stake Mint: ${poolData.stakeMint}`);
    console.log(`Reward Mint: ${poolData.rewardMint}`);
    console.log(`Stake Vault: ${poolData.stakeVault}`);
    console.log(`Reward Vault: ${poolData.rewardVault}`);
    console.log(
      `\nTotal Staked: ${poolData.totalStaked} (${
        Number(poolData.totalStaked) / 1_000_000
      } tokens)`
    );
    console.log(
      `Total Rewards Owed: ${poolData.totalRewardsOwed} (${
        Number(poolData.totalRewardsOwed) / 1_000_000
      } tokens)`
    );
    console.log(
      `\nReward Rate: ${poolData.rewardRate} (${
        Number(poolData.rewardRate) / 10_000_000
      }%)`
    );
    console.log(
      `Min Stake Amount: ${poolData.minStakeAmount} (${
        Number(poolData.minStakeAmount) / 1_000_000
      } tokens)`
    );
    console.log(
      `Lockup Period: ${poolData.lockupPeriod} seconds (${
        Number(poolData.lockupPeriod) / 86400
      } days)`
    );
    console.log(`\nPool Status: ${poolData.isPaused ? '⏸️  PAUSED' : '✅ ACTIVE'}`);
    console.log(`Enforce Lockup: ${poolData.enforceLockup ? 'Yes' : 'No'}`);

    if (poolData.poolEndDate && poolData.poolEndDate.__option === 'Some') {
      const endDate = new Date(Number(poolData.poolEndDate.value) * 1000);
      console.log(`Pool End Date: ${endDate.toLocaleString()}`);
    } else {
      console.log('Pool End Date: None (runs indefinitely)');
    }

    if (poolData.pendingAuthority && poolData.pendingAuthority.__option === 'Some') {
      console.log(`\n⚠️  Pending Authority Transfer: ${poolData.pendingAuthority.value}`);
    }
  }

  console.log('\n' + '═'.repeat(70) + '\n');
}

// Main execution
async function main() {
  const args = process.argv.slice(2);

  if (args.length < 2) {
    console.error('\n❌ Error: Missing required arguments\n');
    console.log('Usage: tsx src/list-pools.ts <authority> <stake_mint>\n');
    console.log('Arguments:');
    console.log('  authority   - Pool authority public key');
    console.log('  stake_mint  - Token mint being staked\n');
    console.log('Example:');
    console.log('  tsx src/list-pools.ts 7xYz3pZD6... TokenkegQfeZyiN...\n');
    console.log('Note: In the current implementation, each authority + stake_mint');
    console.log('      combination can have only ONE pool.\n');
    process.exit(1);
  }

  const authorityAddress = address(args[0]);
  const stakeMintAddress = address(args[1]);

  try {
    await checkPool(authorityAddress, stakeMintAddress);
  } catch (error) {
    console.error('\n❌ Error checking pool:', error);
    process.exit(1);
  }
}

main();

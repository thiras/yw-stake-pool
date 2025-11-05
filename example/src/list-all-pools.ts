/**
 * List All Pools Script
 *
 * This script scans the Solana blockchain for ALL stake pools created by the program.
 * It doesn't require authority or stake_mint - it finds all pools on the network.
 *
 * Usage:
 *   tsx src/list-all-pools.ts [cluster]
 *
 * Example:
 *   tsx src/list-all-pools.ts          # Uses default cluster from config
 *   tsx src/list-all-pools.ts devnet   # Explicitly use devnet
 */

import { address } from '@solana/kit';
import { getStakePoolCodec, Key } from '@yourwallet/stake-pool';
import { createRpc } from './utils';
import { config } from './config';

// Helper to decode pool data
function decodePoolData(data: Uint8Array): any {
  try {
    const codec = getStakePoolCodec();
    const decoded = codec.decode(data);
    
    // Verify it's actually a StakePool by checking the discriminator
    if (decoded.key !== Key.StakePool) {
      return null;
    }
    
    return decoded;
  } catch (error) {
    return null;
  }
}

// Format pool data for display
function formatPoolInfo(pool: any, index: number) {
  console.log(`\n${index}. Pool ${pool.address}`);
  console.log('   ─'.repeat(35));
  console.log(`   Authority: ${pool.data.authority}`);
  console.log(`   Stake Mint: ${pool.data.stakeMint}`);
  console.log(`   Reward Mint: ${pool.data.rewardMint}`);
  
  const totalStaked = Number(pool.data.totalStaked);
  const totalRewardsOwed = Number(pool.data.totalRewardsOwed);
  const rewardRate = Number(pool.data.rewardRate);
  const minStakeAmount = Number(pool.data.minStakeAmount);
  const lockupPeriod = Number(pool.data.lockupPeriod);
  
  console.log(`\n   💰 Staking:`);
  console.log(`      Total Staked: ${(totalStaked / 1_000_000).toFixed(2)} tokens`);
  console.log(`      Min Stake: ${(minStakeAmount / 1_000_000).toFixed(2)} tokens`);
  
  console.log(`\n   🎁 Rewards:`);
  console.log(`      Reward Rate: ${(rewardRate / 10_000_000).toFixed(2)}%`);
  console.log(`      Total Rewards Owed: ${(totalRewardsOwed / 1_000_000).toFixed(2)} tokens`);
  
  console.log(`\n   ⏱️  Lockup:`);
  console.log(`      Period: ${lockupPeriod} seconds (${(lockupPeriod / 86400).toFixed(2)} days)`);
  console.log(`      Enforce Lockup: ${pool.data.enforceLockup ? 'Yes' : 'No'}`);
  
  console.log(`\n   📊 Status:`);
  console.log(`      ${pool.data.isPaused ? '⏸️  PAUSED' : '✅ ACTIVE'}`);
  
  if (pool.data.poolEndDate && pool.data.poolEndDate.__option === 'Some') {
    const endDate = new Date(Number(pool.data.poolEndDate.value) * 1000);
    console.log(`      End Date: ${endDate.toLocaleString()}`);
  } else {
    console.log(`      End Date: None (runs indefinitely)`);
  }
  
  if (pool.data.pendingAuthority && pool.data.pendingAuthority.__option === 'Some') {
    console.log(`      ⚠️  Pending Authority: ${pool.data.pendingAuthority.value}`);
  }
}

// Fetch all program accounts and filter for pools
async function listAllPools() {
  console.log('\n🔍 Scanning for All Stake Pools');
  console.log('═'.repeat(70));
  console.log(`Program ID: ${config.programId}`);
  console.log(`RPC Endpoint: ${config.rpcUrl}`);
  console.log('═'.repeat(70));

  const rpc = createRpc();
  
  console.log('\n⏳ Fetching all program accounts...');
  console.log('   (This may take a moment)\n');

  try {
    // Get all accounts owned by the program
    const response = await rpc
      .getProgramAccounts(address(config.programId), {
        encoding: 'base64',
        filters: [
          {
            // Filter by account size (StakePool is 278 bytes)
            dataSize: 278n,
          },
        ],
      })
      .send();

    if (!response || response.length === 0) {
      console.log('══════════════════════════════════════════════════════════════════════');
      console.log('❌ No pools found');
      console.log('══════════════════════════════════════════════════════════════════════');
      console.log('\nNo stake pools exist on this network.');
      console.log('💡 Create your first pool using the initialize pool instruction.\n');
      return;
    }

    console.log(`✅ Found ${response.length} program account(s)\n`);
    console.log('📝 Decoding pool data...\n');

    // Decode and filter for valid pools
    const pools: Array<{ address: string; data: any; lamports: bigint }> = [];
    
    for (const account of response) {
      const buffer = Buffer.from(account.account.data[0], 'base64');
      const poolData = decodePoolData(buffer);
      
      if (poolData) {
        pools.push({
          address: account.pubkey,
          data: poolData,
          lamports: account.account.lamports,
        });
      }
    }

    if (pools.length === 0) {
      console.log('══════════════════════════════════════════════════════════════════════');
      console.log('❌ No valid pools found');
      console.log('══════════════════════════════════════════════════════════════════════');
      console.log('\nFound program accounts but none were valid stake pools.');
      console.log('They may be stake accounts or other account types.\n');
      return;
    }

    // Display results
    console.log('══════════════════════════════════════════════════════════════════════');
    console.log(`📊 Found ${pools.length} Stake Pool(s)`);
    console.log('══════════════════════════════════════════════════════════════════════');

    // Group pools by authority
    const poolsByAuthority = new Map<string, typeof pools>();
    pools.forEach((pool) => {
      const authority = pool.data.authority;
      if (!poolsByAuthority.has(authority)) {
        poolsByAuthority.set(authority, []);
      }
      poolsByAuthority.get(authority)!.push(pool);
    });

    // Display each pool
    pools.forEach((pool, index) => {
      formatPoolInfo(pool, index + 1);
    });

    // Summary by authority
    console.log('\n' + '═'.repeat(70));
    console.log('📈 Summary by Authority');
    console.log('═'.repeat(70));
    
    poolsByAuthority.forEach((authorityPools, authority) => {
      console.log(`\n${authority}`);
      console.log(`  → ${authorityPools.length} pool(s)`);
      
      const totalStaked = authorityPools.reduce(
        (sum, p) => sum + Number(p.data.totalStaked),
        0
      );
      console.log(`  → Total Staked: ${(totalStaked / 1_000_000).toFixed(2)} tokens`);
    });

    // Overall statistics
    console.log('\n' + '═'.repeat(70));
    console.log('📊 Overall Statistics');
    console.log('═'.repeat(70));
    
    const totalStakedAll = pools.reduce(
      (sum, p) => sum + Number(p.data.totalStaked),
      0
    );
    const totalRewardsOwedAll = pools.reduce(
      (sum, p) => sum + Number(p.data.totalRewardsOwed),
      0
    );
    const activePools = pools.filter((p) => !p.data.isPaused).length;
    const pausedPools = pools.filter((p) => p.data.isPaused).length;
    
    console.log(`Total Pools: ${pools.length}`);
    console.log(`Active Pools: ${activePools}`);
    console.log(`Paused Pools: ${pausedPools}`);
    console.log(`Unique Authorities: ${poolsByAuthority.size}`);
    console.log(`Total Value Staked: ${(totalStakedAll / 1_000_000).toFixed(2)} tokens`);
    console.log(`Total Rewards Owed: ${(totalRewardsOwedAll / 1_000_000).toFixed(2)} tokens`);
    
    console.log('═'.repeat(70) + '\n');
  } catch (error: any) {
    console.error('\n❌ Error fetching program accounts:', error.message);
    
    if (error.message?.includes('getProgramAccounts')) {
      console.log('\n💡 Tip: Make sure your RPC endpoint supports getProgramAccounts.');
      console.log('   Some public RPC endpoints may have this disabled.');
      console.log('   Try using a different RPC or run a local validator.\n');
    }
    
    throw error;
  }
}

// Main execution
async function main() {
  try {
    await listAllPools();
  } catch (error) {
    console.error('\n❌ Error listing pools:', error);
    process.exit(1);
  }
}

main();

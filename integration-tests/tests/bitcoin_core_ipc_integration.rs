// Integration tests for Pool and JDC running with `TemplateProviderType::BitcoinCoreIpc`.
//
// These tests verify that Pool and JDC can connect directly to Bitcoin Core via IPC
// (bypassing sv2-tp) and successfully complete an e2e mining cycle.
//
// Related issue: https://github.com/stratum-mining/sv2-apps/issues/71

use integration_tests_sv2::{template_provider::DifficultyLevel, *};

/// Test Pool e2e mining cycle with BitcoinCoreIpc.
///
/// This test verifies that Pool can:
/// 1. Connect directly to Bitcoin Core via IPC (bypassing sv2-tp)
/// 2. Receive templates from Bitcoin Core
/// 3. Propagate a mined block back to Bitcoin Core
#[tokio::test]
async fn pool_propagates_block_with_bitcoin_core_ipc() {
    start_tracing();

    // Start template provider (Bitcoin Core + sv2-tp)
    // We use the IPC socket path from TemplateProvider to connect Pool directly to Bitcoin Core
    let (tp, _tp_addr) = start_template_provider(None, DifficultyLevel::Low);
    let ipc_socket_path = tp.ipc_socket_path().clone();
    let current_block_hash = tp.get_best_block_hash().unwrap();

    // Start Pool connecting directly to Bitcoin Core via IPC (bypassing sv2-tp)
    let (_pool, pool_addr) = start_pool_ipc(ipc_socket_path, vec![], vec![]).await;

    // Start translator and miner
    let (_translator, tproxy_addr) =
        start_sv2_translator(&[pool_addr], false, vec![], vec![]).await;
    let (_minerd_process, _minerd_addr) = start_minerd(tproxy_addr, None, None, false).await;

    // Wait for a block to be mined (in regtest, every share is a block)
    // Poll for block change with timeout (up to 60 seconds)
    let timeout = tokio::time::Duration::from_secs(60);
    let poll_interval = tokio::time::Duration::from_secs(2);
    let start_time = tokio::time::Instant::now();

    loop {
        tokio::time::sleep(poll_interval).await;

        let new_block_hash = tp.get_best_block_hash().unwrap();
        if new_block_hash != current_block_hash {
            // Block was propagated successfully
            return;
        }

        if start_time.elapsed() > timeout {
            panic!(
                "Pool with BitcoinCoreIpc should have propagated a new block within {} seconds",
                timeout.as_secs()
            );
        }
    }
}

/// Test JDC e2e mining cycle with BitcoinCoreIpc.
///
/// This test verifies that JDC can:
/// 1. Connect directly to Bitcoin Core via IPC (bypassing sv2-tp)
/// 2. Receive templates from Bitcoin Core
/// 3. Declare jobs and propagate a mined block
#[tokio::test]
async fn jdc_propagates_block_with_bitcoin_core_ipc() {
    start_tracing();

    // Start template provider (Bitcoin Core + sv2-tp)
    // We use the IPC socket path from TemplateProvider to connect JDC directly to Bitcoin Core
    let (tp, tp_addr) = start_template_provider(None, DifficultyLevel::Low);
    let ipc_socket_path = tp.ipc_socket_path().clone();
    let current_block_hash = tp.get_best_block_hash().unwrap();

    // Start Pool (using sv2-tp for simplicity - Pool needs a TP connection)
    let (_pool, pool_addr) = start_pool(Some(tp_addr), vec![], vec![]).await;

    // Start JDS (Job Declarator Server) - needed for JDC
    let (_jds, jds_addr) = start_jds(tp.rpc_info());

    // Start JDC connecting directly to Bitcoin Core via IPC (bypassing sv2-tp for template distribution)
    let (_jdc, jdc_addr) = start_jdc_ipc(&[(pool_addr, jds_addr)], ipc_socket_path, vec![], vec![]);

    // Give JDC time to establish IPC connection and receive initial template
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Start translator and miner
    let (_translator, tproxy_addr) = start_sv2_translator(&[jdc_addr], false, vec![], vec![]).await;
    let (_minerd_process, _minerd_addr) = start_minerd(tproxy_addr, None, None, false).await;

    // Wait for a block to be mined (in regtest, every share is a block)
    // Poll for block change with timeout (up to 60 seconds)
    let timeout = tokio::time::Duration::from_secs(60);
    let poll_interval = tokio::time::Duration::from_secs(2);
    let start_time = tokio::time::Instant::now();

    loop {
        tokio::time::sleep(poll_interval).await;

        let new_block_hash = tp.get_best_block_hash().unwrap();
        if new_block_hash != current_block_hash {
            // Block was propagated successfully
            return;
        }

        if start_time.elapsed() > timeout {
            panic!(
                "JDC with BitcoinCoreIpc should have propagated a new block within {} seconds",
                timeout.as_secs()
            );
        }
    }
}

use integration_tests_sv2::{template_provider::DifficultyLevel, *};

#[tokio::test]
async fn tp_low_diff() {
    let (tp, _) = start_template_provider(None, DifficultyLevel::Low);
    let blockchain_info = tp.get_blockchain_info().unwrap();
    assert_eq!(blockchain_info.difficulty, 4.6565423739069247e-10);
    assert_eq!(blockchain_info.chain, "regtest");
}

#[tokio::test]
async fn tp_mid_diff() {
    let (tp, _) = start_template_provider(None, DifficultyLevel::Mid);
    let blockchain_info = tp.get_blockchain_info().unwrap();
    assert_eq!(blockchain_info.difficulty, 0.001126515290698186);
    assert_eq!(blockchain_info.chain, "signet");
}

#[tokio::test]
async fn tp_high_diff() {
    let (tp, _) = start_template_provider(None, DifficultyLevel::High);
    let blockchain_info = tp.get_blockchain_info().unwrap();
    assert_eq!(blockchain_info.difficulty, 77761.1123986095);
    assert_eq!(blockchain_info.chain, "signet");
}

// This test verifies that coinbase outputs survive the full round-trip through the mining stack.
// A mempool transaction triggers a witness commitment output, which must be preserved through
// TP -> Pool -> Translator -> Minerd and back to Bitcoin Core via block submission.
#[tokio::test]
async fn tp_coinbase_outputs_round_trip() {
    start_tracing();
    let sv2_interval = Some(5);
    let (tp, tp_addr) = start_template_provider(sv2_interval, DifficultyLevel::Low);
    tp.fund_wallet().unwrap();
    let current_block_hash = tp.get_best_block_hash().unwrap();

    let (pool, pool_addr, _) = start_pool(sv2_tp_config(tp_addr), vec![], vec![], false).await;

    // Create a mempool transaction to trigger witness commitment output
    tp.create_mempool_transaction().unwrap();

    let (translator, tproxy_addr, _) =
        start_sv2_translator(&[pool_addr], false, vec![], vec![], None, false).await;
    let (_minerd, _) = start_minerd(tproxy_addr, None, None, false).await;

    wait_for_new_block(
        &current_block_hash,
        || tp.get_best_block_hash().unwrap(),
        "Block should have been mined and accepted within 60 seconds, \
         confirming coinbase outputs survived the round-trip",
    )
    .await;
    shutdown_all!(pool, translator);
}

// This test verifies that multiple coinbase outputs survive the full round-trip.
// Requires a custom Bitcoin Core build with -testmulticoinbase support, which adds extra
// OP_RETURN outputs to the coinbase. Set BITCOIN_NODE_BIN to the custom binary path.
#[tokio::test]
async fn tp_multiple_coinbase_outputs_round_trip() {
    start_tracing();

    if std::env::var("BITCOIN_NODE_BIN").is_err() {
        eprintln!(
            "Skipping tp_multiple_coinbase_outputs_round_trip: \
             BITCOIN_NODE_BIN not set (requires custom Bitcoin Core with -testmulticoinbase)"
        );
        return;
    }

    let sv2_interval = Some(5);
    let (tp, tp_addr) = start_template_provider_with_args(
        sv2_interval,
        DifficultyLevel::Low,
        vec!["-testmulticoinbase"],
    );
    tp.fund_wallet().unwrap();
    let current_block_hash = tp.get_best_block_hash().unwrap();

    let (pool, pool_addr, _) = start_pool(sv2_tp_config(tp_addr), vec![], vec![], false).await;

    // Create a mempool transaction to add a witness commitment OP_RETURN
    tp.create_mempool_transaction().unwrap();

    let (translator, tproxy_addr, _) =
        start_sv2_translator(&[pool_addr], false, vec![], vec![], None, false).await;
    let (_minerd, _) = start_minerd(tproxy_addr, None, None, false).await;

    wait_for_new_block(
        &current_block_hash,
        || tp.get_best_block_hash().unwrap(),
        "Block should have been mined and accepted within 60 seconds, \
         confirming all coinbase outputs survived the round-trip",
    )
    .await;
    shutdown_all!(pool, translator);
}

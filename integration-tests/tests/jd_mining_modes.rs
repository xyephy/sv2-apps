use integration_tests_sv2::{interceptor::MessageDirection, template_provider::DifficultyLevel, *};
use stratum_apps::stratum_core::common_messages_sv2::*;

// JDS requires FullTemplate but JDC asks for CoinbaseOnly — SetupConnection rejected
#[tokio::test]
async fn jd_mode_mismatch_setup_connection_fails() {
    start_tracing();
    let (tp, _tp_addr) = start_template_provider(None, DifficultyLevel::Low);
    let (_pool, pool_addr, jds_addr, _) =
        start_pool_with_jds(tp.bitcoin_core(), vec![], vec![], false, true).await;
    let (sniffer, sniffer_addr) = start_sniffer("jdc-jds", jds_addr, false, vec![], None);
    let (_jdc, _jdc_addr, _) = start_jdc(
        &[(pool_addr, sniffer_addr)],
        ipc_config(
            tp.bitcoin_core().data_dir().clone(),
            tp.bitcoin_core().is_signet(),
            None,
        ),
        vec![],
        vec![],
        false,
        Some("COINBASEONLY"),
    );
    sniffer
        .wait_for_message_type(MessageDirection::ToUpstream, MESSAGE_TYPE_SETUP_CONNECTION)
        .await;
    sniffer
        .wait_for_message_type(
            MessageDirection::ToDownstream,
            MESSAGE_TYPE_SETUP_CONNECTION_ERROR,
        )
        .await;
}

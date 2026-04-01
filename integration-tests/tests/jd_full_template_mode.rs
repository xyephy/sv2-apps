use integration_tests_sv2::{interceptor::MessageDirection, template_provider::DifficultyLevel, *};
use stratum_apps::stratum_core::{
    common_messages_sv2::*, job_declaration_sv2::*, template_distribution_sv2::*,
};

// JDC in FullTemplate mode (default) exchanges DeclareMiningJob with JDS
// and propagates blocks via both SubmitSolution (to TP) and PushSolution (to JDS)
#[tokio::test]
async fn jd_full_template_mode_declare_mining_job_exchanged() {
    start_tracing();
    let (tp, tp_addr) = start_template_provider(None, DifficultyLevel::Low);
    let current_block_hash = tp.get_best_block_hash().unwrap();
    let (_pool, pool_addr, jds_addr, _) =
        start_pool_with_jds(tp.bitcoin_core(), vec![], vec![], false, true).await;
    let (jdc_jds_sniffer, jdc_jds_sniffer_addr) =
        start_sniffer("jdc-jds", jds_addr, false, vec![], None);
    let (jdc_tp_sniffer, jdc_tp_sniffer_addr) =
        start_sniffer("jdc-tp", tp_addr, false, vec![], None);
    let (_jdc, jdc_addr, _) = start_jdc(
        &[(pool_addr, jdc_jds_sniffer_addr)],
        sv2_tp_config(jdc_tp_sniffer_addr),
        vec![],
        vec![],
        false,
        None,
    );
    jdc_jds_sniffer
        .wait_for_message_type(MessageDirection::ToUpstream, MESSAGE_TYPE_SETUP_CONNECTION)
        .await;
    jdc_jds_sniffer
        .wait_for_message_type(
            MessageDirection::ToDownstream,
            MESSAGE_TYPE_SETUP_CONNECTION_SUCCESS,
        )
        .await;

    let (_translator, tproxy_addr, _) =
        start_sv2_translator(&[jdc_addr], false, vec![], vec![], None, false).await;
    let (_minerd, _) = start_minerd(tproxy_addr, None, None, false).await;

    // DeclareMiningJob exchanged in both directions
    jdc_jds_sniffer
        .wait_for_message_type(
            MessageDirection::ToUpstream,
            MESSAGE_TYPE_DECLARE_MINING_JOB,
        )
        .await;
    jdc_jds_sniffer
        .wait_for_message_type(
            MessageDirection::ToDownstream,
            MESSAGE_TYPE_DECLARE_MINING_JOB_SUCCESS,
        )
        .await;

    // block propagation from JDC to TP
    jdc_tp_sniffer
        .wait_for_message_type(MessageDirection::ToUpstream, MESSAGE_TYPE_SUBMIT_SOLUTION)
        .await;
    let new_block_hash = tp.get_best_block_hash().unwrap();
    assert_ne!(current_block_hash, new_block_hash);

    // PushSolution sent to JDS in FullTemplate mode
    assert!(
        jdc_jds_sniffer.has_message_type(MessageDirection::ToUpstream, MESSAGE_TYPE_PUSH_SOLUTION),
        "PushSolution should be sent to JDS in FullTemplate mode"
    );
}

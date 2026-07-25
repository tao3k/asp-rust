#![deny(dead_code)]
//! Project-level Rust language harness for policy gates and agent advice.
#![recursion_limit = "256"]

//!
//! The crate provides library APIs for scanning Rust projects, returning
//! deterministic findings, rendering compact diagnostics, and mounting a
//! reusable Cargo test gate.

mod agent_snapshot;
mod build_gate;
#[cfg(feature = "cli")]
mod cli;
mod discovery;
mod downstream_gate_guide;
mod harness_rules;
mod invariant_catalog;
mod macros;
mod model;
mod parser;
mod path;
mod render;
mod rules;
mod runner;
#[cfg(feature = "search")]
mod search;
mod self_policy;
mod semantic_identity;
mod verification;
mod workspace_evidence_graph;

pub use harness_rules::{
    RUST_HARNESS_RULES_MD, render_rust_harness_rules_markdown, rust_harness_rules_markdown,
    write_rust_harness_rules_to_unit_tests,
};
pub use verification::{
    RUST_BEHAVIOR_SNAPSHOT_PROTOCOL_ID, RUST_BEHAVIOR_SNAPSHOT_PROTOCOL_VERSION,
    RUST_BEHAVIOR_SNAPSHOT_SCHEMA_ID, RUST_BEHAVIOR_SNAPSHOT_SCHEMA_VERSION, RustBehaviorSnapshot,
    RustBehaviorSnapshotArtifactUri, RustBehaviorSnapshotExpectTestInput, RustBehaviorSnapshotId,
    RustBehaviorSnapshotInput, RustBehaviorSnapshotLanguageId, RustBehaviorSnapshotLine,
    RustBehaviorSnapshotNamespace, RustBehaviorSnapshotObservation,
    RustBehaviorSnapshotObservationKind, RustBehaviorSnapshotObservationMessage,
    RustBehaviorSnapshotObservedAt, RustBehaviorSnapshotProducer, RustBehaviorSnapshotProtocolId,
    RustBehaviorSnapshotProtocolVersion, RustBehaviorSnapshotProviderId,
    RustBehaviorSnapshotSchemaId, RustBehaviorSnapshotSchemaVersion, RustBehaviorSnapshotSha256,
    RustBehaviorSnapshotStatus, RustBehaviorSnapshotSubject, RustBehaviorSnapshotSubjectKind,
    RustBehaviorSnapshotSymbol, RustBehaviorSnapshotValue, RustBehaviorSnapshotValueFormat,
};

pub use verification::{
    RUST_DETERMINISM_READINESS_PROTOCOL_ID, RUST_DETERMINISM_READINESS_PROTOCOL_VERSION,
    RUST_DETERMINISM_READINESS_SCHEMA_ID, RUST_DETERMINISM_READINESS_SCHEMA_VERSION,
    RustDeterminismReadiness, RustDeterminismReadinessCategory,
    RustDeterminismReadinessEvidenceKind, RustDeterminismReadinessExpression,
    RustDeterminismReadinessId, RustDeterminismReadinessInput, RustDeterminismReadinessLanguageId,
    RustDeterminismReadinessLine, RustDeterminismReadinessNamespace,
    RustDeterminismReadinessObservation, RustDeterminismReadinessObservationId,
    RustDeterminismReadinessProducer, RustDeterminismReadinessProject,
    RustDeterminismReadinessProtocolId, RustDeterminismReadinessProtocolVersion,
    RustDeterminismReadinessProviderId, RustDeterminismReadinessSchemaId,
    RustDeterminismReadinessSchemaVersion, RustDeterminismReadinessSeverity,
    RustDeterminismReadinessStatus, RustDeterminismReadinessSuggestion,
    RustDeterminismReadinessSuggestionKind, RustDeterminismReadinessSuggestionMessage,
    RustDeterminismReadinessSummary, RustDeterminismReadinessSymbol,
    RustDeterminismReadinessTraitName, build_rust_determinism_readiness,
    render_rust_determinism_readiness, render_rust_determinism_readiness_json,
};

pub use verification::{
    RUST_ASSURANCE_CASE_PROTOCOL_ID, RUST_ASSURANCE_CASE_PROTOCOL_VERSION,
    RUST_ASSURANCE_CASE_SCHEMA_ID, RUST_ASSURANCE_CASE_SCHEMA_VERSION, RustAssuranceActionRef,
    RustAssuranceCase, RustAssuranceCaseInput, RustAssuranceCaseSet, RustAssuranceCaseSetProducer,
    RustAssuranceCaseSetProject, RustAssuranceCaseStatus, RustAssuranceCaseSummary,
    RustAssuranceClaim, RustAssuranceClaimKind, RustAssuranceGap, RustAssuranceNodeKind,
    RustAssuranceNodeRef, RustAssuranceNodeStatus, build_rust_assurance_case_set,
    render_rust_assurance_case_set, render_rust_assurance_case_set_json,
};
pub use verification::{
    RUST_EVIDENCE_GRAPH_ANALYSIS_PACKET_KIND, RUST_EVIDENCE_GRAPH_ANALYSIS_PROFILE,
    RUST_EVIDENCE_GRAPH_ANALYSIS_REQUEST_SCHEMA_ID,
    RUST_EVIDENCE_GRAPH_ANALYSIS_REQUEST_SCHEMA_VERSION, RUST_EVIDENCE_GRAPH_PROTOCOL_ID,
    RUST_EVIDENCE_GRAPH_PROTOCOL_VERSION, RUST_EVIDENCE_GRAPH_SCHEMA_ID,
    RUST_EVIDENCE_GRAPH_SCHEMA_VERSION, RustEvidenceEdge, RustEvidenceEdgeKind, RustEvidenceGap,
    RustEvidenceGraph, RustEvidenceGraphAnalysisGraph, RustEvidenceGraphAnalysisInput,
    RustEvidenceGraphAnalysisPacketKind, RustEvidenceGraphAnalysisProducer,
    RustEvidenceGraphAnalysisRequest, RustEvidenceGraphAnalysisSummary, RustEvidenceGraphInput,
    RustEvidenceGraphProducer, RustEvidenceGraphProject, RustEvidenceGraphSummary,
    RustEvidenceLocation, RustEvidenceNode, RustEvidenceNodeKind, RustEvidenceNodeStatus,
    build_rust_evidence_graph, build_rust_evidence_graph_analysis_request,
    render_rust_evidence_graph, render_rust_evidence_graph_analysis_request,
    render_rust_evidence_graph_analysis_request_json, render_rust_evidence_graph_json,
};
pub use verification::{
    RUST_FORMAL_PROOF_PILOT_PROTOCOL_ID, RUST_FORMAL_PROOF_PILOT_PROTOCOL_VERSION,
    RUST_FORMAL_PROOF_PILOT_SCHEMA_ID, RUST_FORMAL_PROOF_PILOT_SCHEMA_VERSION,
    RustFormalProofPilot, RustFormalProofPilotCheck, RustFormalProofPilotCheckId,
    RustFormalProofPilotClaim, RustFormalProofPilotClaimId, RustFormalProofPilotCounterexample,
    RustFormalProofPilotId, RustFormalProofPilotInput, RustFormalProofPilotLanguageId,
    RustFormalProofPilotMethod, RustFormalProofPilotMethodKind, RustFormalProofPilotNamespace,
    RustFormalProofPilotProducer, RustFormalProofPilotProtocolId,
    RustFormalProofPilotProtocolVersion, RustFormalProofPilotProviderId,
    RustFormalProofPilotSchemaId, RustFormalProofPilotSchemaVersion, RustFormalProofPilotStatement,
    RustFormalProofPilotStatus, RustFormalProofPilotSummary, RustFormalProofPilotTarget,
    RustFormalProofPilotTargetKind, RustFormalProofPilotTargetName, RustFormalProofPilotTool,
    build_rust_dependency_graph_acyclicity_proof_pilot, render_rust_formal_proof_pilot,
    render_rust_formal_proof_pilot_json,
};
pub use verification::{
    RUST_REVIEW_PACKET_PROTOCOL_ID, RUST_REVIEW_PACKET_PROTOCOL_VERSION,
    RUST_REVIEW_PACKET_SCHEMA_ID, RUST_REVIEW_PACKET_SCHEMA_VERSION, RustReviewPacket,
    RustReviewPacketInput, RustReviewPacketReceiptKind, RustReviewPacketWaiver,
    RustReviewPacketWaiverStatus, build_rust_review_packet, render_rust_review_packet,
    render_rust_review_packet_json,
};
pub use verification::{
    RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_ID,
    RUST_VERIFICATION_EXECUTION_RECEIPT_PROTOCOL_VERSION,
    RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_ID,
    RUST_VERIFICATION_EXECUTION_RECEIPT_SCHEMA_VERSION, RustVerificationExecutionAdapterId,
    RustVerificationExecutionArtifact, RustVerificationExecutionArtifactKind,
    RustVerificationExecutionArtifactUri, RustVerificationExecutionCommand,
    RustVerificationExecutionDurationMs, RustVerificationExecutionExitCode,
    RustVerificationExecutionLanguageId, RustVerificationExecutionLine,
    RustVerificationExecutionNamespace, RustVerificationExecutionObservation,
    RustVerificationExecutionObservationKind, RustVerificationExecutionObservationMessage,
    RustVerificationExecutionObservedAt, RustVerificationExecutionOutputFormat,
    RustVerificationExecutionPackageName, RustVerificationExecutionProducer,
    RustVerificationExecutionProject, RustVerificationExecutionProjectName,
    RustVerificationExecutionProtocolId, RustVerificationExecutionProtocolVersion,
    RustVerificationExecutionProviderId, RustVerificationExecutionReceipt,
    RustVerificationExecutionReceiptId, RustVerificationExecutionSchemaId,
    RustVerificationExecutionSchemaVersion, RustVerificationExecutionSha256,
    RustVerificationExecutionStatus, RustVerificationExecutionSummary,
    RustVerificationExecutionTaskFingerprint, RustVerificationExecutionTool,
    RustVerificationToolAdapter,
};
#[cfg(test)]
#[path = "../tests/unit/discovery.rs"]
mod discovery_tests;

#[cfg(test)]
#[path = "../tests/unit/runner.rs"]
mod runner_tests;

#[cfg(test)]
#[path = "../tests/unit/parser_source_path.rs"]
mod parser_source_path_tests;

#[cfg(test)]
#[path = "../tests/unit/parser_reasoning_tree.rs"]
mod parser_reasoning_tree_tests;

#[cfg(test)]
#[path = "../tests/unit/parser_native_syntax.rs"]
mod parser_native_syntax_tests;

#[cfg(test)]
#[path = "../tests/unit/parser_native_syntax/control_flow.rs"]
mod parser_native_syntax_control_flow_tests;

#[cfg(test)]
#[path = "../tests/unit/parser_native_syntax/signature.rs"]
mod parser_native_syntax_signature_tests;

#[cfg(test)]
#[path = "../tests/unit/parser_native_syntax/api_shape.rs"]
mod parser_native_syntax_api_shape_tests;

#[cfg(test)]
#[path = "../tests/unit/parser_native_syntax/data_shape.rs"]
mod parser_native_syntax_data_shape_tests;

pub use agent_snapshot::{
    render_rust_project_harness_agent_snapshot,
    render_rust_project_harness_agent_snapshot_with_config,
};
pub use build_gate::{
    RUST_PROJECT_HARNESS_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_ID,
    RUST_PROJECT_HARNESS_DOWNSTREAM_POLICY_RECEIPT_SCHEMA_VERSION,
    RustProjectHarnessDependencyBaseline, RustProjectHarnessDependencyBaselinePackage,
    RustProjectHarnessDependencyBaselinePackageReceipt, RustProjectHarnessDownstreamPolicy,
    RustProjectHarnessDownstreamPolicyReceipt, RustProjectHarnessReportObligationReceipt,
    RustProjectHarnessWorkspacePolicy, assert_rust_project_harness_dependency_baseline,
    assert_rust_project_harness_downstream_policy,
    assert_rust_project_harness_downstream_policy_from_env,
    assert_rust_project_harness_verification_from_env_with_config,
    assert_rust_project_harness_verification_with_config,
    render_rust_project_harness_downstream_policy_receipt_json,
    rust_project_harness_downstream_policy_receipt,
};
#[cfg(feature = "cli")]
pub use cli::run_cli_from_env;
pub use discovery::{DEFAULT_IGNORED_DIR_NAMES, discover_rust_files, rust_project_harness_scope};
pub use downstream_gate_guide::{
    RUST_DOWNSTREAM_VERIFICATION_GATE_GUIDE_MD, rust_downstream_verification_gate_guide_markdown,
};
pub use model::{
    RulePackDescriptor, RustDiagnosticSeverity, RustHarnessConfig, RustHarnessFinding,
    RustHarnessReport, RustHarnessRule, RustInvariantCandidate, RustInvariantCandidateStatus,
    RustInvariantEvidence, RustInvariantEvidenceKind, RustInvariantId, RustInvariantKind,
    RustInvariantReceiptKind, RustInvariantRulePackId, RustInvariantSourceRuleId, RustModuleReport,
    RustProjectHarnessScope, RustRulePack, SourceLocation,
};
pub use render::{
    render_rust_project_harness, render_rust_project_harness_advice,
    render_rust_project_harness_failure_frontier, render_rust_project_harness_json,
    render_rust_project_harness_with_options,
};
pub use rules::{
    rust_agent_policy_rules, rust_modularity_rules, rust_project_policy_rules,
    rust_rule_pack_descriptors, rust_syntax_rules,
};
pub use runner::{
    RustHarnessRunScope, assert_rust_lang_harness_clean,
    assert_rust_project_harness_cargo_test_clean,
    assert_rust_project_harness_cargo_test_clean_with_config, assert_rust_project_harness_clean,
    assert_rust_project_harness_clean_with_config, default_rust_harness_config,
    run_rust_lang_harness, run_rust_lang_harness_with_config, run_rust_project_harness_for_scope,
    run_rust_project_harness_with_config_for_scope, rust_harness_config_for_project,
};
#[cfg(feature = "search")]
pub use search::{
    RustSearchOptions, RustSearchViewRequest,
    render_rust_project_harness_search_compare_json_with_config,
    render_rust_project_harness_search_ingest_with_config,
    render_rust_project_harness_search_prime, render_rust_project_harness_search_prime_with_config,
    render_rust_project_harness_search_view_with_config,
};
#[cfg(all(feature = "cli", feature = "search"))]
pub use search::{
    render_rust_project_harness_dependency_topology_json,
    render_rust_project_harness_dependency_topology_metadata_json,
    render_rust_project_harness_search_semantic_facts_json,
    render_rust_project_harness_workspace_scope_json,
};
pub use verification::{
    RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID, RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION,
    RustOwnerResponsibility, RustVerificationAnalysisProfile, RustVerificationApiPathBaseline,
    RustVerificationDependencySignal, RustVerificationEvidence,
    RustVerificationPackageAnalysisProfile, RustVerificationPerformanceIndex,
    RustVerificationPerformanceRecord, RustVerificationPhase, RustVerificationPlan,
    RustVerificationPolicy, RustVerificationProfileCandidate,
    RustVerificationProfileCandidateState, RustVerificationProfileHint,
    RustVerificationProfileIndex, RustVerificationReceipt, RustVerificationReceiptStatus,
    RustVerificationReportArtifact, RustVerificationReportArtifactRenderError,
    RustVerificationReportArtifactRole, RustVerificationReportArtifactWriteReceipt,
    RustVerificationReportBundle, RustVerificationReportEntryAction,
    RustVerificationReportEntryAdvice, RustVerificationReportEntryArtifact,
    RustVerificationReportEntrySidecar, RustVerificationReportManifestCompatibility,
    RustVerificationReportManifestSchema, RustVerificationReportMaterializationAdvice,
    RustVerificationReportObligation, RustVerificationReportOptions,
    RustVerificationReportPersistence, RustVerificationReportSelectionAdvice,
    RustVerificationReportSelectionArtifact, RustVerificationReportSelectionReason,
    RustVerificationReportSelectionScale, RustVerificationReportSidecar,
    RustVerificationReportSidecarRole, RustVerificationReportSidecarWriteReceipt,
    RustVerificationReportTemplate, RustVerificationReportTraceConfig,
    RustVerificationReportWriteConfig, RustVerificationReportWriteError,
    RustVerificationReportWriteReceipt, RustVerificationRequirement,
    RustVerificationResolutionNote, RustVerificationSkillBinding, RustVerificationSkillDescriptor,
    RustVerificationStabilityBaselineDelta, RustVerificationStabilityDurationSeconds,
    RustVerificationStabilityIndex, RustVerificationStabilityIterationCount,
    RustVerificationStabilityMetricDelta, RustVerificationStabilityPicture,
    RustVerificationStabilityPictureConfig, RustVerificationStabilityPictureConfigReview,
    RustVerificationStabilityPictureConfigWarning, RustVerificationStabilityPictureRecord,
    RustVerificationStabilityRecord, RustVerificationStabilityRunReceipt, RustVerificationTask,
    RustVerificationTaskContract, RustVerificationTaskIndex, RustVerificationTaskKind,
    RustVerificationTaskRecord, RustVerificationTaskState, RustVerificationTraceMaxSeconds,
    RustVerificationTraceSampleIntervalMs, RustVerificationWaiver,
    build_rust_verification_analysis_profile, build_rust_verification_analysis_profile_with_config,
    build_rust_verification_performance_index, build_rust_verification_profile_index,
    build_rust_verification_profile_index_with_config,
    build_rust_verification_profile_index_with_policy, build_rust_verification_report_bundle,
    build_rust_verification_report_bundle_with_options,
    build_rust_verification_report_entry_advice,
    build_rust_verification_report_entry_advice_with_receipt,
    build_rust_verification_report_selection_advice, build_rust_verification_stability_index,
    build_rust_verification_stability_picture,
    build_rust_verification_stability_picture_from_index,
    build_rust_verification_stability_picture_from_index_with_policy,
    build_rust_verification_stability_picture_with_policy, build_rust_verification_task_index,
    check_rust_verification_report_manifest_schema, compare_rust_verification_stability_runs,
    plan_rust_project_verification, plan_rust_project_verification_with_config,
    plan_rust_project_verification_with_policy, plan_rust_verification_from_harness_analysis,
    render_rust_verification_analysis_profile, render_rust_verification_analysis_profile_json,
    render_rust_verification_performance_index, render_rust_verification_performance_index_json,
    render_rust_verification_plan, render_rust_verification_plan_json,
    render_rust_verification_profile_index, render_rust_verification_profile_index_json,
    render_rust_verification_report_artifact_json,
    render_rust_verification_report_artifact_json_with_config,
    render_rust_verification_report_bundle, render_rust_verification_report_bundle_json,
    render_rust_verification_report_entry_advice,
    render_rust_verification_report_entry_advice_json,
    render_rust_verification_report_manifest_compatibility,
    render_rust_verification_report_manifest_schema_compatibility,
    render_rust_verification_report_selection_advice,
    render_rust_verification_report_selection_advice_json,
    render_rust_verification_report_write_receipt,
    render_rust_verification_report_write_receipt_json, render_rust_verification_skill_contracts,
    render_rust_verification_stability_index, render_rust_verification_stability_index_json,
    render_rust_verification_stability_picture, render_rust_verification_stability_picture_json,
    render_rust_verification_task_index_json, write_rust_verification_reports,
    write_rust_verification_reports_with_options,
};
pub use verification::{
    RustScenarioBenchmarkContract, RustScenarioBenchmarkDuration, RustScenarioBenchmarkError,
    RustScenarioBenchmarkManifestKind, RustScenarioBenchmarkMemoryBytes,
    RustScenarioBenchmarkPhase, RustScenarioBenchmarkReceipt, RustScenarioBenchmarkRequirement,
    RustScenarioBenchmarkStatus, RustScenarioBenchmarkSuiteReceipt, RustScenarioBenchmarkViolation,
    RustScenarioBenchmarkViolationKind, RustScenarioMetadata,
    assert_rule_fixture_scenario_benchmarks, discover_required_rust_scenario_benchmarks,
    validate_required_rust_scenario_benchmarks, validate_rust_scenario_benchmark,
};
pub use workspace_evidence_graph::{
    RUST_PROJECT_HARNESS_WORKSPACE_EVIDENCE_GRAPH_RECEIPT_SCHEMA_ID,
    RUST_PROJECT_HARNESS_WORKSPACE_EVIDENCE_GRAPH_RECEIPT_SCHEMA_VERSION,
    RustProjectHarnessVerificationTaskKindCountReceipt,
    RustProjectHarnessWorkspaceEvidenceGraphEdgeKind,
    RustProjectHarnessWorkspaceEvidenceGraphEdgeReceipt,
    RustProjectHarnessWorkspaceEvidenceGraphMemberInput,
    RustProjectHarnessWorkspaceEvidenceGraphMemberReceipt,
    RustProjectHarnessWorkspaceEvidenceGraphNodeKind,
    RustProjectHarnessWorkspaceEvidenceGraphNodeReceipt,
    RustProjectHarnessWorkspaceEvidenceGraphReceipt,
    RustProjectHarnessWorkspaceEvidenceGraphSummaryReceipt,
    RustProjectHarnessWorkspaceTrustLoopStepReceipt,
    RustProjectHarnessWorkspaceTrustLoopStepStatus,
    render_rust_project_harness_workspace_evidence_graph_receipt_json,
    rust_project_harness_workspace_evidence_graph_receipt,
};

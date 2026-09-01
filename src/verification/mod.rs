//! Parser-native verification task planning for external agent skills.

mod analysis;
mod api_path;
mod assurance_case;
mod behavior_snapshot;
mod determinism_readiness;
mod evidence_graph;
mod execution_receipt;
mod fingerprint;
mod formal_proof_pilot;
mod model;
mod module_lookup;
mod performance;
mod planner;
mod profile;
mod profile_index;
mod render;
mod report;
mod report_artifact;
mod report_entry;
mod report_manifest;
mod report_options;
mod report_select;
mod report_write;
mod review_packet;
mod scenario_benchmark;
mod skill_descriptor;
mod stability;
mod stability_config;
mod stability_picture;
mod stability_runtime;
mod task_builder;
mod task_index;

pub use analysis::{
    RustVerificationAnalysisProfile, RustVerificationPackageAnalysisProfile,
    build_rust_verification_analysis_profile, build_rust_verification_analysis_profile_with_config,
    render_rust_verification_analysis_profile, render_rust_verification_analysis_profile_json,
};
#[allow(unused_imports)]
pub use assurance_case::{
    RUST_ASSURANCE_CASE_PROTOCOL_ID, RUST_ASSURANCE_CASE_PROTOCOL_VERSION,
    RUST_ASSURANCE_CASE_SCHEMA_ID, RUST_ASSURANCE_CASE_SCHEMA_VERSION, RustAssuranceActionRef,
    RustAssuranceCase, RustAssuranceCaseInput, RustAssuranceCaseSet, RustAssuranceCaseSetProducer,
    RustAssuranceCaseSetProject, RustAssuranceCaseStatus, RustAssuranceCaseSummary,
    RustAssuranceClaim, RustAssuranceClaimKind, RustAssuranceGap, RustAssuranceNodeKind,
    RustAssuranceNodeRef, RustAssuranceNodeStatus, build_rust_assurance_case_set,
    render_rust_assurance_case_set, render_rust_assurance_case_set_json,
};
pub use behavior_snapshot::{
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
pub use determinism_readiness::{
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
#[allow(unused_imports)]
pub use evidence_graph::{
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
pub use execution_receipt::{
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
pub use formal_proof_pilot::{
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
pub use model::{
    RustOwnerResponsibility, RustVerificationApiPathBaseline, RustVerificationDependencySignal,
    RustVerificationEvidence, RustVerificationPhase, RustVerificationPlan, RustVerificationPolicy,
    RustVerificationProfileHint, RustVerificationReceipt, RustVerificationReceiptStatus,
    RustVerificationReportObligation, RustVerificationRequirement, RustVerificationResolutionNote,
    RustVerificationSkillBinding, RustVerificationTask, RustVerificationTaskContract,
    RustVerificationTaskKind, RustVerificationTaskState, RustVerificationWaiver,
};
pub use performance::{
    RustVerificationPerformanceIndex, RustVerificationPerformanceRecord,
    build_rust_verification_performance_index, render_rust_verification_performance_index,
    render_rust_verification_performance_index_json,
};
pub use planner::plan_rust_verification_from_harness_analysis;
pub use planner::{
    plan_rust_project_verification, plan_rust_project_verification_with_config,
    plan_rust_project_verification_with_policy,
};
pub use profile_index::{
    RustVerificationProfileCandidate, RustVerificationProfileCandidateState,
    RustVerificationProfileIndex, build_rust_verification_profile_index,
    build_rust_verification_profile_index_with_config,
    build_rust_verification_profile_index_with_policy, render_rust_verification_profile_index,
    render_rust_verification_profile_index_json,
};
pub use render::{
    render_rust_verification_plan, render_rust_verification_plan_json,
    render_rust_verification_skill_contracts,
};
pub use report::{
    RustVerificationReportArtifact, RustVerificationReportBundle, RustVerificationReportSidecar,
    build_rust_verification_report_bundle, build_rust_verification_report_bundle_with_options,
    render_rust_verification_report_bundle, render_rust_verification_report_bundle_json,
};
pub use report_artifact::{
    RustVerificationReportArtifactRenderError, render_rust_verification_report_artifact_json,
    render_rust_verification_report_artifact_json_with_config,
};
pub use report_entry::{
    RustVerificationReportEntryAction, RustVerificationReportEntryAdvice,
    RustVerificationReportEntryArtifact, RustVerificationReportEntrySidecar,
    build_rust_verification_report_entry_advice,
    build_rust_verification_report_entry_advice_with_receipt,
    render_rust_verification_report_entry_advice,
    render_rust_verification_report_entry_advice_json,
};
pub use report_manifest::{
    RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_ID, RUST_VERIFICATION_REPORT_MANIFEST_SCHEMA_VERSION,
    RustVerificationReportManifestCompatibility, RustVerificationReportManifestSchema,
    check_rust_verification_report_manifest_schema,
    render_rust_verification_report_manifest_compatibility,
    render_rust_verification_report_manifest_schema_compatibility,
};
pub use report_options::{
    RustVerificationReportArtifactRole, RustVerificationReportOptions,
    RustVerificationReportPersistence, RustVerificationReportSidecarRole,
    RustVerificationReportTemplate, RustVerificationReportTraceConfig,
    RustVerificationTraceMaxSeconds, RustVerificationTraceSampleIntervalMs,
};
pub use report_select::{
    RustVerificationReportSelectionAdvice, RustVerificationReportSelectionArtifact,
    RustVerificationReportSelectionReason, RustVerificationReportSelectionScale,
    build_rust_verification_report_selection_advice,
    render_rust_verification_report_selection_advice,
    render_rust_verification_report_selection_advice_json,
};
pub use report_write::{
    RustVerificationReportArtifactWriteReceipt, RustVerificationReportMaterializationAdvice,
    RustVerificationReportSidecarWriteReceipt, RustVerificationReportWriteConfig,
    RustVerificationReportWriteError, RustVerificationReportWriteReceipt,
    render_rust_verification_report_write_receipt,
    render_rust_verification_report_write_receipt_json, write_rust_verification_reports,
    write_rust_verification_reports_with_options,
};
pub use review_packet::{
    RUST_REVIEW_PACKET_PROTOCOL_ID, RUST_REVIEW_PACKET_PROTOCOL_VERSION,
    RUST_REVIEW_PACKET_SCHEMA_ID, RUST_REVIEW_PACKET_SCHEMA_VERSION, RustReviewPacket,
    RustReviewPacketInput, RustReviewPacketReceiptKind, RustReviewPacketWaiver,
    RustReviewPacketWaiverStatus, build_rust_review_packet, render_rust_review_packet,
    render_rust_review_packet_json,
};
pub use scenario_benchmark::{
    RustScenarioBenchmarkContract, RustScenarioBenchmarkDuration, RustScenarioBenchmarkError,
    RustScenarioBenchmarkManifestKind, RustScenarioBenchmarkMeasurement,
    RustScenarioBenchmarkMemoryBytes, RustScenarioBenchmarkMetric, RustScenarioBenchmarkMetricKind,
    RustScenarioBenchmarkPhase, RustScenarioBenchmarkReceipt, RustScenarioBenchmarkRequirement,
    RustScenarioBenchmarkStatus, RustScenarioBenchmarkSuiteReceipt, RustScenarioBenchmarkViolation,
    RustScenarioBenchmarkViolationKind, RustScenarioMetadata,
    assert_rule_fixture_scenario_benchmarks, discover_required_rust_scenario_benchmarks,
    validate_required_rust_scenario_benchmarks, validate_rust_scenario_benchmark,
};
pub use skill_descriptor::RustVerificationSkillDescriptor;
pub use stability::{
    RustVerificationStabilityIndex, RustVerificationStabilityRecord,
    build_rust_verification_stability_index, render_rust_verification_stability_index,
    render_rust_verification_stability_index_json,
};
pub use stability_config::{
    RustVerificationStabilityPictureConfig, RustVerificationStabilityPictureConfigReview,
    RustVerificationStabilityPictureConfigWarning,
};
pub use stability_picture::{
    RustVerificationStabilityPicture, RustVerificationStabilityPictureRecord,
    build_rust_verification_stability_picture,
    build_rust_verification_stability_picture_from_index,
    build_rust_verification_stability_picture_from_index_with_policy,
    build_rust_verification_stability_picture_with_policy,
    render_rust_verification_stability_picture, render_rust_verification_stability_picture_json,
};
pub use stability_runtime::{
    RustVerificationStabilityBaselineDelta, RustVerificationStabilityDurationSeconds,
    RustVerificationStabilityIterationCount, RustVerificationStabilityMetricDelta,
    RustVerificationStabilityRunReceipt, compare_rust_verification_stability_runs,
};
pub use task_index::{
    RustVerificationTaskIndex, RustVerificationTaskRecord, build_rust_verification_task_index,
    render_rust_verification_task_index_json,
};

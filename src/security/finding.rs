//! Structured security findings and projection into `rig-compose` context.
//!
//! [`SecurityFinding`] is the security-side counterpart to memory hits,
//! behavior patterns, baselines, and graph expansions: a typed record that
//! a host or detector can hand to [`security_finding_to_context_item`] to
//! produce a [`ContextItem`] suitable for `rig_compose::ContextPack::pack`. The
//! projection attaches the shared provenance keys (`source_uri`,
//! `principal`, `scope`, `recorded_at_millis`, `confidence`,
//! `projection_state`) plus security-specific keys (`finding_id`,
//! `severity`, `technique_id`, `tactic`, `source_skill`, `signals`,
//! `detail`) so eval harnesses, dashboards, and host policies can reason
//! about security context without parsing free-form evidence blobs.
//!
//! ```no_run
//! use rig_resources::security::{
//!     security_finding_to_context_item, FindingSeverity, SecurityFinding,
//! };
//!
//! let finding = SecurityFinding::new(
//!     "credential.password_spray",
//!     FindingSeverity::High,
//!     "burst of failed logins across distinct accounts",
//! )
//! .with_principal("host-1")
//! .with_signals(["auth.failure.burst"])
//! .with_technique_id("T1110.003");
//!
//! let item = security_finding_to_context_item(&finding, 0);
//! assert_eq!(item.provenance["finding_id"], "credential.password_spray");
//! assert_eq!(item.provenance["severity"], "high");
//! ```

use rig_compose::{ContextItem, ContextSourceKind};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::projection::IntoContextItem;
use crate::trace::ResourceTraceEnvelope;

const STATE_CANDIDATE: &str = "candidate";
const TRACE_RESOURCE: &str = "security";
const TRACE_OPERATION: &str = "finding";
const TRACE_KIND: &str = "security_finding";

/// Provider-neutral severity tier for a [`SecurityFinding`].
///
/// Severities map to a default confidence weight used when the finding does
/// not carry an explicit `confidence` score; see
/// [`FindingSeverity::confidence_weight`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FindingSeverity {
    /// Informational signal; not actionable on its own.
    Info,
    /// Low-severity finding; benign anomaly or background noise.
    Low,
    /// Medium-severity finding; worth correlating with other signals.
    Medium,
    /// High-severity finding; likely actionable in isolation.
    High,
    /// Critical finding; immediate triage expected.
    Critical,
}

impl FindingSeverity {
    /// Default confidence weight in `[0.0, 1.0]` associated with this
    /// severity tier.
    #[must_use]
    pub fn confidence_weight(self) -> f64 {
        match self {
            Self::Info => 0.10,
            Self::Low => 0.30,
            Self::Medium => 0.55,
            Self::High => 0.80,
            Self::Critical => 0.95,
        }
    }

    /// Stable lowercase string representation, matching the serde wire form.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Structured security finding produced by a security skill or detector.
///
/// Hosts construct findings explicitly; existing built-in security skills
/// continue to emit `rig_compose::Evidence` so this type is purely additive.
/// Callers that want a finding-aware projection can build a `SecurityFinding`
/// from their own detector output and feed it through
/// [`security_finding_to_context_item`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityFinding {
    /// Stable id identifying the finding kind (e.g. `recon.high_fanout`).
    pub id: String,
    /// Severity tier.
    pub severity: FindingSeverity,
    /// Human-readable summary of the finding.
    pub summary: String,
    /// Principal or entity this finding implicates (host, user, account, ip).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal: Option<String>,
    /// Caller-defined scope such as tenant, workspace, or project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// MITRE ATT&CK technique id (e.g. `T1110.003`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technique_id: Option<String>,
    /// MITRE ATT&CK tactic (e.g. `credential-access`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tactic: Option<String>,
    /// Id of the skill or detector that produced the finding.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_skill: Option<String>,
    /// Contributing signals (`auth.failure.burst`, ...).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub signals: Vec<String>,
    /// URI for the original source record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_uri: Option<String>,
    /// Millis since the Unix epoch when the underlying event was recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorded_at_millis: Option<i64>,
    /// Explicit confidence override in `[0.0, 1.0]`; defaults to the
    /// severity weight when unset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    /// Free-form structured detail attached to the finding.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub detail: Value,
}

impl SecurityFinding {
    /// Construct a finding with the required fields populated.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        severity: FindingSeverity,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            severity,
            summary: summary.into(),
            principal: None,
            scope: None,
            technique_id: None,
            tactic: None,
            source_skill: None,
            signals: Vec::new(),
            source_uri: None,
            recorded_at_millis: None,
            confidence: None,
            detail: Value::Null,
        }
    }

    /// Set [`Self::principal`].
    #[must_use]
    pub fn with_principal(mut self, principal: impl Into<String>) -> Self {
        self.principal = Some(principal.into());
        self
    }

    /// Set [`Self::scope`].
    #[must_use]
    pub fn with_scope(mut self, scope: impl Into<String>) -> Self {
        self.scope = Some(scope.into());
        self
    }

    /// Set [`Self::technique_id`].
    #[must_use]
    pub fn with_technique_id(mut self, technique_id: impl Into<String>) -> Self {
        self.technique_id = Some(technique_id.into());
        self
    }

    /// Set [`Self::tactic`].
    #[must_use]
    pub fn with_tactic(mut self, tactic: impl Into<String>) -> Self {
        self.tactic = Some(tactic.into());
        self
    }

    /// Set [`Self::source_skill`].
    #[must_use]
    pub fn with_source_skill(mut self, source_skill: impl Into<String>) -> Self {
        self.source_skill = Some(source_skill.into());
        self
    }

    /// Set [`Self::signals`] from any iterable of string-like values.
    #[must_use]
    pub fn with_signals<I, S>(mut self, signals: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.signals = signals.into_iter().map(Into::into).collect();
        self
    }

    /// Append a single signal.
    #[must_use]
    pub fn add_signal(mut self, signal: impl Into<String>) -> Self {
        self.signals.push(signal.into());
        self
    }

    /// Set [`Self::source_uri`].
    #[must_use]
    pub fn with_source_uri(mut self, source_uri: impl Into<String>) -> Self {
        self.source_uri = Some(source_uri.into());
        self
    }

    /// Set [`Self::recorded_at_millis`].
    #[must_use]
    pub fn with_recorded_at_millis(mut self, recorded_at_millis: i64) -> Self {
        self.recorded_at_millis = Some(recorded_at_millis);
        self
    }

    /// Set [`Self::confidence`] explicitly, overriding the severity weight.
    #[must_use]
    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = Some(confidence);
        self
    }

    /// Set [`Self::detail`].
    #[must_use]
    pub fn with_detail(mut self, detail: Value) -> Self {
        self.detail = detail;
        self
    }

    /// Effective confidence in `[0.0, 1.0]`, defaulting to the severity weight.
    #[must_use]
    pub fn effective_confidence(&self) -> f64 {
        self.confidence
            .unwrap_or_else(|| self.severity.confidence_weight())
    }

    /// Resolve the `source_uri` provenance key, synthesising one from the
    /// finding id when the caller did not provide one.
    #[must_use]
    pub fn resolved_source_uri(&self) -> String {
        self.source_uri
            .clone()
            .unwrap_or_else(|| format!("security-finding://{}", self.id))
    }
}

/// Project one [`SecurityFinding`] into a [`ContextItem`] at `rank`.
///
/// The resulting item is tagged with [`ContextSourceKind::Resource`] and
/// carries both the shared provenance vocabulary used by other resource
/// projections and the security-specific keys (`finding_id`, `severity`,
/// `technique_id`, `tactic`, `source_skill`, `signals`, `detail`).
#[must_use]
pub fn security_finding_to_context_item(finding: &SecurityFinding, rank: usize) -> ContextItem {
    let source_id = match &finding.principal {
        Some(principal) => format!("security.finding/{}/{}", finding.id, principal),
        None => format!("security.finding/{}", finding.id),
    };
    let source_uri = finding.resolved_source_uri();
    let confidence = finding.effective_confidence();

    ContextItem::new(
        ContextSourceKind::Resource,
        source_id,
        finding.summary.clone(),
    )
    .with_rank(rank)
    .with_score(confidence)
    .with_provenance(json!({
        "resource": "security.finding",
        "source_uri": source_uri,
        "principal": finding.principal,
        "scope": finding.scope,
        "recorded_at_millis": finding.recorded_at_millis,
        "confidence": confidence,
        "projection_state": STATE_CANDIDATE,
        "finding_id": finding.id,
        "severity": finding.severity.as_str(),
        "technique_id": finding.technique_id,
        "tactic": finding.tactic,
        "source_skill": finding.source_skill,
        "signals": finding.signals,
        "detail": finding.detail,
    }))
}

/// Project a slice of findings into ranked context items.
#[must_use]
pub fn security_findings_to_context_items(findings: &[SecurityFinding]) -> Vec<ContextItem> {
    findings
        .iter()
        .enumerate()
        .map(|(rank, finding)| security_finding_to_context_item(finding, rank))
        .collect()
}

impl IntoContextItem for SecurityFinding {
    fn to_context_item(&self) -> ContextItem {
        security_finding_to_context_item(self, 0)
    }
}

/// Build a [`ResourceTraceEnvelope`] describing the production of a security
/// finding.
///
/// This complements [`security_finding_to_context_item`] for callers that
/// want a trace-side record (input signals, output severity/confidence,
/// optional MITRE metadata) alongside the prompt-context projection.
#[must_use]
pub fn security_finding_trace_envelope(finding: &SecurityFinding) -> ResourceTraceEnvelope {
    let mut input = json!({
        "finding_id": finding.id,
        "principal": finding.principal,
        "signals": finding.signals,
    });
    if let Some(skill) = &finding.source_skill
        && let Some(map) = input.as_object_mut()
    {
        map.insert("source_skill".into(), Value::String(skill.clone()));
    }

    let output = json!({
        "severity": finding.severity.as_str(),
        "confidence": finding.effective_confidence(),
    });

    let mut envelope = ResourceTraceEnvelope::new(TRACE_RESOURCE, TRACE_OPERATION, TRACE_KIND)
        .with_input_summary(input)
        .with_output_summary(output);

    if finding.technique_id.is_some() || finding.tactic.is_some() {
        envelope = envelope.with_metadata(json!({
            "technique_id": finding.technique_id,
            "tactic": finding.tactic,
        }));
    }

    envelope
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_weights_are_monotonic() {
        let weights = [
            FindingSeverity::Info.confidence_weight(),
            FindingSeverity::Low.confidence_weight(),
            FindingSeverity::Medium.confidence_weight(),
            FindingSeverity::High.confidence_weight(),
            FindingSeverity::Critical.confidence_weight(),
        ];
        for window in weights.windows(2) {
            assert!(window[0] < window[1], "weights must be strictly increasing");
        }
        for w in weights {
            assert!((0.0..=1.0).contains(&w));
        }
    }

    #[test]
    fn projection_attaches_shared_and_security_provenance_keys() {
        let finding = SecurityFinding::new(
            "credential.password_spray",
            FindingSeverity::High,
            "burst of failed logins across distinct accounts",
        )
        .with_principal("host-1")
        .with_scope("workspace")
        .with_source_skill("credential.password_spray")
        .with_signals(["auth.failure.burst"])
        .with_technique_id("T1110.003")
        .with_tactic("credential-access")
        .with_recorded_at_millis(1_700_000_000_000)
        .with_source_uri("siem://event/42")
        .with_detail(json!({"distinct_accounts": 17}));

        let item = security_finding_to_context_item(&finding, 3);

        assert_eq!(item.source, ContextSourceKind::Resource);
        assert_eq!(
            item.source_id,
            "security.finding/credential.password_spray/host-1"
        );
        assert_eq!(item.rank, 3);
        assert!((item.score - FindingSeverity::High.confidence_weight()).abs() < 1e-9);

        let p = &item.provenance;
        // Shared vocabulary
        assert_eq!(p["resource"], "security.finding");
        assert_eq!(p["source_uri"], "siem://event/42");
        assert_eq!(p["principal"], "host-1");
        assert_eq!(p["scope"], "workspace");
        assert_eq!(p["recorded_at_millis"], 1_700_000_000_000_i64);
        assert_eq!(p["projection_state"], "candidate");
        let confidence = p["confidence"].as_f64().unwrap();
        assert!((confidence - FindingSeverity::High.confidence_weight()).abs() < 1e-9);
        // Security-specific
        assert_eq!(p["finding_id"], "credential.password_spray");
        assert_eq!(p["severity"], "high");
        assert_eq!(p["technique_id"], "T1110.003");
        assert_eq!(p["tactic"], "credential-access");
        assert_eq!(p["source_skill"], "credential.password_spray");
        assert_eq!(p["signals"][0], "auth.failure.burst");
        assert_eq!(p["detail"]["distinct_accounts"], 17);
    }

    #[test]
    fn synthesises_source_uri_and_source_id_when_missing() {
        let finding = SecurityFinding::new("recon.high_fanout", FindingSeverity::Medium, "fanout");
        let item = security_finding_to_context_item(&finding, 0);

        assert_eq!(item.source_id, "security.finding/recon.high_fanout");
        assert_eq!(
            item.provenance["source_uri"],
            "security-finding://recon.high_fanout"
        );
        assert!(item.provenance["principal"].is_null());
    }

    #[test]
    fn explicit_confidence_overrides_severity_weight() {
        let finding = SecurityFinding::new("exfil.slow_beacon", FindingSeverity::Low, "beacon")
            .with_confidence(0.99);
        let item = security_finding_to_context_item(&finding, 0);
        let confidence = item.provenance["confidence"].as_f64().unwrap();
        assert!((confidence - 0.99).abs() < 1e-9);
        assert!((item.score - 0.99).abs() < 1e-9);
    }

    #[test]
    fn batched_projection_preserves_rank() {
        let findings = vec![
            SecurityFinding::new("a", FindingSeverity::High, "first"),
            SecurityFinding::new("b", FindingSeverity::Low, "second"),
        ];
        let items = security_findings_to_context_items(&findings);
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].rank, 0);
        assert_eq!(items[1].rank, 1);
        assert_eq!(items[0].source_id, "security.finding/a");
        assert_eq!(items[1].source_id, "security.finding/b");
    }

    #[test]
    fn into_context_item_trait_works() {
        let finding = SecurityFinding::new(
            "lateral.auth_spawn_connect",
            FindingSeverity::Critical,
            "chain",
        );
        let item: ContextItem = finding.to_context_item();
        assert_eq!(item.provenance["severity"], "critical");
    }

    #[test]
    fn trace_envelope_captures_input_output_and_mitre() {
        let finding =
            SecurityFinding::new("lateral.auth_spawn_connect", FindingSeverity::High, "chain")
                .with_principal("host-9")
                .with_signals(["auth.success", "process.spawn", "net.connect"])
                .with_source_skill("lateral.auth_spawn_connect")
                .with_technique_id("T1021")
                .with_tactic("lateral-movement");

        let envelope = security_finding_trace_envelope(&finding);

        assert_eq!(envelope.version, ResourceTraceEnvelope::VERSION);
        assert_eq!(envelope.resource, "security");
        assert_eq!(envelope.operation, "finding");
        assert_eq!(envelope.trace_kind, "security_finding");
        assert_eq!(
            envelope.input_summary["finding_id"],
            "lateral.auth_spawn_connect"
        );
        assert_eq!(envelope.input_summary["principal"], "host-9");
        assert_eq!(
            envelope.input_summary["source_skill"],
            "lateral.auth_spawn_connect"
        );
        assert_eq!(envelope.input_summary["signals"][1], "process.spawn");
        assert_eq!(envelope.output_summary["severity"], "high");
        let confidence = envelope.output_summary["confidence"].as_f64().unwrap();
        assert!((confidence - FindingSeverity::High.confidence_weight()).abs() < 1e-9);
        assert_eq!(envelope.metadata["technique_id"], "T1021");
        assert_eq!(envelope.metadata["tactic"], "lateral-movement");
    }

    #[test]
    fn trace_envelope_omits_metadata_when_mitre_absent() {
        let finding = SecurityFinding::new("recon.high_fanout", FindingSeverity::Low, "fanout");
        let envelope = security_finding_trace_envelope(&finding);
        assert!(envelope.metadata.is_null());
    }

    #[test]
    fn finding_round_trips_through_json() {
        let finding = SecurityFinding::new(
            "credential.password_spray",
            FindingSeverity::Medium,
            "spray",
        )
        .with_principal("alice")
        .with_signals(["auth.failure.burst"]);
        let v = serde_json::to_value(&finding).unwrap();
        let decoded: SecurityFinding = serde_json::from_value(v).unwrap();
        assert_eq!(decoded, finding);
    }
}

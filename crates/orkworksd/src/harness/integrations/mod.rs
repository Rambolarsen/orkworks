//! Closed dispatch for compiled integration handlers.
//!
//! Task 5 intentionally supplies only truthful no-mutation stubs. Task 6
//! replaces each binding with its independently verified tool contract.

#![allow(dead_code)]

use crate::harness::definition::IntegrationBinding;
use crate::harness::integration::{
    DetectedTool, IntegrationActivation, IntegrationContext, IntegrationCoverage,
    IntegrationDiagnostic, IntegrationError, IntegrationHandler, IntegrationOwnership,
    IntegrationRegistration, IntegrationStatus,
};

pub(crate) fn handler(binding: &IntegrationBinding) -> &'static dyn IntegrationHandler {
    match binding {
        IntegrationBinding::Claude => &CLAUDE,
        IntegrationBinding::Codex => &CODEX,
        IntegrationBinding::OpenCode => &OPENCODE,
        IntegrationBinding::Gemini => &GEMINI,
        IntegrationBinding::Copilot => &COPILOT,
        IntegrationBinding::Aider => &AIDER,
    }
}

struct StubHandler {
    harness_id: &'static str,
}

impl StubHandler {
    fn status_for(&self, ctx: &IntegrationContext<'_>) -> IntegrationStatus {
        let detected_tool: Option<&DetectedTool> = ctx.detected_tool;
        IntegrationStatus {
            harness_id: self.harness_id.into(),
            enabled: ctx.enabled,
            tool_detected: detected_tool.is_some(),
            registration: IntegrationRegistration::Unsupported,
            ownership: IntegrationOwnership::None,
            activation: IntegrationActivation::NotApplicable,
            coverage: IntegrationCoverage::None,
            diagnostics: vec![IntegrationDiagnostic {
                code: "integration_not_implemented".into(),
                message: "This coding tool has no verified workspace integration handler yet."
                    .into(),
                action: None,
            }],
            confirmation: None,
        }
    }
}

impl IntegrationHandler for StubHandler {
    fn status(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        Ok(self.status_for(ctx))
    }

    fn install(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        Ok(self.status_for(ctx))
    }

    fn uninstall(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<IntegrationStatus, IntegrationError> {
        Ok(self.status_for(ctx))
    }
}

static CLAUDE: StubHandler = StubHandler {
    harness_id: "claude-code",
};
static CODEX: StubHandler = StubHandler {
    harness_id: "codex",
};
static OPENCODE: StubHandler = StubHandler {
    harness_id: "opencode",
};
static GEMINI: StubHandler = StubHandler {
    harness_id: "gemini",
};
static COPILOT: StubHandler = StubHandler {
    harness_id: "copilot",
};
static AIDER: StubHandler = StubHandler {
    harness_id: "aider",
};

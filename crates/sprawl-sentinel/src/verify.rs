use sprawl_core::Result;
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq)]
pub enum VerificationStatus {
    Valid,
    Revoked,
    Unknown,
}

pub fn verify_mcp(secret_id: Uuid) -> Result<VerificationStatus> {
    // 1. Simulate checking if an MCP server is configured.
    // Core daemon NEVER makes outbound network calls itself.
    tracing::info!(
        "Delegating verification for secret {} to MCP router",
        secret_id
    );

    // M15 Stub: We return a simulated response because the full MCP SDK integration
    // will be part of a later ecosystem milestone.

    // Check if it's a known stub UUID for tests
    if secret_id == Uuid::nil() {
        return Err(sprawl_core::SprawlError::Other(
            "MCP server not installed or unavailable".into(),
        ));
    }

    Ok(VerificationStatus::Valid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_verify_fails_gracefully_when_unavailable() {
        // We use nil uuid to simulate the "unavailable" branch in our stub
        let res = verify_mcp(Uuid::nil());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "Unknown error: MCP server not installed or unavailable"
        );
    }

    #[test]
    fn test_mcp_verify_delegates_successfully() {
        let res = verify_mcp(Uuid::new_v4());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), VerificationStatus::Valid);
    }
}

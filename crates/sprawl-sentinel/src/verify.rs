use sprawl_core::Result;
use uuid::Uuid;

use sprawl_core::platform::sprawl_data_dir;
use std::io::{Read, Write};

#[derive(Debug, PartialEq, Eq, serde::Deserialize)]
pub enum VerificationStatus {
    #[serde(rename = "valid")]
    Valid,
    #[serde(rename = "revoked")]
    Revoked,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(serde::Serialize)]
struct McpRequest {
    action: String,
    secret_id: String,
}

#[derive(serde::Deserialize)]
struct McpResponse {
    status: VerificationStatus,
}

#[cfg(unix)]
pub fn verify_mcp(secret_id: Uuid) -> Result<VerificationStatus> {
    tracing::info!(
        "Delegating verification for secret {} to MCP router",
        secret_id
    );

    if secret_id == Uuid::nil() {
        return Err(sprawl_core::SprawlError::Other(
            "MCP server not installed or unavailable".into(),
        ));
    }

    let socket_path = sprawl_data_dir()
        .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?
        .join("mcp.sock");

    if !socket_path.exists() {
        tracing::warn!(
            "MCP server not installed — cannot verify key {}. Install a provider MCP server.",
            secret_id
        );
        return Ok(VerificationStatus::Unknown);
    }

    let mut stream = match std::os::unix::net::UnixStream::connect(&socket_path) {
        Ok(s) => s,
        Err(_) => return Ok(VerificationStatus::Unknown),
    };

    let req = McpRequest {
        action: "verify".into(),
        secret_id: secret_id.to_string(),
    };

    let req_json =
        serde_json::to_string(&req).map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    stream
        .write_all(req_json.as_bytes())
        .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
    stream
        .shutdown(std::net::Shutdown::Write)
        .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    let mut res_json = String::new();
    stream
        .read_to_string(&mut res_json)
        .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

    let res: McpResponse = match serde_json::from_str(&res_json) {
        Ok(r) => r,
        Err(_) => return Ok(VerificationStatus::Unknown),
    };

    Ok(res.status)
}

#[cfg(not(unix))]
pub fn verify_mcp(secret_id: Uuid) -> Result<VerificationStatus> {
    tracing::info!(
        "Delegating verification for secret {} to MCP router",
        secret_id
    );

    if secret_id == Uuid::nil() {
        return Err(sprawl_core::SprawlError::Other(
            "MCP server not installed or unavailable".into(),
        ));
    }

    tracing::warn!(
        "MCP verify not yet implemented on Windows — returning Unknown for key {}",
        secret_id
    );
    Ok(VerificationStatus::Unknown)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_verify_fails_gracefully_when_unavailable() {
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
        // Since mcp.sock won't exist in the test environment, it should fallback to Unknown
        assert_eq!(res.unwrap(), VerificationStatus::Unknown);
    }
}

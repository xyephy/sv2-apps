use crate::key_utils::Secp256k1PublicKey;
use std::path::PathBuf;

/// Bitcoin network for determining node.sock location
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BitcoinNetwork {
    Mainnet,
    Testnet4,
    Signet,
    Regtest,
}

impl BitcoinNetwork {
    /// Returns the subdirectory name for this network.
    /// Mainnet uses the root data directory.
    fn subdir(&self) -> Option<&'static str> {
        match self {
            BitcoinNetwork::Mainnet => None,
            BitcoinNetwork::Testnet4 => Some("testnet4"),
            BitcoinNetwork::Signet => Some("signet"),
            BitcoinNetwork::Regtest => Some("regtest"),
        }
    }
}

/// Returns the default Bitcoin Core data directory for the current OS.
fn default_bitcoin_data_dir() -> Option<PathBuf> {
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir().map(|h| h.join(".bitcoin"))
    }
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir().map(|h| h.join("Library/Application Support/Bitcoin"))
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        None
    }
}

/// Resolves the IPC socket path from config options.
/// If unix_socket_path is provided, uses it directly.
/// Otherwise, constructs path from network + optional data_dir (or OS default).
pub fn resolve_ipc_socket_path(
    unix_socket_path: Option<PathBuf>,
    network: Option<&BitcoinNetwork>,
    data_dir: Option<PathBuf>,
) -> Result<PathBuf, String> {
    if let Some(path) = unix_socket_path {
        return Ok(path);
    }

    let network = network.ok_or(
        "BitcoinCoreIpc: either 'unix_socket_path' or 'network' must be specified".to_string(),
    )?;

    let base_dir = data_dir.or_else(default_bitcoin_data_dir).ok_or(
        "Could not determine Bitcoin data directory. Please specify 'unix_socket_path' or 'data_dir'."
            .to_string(),
    )?;

    let socket_path = match network.subdir() {
        Some(subdir) => base_dir.join(subdir).join("node.sock"),
        None => base_dir.join("node.sock"),
    };

    Ok(socket_path)
}

/// Which type of Template Provider will be used,
/// along with the relevant config parameters for each.
#[derive(Clone, Debug, serde::Deserialize)]
pub enum TemplateProviderType {
    Sv2Tp {
        address: String,
        public_key: Option<Secp256k1PublicKey>,
    },
    BitcoinCoreIpc {
        /// Explicit socket path. If set, takes priority.
        unix_socket_path: Option<PathBuf>,
        /// Network for auto-detecting socket path. Required if unix_socket_path not set.
        network: Option<BitcoinNetwork>,
        /// Custom Bitcoin data directory. Uses OS default if not set.
        data_dir: Option<PathBuf>,
        fee_threshold: u64,
        min_interval: u8,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_path_takes_priority() {
        let explicit = PathBuf::from("/custom/path/node.sock");
        let result = resolve_ipc_socket_path(
            Some(explicit.clone()),
            Some(&BitcoinNetwork::Regtest),
            Some(PathBuf::from("/other")),
        );
        assert_eq!(result.unwrap(), explicit);
    }

    #[test]
    fn network_with_data_dir_mainnet() {
        let result = resolve_ipc_socket_path(
            None,
            Some(&BitcoinNetwork::Mainnet),
            Some(PathBuf::from("/data")),
        );
        assert_eq!(result.unwrap(), PathBuf::from("/data/node.sock"));
    }

    #[test]
    fn network_with_data_dir_regtest() {
        let result = resolve_ipc_socket_path(
            None,
            Some(&BitcoinNetwork::Regtest),
            Some(PathBuf::from("/data")),
        );
        assert_eq!(result.unwrap(), PathBuf::from("/data/regtest/node.sock"));
    }

    #[test]
    fn network_with_data_dir_signet() {
        let result = resolve_ipc_socket_path(
            None,
            Some(&BitcoinNetwork::Signet),
            Some(PathBuf::from("/data")),
        );
        assert_eq!(result.unwrap(), PathBuf::from("/data/signet/node.sock"));
    }

    #[test]
    fn network_with_data_dir_testnet4() {
        let result = resolve_ipc_socket_path(
            None,
            Some(&BitcoinNetwork::Testnet4),
            Some(PathBuf::from("/data")),
        );
        assert_eq!(result.unwrap(), PathBuf::from("/data/testnet4/node.sock"));
    }

    #[test]
    fn missing_network_and_path_fails() {
        let result = resolve_ipc_socket_path(None, None, Some(PathBuf::from("/data")));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("either 'unix_socket_path' or 'network'"));
    }
}

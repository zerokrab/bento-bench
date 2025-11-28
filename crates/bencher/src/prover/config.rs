use crate::DEFAULT_BENTO_API_URL;
use boundless_market::Deployment;
use clap::Args;
use url::Url;

/// Configuration options for commands that utilize proving.
#[derive(Args, Debug, Clone)]
pub struct ProverConfig {
    /// RPC URL for the prover network
    #[clap(long = "rpc-url", env = "RPC_URL")]
    pub rpc_url: Option<Url>,
    /// Configuration for the Boundless deployment to use.
    #[clap(flatten, next_help_heading = "Boundless Deployment")]
    pub deployment: Option<Deployment>,
    /// Proving backend configuration
    #[clap(flatten, next_help_heading = "Proving Backend")]
    pub proving_backend: ProvingBackendConfig,
}

/// Configuration for the proving backend (Bento cluster or local prover)
#[derive(Args, Debug, Clone)]
pub struct ProvingBackendConfig {
    /// Bento API URL
    ///
    /// URL at which your Bento cluster is running.
    #[clap(
        long,
        env = "BENTO_API_URL",
        visible_alias = "bonsai-api-url",
        default_value = DEFAULT_BENTO_API_URL
    )]
    pub bento_api_url: String,

    /// Bento API Key
    ///
    /// Not necessary if using Bento without authentication, which is the default.
    #[clap(
        long,
        env = "BENTO_API_KEY",
        visible_alias = "bonsai-api-key",
        hide_env_values = true
    )]
    pub bento_api_key: Option<String>,

    /// Use the default prover instead of defaulting to Bento.
    ///
    /// When enabled, the prover selection follows the default zkVM behavior
    /// based on environment variables like RISC0_PROVER, RISC0_DEV_MODE, etc.
    #[clap(long, conflicts_with = "bento_api_url")]
    pub use_default_prover: bool,
}
impl ProvingBackendConfig {
    pub fn configure_proving_backend(&self) {
        if self.use_default_prover {
            println!("Using default prover behavior (respects RISC0_PROVER, RISC0_DEV_MODE, etc.)");
            return;
        }
        unsafe {
            std::env::set_var("BONSAI_API_URL", &self.bento_api_url);
            if let Some(ref api_key) = self.bento_api_key {
                std::env::set_var("BONSAI_API_KEY", api_key);
            } else {
                std::env::set_var("BONSAI_API_KEY", "v1:reserved:50");
            }
        }
    }
}

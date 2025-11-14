pub mod bench;
pub mod fetch;
pub mod prepare;

use boundless_market::Deployment;
use clap::Args;

use url::Url;

#[derive(Args, Clone, Debug)]
pub struct NetworkArgs {
    /// RPC URL for the prover network
    #[clap(long = "rpc-url", env = "RPC_URL")]
    pub rpc_url: Option<Url>,
    /// Configuration for the Boundless deployment to use.
    #[clap(flatten, next_help_heading = "Boundless Deployment")]
    pub deployment: Option<Deployment>,
}

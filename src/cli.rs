use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "ileap", about = "CLI tool for iLEAP API", version)]
pub struct Cli {
    /// API base URL
    #[arg(
        long,
        env = "ILEAP_BASE_URL",
        default_value = "https://ileap-preview.fly.dev"
    )]
    pub base_url: String,

    /// Bearer token (use `auth` to obtain one). When set, --username and --password are not required.
    #[arg(long, short = 't', env = "ILEAP_TOKEN")]
    pub token: Option<String>,

    /// Username for OAuth2 client credentials
    #[arg(long, short = 'u', env = "ILEAP_USERNAME")]
    pub username: Option<String>,

    /// Password for OAuth2 client credentials
    #[arg(long, short = 'p', env = "ILEAP_PASSWORD")]
    pub password: Option<String>,

    /// Output format
    #[arg(long, short = 'o', default_value = "pretty", value_enum)]
    pub output: OutputFormat,

    /// Request timeout in seconds
    #[arg(long, env = "ILEAP_TIMEOUT")]
    pub timeout: Option<u64>,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    /// Indented, human-readable JSON (default)
    Pretty,
    /// Compact single-line JSON, suited for machine consumption
    Compact,
}

#[derive(Subcommand)]
pub enum Command {
    /// PACT-based iLEAP data (DT1 and DT2) [/2/footprints]
    Footprints {
        #[command(subcommand)]
        cmd: FootprintsCmd,
    },
    /// iLEAP standalone ShipmentFootprints (DT1) [/v1/ileap/shipments]
    Shipments {
        #[command(subcommand)]
        cmd: ListCmd,
    },
    /// iLEAP standalone TOCs (DT2) [/v1/ileap/tocs]
    Tocs {
        #[command(subcommand)]
        cmd: ListCmd,
    },
    /// iLEAP standalone HOCs (DT2) [/v1/ileap/hocs]
    Hocs {
        #[command(subcommand)]
        cmd: ListCmd,
    },
    /// iLEAP Transport Activity Data (DT3) [/v1/ileap/tad]
    Tad {
        #[command(subcommand)]
        cmd: ListCmd,
    },
    /// iLEAP Aggregated Emissions Data (DT4) [/v1/ileap/aed]
    Aed {
        #[command(subcommand)]
        cmd: ListCmd,
    },
    /// Manage authentication
    Auth {
        #[command(subcommand)]
        cmd: AuthCmd,
    },
}

#[derive(Subcommand)]
pub enum AuthCmd {
    /// Authenticate and cache a token. Idempotent — skips re-auth if a valid token is already cached
    Login,
    /// Show whether a valid cached token exists
    Status,
}

#[derive(Subcommand)]
pub enum ListCmd {
    /// List records
    List(ListArgs),
}

#[derive(Subcommand)]
pub enum FootprintsCmd {
    /// List footprints
    List(ListArgs),
    /// Get a footprint by UUID
    Get {
        /// Footprint UUID
        id: String,
        /// Print the request that would be sent without executing it
        #[arg(long, short = 'n')]
        dry_run: bool,
    },
}

#[derive(Args, Clone, Debug, Default)]
pub struct ListArgs {
    /// Maximum number of results (page size; must be at least 1)
    #[arg(long, short = 'l', value_parser = clap::value_parser!(u32).range(1..))]
    pub limit: Option<u32>,

    /// Filter expression (repeatable).
    ///
    /// PACT-based endpoints use OData syntax: -f "created lt '2024-01-01T00:00:00Z'"
    ///
    /// iLEAP standalone endpoints use key=value pairs: -f mode=road
    /// To retrieve a single resource by ID: -f id=abc-123
    /// Dot notation for nested attributes: -f shipment.id=abc-123
    /// Interval filtering: -f created=gt:2024-01-01T00:00:00Z
    #[arg(long, short = 'f')]
    pub filter: Vec<String>,

    /// Print the request that would be sent without executing it
    #[arg(long, short = 'n')]
    pub dry_run: bool,

    /// Maximum number of pages to fetch when paginating
    #[arg(long, short = 'm')]
    pub max_pages: Option<u32>,
}

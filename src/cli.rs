use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "ileap", about = "CLI tool for iLEAP API", version)]
pub struct Cli {
    /// API base URL
    #[arg(long, env = "ILEAP_BASE_URL", default_value = "https://api.ileap.sine.dev")]
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

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    /// Compact JSON
    Json,
    /// Indented JSON
    Pretty,
}

#[derive(Subcommand)]
pub enum Command {
    /// PACT-based iLEAP data (DT1 and DT2) [/2/footprints]
    Footprints {
        #[command(subcommand)]
        cmd: FootprintsCmd,
    },
    /// iLEAP standalone ShipmentFootprints (DT1) [/v1/ileap/shipments]
    Shipments(ListArgs),
    /// iLEAP standalone TOCs (DT2) [/v1/ileap/tocs]
    Tocs(ListArgs),
    /// iLEAP standalone HOCs (DT2) [/v1/ileap/hocs]
    Hocs(ListArgs),
    /// iLEAP Transport Activity Data (DT3) [/v1/ileap/tad]
    Tad(ListArgs),
    /// iLEAP Aggregated Emissions Data (DT4) [/v1/ileap/aed]
    Aed(ListArgs),
}

#[derive(Subcommand)]
pub enum FootprintsCmd {
    /// List footprints
    List(ListArgs),
    /// Get a footprint by UUID
    Get {
        /// Footprint UUID
        id: String,
    },
}

#[derive(Args, Clone, Debug, Default)]
pub struct ListArgs {
    /// Maximum number of results
    #[arg(long, short = 'l')]
    pub limit: Option<u32>,

    /// Filter expression. PACT-based endpoints use OData syntax (e.g. "created lt '2024-01-01T00:00:00Z'").
    /// iLEAP standalone endpoints use key=value pairs (e.g. "mode=road"), repeatable. Filtering can be used to get a
    /// single resource by specifying its unique attributes. Dot notation can be used for nested attributes
    /// (e.g. "shipment.id=123"). Interval filtering is supported for iLEAP standalone endpoints using the syntax
    /// "key=gt:value" and "key=lt:value" (e.g. "created=gt:2024-01-01T00:00:00Z"). See the iLEAP Technical
    /// Specifications for more details.
    #[arg(long, short = 'f')]
    pub filter: Vec<String>,
}

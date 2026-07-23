use clap::{Parser, Subcommand};
use mycelia_node::config::NodeConfig;
use mycelia_node::control::{control_call, ControlRequest};
use mycelia_node::runtime::NodeRuntime;
use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "mycelisd", version, about = "Mycelia node daemon")]
struct Cli {
    #[arg(long, global = true, default_value = ".mycelis")]
    data_dir: PathBuf,

    #[arg(long, global = true)]
    control: Option<SocketAddr>,

    #[arg(long, global = true)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Start {
        #[arg(long, default_value = "127.0.0.1:0")]
        listen: SocketAddr,
        #[arg(long, default_value = "127.0.0.1:0")]
        control_bind: SocketAddr,
        #[arg(long, default_value = "rns")]
        transport: String,
        #[arg(long)]
        enable_dht: bool,
        #[arg(long)]
        enable_gateway: bool,
        #[arg(long, default_value = "127.0.0.1:1080")]
        gateway_bind: SocketAddr,
    },
    Status,
    #[command(subcommand)]
    Domains(DomainsCmd),
    #[command(subcommand)]
    Communities(CommunitiesCmd),
    #[command(subcommand)]
    Services(ServicesCmd),
    #[command(subcommand)]
    Peers(PeersCmd),
    #[command(subcommand)]
    Gateway(GatewayCmd),
}

#[derive(Subcommand, Debug)]
enum DomainsCmd {
    List,
    Create { name: String },
}

#[derive(Subcommand, Debug)]
enum CommunitiesCmd {
    List,
    Create { name: String },
    Invite {
        community: String,
        subject: String,
    },
}

#[derive(Subcommand, Debug)]
enum ServicesCmd {
    List,
    Advertise {
        name: String,
        #[arg(long)]
        domain: Option<String>,
        #[arg(long, default_value = "domain")]
        visibility: String,
    },
}

#[derive(Subcommand, Debug)]
enum PeersCmd {
    List,
}

#[derive(Subcommand, Debug)]
enum GatewayCmd {
    Status,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cli = Cli::parse();
    match cli.cmd {
        Commands::Start {
            listen,
            control_bind,
            transport,
            enable_dht,
            enable_gateway,
            gateway_bind,
        } => {
            let mut cfg = if let Some(path) = &cli.config {
                NodeConfig::load_or_default(path)?
            } else {
                NodeConfig::default()
            };
            cfg.data_dir = cli.data_dir;
            cfg.listen = listen;
            cfg.control_bind = control_bind;
            cfg.transport = transport;
            cfg.enable_dht = enable_dht;
            cfg.enable_gateway = enable_gateway;
            cfg.gateway_bind = gateway_bind;
            cfg.apply_env_overrides();
            let rt = NodeRuntime::start(cfg, None).await?;
            tracing::info!(
                "mycelisd started; control={} listen={}",
                rt.control_addr(),
                rt.handle.listen_addr
            );
            tokio::signal::ctrl_c().await?;
            rt.shutdown();
            Ok(())
        }
        other => {
            let addr = resolve_control(&cli.data_dir, cli.control)?;
            let req = match other {
                Commands::Status => ControlRequest::Status,
                Commands::Domains(DomainsCmd::List) => ControlRequest::DomainsList,
                Commands::Domains(DomainsCmd::Create { name }) => {
                    ControlRequest::DomainsCreate { name }
                }
                Commands::Communities(CommunitiesCmd::List) => ControlRequest::CommunitiesList,
                Commands::Communities(CommunitiesCmd::Create { name }) => {
                    ControlRequest::CommunitiesCreate { name }
                }
                Commands::Communities(CommunitiesCmd::Invite { community, subject }) => {
                    ControlRequest::CommunitiesInvite {
                        community_hex: community,
                        subject_hex: subject,
                    }
                }
                Commands::Services(ServicesCmd::List) => ControlRequest::ServicesList,
                Commands::Services(ServicesCmd::Advertise {
                    name,
                    domain,
                    visibility,
                }) => ControlRequest::ServicesAdvertise {
                    name,
                    domain_hex: domain,
                    visibility,
                },
                Commands::Peers(PeersCmd::List) => ControlRequest::PeersList,
                Commands::Gateway(GatewayCmd::Status) => ControlRequest::GatewayStatus,
                Commands::Start { .. } => unreachable!(),
            };
            let resp = control_call(addr, &req).await?;
            println!("{}", serde_json::to_string_pretty(&resp)?);
            if !resp.ok {
                std::process::exit(1);
            }
            Ok(())
        }
    }
}

fn resolve_control(
    data_dir: &std::path::Path,
    control: Option<SocketAddr>,
) -> anyhow::Result<SocketAddr> {
    if let Some(a) = control {
        return Ok(a);
    }
    let path = data_dir.join("control.addr");
    let s = std::fs::read_to_string(&path)
        .map_err(|_| anyhow::anyhow!("control.addr missing; is mycelisd start running?"))?;
    Ok(s.trim().parse()?)
}

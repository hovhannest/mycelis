use clap::{Parser, Subcommand};
use mycelia_node::config::NodeConfig;
use mycelia_node::control::{control_call, ControlRequest};
use mycelia_node::runtime::NodeRuntime;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "mycelisd", version, about = "Mycelia node daemon")]
struct Cli {
    #[arg(long, global = true, default_value = ".mycelis")]
    data_dir: PathBuf,

    #[arg(long, global = true)]
    control: Option<SocketAddr>,

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
}

#[derive(Subcommand, Debug)]
enum DomainsCmd {
    List,
    Create { name: String },
}

#[derive(Subcommand, Debug)]
enum CommunitiesCmd {
    List,
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
        } => {
            let cfg = NodeConfig {
                data_dir: cli.data_dir,
                listen,
                control_bind,
                ..Default::default()
            };
            let rt = NodeRuntime::start(cfg, None).await?;
            tracing::info!("mycelisd started; control={}", rt.control_addr());
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

#[allow(dead_code)]
fn _wait() {
    let _ = Duration::from_secs(1);
}

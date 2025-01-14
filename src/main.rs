#[cfg(feature = "mimalloc")]
#[global_allocator]
static ALLOC: mimalloc::MiMalloc = mimalloc::MiMalloc;

pub mod asset;
pub mod constant;
mod daemon;
mod install;
mod serve;
pub mod util;

use clap::{Args, Parser, Subcommand};
use std::io::{BufRead, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

pub trait Running {
    fn run(self) -> anyhow::Result<()>;
}

#[derive(Parser)]
#[clap(author, version, about, arg_required_else_help = true)]
#[command(args_conflicts_with_subcommands = true)]
struct Opt {
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Install thunder
    Install(InstallConfig),
    /// Uninstall thunder
    Uninstall,
    /// Run thunder
    Run(ServeConfig),
    /// Start thunder daemon
    Start(ServeConfig),
    /// Stop thunder daemon
    Stop,
    /// Show the Http server daemon process
    Status,
    /// Show the Http server daemon log
    Log,
}

#[derive(Args, Clone)]
pub struct InstallConfig {
    /// Thunder UID permission
    #[clap(short = 'U', long, env = "THUNDER_UID", default_value = "0")]
    uid: u32,
    /// Thunder GID permission
    #[clap(short = 'G', long, env = "THUNDER_GID", default_value = "0")]
    gid: u32,
    /// Install thunder from package
    package: Option<PathBuf>,
    /// Thunder config directory
    #[clap(short, long, default_value = constant::DEFAULT_CONFIG_PATH)]
    config_path: PathBuf,
    /// Thunder download directory
    #[clap(short, long, default_value = constant::DEFAULT_DOWNLOAD_PATH)]
    download_path: PathBuf,
    /// Thunder mount bind download directory
    #[clap(short, long, default_value = constant::DEFAULT_BIND_DOWNLOAD_PATH)]
    mount_bind_download_path: PathBuf,
}

impl InstallConfig {
    const PATH: &'static str = "/etc/.thunder";

    /// Remove config file
    pub fn remove_file(self) -> anyhow::Result<()> {
        let path = Path::new(Self::PATH);
        if path.exists() {
            std::fs::remove_file(&Self::PATH)?;
        }
        Ok(())
    }

    /// Write to file
    fn write_to_file(&self) -> anyhow::Result<()> {
        let path = Path::new(Self::PATH);
        if !path.exists() {
            let mut file = std::fs::File::create(path)?;
            writeln!(file, "uid={}", self.uid)?;
            writeln!(file, "gid={}", self.gid)?;
            writeln!(file, "config_path={}", self.config_path.display())?;
            writeln!(file, "download_path={}", self.download_path.display())?;
            writeln!(
                file,
                "mount_bind_download_path={}",
                self.mount_bind_download_path.display()
            )?;
            file.flush()?;
            drop(file)
        }
        Ok(())
    }

    /// Read from file
    fn read_from_file() -> anyhow::Result<Self> {
        let path = Path::new(Self::PATH);
        if !path.exists() {
            anyhow::bail!("`{}` not found", path.display());
        }

        let mut uid = 0;
        let mut gid = 0;
        let mut config_path = PathBuf::new();
        let mut download_path = PathBuf::new();
        let mut mount_bind_download_path = PathBuf::new();

        let file = std::fs::File::open(&Self::PATH)?;
        let reader = std::io::BufReader::new(file);
        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut split = line.split('=');
            let key = split.next().unwrap_or_default();
            let value = split.next().unwrap_or_default();
            match key {
                "uid" => {
                    uid = value.parse()?;
                }
                "gid" => {
                    gid = value.parse()?;
                }
                "config_path" => {
                    config_path = value.parse()?;
                }
                "download_path" => {
                    download_path = value.parse()?;
                }
                "mount_bind_download_path" => {
                    mount_bind_download_path = value.parse()?;
                }
                _ => {}
            }
        }

        Ok(Self {
            uid,
            gid,
            config_path,
            download_path,
            mount_bind_download_path,
            package: None,
        })
    }
}
#[derive(Args, Clone)]
pub struct ServeConfig {
    /// enable debug
    #[clap(long, env = "THUNDER_DEBUG")]
    debug: bool,
    /// Authentication password
    #[arg(short = 'w', long, env = "THUNDER_AUTH_PASS")]
    auth_password: Option<String>,
    /// Thunder server bind address
    #[clap(
        short = 'B',
        long,
        env = "THUNDER_BIND",
        default_value = "0.0.0.0:5055"
    )]
    bind: SocketAddr,
    /// TLS certificate file
    #[clap(short = 'C', long, env = "THUNDER_TLS_CERT")]
    tls_cert: Option<PathBuf>,
    /// TLS private key file
    #[clap(short = 'K', long, env = "THUNDER_TLS_KEY")]
    tls_key: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let opt = Opt::parse();

    match opt.commands {
        Commands::Install(config) => {
            config.write_to_file()?;
            install::XunleiInstall(config).run()?;
        }
        Commands::Uninstall => {
            let install_config = InstallConfig::read_from_file().map_or(None, |v| Some(v));
            install::XunleiUninstall(install_config).run()?;
        }
        Commands::Run(config) => {
            serve::Serve::new(config, InstallConfig::read_from_file()?).run()?;
        }
        Commands::Start(config) => {
            daemon::start()?;
            serve::Serve::new(config, InstallConfig::read_from_file()?).run()?;
        }
        Commands::Stop => {
            daemon::stop()?;
        }
        Commands::Status => {
            daemon::status()?;
        }
        Commands::Log => {
            daemon::log()?;
        }
    }
    Ok(())
}

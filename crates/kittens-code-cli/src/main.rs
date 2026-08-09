#![forbid(unsafe_code)]
//! `kittens-code`: the KC0 headless composition-root binary.

use std::env;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use kittens_code_cli::{fresh_header, run};
use kittens_code_driver_tokio::model::{JailClient, JailStep, ModelClient};
use kittens_code_driver_tokio::runner::Runner;
use kittens_code_protocol::config::SessionConfig;
use kittens_code_protocol::ids::SessionId;
use tokio::io::{BufReader, BufWriter};

const HELP: &str = "\
Usage: kittens-code [OPTIONS]

Reads one Op JSON object per stdin line and writes Event JSONL to stdout.

Options:
  --log PATH             Session log [env: KITTENS_CODE_LOG]
  --root PATH            Workspace root [env: KITTENS_CODE_ROOT]
  --backend jail|live    Model backend [env: KITTENS_CODE_BACKEND]
  --scenario PATH        Jail scenario JSON [env: KITTENS_CODE_SCENARIO]
  -h, --help             Print help

Defaults: ./kittens-code-session.jsonl, current directory, jail backend,
and ./kittens-code-scenario.json. The live backend requires --features live,
KITTENS_CODE_API_KEY, and KITTENS_CODE_MODEL_ID; KITTENS_CODE_ENDPOINT is
optional.";

#[derive(Clone, Copy)]
enum Backend {
    Jail,
    Live,
}

struct Config {
    log: PathBuf,
    root: PathBuf,
    backend: Backend,
    scenario: PathBuf,
}

impl Config {
    fn parse() -> Result<Option<Self>, String> {
        let mut log = env::var_os("KITTENS_CODE_LOG").map(PathBuf::from);
        let mut root = env::var_os("KITTENS_CODE_ROOT").map(PathBuf::from);
        let mut scenario = env::var_os("KITTENS_CODE_SCENARIO").map(PathBuf::from);
        let mut backend = env::var("KITTENS_CODE_BACKEND")
            .ok()
            .map(|value| parse_backend(&value))
            .transpose()?;
        let mut args = env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-h" | "--help" => return Ok(None),
                "--log" => log = Some(PathBuf::from(next_value(&mut args, "--log")?)),
                "--root" => root = Some(PathBuf::from(next_value(&mut args, "--root")?)),
                "--scenario" => {
                    scenario = Some(PathBuf::from(next_value(&mut args, "--scenario")?));
                }
                "--backend" => {
                    backend = Some(parse_backend(&next_value(&mut args, "--backend")?)?);
                }
                _ => return Err(format!("unknown argument: {arg}")),
            }
        }

        let root = match root {
            Some(root) => root,
            None => env::current_dir()
                .map_err(|error| format!("could not determine the current directory: {error}"))?,
        };
        Ok(Some(Self {
            log: log.unwrap_or_else(|| PathBuf::from("./kittens-code-session.jsonl")),
            root,
            backend: backend.unwrap_or(Backend::Jail),
            scenario: scenario.unwrap_or_else(|| PathBuf::from("./kittens-code-scenario.json")),
        }))
    }
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn parse_backend(value: &str) -> Result<Backend, String> {
    match value {
        "jail" => Ok(Backend::Jail),
        "live" => Ok(Backend::Live),
        _ => Err(format!("unknown backend {value:?}; expected jail or live")),
    }
}

fn jail_model(path: &std::path::Path) -> Result<Arc<dyn ModelClient>, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("could not read jail scenario {}: {error}", path.display()))?;
    let steps = serde_json::from_slice::<Vec<JailStep>>(&bytes)
        .map_err(|error| format!("could not parse jail scenario {}: {error}", path.display()))?;
    Ok(Arc::new(JailClient::new(steps)))
}

#[cfg(feature = "live")]
fn live_model() -> Result<Arc<dyn ModelClient>, String> {
    use kittens_code_driver_tokio::model::{LiveClient, LiveConfig, RetryConfig};

    let api_key = required_env("KITTENS_CODE_API_KEY")?;
    let model = required_env("KITTENS_CODE_MODEL_ID")?;
    let endpoint_base_url = env::var("KITTENS_CODE_ENDPOINT")
        .unwrap_or_else(|_| String::from("https://api.anthropic.com"));
    let client = LiveClient::new(LiveConfig {
        endpoint_base_url,
        api_key,
        model,
        max_output_tokens: 4_096,
        retry: RetryConfig::default(),
    })
    .map_err(|(code, message)| format!("live model configuration failed ({code:?}): {message}"))?;
    Ok(Arc::new(client))
}

#[cfg(not(feature = "live"))]
fn live_model() -> Result<Arc<dyn ModelClient>, String> {
    Err(String::from(
        "the live backend requires rebuilding kittens-code-cli with --features live",
    ))
}

#[cfg(feature = "live")]
fn required_env(name: &str) -> Result<String, String> {
    env::var(name).map_err(|_| format!("the live backend requires {name}"))
}

fn new_session_id() -> Result<SessionId, String> {
    let mut bytes = [0_u8; 16];
    File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut bytes))
        .map_err(|error| format!("could not obtain session-id entropy: {error}"))?;
    Ok(SessionId(bytes))
}

async fn entry(config: Config) -> Result<(), String> {
    let model = match config.backend {
        Backend::Jail => jail_model(&config.scenario)?,
        Backend::Live => live_model()?,
    };
    let header = if config.log.exists() {
        None
    } else {
        Some(
            fresh_header(new_session_id()?)
                .map_err(|error| format!("could not build fresh session header: {error:?}"))?,
        )
    };
    let mut runner = Runner::open(
        &config.log,
        header,
        SessionConfig::default(),
        model,
        config.root,
    )
    .map_err(|error| format!("could not open session {}: {error:?}", config.log.display()))?;

    let reader = BufReader::new(tokio::io::stdin());
    let writer = BufWriter::new(tokio::io::stdout());
    run(reader, writer, &mut runner)
        .await
        .map_err(|error| format!("protocol stream failed: {error}"))
}

#[tokio::main]
async fn main() -> ExitCode {
    let config = match Config::parse() {
        Ok(Some(config)) => config,
        Ok(None) => {
            println!("{HELP}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("kittens-code: {error}\n\n{HELP}");
            return ExitCode::FAILURE;
        }
    };
    match entry(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("kittens-code: {error}");
            ExitCode::FAILURE
        }
    }
}

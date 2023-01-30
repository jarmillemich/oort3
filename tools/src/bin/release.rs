use anyhow::{anyhow, bail, Result};
use clap::Parser as _;
use std::process::{ExitStatus, Output};
use tokio::process::Command;

const WORKSPACES: &[&str] = &["frontend", "tools", "shared", "services"];

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
enum Component {
    App,
    Telemetry,
    Leaderboard,
    Compiler,
    Doc,
}

const ALL_COMPONENTS: &[Component] = &[
    Component::App,
    Component::Telemetry,
    Component::Leaderboard,
    Component::Compiler,
    Component::Doc,
];

#[derive(clap::Parser, Debug)]
struct Arguments {
    #[clap(
        short,
        long,
        value_enum,
        value_delimiter = ',',
        default_value = "app,telemetry,leaderboard,compiler,doc"
    )]
    /// Components to push.
    components: Vec<Component>,

    #[clap(short)]
    /// Skip bumping version.
    skip_version_bump: bool,

    #[clap(short = 'n')]
    /// Skip pushing.
    dry_run: bool,

    #[clap(long)]
    /// Allow pushing with uncommitted changes or on a non-master branch.
    skip_git_checks: bool,

    #[clap(long)]
    skip_github: bool,

    #[clap(long)]
    skip_discord: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("release=info"))
        .init();

    let args = Arguments::parse();
    let dry_run = args.dry_run;

    let secrets = std::fs::read_to_string(".secrets/secrets.toml")?.parse::<toml::Table>()?;
    for (k, v) in secrets.iter() {
        std::env::set_var(k, v.as_str().expect("invalid secret value"));
    }

    if !args.skip_git_checks {
        sync_cmd_ok(&["git", "diff", "HEAD", "--quiet"])
            .await
            .map_err(|_| anyhow!("Uncommitted changes, halting release"))?;
    }

    let bump_version = !args.skip_version_bump;
    if bump_version {
        if args.components != ALL_COMPONENTS {
            bail!("Attempted to bump version without pushing all components");
        }

        if sync_cmd_ok(&["git", "rev-parse", "--abbrev-ref", "HEAD"])
            .await?
            .stdout_string()
            .trim()
            != "master"
            && !args.skip_git_checks
        {
            bail!("Not on master branch, halting release");
        }

        let changelog = sync_cmd_ok(&["sed", "/^#/Q", "CHANGELOG.md"])
            .await?
            .stdout_string();
        if changelog.is_empty() {
            bail!("Changelog empty, halting release");
        }

        println!("Changelog:\n{}", changelog.trim());

        cmd_argv(&[
            "cargo",
            "workspaces",
            "version",
            "--all",
            "--force=*",
            "--no-git-commit",
            "--yes",
        ])
        .current_dir("frontend")
        .status()
        .await?
        .check_success()?;

        let version = {
            let manifest = std::fs::read_to_string("frontend/app/Cargo.toml")?;
            let manifest = manifest.parse::<toml::Table>()?;
            manifest["package"]["version"]
                .as_str()
                .ok_or_else(|| anyhow!("Failed to find version"))?
                .to_string()
        };
        log::info!("Version {}", version);

        for workspace in WORKSPACES {
            sync_cmd_ok(&[
                "cargo",
                "workspaces",
                "--manifest-path",
                &format!("{workspace}/Cargo.toml"),
                "version",
                "--all",
                "--force=*",
                "--no-git-commit",
                "--yes",
                "custom",
                &version,
            ])
            .await?;
        }

        for workspace in WORKSPACES {
            sync_cmd_ok(&[
                "cargo",
                "update",
                "--manifest-path",
                &format!("{workspace}/Cargo.toml"),
                "--workspace",
            ])
            .await?;
        }

        for workspace in WORKSPACES {
            sync_cmd_ok(&[
                "cargo",
                "verify-project",
                "--manifest-path",
                &format!("{workspace}/Cargo.toml"),
                "--frozen",
                "--locked",
            ])
            .await?;
        }

        sync_cmd_ok(&[
            "git",
            "commit",
            "-n",
            "-a",
            "-m",
            &format!("bump version to {version}"),
        ])
        .await?;

        sync_cmd_ok(&["git", "tag", &format!("v{version}")]).await?;
    }

    let mut tasks = tokio::task::JoinSet::new();

    if args.components.contains(&Component::App) {
        tasks.spawn(async move {
            log::info!("Building frontend (prebuild)");
            sync_cmd_ok(&[
                "cargo",
                "build",
                "--manifest-path",
                "frontend/Cargo.toml",
                "--release",
                "--bins",
                "--target",
                "wasm32-unknown-unknown",
            ])
            .await?;

            log::info!("Building frontend (trunk)");
            if std::fs::metadata("frontend/app/dist").is_ok() {
                std::fs::remove_dir_all("frontend/app/dist")?;
            }
            sync_cmd_ok(&[
                "trunk",
                "build",
                "--release",
                "--dist",
                "frontend/app/dist",
                "frontend/app/index.html",
            ])
            .await?;

            if !dry_run {
                log::info!("Deploying frontend");
                sync_cmd_ok(&[
                    "sh",
                    "-c",
                    r#"cd firebase && eval "$(fnm env)" && fnm use && npx firebase deploy"#,
                ])
                .await?;
            }

            log::info!("Finished frontend");
            anyhow::Ok(())
        });
    }

    if args.components.contains(&Component::Compiler) {
        tasks.spawn(async move {
            log::info!("Building compiler service");
            sync_cmd_ok(&["scripts/build-compiler-service-docker-image.sh"]).await?;

            if !dry_run {
                log::info!("Deploying compiler service");
                sync_cmd_ok(&["scripts/deploy-compiler-service.sh"]).await?;
            }

            log::info!("Finished compiler service");
            Ok(())
        });
    }

    if args.components.contains(&Component::Telemetry) {
        tasks.spawn(async move {
            log::info!("Building telemetry service");
            sync_cmd_ok(&["scripts/build-telemetry-service-docker-image.sh"]).await?;

            if !dry_run {
                log::info!("Deploying telemetry service");
                sync_cmd_ok(&["scripts/deploy-telemetry-service.sh"]).await?;
            }

            log::info!("Finished telemetry service");
            Ok(())
        });
    }

    if args.components.contains(&Component::Leaderboard) {
        tasks.spawn(async move {
            log::info!("Building leaderboard service");
            sync_cmd_ok(&["scripts/build-leaderboard-service-docker-image.sh"]).await?;

            if !dry_run {
                log::info!("Deploying leaderboard service");
                sync_cmd_ok(&["scripts/deploy-leaderboard-service.sh"]).await?;
            }

            log::info!("Finished leaderboard service");
            Ok(())
        });
    }

    if args.components.contains(&Component::Doc) {
        tasks.spawn(async move {
            log::info!("Building docs");
            sync_cmd_ok(&[
                "cargo",
                "doc",
                "--manifest-path",
                "shared/Cargo.toml",
                "-p",
                "oort_api",
            ])
            .await?;

            if !dry_run && bump_version {
                log::info!("Pushing docs");
                sync_cmd_ok(&[
                    "cargo",
                    "publish",
                    "--manifest-path",
                    "shared/Cargo.toml",
                    "-p",
                    "oort_api",
                ])
                .await?;
            }

            log::info!("Finished docs");
            Ok(())
        });
    }

    let mut failed = false;
    while let Some(res) = tasks.join_next().await {
        let res = res.map_err(anyhow::Error::msg).and_then(|x| x);
        if let Err(e) = &res {
            log::error!("Task failed: {}", e);
            failed = true;
        }
    }
    if failed {
        bail!("Release task failed");
    }

    if !args.skip_github {
        log::info!("Pushing to github");
        sync_cmd_ok(&["git", "push"]).await?;
    }

    if !args.skip_discord {
        log::info!("Sending Discord message");
        sync_cmd_ok(&["scripts/send-changelog-discord-message.sh"]).await?;
    }

    log::info!("Finished");
    Ok(())
}

trait ExtendedOutput {
    fn stdout_string(&self) -> String;
    fn stderr_string(&self) -> String;
    fn check_success(&self) -> Result<&Self>;
}

impl ExtendedOutput for Output {
    fn stdout_string(&self) -> String {
        std::str::from_utf8(&self.stdout)
            .expect("invalid utf8")
            .to_string()
    }

    fn stderr_string(&self) -> String {
        std::str::from_utf8(&self.stderr)
            .expect("invalid utf8")
            .to_string()
    }

    fn check_success(&self) -> Result<&Self> {
        if !self.status.success() {
            bail!(
                "Command failed with status {}.\nstderr:\n{}",
                self.status,
                self.stderr_string(),
            );
        }
        Ok(self)
    }
}

trait ExtendedExitStatus {
    fn check_success(&self) -> Result<&Self>;
}

impl ExtendedExitStatus for ExitStatus {
    fn check_success(&self) -> Result<&Self> {
        if !self.success() {
            bail!("Command failed with status {}", self);
        }
        Ok(self)
    }
}

fn cmd_argv(argv: &[&str]) -> Command {
    log::info!("Executing {:?}", shell_words::join(argv));
    let mut cmd = Command::new(argv[0]);
    cmd.kill_on_drop(true);
    cmd.args(&argv[1..]);
    cmd
}

async fn sync_cmd(argv: &[&str]) -> Result<Output> {
    let result = cmd_argv(argv).output().await;
    if let Ok(output) = &result {
        if !output.stdout.is_empty() {
            log::debug!("stdout:\n{}", output.stdout_string());
        }
        if !output.stderr.is_empty() {
            log::debug!("stderr:\n{}", output.stderr_string());
        }
    }
    result.map_err(anyhow::Error::msg)
}

async fn sync_cmd_ok(argv: &[&str]) -> Result<Output> {
    let output = sync_cmd(argv).await?;
    if !output.status.success() {
        bail!(
            "Command {:?} failed with status {}.\nstderr:\n{}",
            argv,
            output.status,
            output.stderr_string(),
        );
    }
    Ok(output)
}

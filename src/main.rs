use anyhow::{Context as _, Result};
use std::{
    env, fs,
    io::Write,
    path::{Path, PathBuf},
};

mod config;
mod sys;

use config::{Config, RepositoryConfig};

const USAGE: &str = "Usage: restig-generator <normal-dir> <early-dir> <late-dir>";

#[derive(Debug)]
struct Context {
    config_path: PathBuf,
    program_name: String,
    hostname: String,
}

fn main() -> anyhow::Result<()> {
    let Some(normal_dir) = env::args().nth(1).map(PathBuf::from) else {
        eprintln!("{}", USAGE);
        std::process::exit(1);
    };
    let is_user = env::var("USER").is_ok(); // Indicate we're generating user-level units
    let config_path = env::var("RESTIC_GENERATOR_CONFIG")
        .map(PathBuf::from)
        .unwrap_or(default_config_path(is_user)?);
    let context = Context {
        config_path,
        program_name: env!("CARGO_BIN_NAME").into(),
        hostname: sys::hostname()?,
    };
    eprintln!("Using config file {}", context.config_path.display());
    let config: Config =
        read_config(&context.config_path).with_context(|| "error reading config")?;

    for repository in config.repositories.iter() {
        generate_backup_service(
            &normal_dir.join(format!("restic-{}-backup.service", repository.name)),
            &context,
            &config,
            repository,
        )?;
        generate_forget_service(
            &normal_dir.join(format!("restic-{}-forget.service", repository.name)),
            &context,
            &config,
            repository,
        )?;
        generate_prune_service(
            &normal_dir.join(format!("restic-{}-prune.service", repository.name)),
            &context,
            &config,
            repository,
        )?;
    }
    Ok(())
}

fn default_config_path(user: bool) -> Result<PathBuf> {
    if user {
        let home = env::var("HOME").with_context(|| "HOME environment variable not found")?;
        Ok(PathBuf::from(home).join(".config/restic-generator/config.toml"))
    } else {
        Ok(PathBuf::from("/etc/restic-generator/config.toml"))
    }
}

fn read_config(path: &Path) -> Result<Config> {
    let content = fs::read(path)?;
    let config = toml::from_slice(&content)?;
    Ok(config)
}

fn generate_backup_service(
    path: &Path,
    context: &Context,
    config: &Config,
    repository: &RepositoryConfig,
) -> anyhow::Result<()> {
    let mut file = fs::File::create(path)
        .with_context(|| format!("{}: error creating file", path.display()))?;
    writeln!(file, "# generated by {}", context.program_name)?;
    writeln!(file, "[Unit]",)?;
    writeln!(
        file,
        "Description=backup {} to {}",
        &config.source, &repository.location
    )?;
    writeln!(file, "SourcePath={}", context.config_path.display())?;
    writeln!(file, "ConditionPathExists={}", config.source)?;
    if is_local_repository(&repository.location) {
        writeln!(file, "ConditionPathExists={}", repository.location)?;
    }
    writeln!(file)?;
    writeln!(file, "[Service]")?;
    writeln!(
        file,
        "Environment=RESTIC_REPOSITORY=\"{}\"",
        repository.location
    )?;
    if let Some(value) = &repository.password_file {
        writeln!(file, "Environment=RESTIC_PASSWORD_FILE=\"{}\"", value)?;
    }
    if let Some(value) = &repository.password_command {
        writeln!(file, "Environment=RESTIC_PASSWORD_COMMAND=\"{}\"", value)?;
    }
    if let Some(value) = &repository.aws_access_key {
        writeln!(file, "Environment=AWS_ACCESS_KEY=\"{}\"", value)?;
    }
    if let Some(value) = &repository.aws_secret_access_key {
        writeln!(file, "Environment=AWS_SECRET_ACCESS_KEY=\"{}\"", value)?;
    }
    writeln!(file, "Type=oneshot")?;
    writeln!(file, "ExecStartPre=restic unlock")?;
    writeln!(
        file,
        "ExecStart={}",
        backup_cmd(
            &config.source,
            config.host.as_deref().unwrap_or(&context.hostname),
            config.exclude.as_slice()
        )
    )?;
    // 3 is returned when a file cannot be read (e.g. it is removed during the backup.)
    writeln!(file, "SuccessExitStatus=3",)?;
    writeln!(file, "Nice=10",)?;
    writeln!(file, "IOSchedulingClass=idle",)?;
    Ok(())
}

fn generate_forget_service(
    path: &Path,
    context: &Context,
    config: &Config,
    repository: &RepositoryConfig,
) -> anyhow::Result<()> {
    if !repository.has_forget_policy() {
        return Ok(());
    }
    let mut file = fs::File::create(path)
        .with_context(|| format!("{}: error creating file", path.display()))?;
    writeln!(file, "# generated by {}", context.program_name)?;
    writeln!(file, "[Unit]",)?;
    writeln!(
        file,
        "Description=forget {} from {}",
        &config.source, &repository.location
    )?;
    writeln!(file, "SourcePath={}", context.config_path.display())?;
    writeln!(file)?;
    writeln!(file, "[Service]")?;
    writeln!(
        file,
        "Environment=RESTIC_REPOSITORY=\"{}\"",
        repository.location
    )?;
    if let Some(value) = &repository.password_file {
        writeln!(file, "Environment=RESTIC_PASSWORD_FILE=\"{}\"", value)?;
    }
    if let Some(value) = &repository.password_command {
        writeln!(file, "Environment=RESTIC_PASSWORD_COMMAND=\"{}\"", value)?;
    }
    if let Some(value) = &repository.aws_access_key {
        writeln!(file, "Environment=AWS_ACCESS_KEY=\"{}\"", value)?;
    }
    if let Some(value) = &repository.aws_secret_access_key {
        writeln!(file, "Environment=AWS_SECRET_ACCESS_KEY=\"{}\"", value)?;
    }
    writeln!(file, "Type=oneshot")?;
    writeln!(file, "ExecStartPre=restic unlock")?;
    writeln!(
        file,
        "ExecStart={}",
        forget_cmd(
            config.host.as_deref().unwrap_or(&context.hostname),
            &config.source,
            repository
        )
    )?;
    writeln!(file, "Nice=10",)?;
    writeln!(file, "IOSchedulingClass=idle",)?;
    Ok(())
}

fn generate_prune_service(
    path: &Path,
    context: &Context,
    _config: &Config,
    repository: &RepositoryConfig,
) -> anyhow::Result<()> {
    if !repository.has_forget_policy() {
        return Ok(());
    }
    let mut file = fs::File::create(path)
        .with_context(|| format!("{}: error creating file", path.display()))?;
    writeln!(file, "# generated by {}", context.program_name)?;
    writeln!(file, "[Unit]",)?;
    writeln!(file, "Description=Prune {}", &repository.location)?;
    writeln!(file, "SourcePath={}", context.config_path.display())?;
    writeln!(file)?;
    writeln!(file, "[Service]")?;
    writeln!(
        file,
        "Environment=RESTIC_REPOSITORY=\"{}\"",
        repository.location
    )?;
    if let Some(value) = &repository.password_file {
        writeln!(file, "Environment=RESTIC_PASSWORD_FILE=\"{}\"", value)?;
    }
    if let Some(value) = &repository.password_command {
        writeln!(file, "Environment=RESTIC_PASSWORD_COMMAND=\"{}\"", value)?;
    }
    if let Some(value) = &repository.aws_access_key {
        writeln!(file, "Environment=AWS_ACCESS_KEY=\"{}\"", value)?;
    }
    if let Some(value) = &repository.aws_secret_access_key {
        writeln!(file, "Environment=AWS_SECRET_ACCESS_KEY=\"{}\"", value)?;
    }
    writeln!(file, "Type=oneshot")?;
    writeln!(file, "ExecStartPre=restic unlock")?;
    writeln!(file, "ExecStart=restic prune")?;
    writeln!(file, "Nice=10")?;
    writeln!(file, "IOSchedulingClass=idle")?;
    Ok(())
}

fn is_local_repository(location: &str) -> bool {
    !location.starts_with("azure:")
        && !location.starts_with("b2:")
        && !location.starts_with("gs:")
        && !location.starts_with("rclone:")
        && !location.starts_with("s3:")
        && !location.starts_with("sftp:")
        && !location.starts_with("swift:")
}

/// A macro that pushes the given value serialized with the given format if the value is Some
macro_rules! pushopt {
    ($vec:expr, $format:expr, $value:expr) => {
        if let Some(value) = $value {
            $vec.push(format!($format, value));
        }
    };
}

fn backup_cmd<T: AsRef<str>>(source: &str, host: &str, exclude: &[T]) -> String {
    let mut result = vec![
        format!("restic"),
        format!("backup"),
        format!("--host=\"{}\"", host),
    ];
    for pattern in exclude.iter() {
        result.push(format!("--exclude=\"{}\"", pattern.as_ref()));
    }
    result.push(source.to_string());
    result.join(" ")
}

fn forget_cmd(host: &str, path: &str, repository: &RepositoryConfig) -> String {
    let mut result = vec![
        format!("restic"),
        format!("forget"),
        format!("--host=\"{}\"", host),
        format!("--path=\"{}\"", path),
    ];
    pushopt!(result, "--keep-last=\"{}\"", repository.keep_last);
    pushopt!(result, "--keep-hourly=\"{}\"", repository.keep_hourly);
    pushopt!(result, "--keep-daily=\"{}\"", repository.keep_daily);
    pushopt!(result, "--keep-weekly=\"{}\"", repository.keep_weekly);
    pushopt!(result, "--keep-monthly=\"{}\"", repository.keep_monthly);
    pushopt!(result, "--keep-yearly=\"{}\"", repository.keep_yearly);
    pushopt!(result, "--keep-tag=\"{}\"", &repository.keep_tag);
    pushopt!(result, "--keep-within=\"{}\"", &repository.keep_within);
    result.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_cmd_default() {
        assert_eq!(
            backup_cmd::<&str>("/", "laptop", &[]),
            r#"restic backup --host="laptop" /"#
        );
    }

    #[test]
    fn backup_cmd_exclude() {
        assert_eq!(
            backup_cmd::<&str>("/", "laptop", &["foo", "bar.baz"]),
            r#"restic backup --host="laptop" --exclude="foo" --exclude="bar.baz" /"#
        );
    }

    #[test]
    fn backup_cmd_with_host() {
        assert_eq!(
            backup_cmd::<&str>("/", "laptop", &[]),
            r#"restic backup --host="laptop" /"#
        );
    }

    macro_rules! test_forget_cmd {
        ($testname:ident, $attr:ident: $value:expr, $expected:expr) => {
            #[test]
            fn $testname() {
                let repo = RepositoryConfig {
                    $attr: Some($value),
                    ..Default::default()
                };
                assert_eq!(forget_cmd("laptop", "/", &repo), $expected);
            }
        };
    }

    test_forget_cmd!(forget_cmd_keep_last, keep_last: 42, r#"restic forget --host="laptop" --path="/" --keep-last="42""#);
    test_forget_cmd!(forget_cmd_keep_hourly, keep_hourly: 42, r#"restic forget --host="laptop" --path="/" --keep-hourly="42""#);
    test_forget_cmd!(forget_cmd_keep_daily, keep_daily: 42, r#"restic forget --host="laptop" --path="/" --keep-daily="42""#);
    test_forget_cmd!(forget_cmd_keep_weekly, keep_weekly: 42, r#"restic forget --host="laptop" --path="/" --keep-weekly="42""#);
    test_forget_cmd!(forget_cmd_keep_monthly, keep_monthly: 42, r#"restic forget --host="laptop" --path="/" --keep-monthly="42""#);
    test_forget_cmd!(forget_cmd_keep_yearly, keep_yearly: 42, r#"restic forget --host="laptop" --path="/" --keep-yearly="42""#);
    test_forget_cmd!(forget_cmd_keep_tag, keep_tag: "important".into(), r#"restic forget --host="laptop" --path="/" --keep-tag="important""#);
    test_forget_cmd!(forget_cmd_keep_within, keep_within: "2y5m7d3h".into(), r#"restic forget --host="laptop" --path="/" --keep-within="2y5m7d3h""#);

    macro_rules! test_is_local_repository {
        ($name:ident, $location:expr) => {
            #[test]
            fn $name() {
                assert!(is_local_repository($location));
            }
        };
        (!$name:ident, $location:expr) => {
            #[test]
            fn $name() {
                assert!(!is_local_repository($location));
            }
        };
    }

    test_is_local_repository!(abs_path_is_local, "/media/backup");
    test_is_local_repository!(systmed_home_is_local, "%h/backup");
    test_is_local_repository!(!sftp_is_not_local, "sftp:user@host:/srv/restic-repo");
    test_is_local_repository!(!s3_is_not_local, "s3:s3.amazonaws.com/bucket_name");
    test_is_local_repository!(!swift_is_not_local, "swift:container_name:/path");
    test_is_local_repository!(!b2_is_not_local, "b2:bucketname:path/to/repo");
    test_is_local_repository!(!azure_is_not_local, "azure:foo:/");
    test_is_local_repository!(!gs_is_not_local, "gs:foo:/");
    test_is_local_repository!(!rclone_is_not_local, "rclone:foo:bar");
}

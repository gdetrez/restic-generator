use assert_cmd::prelude::*; // Add methods on commands
use std::{error::Error, fs::read_to_string, process::Command};
use tempfile::TempDir;

macro_rules! snapshot_test {
    ($name:ident, $config:expr, $output:expr) => {
        #[test]
        fn $name() -> Result<(), Box<dyn Error>> {
            let normal_dir = TempDir::new()?;
            let early_dir = TempDir::new()?;
            let late_dir = TempDir::new()?;
            let mut cmd = Command::cargo_bin("restic-generator")?;
            cmd.arg("-c")
                .arg($config)
                .arg(normal_dir.path())
                .arg(early_dir.path())
                .arg(late_dir.path());
            cmd.assert().success();

            let paths = std::fs::read_dir(normal_dir.path()).unwrap();
            for path in paths {
                println!("Name: {}", path.unwrap().path().display())
            }

            insta::assert_snapshot!(read_to_string(normal_dir.path().join($output))?);
            Ok(())
        }
    };
}

snapshot_test!(
    local_backup_service,
    "example-config.toml",
    "restic-myrepo-backup.service"
);

snapshot_test!(
    local_forget_service,
    "example-config.toml",
    "restic-myrepo-forget.service"
);

snapshot_test!(
    local_prune_service,
    "example-config.toml",
    "restic-myrepo-prune.service"
);

snapshot_test!(
    remote_backup_service,
    "example-config.toml",
    "restic-sftprepo-backup.service"
);

snapshot_test!(
    s3_backup_service,
    "example-config.toml",
    "restic-s3bucket-backup.service"
);

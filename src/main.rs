use std::{env, fs, path::PathBuf, process::ExitCode};

use anyhow::{Context, Result, bail};
use file_sharing::{
    manifest::{ShareManifest, build_manifest, manifest_json_pretty},
    receiver::{ReceivePlanEntry, build_receive_plan},
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let mut args = env::args_os().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        bail!("missing command");
    };

    match command.to_string_lossy().as_ref() {
        "manifest" => {
            let path = args
                .next()
                .context("manifest requires a file or folder path")?;
            if args.next().is_some() {
                bail!("manifest accepts exactly one file or folder path");
            }

            let manifest = build_manifest(PathBuf::from(path))?;
            println!("{}", manifest_json_pretty(&manifest)?);
            Ok(())
        }
        "receive-plan" => {
            let manifest_path = args
                .next()
                .context("receive-plan requires a manifest JSON path")?;
            let destination = args
                .next()
                .context("receive-plan requires a destination directory")?;
            if args.next().is_some() {
                bail!("receive-plan accepts exactly a manifest path and destination directory");
            }

            let bytes = fs::read(&manifest_path).with_context(|| {
                format!(
                    "failed to read manifest {}",
                    PathBuf::from(&manifest_path).display()
                )
            })?;
            let manifest: ShareManifest =
                serde_json::from_slice(&bytes).context("failed to parse share manifest JSON")?;
            let plan = build_receive_plan(&manifest, PathBuf::from(destination))?;

            println!("root\t{}", plan.root_path.display());
            for entry in plan.entries {
                match entry {
                    ReceivePlanEntry::Directory {
                        manifest_path,
                        destination_path,
                    } => println!(
                        "directory\t{manifest_path}\t{}",
                        destination_path.display()
                    ),
                    ReceivePlanEntry::File {
                        manifest_path,
                        destination_path,
                        size_bytes,
                        sha256,
                    } => println!(
                        "file\t{manifest_path}\t{}\t{size_bytes}\t{sha256}",
                        destination_path.display()
                    ),
                }
            }

            Ok(())
        }
        "help" | "--help" | "-h" => {
            print_usage();
            Ok(())
        }
        other => {
            print_usage();
            bail!("unknown command: {other}");
        }
    }
}

fn print_usage() {
    eprintln!("file-sharing");
    eprintln!();
    eprintln!("USAGE:");
    eprintln!("  file-sharing manifest <FILE_OR_FOLDER>");
    eprintln!("  file-sharing receive-plan <MANIFEST_JSON> <DESTINATION_DIRECTORY>");
}

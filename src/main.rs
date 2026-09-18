use std::{env, path::PathBuf, process::ExitCode};

use anyhow::{Context, Result, bail};
use file_sharing::manifest::{build_manifest, manifest_json_pretty};

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
}

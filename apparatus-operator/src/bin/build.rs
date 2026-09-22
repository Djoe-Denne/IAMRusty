//! Binaire `apparatus-build` — worker de build P4, sans HTTP.
//!
//! Ce binaire n'active pas les features `admit` / `controller`. `--version`
//! s'affiche et quitte avant le contrôle d'identité.

use std::process::ExitCode;

fn main() -> ExitCode {
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("{}", apparatus_operator::PKG_VERSION);
        return ExitCode::SUCCESS;
    }
    match apparatus_operator::refuse_privileged_identity() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

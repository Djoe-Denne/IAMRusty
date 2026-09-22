//! Binaire `apparatus-admit` — T2 `--version` sans features ; T10 signe + CR.

fn main() {
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("{}", apparatus_operator::PKG_VERSION);
        return;
    }

    #[cfg(all(feature = "admit", feature = "controller"))]
    {
        if let Err(err) = admit_sign_and_apply() {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
    #[cfg(not(all(feature = "admit", feature = "controller")))]
    {
        eprintln!("apparatus-admit requires --features admit,controller (hors --version)");
        std::process::exit(1);
    }
}

#[cfg(all(feature = "admit", feature = "controller"))]
fn admit_sign_and_apply() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("tokio runtime: {err}"))?;
    runtime.block_on(apparatus_operator::controller::sign_envelope_and_apply_from_env())?;
    Ok(())
}

//! Binaire `apparatus-controller` — T2 `--version` sans features ; T10 watch CR.

fn main() {
    if std::env::args().any(|arg| arg == "--version" || arg == "-V") {
        println!("{}", apparatus_operator::PKG_VERSION);
        return;
    }

    #[cfg(feature = "controller")]
    {
        if let Err(err) = controller_watch() {
            eprintln!("{err}");
            std::process::exit(1);
        }
    }
    #[cfg(not(feature = "controller"))]
    {
        eprintln!("apparatus-controller requires --features controller (hors --version)");
        std::process::exit(1);
    }
}

#[cfg(feature = "controller")]
fn controller_watch() -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|err| format!("tokio runtime: {err}"))?;
    runtime.block_on(apparatus_operator::desired_state::run_controller())?;
    Ok(())
}

use rustycog_config::ServerConfig;

pub struct MonolithConfig {
    pub server: ServerConfig,
    pub iam: iam_configuration::AppConfig,
    pub telegraph: telegraph_configuration::TelegraphConfig,
    pub hive: hive_configuration::AppConfig,
    pub manifesto: manifesto_configuration::AppConfig,
}

pub fn load_monolith_config() -> anyhow::Result<MonolithConfig> {
    let iam = iam_configuration::load_config()?;
    let telegraph = telegraph_configuration::load_config()?;
    let hive = hive_configuration::load_config()?;
    let manifesto = manifesto_configuration::load_config()?;

    let server = ServerConfig {
        host: "0.0.0.0".to_string(),
        port: 8080,
        tls_enabled: false,
        tls_cert_path: String::new(),
        tls_key_path: String::new(),
        tls_port: 0,
    };

    Ok(MonolithConfig {
        server,
        iam,
        telegraph,
        hive,
        manifesto,
    })
}

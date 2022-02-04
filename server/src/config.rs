use dotenv::dotenv;
use lazy_static::lazy_static;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub sap_server_listen: String,
    pub database_url: String,
    pub nfs_dir: String,
    pub tmp_dir: String,
}

fn init_config() -> Config {
    dotenv().ok();

    match envy::from_env::<Config>() {
        Ok(config) => config,
        Err(err) => panic!("Couldn't process env variables: {:#?}", err),
    }
}

lazy_static! {
    pub static ref CONFIG: Config = init_config();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_inits_a_config() {
        let config = init_config();
        assert_ne!(config.sap_server_listen, "".to_string())
    }

    #[test]
    fn it_gets_a_config_from_the_lazy_static() {
        let config = &CONFIG;
        assert_ne!(config.sap_server_listen, "".to_string())
    }
}

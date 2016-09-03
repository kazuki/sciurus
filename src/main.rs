#![cfg_attr(feature="clippy", feature(plugin))]
#![cfg_attr(feature="clippy", plugin(clippy))]

extern crate json;
extern crate hyper;
extern crate base64;

use std::sync::Arc;
use std::sync::RwLock;

pub mod config;
pub mod objectstore;

use config::JsonConfig;

fn main() {
    let config = Arc::new(RwLock::new({
        let mut path = config::get_config_dir_path();
        std::fs::create_dir_all(&path).unwrap();
        path.push("config.json");
        JsonConfig::new(&path, true)
    }));
    config.write().unwrap().load().unwrap();

    let mut onedrive = objectstore::OneDriveClient::new(env!("SCIURUS_ONEDRIVE_CLIENT_ID")
                                                            .to_string(),
                                                        config.clone());
    onedrive.access_test();
}

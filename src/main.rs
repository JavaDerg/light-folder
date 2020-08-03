mod config;
mod error;
mod img;
mod proxy_controller;
mod requester;

pub use log::{ info, warn, error, debug, trace };

use crate::proxy_controller::ProxyController;
use saphir::prelude::*;

pub use error::*;
use env_logger::Env;

#[tokio::main]
async fn main() -> std::result::Result<(), SaphirError> {
    env_logger::init();

    img::start();

    info!("Starting server");
    let server = Server::builder()
        .configure_listener(|mut l| {
            if config::INTERFACES.is_empty() {
                crash("No interfaces found, please supply environment variables like this 'LF_INTERFACE_ID=0.0.0.0:80' ('ID' can be replaced by anything)");
            }
            for interface in config::INTERFACES.iter() {
                info!("Listening on {}", &interface);
                l = l.interface(interface.as_str());
            }
            l
        })
        .configure_router(|r| r.controller(ProxyController::new("proxy")))
        .build();

    server.run().await
}

fn crash<S: ToString>(reason: S) -> ! {
    error!("{}", reason.to_string());
    warn!("Exiting");
    std::process::exit(1)
}

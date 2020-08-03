mod config;
mod error;
mod img;
mod proxy_controller;
mod requester;

pub use log::{ info, warn, error, debug, trace };

use crate::proxy_controller::ProxyController;

pub use error::*;
use env_logger::Env;
use saphir::server::Server;

#[tokio::main]
async fn main() {
    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    img::start();

    info!("Starting server");
    let server = Server::builder()
        .configure_listener(|mut l| {
            if config::INTERFACES.is_empty() {
                crash("No interfaces found, please supply environment variables like this 'LF_INTERFACE_ID=0.0.0.0:80' ('ID' can be replaced by anything)");
            }
            for interface in config::INTERFACES.iter() {
                l = l.interface(interface.as_str());
            }
            l
        })
        .configure_router(|r| r.controller(ProxyController::new("proxy")))
        .build();

    if let Err(err) = server.run().await.map_err(|err| Error::SaphirError(err)) {
        crash(err)
    }
}

fn crash<S: ToString>(reason: S) -> ! {
    error!("{}", reason.to_string());
    warn!("Exiting");

    std::process::exit(1)
}

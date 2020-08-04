mod config;
mod error;
mod img;
mod proxy_controller;
mod requester;

pub use log::{debug, error, info, trace, warn};

use crate::proxy_controller::ProxyController;

use env_logger::Env;
pub use error::*;
use saphir::server::Server;

#[tokio::main]
async fn main() -> ! {
    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    img::start_worker_threads();

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
        .configure_router(|r| r.controller(ProxyController::new()))
        .build();

    tokio::spawn(run_server(server));

    if let Ok(_) = tokio::signal::ctrl_c().await {
        info!("Received ^C");
    }
    shutdown(0)
}

async fn run_server(server: Server) {
    if let Err(err) = server.run().await.map_err(|err| Error::SaphirError(err)) {
        crash(err)
    }
}

fn shutdown(code: i32) -> ! {
    warn!("Shutting down server");
    img::shutdown();

    info!("Have a nice day!");
    std::process::exit(code)
}

fn crash<S: ToString>(reason: S) -> ! {
    error!("{}", reason.to_string());

    shutdown(1)
}

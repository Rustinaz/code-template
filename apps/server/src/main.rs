//! The backend binary.
//!
//! `cargo run --package server` starts it on `127.0.0.1:8080` with two demo
//! users already registered, which is what makes `scripts/dev.sh` useful: the
//! desktop client can show real data the moment it opens.

use std::net::TcpListener;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Arg, ArgAction, Command};
use futures::executor::block_on;
use server::{serve, Server};
use shared::services::factory;
use shared::ServiceContainer;

/// The demo accounts `--no-demo` suppresses.
const DEMO_USERS: [(&str, &str, &str); 2] = [
    ("alice@example.com", "alice", "Alice Wonder"),
    ("bob@example.com", "bob", "Bob Builder"),
];

/// The password every demo account shares.
const DEMO_PASSWORD: &str = "password123";

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("server: {error:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let matches = Command::new("server")
        .about("Bundled JSON backend for the cross-platform template")
        .arg(
            Arg::new("bind")
                .long("bind")
                .value_name("ADDR")
                .default_value("127.0.0.1:8080")
                .help("Address to listen on, e.g. 0.0.0.0:8080 to accept LAN clients"),
        )
        .arg(
            Arg::new("no-demo")
                .long("no-demo")
                .action(ArgAction::SetTrue)
                .help("Do not register the two demo users"),
        )
        .get_matches();

    let bind = matches
        .get_one::<String>("bind")
        .expect("bind has a default value");

    let config = shared::init_blocking(None)?;
    let services = Arc::new(block_on(factory::create_services(config))?);

    if !matches.get_flag("no-demo") {
        seed_demo_users(&services);
    }

    let listener = TcpListener::bind(bind).with_context(|| format!("cannot bind {bind}"))?;
    let address = listener.local_addr()?;

    tracing::info!("listening on http://{address}");
    println!("Backend listening on http://{address}");
    println!("Run the client with: cargo run --package example-app -- --server http://{address}");

    serve(listener, Arc::new(Server::new(services)))?;
    Ok(())
}

fn seed_demo_users(services: &ServiceContainer) {
    for (email, username, display_name) in DEMO_USERS {
        match block_on(
            services
                .auth
                .register(email, username, DEMO_PASSWORD, display_name),
        ) {
            Ok(user) => tracing::info!("seeded demo user {}", user.username),
            Err(error) => tracing::debug!("demo user {username} was not seeded: {error}"),
        }
    }
}

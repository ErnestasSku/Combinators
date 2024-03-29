#![allow(warnings)]
#![allow(unused)]


use std::net::SocketAddr;
use std::process::exit;
use std::{future::Future, time::Duration};

use color_eyre::Report;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod comb;
mod examples;


#[tokio::main]
async fn main() -> Result<(), Report> {
    setup()?;


    examples::case1::run().await;
    examples::case2::run().await;


    Ok(())
}



fn setup() -> Result<(), Report> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1")
    }
    color_eyre::install()?;

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info")
    }
    tracing_subscriber::fmt::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    Ok(())
}



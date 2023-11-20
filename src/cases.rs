use std::{net::SocketAddr, time::Duration};

use tracing::info;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream, time::Sleep,
};

use crate::tj;

const URL_1: &str = "https://fasterthanli.me/articles/whats-in-the-box";
const URL_2: &str = "https://fasterthanli.me/series/advent-of-code-2020/part-13";

pub async fn case1() {
    let res = tj::join_futures(fetch_thing(URL_1), fetch_thing(URL_2)).await;

    info!(?res, "All done!");
}

pub async fn case2() {
    let res = tj::sequential(sleep5(), fetch_thing(URL_1)).await;    
    
    info!(?res, "All done!");
}

pub async fn case3() {}

pub async fn case4() {}

pub async fn case5() {}

async fn sleep5() {
    info!("Sleep start");
    tokio::time::sleep(Duration::from_secs(5)).await;
    info!("Sleep ended");
}

async fn fetch_thing(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = ([1, 1, 1, 1], 80).into();
    let mut socket = TcpStream::connect(addr).await?;

    socket.write_all(b"GET / HTTP/1.1\r\n").await?;
    socket.write_all(b"Host: 1.1.1.1\r\n").await?;
    socket.write_all(b"User-Agent: cool-bear\r\n").await?;
    socket.write_all(b"Connection: close\r\n").await?;
    socket.write_all(b"\r\n").await?;

    let mut response = String::with_capacity(256);
    socket.read_to_string(&mut response).await?;

    let status = response.lines().next().unwrap_or_default();
    info!(%status, %name, "Got response!");

    // dropping the socket will close the connection

    Ok(())
}



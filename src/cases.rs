use std::{net::SocketAddr, time::Duration};

use tracing::info;

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::Sleep,
};

use reqwest::{self, header::CONTENT_TYPE};

use crate::comb;

const URL_1: &str = "https://mif.vu.lt/lt3/";
const URL_2: &str = "https://www.vu.lt/en/";

pub async fn case1() {
    let res = comb::join_futures(fetch_thing(URL_1), fetch_thing(URL_2)).await;

    info!(?res, "All done!");
}

pub async fn case2() {
    // let res = comb::sequential(sleep5(), fetch_thing(URL_1)).await;

    // info!(?res, "All done!");
}

pub async fn case3() {
    let first = comb::sequence(call_random_user(), |user| {
        comb::sequence(make_json(user), |score| {
            comb::map(call_user_score(score), |score_str| score_str.unwrap_or(0))
        })
    })
    .await
    .await;
    let second = comb::sequence(call_random_user(), |user| {
        comb::sequence(make_json(user), |score| {
            comb::map(call_user_score(score), |score_str| score_str.unwrap_or(0))
        })
    })
    .await
    .await;

    let combined_score = comb::combine_with(first, second, |first, second| first + second).await;
    println!("Combined score = {combined_score}")
}

pub async fn case4() {
    let a = 
            comb::sequence(
                fetch_thing(URL_1), 
                |x| match x {
                    Ok(o) => println!("Okay"),
                    Err(e) => println!("Bad"),
                } 
            ).await;
    dbg!(a);
}

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
    socket.write_all(b"User-Agent: rust-cli\r\n").await?;
    socket.write_all(b"Connection: close\r\n").await?;
    socket.write_all(b"\r\n").await?;

    let mut response = String::with_capacity(256);
    socket.read_to_string(&mut response).await?;

    let status = response.lines().next().unwrap_or_default();
    info!(%status, %name, "Got response!");

    Ok(())
}

pub async fn call_random_user() -> Result<String, reqwest::Error> {
    let url = "http://127.0.0.1:3000/user_random";
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    info!("Response from: {}", body);
    Ok(body)
}

pub async fn call_user_score(user: String) -> Result<i32, reqwest::Error> {
    let url = "http://127.0.0.1:3000/user_score";
    let client = reqwest::Client::new();

    let response = client
        .get(url)
        .header(CONTENT_TYPE, "application/json")
        .body(user)
        .send()
        .await?;

    let body = response.text().await?;
    info!("Response: {}", body);
    Ok(body.parse::<i32>().unwrap_or(0))
}

pub async fn make_json(user: Result<String, reqwest::Error>) -> String {
    let user = user.unwrap_or("A".to_owned());

    let json = format!("{{\"user\":\"{user}\"}}");

    json
}

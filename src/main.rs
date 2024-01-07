use std::net::SocketAddr;
use std::{future::Future, time::Duration};

use color_eyre::Report;
// use futures::io;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod old;
mod comb;
mod cases;


#[tokio::main]
async fn main() -> Result<(), Report> {
    setup()?;


    let a = async move { 0 };

    // let b: String = comb::map(a, |x| x.to_string()).await;

    // let a = cases::call_random_user().await;
    // println!("{a:?}");
    // let a = cases::call_user_score("{\"user\": \"B\"}").await;
    // println!("{a:?}");

    println!("Enter a case number");
    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer)?;
    let case = buffer.trim().parse::<i32>()?; 

    match case {
        1 => cases::case1().await,
        2 => cases::case2().await,
        3 => cases::case3().await,
        4 => cases::case4().await,
        5 => cases::case5().await,
        _ => println!("Such case is not implemented"),
    };


    

    // Test2
    // let res = tj::sequential(
    // fetch_thing(URL_1),
    // tj::sequential(test_async_num::<Report>(), fetch_thing(URL_1)),
    // )
    // .await;
    // info!(?res, "All done!");

    //
    // let a1 = tj::sequence(test_async_num(), |num: Result<i32, f32>| Ok(num.unwrap() * 2));
    // let a1 = tj::sequence(test_async_num::<Box<dyn std::error::Error>>(), |a| Ok(a.unwrap() / 2) );
    // let a2 = fetch_thing(URL_1);
    // let res = tj::try_join(a1, a2);
    // let a3 = old::sequential(test_async_num::<Box<dyn std::error::Error>>(), test_async_num::<Box<dyn std::error::Error>>());
    // let a4 = old::sequential(test_async_num::<Box<dyn std::error::Error>>(), test_async_num::<Box<dyn std::error::Error>>());

    // let a5 = old::sequential(a3, a4);
    // let r = old::sequential(res, a5).await;

    // info!(?res, "All done");
    // println!("{:?}", r);

    Ok(())
}


async fn test_async_num<E>() -> Result<i32, Box<dyn std::error::Error>> {
    info!("Sleep start");
    // std::thread::sleep(Duration::from_millis(9000));
    tokio::time::sleep(Duration::from_millis(2500)).await;
    info!("Sleep end");
    Ok(42)
}

fn b(a: Result<i32, Report>) -> Result<i32, Box<dyn std::error::Error>> {
    match a {
        Ok(a) => Ok(a * 3),
        Err(e) => Err(e.into()),
    }
}

async fn a(a: Result<i32, Report>) -> impl Future<Output = Result<i32, Report>> {
    async move {
        match a {
            Ok(a) => Ok(a * 5),
            Err(e) => Err(e),
        }
    }
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

async fn fetch_thing(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let addr: SocketAddr = ([1, 1, 1, 1], 80).into();
    let mut socket = TcpStream::connect(addr).await?;

    // we're writing straight to the socket, there's no buffering
    // so no need to flush
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


fn foo1(input: i32) -> impl Future<Output = i32>
{
    async move {
        input * 5
    }
}

async fn foo2(input: i32) -> i32
{
    input * 2
}

// fn foo2(input: impl Future<Output = i32>) -> impl Future<Output = i32>
// {
//     async move {
//         let input = input.await;
//         input * 5
//     }
// }
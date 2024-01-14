#![allow(warnings)]
#![allow(unused)]

use std::{pin::Pin, future::Future};

use crate::comb::*;

#[derive(Debug, Clone, Copy)]
struct SumMax(i32, i32);

impl Unpin for SumMax {

}

impl Monoid for SumMax {
    fn identity() -> Self {
        SumMax(0, 0)
    }

    fn combine(&self, other: &Self) -> Self {
        SumMax(self.0 + other.0, i32::max(self.1, other.1))
    }
}


pub async fn run() {
    let ids = vec![1, 2, 3, 4, 5];

    let futures = ids.into_iter()
                     .map(|id| Box::pin(to_sum_max(id)) as Pin<Box<dyn Future<Output = SumMax>>>)
                     .collect::<Vec<_>>();

    let initial_future: Pin<Box<dyn Future<Output = SumMax>>> = Box::pin(async { SumMax::identity() });
    let mut combined_future = initial_future;

    for future in futures {
        combined_future = combine(combined_future, future);
    }

    let final_result = combined_future.await;
    dbg!(final_result);
}


async fn to_sum_max(id: i32) -> SumMax {
    let score = match get_score(id).await {
        Ok(score) => {
            // println!("{:?}", score);
            score
        },
        Err(_) => 0, // Handle the error case, maybe log it or use a default value
    };
    SumMax(score, score)
}
// async fn get_score(id: i32) -> Result<i32, reqwest::Error> {
//     reqwest::get(format!("http://127.0.0.1:3000/score/{}", id))
//         .await?
//         .json::<i32>()
//         .await
// }


async fn get_score(id: i32) -> Result<i32, i32> {
    match id {
        1 => Ok(45),
        2 => Ok(25),
        3 => Ok(37),
        4 => Ok(14),
        5 => Ok(61),
        _ => Ok(0),
    }
}

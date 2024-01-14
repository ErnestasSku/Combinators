#![allow(warnings)]
#![allow(unused)]

use std::time::Duration;

use crate::comb::*;

pub async fn run() {
    let products = join_futures_bimap(
        get_product(1),
        get_product(2),
        get_product_price,
        get_product_price,
    );

    let discount = sequence(apply_discount_code(123), sequence(sleep5(), get_discount()));

    let info = get_info();

    let discounted = combine_with(products, discount, apply_discounts_to_products);


    let result = map(join_futures(discounted, info), format_data).await;
    dbg!(result);
}

fn format_data(data: ((f32, f32), Info)) -> String {
    let (prices, info) = data;

    format!("Using discount, the prices for the products are {prices:?} from {0}", info.name)
}

#[derive(Debug)]
struct Product {
    code: i32,
    price: f32,
}

fn apply_discounts_to_products(product: (f32, f32), discount: i32) -> (f32, f32) {
    let (first, second) = product;

    let first = first - (first * (discount as f32 / 100.0));
    let second = second - (second * (discount as f32 / 100.0));

    (first, second)
}

async fn get_product(id: i32) -> Product {
    match id {
        1 => Product {
            code: 14,
            price: 12.41,
        },
        2 => Product {
            code: 123,
            price: 41.74,
        },
        _ => panic!(),
    }
}

async fn apply_discount_code(code: i32) -> () {}

async fn get_discount() -> i32 {
    14
}

fn get_product_price(product: Product) -> f32 {
    product.price
}

async fn sleep5() {
    tokio::time::sleep(Duration::from_secs(2)).await;
}

#[derive(Debug)]
struct Info {
    name: String,
    code: i32,
    location: String,
}

async fn get_info() -> Info {
    Info {
        code: 13,
        location: String::from("Middle st. 15"),
        name: String::from("Local shop"),
    }
}

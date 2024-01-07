use comb;

#[tokio::test]
async fn associativity() {
    assert_eq!(
        identity(async move { 0 }.await)
        async move { identity(0) }.await
    )
}
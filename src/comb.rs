use std::{
    future::Future,
    pin::Pin,
    process::Output,
    task::{Context, Poll},
    thread::JoinHandle,
};

use tracing::info;

fn identity<T>(id: T) -> T {
    id
}

fn unit() -> impl Future<Output = ()> {
    async move { () }
}

fn unit_g<T: Default>() -> impl Future<Output = T> {
    async move { T::default() }
}

enum State<F, T, E>
where
    F: Future<Output = Result<T, E>>,
{
    Future(F),
    Ok(T),
    Gone,
}

enum TryJoin<A, B, AR, BR, E>
where
    A: Future<Output = Result<AR, E>>,
    B: Future<Output = Result<BR, E>>,
{
    Polling {
        a: State<A, AR, E>,
        b: State<B, BR, E>,
    },
    Done,
}

pub fn try_join<A, B, AR, BR, E>(a: A, b: B) -> impl Future<Output = Result<(AR, BR), E>>
where
    A: Future<Output = Result<AR, E>>,
    B: Future<Output = Result<BR, E>>,
{
    TryJoin::Polling {
        a: State::Future(a),
        b: State::Future(b),
    }
}

impl<A, B, AR, BR, E> Future for TryJoin<A, B, AR, BR, E>
where
    A: Future<Output = Result<AR, E>>,
    B: Future<Output = Result<BR, E>>,
{
    type Output = Result<(AR, BR), E>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let (a, b) = match this {
            TryJoin::Polling { a, b } => (a, b),
            TryJoin::Done => panic!("TryJoined polled after completion"),
        };

        if let State::Future(fut) = a {
            if let Poll::Ready(res) = unsafe { Pin::new_unchecked(fut) }.poll(cx) {
                *a = State::Ok(res?);
            }
        }

        if let State::Future(fut) = b {
            if let Poll::Ready(res) = unsafe { Pin::new_unchecked(fut) }.poll(cx) {
                *b = State::Ok(res?);
            }
        }

        match (a, b) {
            (State::Ok(_), State::Ok(_)) => match std::mem::replace(this, Self::Done) {
                Self::Polling {
                    a: State::Ok(a),
                    b: State::Ok(b),
                } => Ok((a, b)).into(),
                _ => unreachable!(),
            },
            _ => Poll::Pending,
        }
    }
}

enum SimpleState<F, T>
where
    F: Future<Output = T>,
{
    Future(F),
    Ok(T),
    Gone,
}

enum JoinFutures<A, B, AR, BR>
where
    A: Future<Output = AR>,
    B: Future<Output = BR>,
{
    Polling {
        a: SimpleState<A, AR>,
        b: SimpleState<B, BR>,
    },
    Done,
}

pub fn join_futures<A, B, AR, BR>(a: A, b: B) -> impl Future<Output = (AR, BR)>
where
    A: Future<Output = AR>,
    B: Future<Output = BR>,
{
    JoinFutures::Polling {
        a: SimpleState::Future(a),
        b: SimpleState::Future(b),
    }
}

impl<A, B, AR, BR> Future for JoinFutures<A, B, AR, BR>
where
    A: Future<Output = AR>,
    B: Future<Output = BR>,
{
    type Output = (AR, BR);

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let (a, b) = match this {
            JoinFutures::Polling { a, b } => (a, b),
            _ => panic!("Join futures polled after completion"),
        };

        if let SimpleState::Future(fut) = a {
            if let Poll::Ready(res) = unsafe { Pin::new_unchecked(fut) }.poll(cx) {
                *a = SimpleState::Ok(res);
            }
        }

        if let SimpleState::Future(fut) = b {
            if let Poll::Ready(res) = unsafe { Pin::new_unchecked(fut) }.poll(cx) {
                *b = SimpleState::Ok(res);
            }
        }

        match (a, b) {
            (SimpleState::Ok(_), SimpleState::Ok(_)) => match std::mem::replace(this, Self::Done) {
                JoinFutures::Polling {
                    a: SimpleState::Ok(a),
                    b: SimpleState::Ok(b),
                } => Poll::Ready((a, b)),
                _ => unreachable!(),
            },
            _ => Poll::Pending,
        }
    }
}

enum Sequential<A, B, AR, BR>
where
    A: Future<Output = AR>,
    B: Future<Output = BR>,
{
    Polling {
        first: A,
        second: Option<Box<dyn FnOnce(AR) -> B>>,
    },
    Done,
}

pub fn sequential<A, B, AR, BR>(
    first: A,
    second_fn: impl FnOnce(AR) -> B + 'static,
) -> impl Future<Output = BR>
where
    A: Future<Output = AR>,
    B: Future<Output = BR>,
{
    Sequential::Polling {
        first,
        second: Some(Box::new(second_fn)),
    }
}

impl<A, B, AR, BR> Future for Sequential<A, B, AR, BR>
where
    A: Future<Output = AR>,
    B: Future<Output = BR>,
{
    type Output = BR;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let (first, second_fn) = match this {
            Sequential::Polling { first, second } => (first, second.take()),
            Sequential::Done => panic!("Sequential polled after completion"),
        };

        let first = unsafe { Pin::new_unchecked(first) };
        match first.poll(cx) {
            Poll::Ready(res) => {
                if let Some(second_fn) = second_fn {
                    let mut second = (second_fn)(res);
                    let second = unsafe { Pin::new_unchecked(&mut second) };
                    match second.poll(cx) {
                        Poll::Ready(second_res) => {
                            return Poll::Ready(second_res);
                            // *this = Sequential::Done
                            // Poll::Ready((res, second_res))
                        }
                        Poll::Pending => {
                            return Poll::Pending;
                            // this.second = Some(second_fn);
                            // Poll::Pending
                        }
                    }
                } else {
                    // No second future function provided, should not happen
                    unreachable!()
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

enum Sequence<F, G, T>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> T,
{
    Polling { first: F, second: G },
    Done,
}

pub fn sequence<F, G, T>(first: F, second: G) -> impl Future<Output = T>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> T,
{
    Sequence::Polling { first, second }
}

impl<F, G, T> Future for Sequence<F, G, T>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> T,
{
    type Output = T;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this: &mut Sequence<F, G, T> = unsafe { self.get_unchecked_mut() };
        let (first, second) = match this {
            Self::Polling { first, second } => (first, second),
            Self::Done => panic!("Sequential polled after completion"),
        };

        let first = unsafe { Pin::new_unchecked(first) };
        let res = match first.poll(cx) {
            Poll::Ready(res) => res,
            Poll::Pending => return Poll::Pending,
        };

        match std::mem::replace(this, Self::Done) {
            Sequence::Polling { first, second } => Poll::Ready(second(res)),
            _ => unreachable!(),
        }
    }
}

enum Mapping<F, T, M, U>
where
    F: Future<Output = T>,
    M: FnOnce(T) -> U,
{
    Polling { future: F, mapper: M },
    Done,
}

pub fn map<T, U, F>(task: F, mapper: impl FnOnce(T) -> U) -> impl Future<Output = U>
where
    F: Future<Output = T>,
{
    Mapping::Polling {
        future: task,
        mapper,
    }
}

impl<F, T, M, U> Future for Mapping<F, T, M, U>
where
    F: Future<Output = T>,
    M: FnOnce(T) -> U,
{
    type Output = U;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let (first, second) = match this {
            Self::Polling { future, mapper } => (future, mapper),
            Self::Done => panic!("Sequential polled after completion"),
        };

        let first = unsafe { Pin::new_unchecked(first) };
        let res = match first.poll(cx) {
            Poll::Ready(res) => res,
            Poll::Pending => return Poll::Pending,
        };

        match std::mem::replace(this, Self::Done) {
            Mapping::Polling { future, mapper } => Poll::Ready(mapper(res)),
            _ => unreachable!(),
        }
    }
}

enum CombineWith<A, B, AR, BR, M, MR>
where
    A: Future<Output = AR>,
    B: Future<Output = BR>,
    M: FnOnce(AR, BR) -> MR,
{
    Polling {
        a: SimpleState<A, AR>,
        b: SimpleState<B, BR>,
        combine: M,
    },
    Done,
}

pub fn combine_with<A, B, AR, BR, M, MR>(a: A, b: B, combine: M) -> impl Future<Output = MR>
where
    A: Future<Output = AR>,
    B: Future<Output = BR>,
    M: FnOnce(AR, BR) -> MR,
{
    CombineWith::Polling {
        a: SimpleState::Future(a),
        b: SimpleState::Future(b),
        combine,
    }
}

impl<A, B, AR, BR, M, MR> Future for CombineWith<A, B, AR, BR, M, MR>
where
    A: Future<Output = AR>,
    B: Future<Output = BR>,
    M: FnOnce(AR, BR) -> MR,
{
    type Output = MR;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let (a, b, _combine) = match this {
            CombineWith::Polling { a, b, combine } => (a, b, combine),
            _ => unreachable!(),
        };

        if let SimpleState::Future(fut) = a {
            if let Poll::Ready(res) = unsafe { Pin::new_unchecked(fut) }.poll(cx) {
                *a = SimpleState::Ok(res);
            }
        }

        if let SimpleState::Future(fut) = b {
            if let Poll::Ready(res) = unsafe { Pin::new_unchecked(fut) }.poll(cx) {
                *b = SimpleState::Ok(res);
            }
        }

        match (a, b) {
            (SimpleState::Ok(_), SimpleState::Ok(_)) => match std::mem::replace(this, Self::Done) {
                CombineWith::Polling {
                    a: SimpleState::Ok(a),
                    b: SimpleState::Ok(b),
                    combine,
                } => Poll::Ready(combine(a, b)),
                _ => unreachable!(),
            },
            _ => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::comb::*;

    #[tokio::test]
    async fn identity_law() {
        let result1 = identity(async move { 0 }.await);
        let result2 = sequential(async move { 0 }, |x| async move { identity(x) }).await;

        assert_eq!(result1, result2);
    }

    #[tokio::test]
    async fn associativity() {
        let a = async { 1 };
        let b = |x: i32| async move { x + 1 };
        let c = |x: i32| async move { x * 2 };

        // Applying sequential(sequential(A, B), C)
        let res1 = sequential(sequential(a, b), c).await;

        // Reset a + to avoid using moved values
        let a = async { 1 };

        // Applying sequential(A, sequential(B, C))
        let res2 = sequential(a, move |x| sequential(b(x), c)).await;

        assert_eq!(res1, res2, "Sequential composition should be associative");
    }
}

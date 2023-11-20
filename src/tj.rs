use std::{future::Future, pin::Pin, process::Output, task::Poll, thread::JoinHandle};

use tracing::info;

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
        first: SimpleState<A, AR>,
        second: SimpleState<B, BR>,
    },
    Done,
}

pub fn sequential<A, B, AR, BR>(first: A, second: B) -> impl Future<Output = (AR, BR)>
where
    A: Future<Output = AR>,
    B: Future<Output = BR>,
{
    Sequential::Polling {
        first: SimpleState::Future(first),
        second: SimpleState::Future(second),
    }
}

impl<A, B, AR, BR> Future for Sequential<A, B, AR, BR>
where
    A: Future<Output = AR>,
    B: Future<Output = BR>,
{
    type Output = (AR, BR);

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let (first, second) = match this {
            Sequential::Polling { first, second } => (first, second),
            Self::Done => panic!("Sequential polled after completion"),
        };

        if let SimpleState::Future(fut) = first {
            match unsafe { Pin::new_unchecked(fut) }.poll(cx) {
                Poll::Ready(res) => {
                    *first = SimpleState::Ok(res);
                },
                Poll::Pending => return  Poll::Pending,
            }
        }

        if let SimpleState::Future(fut) = second {
            match unsafe { Pin::new_unchecked(fut) }.poll(cx) {
                Poll::Ready(res) => {
                    *second = SimpleState::Ok(res);
                },
                Poll::Pending => return  Poll::Pending,
            }
        };


        match (first, second) {
            (SimpleState::Ok(_), SimpleState::Ok(_)) => match std::mem::replace(this, Self::Done) {
                Sequential::Polling {
                    first: SimpleState::Ok(first),
                    second: SimpleState::Ok(second),
                } => Poll::Ready((first, second)),
                _ => unreachable!(),
            },
            _ => Poll::Pending,
        }
    }
}

enum Sequence<F, G, T, U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> U,
{
    Polling { first: F, second: G },
    Done,
}

pub fn sequence<F, G, T, U>(first: F, second: G) -> impl Future<Output = U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> U,
{
    Sequence::Polling { first, second }
}

impl<F, G, T, U> Future for Sequence<F, G, T, U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> U,
{
    type Output = U;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
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

//  --------
// pub enum TaskState {
//     Pending,
//     Running,
//     Complete,
// }

// pub struct AsyncTask<F>
// where
//     F: Future<Output = i32>,
// {
//     pub state: TaskState,
//     // async_fn: Box<dyn FnOnce() -> dyn Future<Output = ()>>,
//     pub async_a: F,
//     // async_fn: Box<dyn FnOnce() -> dyn Future<Output = ()> + Send>,
// }

// impl<F> Future for AsyncTask<F>
// where
//     F: Future<Output = i32>,
// {
//     type Output = i32;

//     fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
//         let a = unsafe { self.get_unchecked_mut() };

//         let b = unsafe { Pin::new_unchecked(&mut a.async_a) };
//         match b.poll(cx) {
//             Poll::Ready(res) => res.into(),
//             Poll::Pending => Poll::Pending,
//         }
//     }
// }

// pub fn block_on<F>(future: F)
// where
//     F: Future,
// {
// }

// pub async fn spawn<T>() -> JoinHandle<T> {
//     todo!()
// }

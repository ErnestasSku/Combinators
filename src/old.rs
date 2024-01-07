use std::{pin::Pin, task::Poll, future::Future};

// use futures::Future;

enum State<F, T, E>
where
    F: Future<Output = Result<T, E>>,
{
    Future(F),
    Ok(T),
    Gone,
}

enum Sequential<A, B, AR, BR, E>
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

pub fn sequential<A, B, AR, BR, E>(a: A, b: B) -> impl Future<Output = Result<(AR, BR), E>>
where
    A: Future<Output = Result<AR, E>>,
    B: Future<Output = Result<BR, E>>,
{
    Sequential::Polling {
        a: State::Future(a),
        b: State::Future(b),
    }
}

impl<A, B, AR, BR, E> Future for Sequential<A, B, AR, BR, E>
where
    A: Future<Output = Result<AR, E>>,
    B: Future<Output = Result<BR, E>>,
{
    type Output = Result<(AR, BR), E>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let (a, b) = match this {
            Self::Polling { a, b } => (a, b),
            Self::Done => panic!("Sequential polled after completion"),
        };

        if let State::Future(fut) = a {
            match unsafe { Pin::new_unchecked(fut) }.poll(cx) {
                Poll::Ready(res) => *a = State::Ok(res?),
                Poll::Pending => return Poll::Pending,
            }
        }

        if let State::Future(fut) = b {
            match unsafe { Pin::new_unchecked(fut) }.poll(cx) {
                Poll::Ready(res) => *b = State::Ok(res?),
                Poll::Pending => return Poll::Pending,
            }
        }

        match (a, b) {
            (State::Ok(_), State::Ok(_)) => match std::mem::replace(this, Self::Done) {
                Sequential::Polling {
                    a: State::Ok(a),
                    b: State::Ok(b),
                } => Ok((a, b)).into(),
                Sequential::Done => unreachable!(),
                _ => unreachable!(),
            },
            _ => Poll::Pending,
        }
    }
}
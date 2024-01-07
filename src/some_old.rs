enum SequenceA<F, G, T, U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> Pin<Box<dyn Future<Output = U>>> + Send + 'static,
{
    Polling { first: F, second: Option<G>, result: Option<U> },
    Done,
}

pub fn sequence_a<F, G, T, U>(first: F, second: G) -> impl Future<Output = U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> Pin<Box<dyn Future<Output = U>>> + Send + 'static,
{
    SequenceA::Polling { first, second: Some(second), result: None }
}

impl<F, G, T, U> Future for SequenceA<F, G, T, U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> Pin<Box<dyn Future<Output = U>>> + Send + 'static,
{
    type Output = U;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // let this = &mut *self;
        let this = unsafe { self.get_unchecked_mut() };

        let (first, second, result) = match this {
            SequenceA::Polling { first, second, result } => (first, second.take(), result),
            SequenceA::Done => panic!("Sequential polled after completion"),
        };

        let first = unsafe { Pin::new_unchecked(first) };
        let res = match first.poll(cx) {
            Poll::Ready(res) => res,
            Poll::Pending => return Poll::Pending,
        };

        let mut second = unsafe { Pin::new_unchecked(&&mut second.unwrap()) };
        let mut future = second(res);
        // this.result = Some(Box::pin(future));

        // let res2 = this.result.take().as_mut().unwrap().as_mut().poll(cx);

        let res2 = match future.as_mut().poll(cx) {
            Poll::Ready(res2) => res2,
            Poll::Pending => return Poll::Pending,
        };

        match std::mem::replace(this, SequenceA::Done) {
            SequenceA::Polling { .. } => Poll::Ready(res2),
            _ => unreachable!(),
        }
    }
}

enum SequenceA<F, G, T, U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> Box<dyn Future<Output = U>> + Send + 'static,
{
    Polling { first: F, second: G },
    Done,
}

pub fn sequence_a<F, G, T, U>(first: F, second: G) -> impl Future<Output = U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> Box<dyn Future<Output = U>> + Send + 'static,
{
    SequenceA::Polling { first, second }
}

impl<F, G, T, U> Future for SequenceA<F, G, T, U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> Box<dyn Future<Output = U>> + Send + 'static,
{
    type Output = U;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let (first, second) = match this {
            Self::Polling { first, second } => (first, second),
            Self::Done => panic!("Sequential polled after completion"),
        };

        let first = unsafe { Pin::new_unchecked(first) };

        // Pin the entire enum and use as_mut() to access inner pinned values
        let mut second = unsafe { Pin::new_unchecked(second) };

        let res = match first.poll(cx) {
            Poll::Ready(res) => res,
            Poll::Pending => return Poll::Pending,
        };

        let future = second.take();

        // let function: Box<dyn Future<Output = U>> = second(res);
        // let boxed = Box::pin(function);
        // let mut context = Context::from_waker(futures::task::noop_waker_ref());

        match std::mem::replace(this, Self::Done) {
            SequenceA::Polling { .. } => Poll::Ready(res2),
            _ => unreachable!(),
        }
    }
}

enum SequenceA<F, G, T, U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> Box<dyn Future<Output = U>> + Send + 'static,
    // G: FnOnce(T) -> U + Send + 'static,
{
    Polling { first: F, second: G },
    Done,
}

pub fn sequence_a<F, G, T, U>(first: F, second: G) -> impl Future<Output = U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> Box<dyn Future<Output = U>> + Send + 'static,
    // G: FnOnce(T) -> U + Send + 'static,
{
    SequenceA::Polling { first, second }
}

impl<F, G, T, U> Future for SequenceA<F, G, T, U>
where
    F: Future<Output = T>,
    G: FnOnce(T) -> Box<dyn Future<Output = U>> + Send + 'static,
    // G: FnOnce(T) -> U + Send + 'static,
{
    type Output = U;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        let (first, second) = match this {
            Self::Polling { first, second } => (first, second),
            Self::Done => panic!("Sequential polled after completion"),
        };

        let first = unsafe { Pin::new_unchecked(first) };
        // let second = unsafe { Pin::new_unchecked(second) };

        let res = match first.poll(cx) {
            Poll::Ready(res) => res,
            Poll::Pending => return Poll::Pending,
        };

        let seq = second(res);

        let a = Box::pin(seq);

        // let res2 = match Box::pin(seq).poll_unpin(cx) {
        //     Poll::Ready(res2) => res2,
        //     Poll::Pending => return Poll::Pending,
        // };

        match std::mem::replace(this, Self::Done) {
            SequenceA::Polling { first, second } => Poll::Ready(res2),
            _ => unreachable!(),
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

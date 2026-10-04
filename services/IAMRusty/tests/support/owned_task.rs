//! Test-owned tasks never detach when an assertion unwinds.
use std::{
    future::Future,
    ops::Deref,
    pin::Pin,
    task::{Context, Poll},
};
pub struct OwnedTask<T>(tokio::task::JoinHandle<T>);
impl<T> OwnedTask<T> {
    pub fn new(handle: tokio::task::JoinHandle<T>) -> Self {
        Self(handle)
    }
}
impl<T> Deref for OwnedTask<T> {
    type Target = tokio::task::JoinHandle<T>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> Future for OwnedTask<T> {
    type Output = Result<T, tokio::task::JoinError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx)
    }
}
impl<T> Drop for OwnedTask<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}
pub fn spawn<F>(future: F) -> OwnedTask<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    OwnedTask::new(tokio::spawn(future))
}

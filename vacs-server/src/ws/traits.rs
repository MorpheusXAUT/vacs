use axum::extract::ws::Message;
use axum::Error;
use futures_util::{Sink, Stream};

pub trait WebSocketSink: Sink<Message, Error = Error> + Unpin + Send {}
impl<T> WebSocketSink for T where T: Sink<Message, Error = Error> + Unpin + Send {}

pub trait WebSocketStream: Stream<Item = Result<Message, Error>> + Unpin + Send {}
impl<T> WebSocketStream for T where T: Stream<Item = Result<Message, Error>> + Unpin + Send {}

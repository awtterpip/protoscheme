use std::sync::Arc;

use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::mpsc;

use crate::{read_message, write_message};
use crate::protocol::{Message, Result};

pub struct ServerReceiver<T> {
    read: OwnedReadHalf,
    executor: MethodExecutor<T>,
}

impl<T: ProcessMessage + Send + Sync + 'static> ServerReceiver<T> {
    fn new(read: OwnedReadHalf, message_tx: mpsc::UnboundedSender<Message>, app: Arc<T>) -> Self {
        Self {
            read,
            executor: MethodExecutor {
                app,
                message_tx,
            }
        }
    }

    pub async fn dispatch(&mut self) -> Result<()> {
        let message = read_message(&mut self.read).await?;
        self.executor.process_message(message).await
    }
}

pub struct ServerSender {
    write: OwnedWriteHalf,
    message_rx: mpsc::UnboundedReceiver<Message>,
}

impl ServerSender {
    fn new(write: OwnedWriteHalf, message_rx: mpsc::UnboundedReceiver<Message>) -> Self {
        Self {
            write,
            message_rx
        }
    }

    pub async fn flush(&mut self) -> Result<()> {
        while let Some(message) = self.message_rx.recv().await {
            write_message(&mut self.write, message).await?;
        }
        Ok(())
    }
}

pub struct MethodExecutor<T> {
    pub(crate) app: Arc<T>,
    pub(crate) message_tx: mpsc::UnboundedSender<Message>,
}

impl<T> Clone for MethodExecutor<T> {
    fn clone(&self) -> Self {
        Self { app: self.app.clone(), message_tx: self.message_tx.clone() }
    }
}

impl<T: ProcessMessage + Send + Sync + 'static> MethodExecutor<T> {
    async fn process_message(&self, message: Message) -> Result<()> {
        let executor = self.clone();
        match message {
            Message::Signal { name, data } => tokio::spawn(async move {
                executor.app.execute_method(name, data);
            }),
            Message::Request { id, name, data } => tokio::spawn(async move {
                let data = executor.app.execute_method(name, data);
                let message = Message::Response { id, data};
                let _ = executor.message_tx.send(message);
            }),
            Message::Response { id: _, data: _ } => panic!("This should never happen"),
        };
        Ok(())
    }
}

pub trait ProcessMessage {
    fn execute_method(&self, name: u64, data: Vec<u8> ) -> Vec<u8>;
    fn execute_signal(&self, name: u64, data: Vec<u8> );
}

pub fn create_server<T: ProcessMessage + Send + Sync + 'static>(connection: UnixStream, app: Arc<T>) -> (ServerSender, ServerReceiver<T>) {
    let (read, write) = connection.into_split();
    let (message_tx, message_rx) = mpsc::unbounded_channel();
    (ServerSender::new(write, message_rx), ServerReceiver::new(read, message_tx, app))
}
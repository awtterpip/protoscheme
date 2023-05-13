use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rustc_hash::FxHashMap;
use tokio::net::UnixStream;
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::sync::{mpsc, oneshot};

use crate::{read_message, write_message};
use crate::protocol::{Message, Result, Error};

type PendingFuture = oneshot::Sender<Vec<u8>>;
type PendingFutureSender = mpsc::UnboundedSender<(u64, PendingFuture)>;
type PendingFutureReceiver = mpsc::UnboundedReceiver<(u64, PendingFuture)>;

pub struct ClientReceiver {
    read: OwnedReadHalf,
    pending_futures: FxHashMap<u64, PendingFuture>,
    pending_future_rx: PendingFutureReceiver,
}

impl ClientReceiver {
    fn new(read: OwnedReadHalf, pending_future_rx: PendingFutureReceiver) -> Self {
        Self {
            read, 
            pending_future_rx,
            pending_futures: Default::default(),
        }
    }

    pub async fn dispatch(&mut self) -> Result<()> {
        let message = read_message(&mut self.read).await?;

        self.update_pending_futures();

        if let Message::Response { id, data } = message {
            if let Some(future) = self.pending_futures.remove(&id) {
                let _ = future.send(data);
            }
        }
        todo!()
    }

    fn update_pending_futures(&mut self) {
        while let Ok((id, future)) = self.pending_future_rx.try_recv() {
            let _ = self.pending_futures.insert(id, future);
        }
    }
}

pub struct ClientSender {
    write: OwnedWriteHalf,
    handle: ClientHandle,
    message_rx: mpsc::UnboundedReceiver<Message>,
}

impl ClientSender {
    fn new(write: OwnedWriteHalf, pending_future_tx: PendingFutureSender) -> Self {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let max_message_id = Arc::new(AtomicU64::new(0));
        Self {
            write,
            message_rx,
            handle: ClientHandle {
                message_tx,
                pending_future_tx,
                max_message_id,
            },
        }
    }

    pub async fn flush(&mut self) -> Result<()> {
        while let Some(message) = self.message_rx.recv().await {
            write_message(&mut self.write, message).await?;
        }
        Ok(())
    }

    pub fn handle(&self) -> ClientHandle {
        self.handle.clone()
    }
}

#[derive(Clone)]
pub struct ClientHandle {
    message_tx: mpsc::UnboundedSender<Message>,
    pending_future_tx: PendingFutureSender,
    max_message_id: Arc<AtomicU64>,
}

impl ClientHandle {
    fn send(&self, message: Message) -> Result<()> {
        self.message_tx.send(message).map_err(|_| Error::ReceiverDropped)
    }

    pub async fn method(&self, name: u64, data: Vec<u8>) -> Result<Vec<u8>> {
        let (tx, rx) = oneshot::channel();
        let id = self.max_message_id.fetch_add(1, Ordering::Relaxed);
        self.pending_future_tx
            .send((id, tx))
            .map_err(|_| Error::ReceiverDropped)?;
        self.send(Message::Request { id, name, data })?;
        rx.await.map_err(|_| Error::ReceiverDropped)
    }
}

pub fn create_client(connection: UnixStream) -> (ClientSender, ClientReceiver) {
    let (read, write) = connection.into_split();
    let (pending_future_tx, pending_future_rx) = mpsc::unbounded_channel();
    (ClientSender::new(write, pending_future_tx), ClientReceiver::new(read, pending_future_rx))
}
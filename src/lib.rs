pub mod client;
pub mod server;
pub mod protocol;

use protocol::{Message, Error};
use serde::{Deserialize, Serialize};
use tokio::net::unix::{OwnedReadHalf, OwnedWriteHalf};
use tokio::io::{AsyncWriteExt, AsyncReadExt};

pub(crate) type SerializationError = flexbuffers::SerializationError;
pub(crate) type DeserializationError = flexbuffers::DeserializationError;

pub fn deserialize<'a, T: Deserialize<'a>>(data: &'a [u8]) -> Result<T, DeserializationError> {
    let root = flexbuffers::Reader::get_root(data)?;
    T::deserialize(root)
}

pub fn serialize<S: Serialize>(to_serialize: S) -> Result<Vec<u8>, SerializationError> {
    let mut fs = flexbuffers::FlexbufferSerializer::new();
    to_serialize.serialize(&mut fs)?;
    Ok(fs.view().to_vec())
}

pub(crate) async fn write_message(write: &mut OwnedWriteHalf, message: Message) -> Result<(), Error> {
    let message = serialize(message)?;
    let message_length = message.len() as u32;
    write.write_all(&message_length.to_be_bytes()).await?;
    write.write_all(&message).await?;
    Ok(())
}

pub(crate) async fn read_message(read: &mut OwnedReadHalf) -> Result<Message, Error> {
    let mut message_length_buffer: [u8; 4] = [0; 4];
    read.read_exact(&mut message_length_buffer).await?;
    let message_length: u32 = u32::from_ne_bytes(message_length_buffer);

    let mut message_buffer: Vec<u8> = std::vec::from_elem(0, message_length as usize);
    read.read_exact(&mut message_buffer).await?;
    Ok(deserialize(&message_buffer)?)
}
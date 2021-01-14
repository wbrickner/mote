use std::io;
use futures::{
  stream::Next,
  channel::mpsc::{unbounded, UnboundedReceiver},
  StreamExt
};

use termion::{
  event::Key,
  input::TermRead
};

pub struct UserInput {
  key_rx: UnboundedReceiver<Key>
}

impl UserInput {
  pub fn new() -> Self {
    let (key_tx, key_rx) = unbounded();

    // start and forget, we don't need to gracefully shut down
    tokio::spawn(async move {
      let stdin = io::stdin();
      for key_result in stdin.keys() {
        if let Ok(key) = key_result {
          key_tx.unbounded_send(key).unwrap()
        }
      }
    });

    UserInput { key_rx }
  }

  pub fn next<'a>(&'a mut self) -> Next<'a, UnboundedReceiver<Key>> {
    self.key_rx.next()
  }
}
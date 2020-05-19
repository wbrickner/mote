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

pub struct UserInputEvents {
  key_rx: UnboundedReceiver<Key>,
  input_handle: tokio::task::JoinHandle<()>,
}

impl UserInputEvents {
  pub fn new() -> Self {
    let (key_tx, key_rx) = unbounded();

    let input_handle = {
      let key_tx = key_tx.clone();

      tokio::spawn(async move {
        let stdin = io::stdin();
        for key_result in stdin.keys() {
          match key_result {
            Ok(key) => match key_tx.unbounded_send(key) {
              Err(_) => return,
              _ => ()
            },
            Err(_) => {}
          }
        }
      })
    };

    UserInputEvents { key_rx, input_handle }
  }

  pub fn next<'a>(&'a mut self) -> Next<'a, UnboundedReceiver<Key>> {
    self.key_rx.next()
  }
}
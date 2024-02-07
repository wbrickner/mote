use futures::{future::ready, Stream, StreamExt};
use termion::{event::Key, input::TermRead};
use tokio_stream::iter;

pub fn user_input() -> impl Stream<Item = Key> + Unpin {
  iter(std::io::stdin().keys()).filter_map(|r| ready(r.ok()))
}
#[macro_use] extern crate serde_derive;

use futures::channel::mpsc::unbounded;

mod discovery;
mod ui_util;
mod ui;
use ui::UI;

#[tokio::main]
async fn main() {
	let (roku_discovery_tx, mut roku_discovery_rx) = unbounded::<discovery::Device>();

	discovery::begin_device_discovery(roku_discovery_tx);

	// scope exit calls destructor on UI, which in turn destroys terminal abstraction,
	// which returns the terminal to normal mode automatically
	{
		let mut ui = UI::new();
		
		// (will block this thread, but that's ok)
		ui.render_ui(&mut roku_discovery_rx)
			.await
			.expect("Failed to render UI");
	}

	std::process::exit(0);
}

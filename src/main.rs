mod devices;
mod ui;

use devices::Discoverer;
use ui::UI;

#[tokio::main]
async fn main() {
  let mut discovery_rx = Discoverer::begin();

	// scope exit calls destructor on UI, which in turn destroys terminal abstraction,
	// which returns the terminal to "normal mode" automatically
	{ 
    UI::new().listen(&mut discovery_rx).await
  }

  // forcibly exit process whenever UI task finishes
	std::process::exit(0);
}

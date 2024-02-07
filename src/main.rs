mod devices;
mod ui;

#[tokio::main]
async fn main() {
  // drop returns terminal to normal mode
  ui::UI::new()
    .listen(
      devices::discover()
    ).await;

  // forcibly exit process whenever UI task finishes
  std::process::exit(0);
}

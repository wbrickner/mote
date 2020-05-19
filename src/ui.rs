use crate::discovery::{Device, DeviceInput, RokuInput, RokuKey};
use crate::ui_util::UserInputEvents;
use futures::{
	channel::mpsc::UnboundedReceiver,
	future::{select, Either},
	stream::StreamExt,
};
use std::collections::{HashMap, HashSet};
use std::time::SystemTime;
use std::{error::Error, io};
use termion::{event::Key, input::MouseTerminal, raw::{IntoRawMode, RawTerminal}, screen::AlternateScreen};
use tui::{
	backend::TermionBackend,
	layout::{Constraint, Direction, Layout, Rect},
	style::{Color, Modifier, Style},
	widgets::{Block, BorderType, Borders},
	Terminal,
};

/// height / width - portrait
const REMOTE_ASPECT_RATIO: f64 = 2.0 / 5.5;
const REMOTE_WIDTH_PIXELS: f64 = 512.0;

enum UIContext {
	Main,
	DeviceInfo
}

pub struct UI {
	terminal: Terminal<TermionBackend<RawTerminal<std::io::Stdout>>>,

	/// keyboard state
	/// Stores which keys are down. ONLY used for rendering, not control flow! keys may seem active for many render cycles when they were not just pressed!
	active_keys: HashSet<char>,

	/// stores the system time of the last key received (used for throttling)
	key_last_activity: HashMap<char, SystemTime>,

	/// device states
	devices: Vec<Device>,

	/// Which device is active?
	selected_device_index: usize,

	context: UIContext
}

impl UI {
	pub fn new() -> Self {
		let stdout = io::stdout()
			.into_raw_mode()
			.expect("Failed to put terminal into 'raw mode'");
		// let stdout = MouseTerminal::from(stdout);
		// let stdout = AlternateScreen::from(stdout);
		let backend = TermionBackend::new(stdout);
		let mut terminal = Terminal::new(backend).expect("Failed to initialize terminal abstraction");
		
		terminal.clear().expect("Failed to clear terminal");
		terminal.hide_cursor().expect("Failed to hide cursor");

		println!("Searching. Devices appear as they're discovered.");

		UI {
			terminal,
			active_keys: HashSet::new(),
			key_last_activity: HashMap::new(),
			devices: Vec::new(),
			selected_device_index: 0,
			context: UIContext::Main
		}
	}

	// draw based on state
	fn render(&mut self) {
		let tab_titles: Vec<&String> = self.devices.iter().map(|device| device.get_human_name()).collect();
		let selected_index = self.selected_device_index;
		let selected_device = &self.devices[selected_index];
		let context = &self.context;

		self.terminal.draw(move |mut f| {
			let (terminal_char_width, terminal_char_height) = termion::terminal_size().expect("Failed to get information about terminal size (in chars)");
			let (terminal_px_width, terminal_px_height) = termion::terminal_size_pixels().expect("Failed to get information about terminal size (in pixels)");
		
			let terminal_font_px_width = (terminal_px_width as f64) / (terminal_char_width as f64);
			let terminal_font_px_height = (terminal_px_height as f64) / (terminal_char_height as f64);
		
			let remote_char_width = REMOTE_WIDTH_PIXELS / terminal_font_px_width;
			let remote_char_height = REMOTE_ASPECT_RATIO * REMOTE_WIDTH_PIXELS / terminal_font_px_height;
		
			let tabs = tui::widgets::Tabs::default()
										.block(Block::default().title("Devices").borders(Borders::ALL))
										.titles(&tab_titles[..])
										.style(Style::default().fg(Color::White))
										.highlight_style(Style::default().fg(Color::Yellow))
										.divider(tui::symbols::line::VERTICAL)
										.select(selected_index);
			f.render_widget(tabs, Rect::new(0, 0, terminal_char_width / 2, 3));

			let (info_height, info_contents) = match context {
				UIContext::Main => (1, vec![
					tui::widgets::Text::raw(format!(" {} ({})", selected_device.get_human_model_name(), selected_device.get_ip()))
				]),
				UIContext::DeviceInfo => (4, vec![
					// tui::widgets::Text::raw(format!(" {} ({})\n\r", selected_device.get_human_model_name(), selected_device.get_ip())),
					// tui::widgets::Text::raw(format!(" {} ({})\n\r", selected_device.get_human_model_name(), selected_device.get_ip())),
					// tui::widgets::Text::raw(format!(" {} ({})\n\r", selected_device.get_human_model_name(), selected_device.get_ip())),
					// tui::widgets::Text::raw(format!(" {} ({})\n\r", selected_device.get_human_model_name(), selected_device.get_ip()))
				])
			};

			let info = tui::widgets::Paragraph::new(info_contents.iter()).wrap(true).alignment(tui::layout::Alignment::Left);
			
			let remote = Block::default()
				.borders(Borders::ALL)
				// .title("Main block with round corners")
				.border_type(BorderType::Plain);
		
			f.render_widget(info, Rect::new(0, 3, terminal_char_width, 1));
			f.render_widget(remote, Rect::new(0, 3 + info_height, remote_char_width.round() as u16, remote_char_height.round() as u16));

		}).expect("Failed to render")
	}

	pub async fn render_ui(
		&mut self,
		roku_discovery_rx: &mut UnboundedReceiver<Device>,
	) -> Result<(), Box<dyn Error>> {
		// note: because I'm using the UnboundedReceiver, I can't miss messages, so I can take a little time configuring the terminal
		// note: uses "dynamic refresh rate", which only redraws when something has actually changed (using futures!)

		let mut user_input_events = UserInputEvents::new();
		let mut key_future = user_input_events.next();
		let mut device_future = roku_discovery_rx.next();

		loop {
			match select(key_future, device_future).await {
				// new key
				Either::Left((key, device_future_continue)) => {
					match key {
						Some(key) => {
							match key {
								Key::Up => {
									if self.devices.len() != 0 {
										self.devices[self.selected_device_index].supply_input(DeviceInput::Roku(RokuInput::KeyPress(RokuKey::VolumeUp))).await.expect("Failed to send input");
									}
								},
								Key::Down => {
									if self.devices.len() != 0 {
										self.devices[self.selected_device_index].supply_input(DeviceInput::Roku(RokuInput::KeyPress(RokuKey::VolumeDown))).await.expect("Failed to send input");
									}
								},
								Key::Delete | Key::Backspace => {
									if self.devices.len() != 0 {
										self.devices[self.selected_device_index].supply_input(DeviceInput::Roku(RokuInput::KeyPress(RokuKey::Back))).await.expect("Failed to send input");
									}
								}
								Key::Char(k) => {
									// set the key to be active, set/reset a timer to assume inactivity
									// however all functionality tied to keys must be handled here, I don't want to
									// double-dispatch on keys because I think they're still active.
									// This means that `self.active_keys` *is only used for rendering*, not control flow!

									// Another issue is if I get the same key many times, I should keep it active like described above, but...
									// EX: I get some key mapped to volume down (VD). I keep getting VD, so it remains active.
									//     What should happen is the key remains visually active but there is modest throttling on the rate at which new actions will be triggered.
									//     For each key I should also record the time it was last received, so I can perform throttling.  The user will have to wait ~100ms between keypresses or something for them.
									//     to register.

									// TODO: once we have active key map, create shift-tab to go backwards

									let lc = k.to_ascii_lowercase();

									match lc {
										'\t' => self.selected_device_index = (self.selected_device_index + 1) % self.devices.len(),
										'p'  => {
											if self.devices.len() != 0 {
												self.devices[self.selected_device_index].supply_input(DeviceInput::Roku(RokuInput::KeyPress(RokuKey::Power))).await.expect("Failed to send input");
											}
										},
										'w'  => {
											if self.devices.len() != 0 {
												self.devices[self.selected_device_index].supply_input(DeviceInput::Roku(RokuInput::KeyPress(RokuKey::PadUp))).await.expect("Failed to send input");
											}
										},
										'a'  => {
											if self.devices.len() != 0 {
												self.devices[self.selected_device_index].supply_input(DeviceInput::Roku(RokuInput::KeyPress(RokuKey::PadLeft))).await.expect("Failed to send input");
											}
										},
										's'  => {
											if self.devices.len() != 0 {
												self.devices[self.selected_device_index].supply_input(DeviceInput::Roku(RokuInput::KeyPress(RokuKey::PadDown))).await.expect("Failed to send input");
											}
										},
										'd'  => {
											if self.devices.len() != 0 {
												self.devices[self.selected_device_index].supply_input(DeviceInput::Roku(RokuInput::KeyPress(RokuKey::PadRight))).await.expect("Failed to send input");
											}
										},
										' ' => {
											if self.devices.len() != 0 {
												self.devices[self.selected_device_index].supply_input(DeviceInput::Roku(RokuInput::KeyPress(RokuKey::Ok))).await.expect("Failed to send input");
											}
										},
										'm' => {
											if self.devices.len() != 0 {
												self.devices[self.selected_device_index].supply_input(DeviceInput::Roku(RokuInput::KeyPress(RokuKey::VolumeMute))).await.expect("Failed to send input");
											}
										},
										'i' => {
											self.context = match self.context {
												UIContext::Main => UIContext::DeviceInfo,
												UIContext::DeviceInfo => UIContext::Main
											}
										},
										'h' => {
											if self.devices.len() != 0 {
												self.devices[self.selected_device_index].supply_input(DeviceInput::Roku(RokuInput::KeyPress(RokuKey::Home))).await.expect("Failed to send input");
											}
										}
										_ => ()
									};

									self.render();
								},
								Key::Ctrl(k) => {
									let lc = k.to_ascii_lowercase();
									if lc == 'c' || lc == 'd' { break; }
								}
								_ => (),
							}
						}
						None => break,
					}

					// refresh consumed future, reuse other future
					key_future = user_input_events.next();
					device_future = device_future_continue;
				}

				// new device
				Either::Right((device, key_future_continue)) => {
					match device {
						None => break,
						Some(d) => self.devices.push(d),
					};

					// println!("Got devices: {:?}", self.devices);

					self.render();

					// refresh consumed future, reuse other future
					key_future = key_future_continue;
					device_future = roku_discovery_rx.next();
				}
			}
		}

		Ok(())
	}
}

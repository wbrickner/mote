use futures::{channel::mpsc::UnboundedReceiver, future::{select, Either}, stream::StreamExt};
use std::{collections::HashMap, time::Instant};
use std::io;
use std::sync::{Arc, Mutex};
use termion::{event::Key, raw::{IntoRawMode, RawTerminal}};
use tui::{
  Terminal, 
  backend::TermionBackend, 
  layout::{Alignment, Rect}, 
  style::{Color, Style}, 
  text::{Span, Spans}, 
  widgets::{Block, BorderType, Borders, Paragraph, Tabs},
  symbols::line::VERTICAL
};

mod user_input;
use user_input::UserInput;

use crate::devices::{Device, DeviceInput, RokuKey};

const REMOTE_ASPECT_RATIO: f64 = 2.0 / 5.5;
const REMOTE_WIDTH_PIXELS: f64 = 512.0;

enum UIContext { Main, DeviceInfo }

pub struct UI {
  /// terminal abstraction
	terminal: Terminal<TermionBackend<RawTerminal<std::io::Stdout>>>,

	/// Device states
	devices: Vec<Device>,

	/// Which device is active
	selected_device_index: usize,

  /// Which subscreen the user is viewing
  context: UIContext,
  
  /// track the active keys for rendering purposes
  active_keys: Arc<Mutex<HashMap<Key, (bool, Instant)>>>
}

impl UI {
	pub fn new() -> Self {
		let stdout = io::stdout()
			.into_raw_mode()
      .expect("Failed to put terminal into 'raw mode'");
    
		let backend = TermionBackend::new(stdout);
		let mut terminal = Terminal::new(backend).expect("Failed to initialize terminal abstraction");
		
		terminal.clear().expect("Failed to clear terminal");
		terminal.hide_cursor().expect("Failed to hide cursor");

    println!("Searching. Devices appear as they're discovered.");

		UI {
			terminal,
			devices: vec![],
			selected_device_index: 0,
      context: UIContext::Main,
      active_keys: Arc::from(Mutex::from(HashMap::new()))
		}
	}

	// draw based on state
	fn render(&mut self) {
    let tab_titles: Vec<Spans> = self.devices.iter().map(|d| Spans::from(d.device_info().name.clone())).collect();
    let selected_index = self.selected_device_index;
    let selected_device = &self.devices[selected_index];
    let ip = selected_device.ip().clone();
    let info = selected_device.device_info();
    let context = &self.context;

    let (
      wpad_state, 
      apad_state, 
      spad_state, 
      dpad_state, 
      kpad_state
    ) = {
      let keystates = self.active_keys.lock().unwrap();

      (
        keystates.get(&Key::Char('w')).and_then(|&(s, _)| Some(s)).unwrap_or(false),
        keystates.get(&Key::Char('a')).and_then(|&(s, _)| Some(s)).unwrap_or(false),
        keystates.get(&Key::Char('s')).and_then(|&(s, _)| Some(s)).unwrap_or(false),
        keystates.get(&Key::Char('d')).and_then(|&(s, _)| Some(s)).unwrap_or(false),
        keystates.get(&Key::Char(' ')).and_then(|&(s, _)| Some(s)).unwrap_or(false)
      )
    };

		self.terminal.draw(move |f| {
			let (terminal_char_width, terminal_char_height) = termion::terminal_size().expect("Failed to get information about terminal size (in chars)");
			let (terminal_px_width, terminal_px_height) = termion::terminal_size_pixels().expect("Failed to get information about terminal size (in pixels)");
		
			let terminal_font_px_width = (terminal_px_width as f64) / (terminal_char_width as f64);
			let terminal_font_px_height = (terminal_px_height as f64) / (terminal_char_height as f64);
		
			let remote_char_width = REMOTE_WIDTH_PIXELS / terminal_font_px_width;
			let remote_char_height = REMOTE_ASPECT_RATIO * REMOTE_WIDTH_PIXELS / terminal_font_px_height;
		
      let tabs = 
        Tabs::new(tab_titles)
          .block(
            Block::default()
              .title("Devices")
              .borders(Borders::ALL)
            )
          .style(
            Style::default()
              .bg(Color::Black)
              .fg(Color::White)
          )
          .highlight_style(
            Style::default()
              .bg(Color::Black)
              .fg(Color::Yellow)
          )
          .divider(VERTICAL)
          .select(selected_index);
			f.render_widget(tabs, Rect::new(0, 0, remote_char_width.round() as u16, 3));

			let info_contents = match context {
        UIContext::Main => vec![
          Spans::from(Span::raw(format!(" {} ({})", info.name, ip)))
        ],
        UIContext::DeviceInfo => vec![
          Spans::from(Span::raw(format!(" {}", info.name))),
          Spans::from(Span::raw(format!(" ├── Network"))),
          Spans::from(Span::raw(format!(" │   ├── Name: {}",        info.network.network_name))),
          Spans::from(Span::raw(format!(" │   ├── Type: {}",        info.network.network_type))),
          Spans::from(Span::raw(format!(" │   ├── IP: {}",          ip))),
          Spans::from(Span::raw(format!(" │   └── MAC Address: {}", info.network.mac_address))),
          Spans::from(Span::raw(format!(" ├── Product"))),
          Spans::from(Span::raw(format!(" │   ├── Vendor: {}",        info.product.vendor))),
          Spans::from(Span::raw(format!(" │   ├── Model Name: {}",    info.product.model.name))),
          Spans::from(Span::raw(format!(" │   ├── Model Number: {}",  info.product.model.number))),
          Spans::from(Span::raw(format!(" │   └── Serial Number: {}", info.product.serial_number))),
          Spans::from(Span::raw(format!(" └── System"))),
          Spans::from(Span::raw(format!("     └── Uptime: {}", match &info.system.uptime {
            None => "unknown".into(),
            Some(u) => u.pretty()
          })))
        ]
      };
      let info_height = info_contents.len() as u16;

      // render dynamic info widget
      let info = Paragraph::new(info_contents)
        // .wrap(Wrap { trim: false })
        .alignment(Alignment::Left);
      f.render_widget(info, Rect::new(0, 3, terminal_char_width, info_height));

      let remote_y = 3 + info_height;
      // do not respect the exact ratio, it looks ugly because it ends up such an odd line-snapping
      let remote_width = 1 + remote_char_width.round() as u16;
      let remote_height = 1 + remote_char_height.round() as u16;

      let remote_body = Block::default()
        .style(
          Style::default()
            .bg(Color::Black)
            .fg(Color::White)
        )
        .borders(Borders::ALL)
        .border_type(BorderType::Plain);

      f.render_widget(
        remote_body,
        Rect::new(0, remote_y, remote_width, remote_height)
      );

      // render the direction pads
      let w_pad  = Paragraph::new("\nW").style(    Style::default().bg(if wpad_state { Color::Blue } else { Color::LightBlue }).fg(Color::White)).alignment(Alignment::Center);
      let a_pad  = Paragraph::new("\nA").style(    Style::default().bg(if apad_state { Color::Blue } else { Color::LightBlue }).fg(Color::White)).alignment(Alignment::Center);
      let s_pad  = Paragraph::new("\nS").style(    Style::default().bg(if spad_state { Color::Blue } else { Color::LightBlue }).fg(Color::White)).alignment(Alignment::Center);
      let d_pad  = Paragraph::new("\nD").style(    Style::default().bg(if dpad_state { Color::Blue } else { Color::LightBlue }).fg(Color::White)).alignment(Alignment::Center);
      let ok_pad = Paragraph::new("\nSPACE").style(Style::default().bg(if kpad_state { Color::Blue } else { Color::LightBlue }).fg(Color::White)).alignment(Alignment::Center);
      
      let dirpad_y_offset = remote_height / 2 - 1;
      let dirpad_x_offset = 4;
      let pad_width = 7;
      let pad_height = 3;
      let x_extension = 7;
      let y_extension = 3;

      f.render_widget(w_pad,  Rect::new(dirpad_x_offset + x_extension,     remote_y + dirpad_y_offset - y_extension, pad_width, pad_height));
      f.render_widget(s_pad,  Rect::new(dirpad_x_offset + x_extension,     remote_y + dirpad_y_offset + y_extension, pad_width, pad_height));
      f.render_widget(a_pad,  Rect::new(dirpad_x_offset + 0,               remote_y + dirpad_y_offset - 0,           pad_width, pad_height));
      f.render_widget(d_pad,  Rect::new(dirpad_x_offset + 2 * x_extension, remote_y + dirpad_y_offset - 0,           pad_width, pad_height));
      f.render_widget(ok_pad, Rect::new(dirpad_x_offset + x_extension,     remote_y + dirpad_y_offset - 0,           pad_width, pad_height));

      // render the back and home buttons
      let buttons_x_offset = remote_width - 45;
      let buttons_y_offset = remote_y + dirpad_y_offset - y_extension + 1;
      let button_pad_width = 11;
      let button_pad_height = 3;
      let button_pad_margin = 4;

      let back_pad = Paragraph::new("\n⌫").style(Style::default().bg(Color::DarkGray).fg(Color::White)).alignment(Alignment::Center);
      let home_pad = Paragraph::new("\nH").style(Style::default().bg(Color::DarkGray).fg(Color::White)).alignment(Alignment::Center);
      let power_pad = Paragraph::new("\nP").style(Style::default().bg(Color::DarkGray).fg(Color::LightRed)).alignment(Alignment::Center);

      let replay_pad = Paragraph::new("\n↺").style(Style::default().bg(Color::DarkGray).fg(Color::White)).alignment(Alignment::Center);
      let star_pad = Paragraph::new("\n*").style(Style::default().bg(Color::DarkGray).fg(Color::White)).alignment(Alignment::Center);
      let mute_pad = Paragraph::new("\nM").style(Style::default().bg(Color::DarkGray).fg(Color::White)).alignment(Alignment::Center);

      f.render_widget(back_pad,  Rect::new(buttons_x_offset,                                            buttons_y_offset, button_pad_width, button_pad_height));
      f.render_widget(home_pad,  Rect::new(buttons_x_offset + button_pad_width + button_pad_margin,     buttons_y_offset, button_pad_width, button_pad_height));
      f.render_widget(power_pad, Rect::new(buttons_x_offset + 2*(button_pad_width + button_pad_margin), buttons_y_offset, button_pad_width, button_pad_height));

      f.render_widget(replay_pad, Rect::new(buttons_x_offset,                                            buttons_y_offset + button_pad_height + 1, button_pad_width, button_pad_height));
      f.render_widget(star_pad,   Rect::new(buttons_x_offset + button_pad_width + button_pad_margin,     buttons_y_offset + button_pad_height + 1, button_pad_width, button_pad_height));
      f.render_widget(mute_pad,   Rect::new(buttons_x_offset + 2*(button_pad_width + button_pad_margin), buttons_y_offset + button_pad_height + 1, button_pad_width, button_pad_height));
    })
    .expect("Failed to render")
  }

  pub fn supply_input(&mut self, input: DeviceInput) {
    if let Some(device) = self.devices.get(self.selected_device_index) {
      device.supply_input(input)
    }
  }

  async fn handle_key(&mut self, key: Key) -> bool {
    match key {
      Key::Delete | Key::Backspace => self.supply_input(RokuKey::Back.into()),
      Key::Esc => self.supply_input(RokuKey::Home.into()),

      Key::Up => self.supply_input(RokuKey::VolumeUp.into()),
      Key::Down => self.supply_input(RokuKey::VolumeDown.into()),
      Key::Left => self.supply_input(RokuKey::InstantReplay.into()),
      Key::BackTab => {
        if self.selected_device_index == 0 {
          if self.devices.len() != 0 {
            self.selected_device_index = self.devices.len()
          }
        } else {
          self.selected_device_index -= 1;
        }
      },

      Key::Char(k) => match k.to_ascii_lowercase() {
        // CLU UI controls
        '\t' => self.selected_device_index = (self.selected_device_index + 1) % self.devices.len(),
        'i' | 'I'  => self.context = match self.context {
          UIContext::Main => UIContext::DeviceInfo,
          UIContext::DeviceInfo => UIContext::Main
        },

        // special control keys
        'p' | 'P'  => self.supply_input(RokuKey::Power.into()),
        'h' | 'H'  => self.supply_input(RokuKey::Home.into()),
        'm' | 'M'  => self.supply_input(RokuKey::VolumeMute.into()),
        '*'        => self.supply_input(RokuKey::Info.into()),

        // arrow pad keys
        'w' | 'W' => self.supply_input(RokuKey::PadUp.into()),
        'a' | 'A' => self.supply_input(RokuKey::PadLeft.into()),
        's' | 'S' => self.supply_input(RokuKey::PadDown.into()),
        'd' | 'D' => self.supply_input(RokuKey::PadRight.into()),
        ' '       => self.supply_input(RokuKey::Ok.into()),

        _ => return false
      },

      Key::Ctrl(k) => {
        match k.to_ascii_lowercase() {
          'c' | 'd' | 'C' | 'D' => return true,
          _ => ()
        }
      },
      _ => return false
    }

    // mark key as active and immediately drop lock (unlock)
    // let instant = Instant::now();
    // let storage_key = match key {
    //   Key::Char(c) => Key::Char(c.to_ascii_uppercase()),
    //   k => k
    // };

    // {
    //   (*self.active_keys).lock().unwrap().insert(storage_key, (true, instant.clone()));
    // }

    // spawn task to mark inactive in 100ms
    // let active_keys_ref = Arc::clone(&self.active_keys);

    // tokio::spawn(async move {
    //   delay_for(Duration::from_millis(100)).await;
    //   let mut keystates = (*active_keys_ref).lock().unwrap();
    //   if let Some(&(_, event_instant)) = keystates.get(&storage_key) {
    //     // a newer event created the current active state, do nothing and let it live a full life (<3)
    //     if event_instant != instant { return }
    //     // otherwise, set it to inactive
    //     keystates.insert(storage_key, (false, event_instant));
    //   }
    // });

    false
  }

  /// Handles input and discovery events, refreshing the UI after eache event.
  /// Only redraws when an event has occurred, however does not perform any logic
  /// to determine if the UI actually needs to be re-rendered, so is maybe 
  /// slightly suboptimal depending on the cost of this logic.
	pub async fn listen(&mut self, roku_discovery_rx: &mut UnboundedReceiver<Device>) {
    let mut user_input_events = UserInput::new();
    let mut key_future = user_input_events.next();
		let mut device_future = roku_discovery_rx.next();

		loop {
      match select(key_future, device_future).await {
        Either::Left((key, new_device_and_render_future)) => {
          if let Some(key) = key {
            if self.handle_key(key).await { break; }
            self.render();
          } else {
            break // channel corrupted, input task died, end the UI loop as well
          }

          key_future = user_input_events.next();
          device_future = new_device_and_render_future;
        },
        Either::Right((device, new_key_future)) => {
          if let Some(device) = device {
            self.devices.push(device);
            self.render();
          } else {
            break // channel corrupted, discovery task died, end the UI loop as well
          }

          key_future = new_key_future;
          device_future = roku_discovery_rx.next();
        }
      }
		}
  }
}
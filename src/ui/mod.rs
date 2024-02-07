use futures::future::{select, Either};
use std::{collections::HashMap, time::Instant};
use tokio::sync::mpsc::UnboundedReceiver;
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
use tokio_stream::{wrappers::UnboundedReceiverStream, StreamExt};
use crate::devices::device_input::DeviceInput;
use self::user_input::user_input;

use super::devices::{Device, roku::RokuKey};
mod user_input;

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
    let ip = selected_device.ip_string().clone();
    let info = selected_device.device_info();
    let context = &self.context;

    let (
      wpad_state, 
      apad_state, 
      spad_state, 
      dpad_state, 
      kpad_state
    ) = {
      let ks = self.active_keys.lock().unwrap();
      (
        ks.contains_key(&Key::Char('w')),
        ks.contains_key(&Key::Char('a')),
        ks.contains_key(&Key::Char('s')),
        ks.contains_key(&Key::Char('d')),
        ks.contains_key(&Key::Char(' '))
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
      let pad = |name, state| {
        Paragraph::new(name)
          .style(
            Style::default()
              .bg(if state { Color::Blue } else { Color::LightBlue })
              .fg(Color::White)
          )
          .alignment(Alignment::Center)
      };

      let w_pad  = pad("\nW", wpad_state);
      let a_pad  = pad("\nA", apad_state);
      let s_pad  = pad("\nS", spad_state);
      let d_pad  = pad("\nD", dpad_state);
      let ok_pad = pad("\nSPACE", kpad_state);

      let dirpad_y_offset = remote_height / 2 - 1;
      let dirpad_x_offset = 4;
      let pad_width       = 7;
      let pad_height      = 3;
      let x_extension     = 7;
      let y_extension     = 3;

      f.render_widget(w_pad,  Rect::new(dirpad_x_offset + x_extension,     remote_y + dirpad_y_offset - y_extension, pad_width, pad_height));
      f.render_widget(s_pad,  Rect::new(dirpad_x_offset + x_extension,     remote_y + dirpad_y_offset + y_extension, pad_width, pad_height));
      f.render_widget(a_pad,  Rect::new(dirpad_x_offset + 0,               remote_y + dirpad_y_offset - 0,           pad_width, pad_height));
      f.render_widget(d_pad,  Rect::new(dirpad_x_offset + 2 * x_extension, remote_y + dirpad_y_offset - 0,           pad_width, pad_height));
      f.render_widget(ok_pad, Rect::new(dirpad_x_offset + x_extension,     remote_y + dirpad_y_offset - 0,           pad_width, pad_height));

      // render the back and home buttons
      let buttons_x_offset  = remote_width - 45;
      let buttons_y_offset  = remote_y + dirpad_y_offset - y_extension + 1;
      let button_pad_width  = 11;
      let button_pad_height = 3;
      let button_pad_margin = 4;

      let back_pad   = Paragraph::new("\n⌫").style(Style::default().bg(Color::DarkGray).fg(Color::White)).alignment(Alignment::Center);
      let home_pad   = Paragraph::new("\nH").style(Style::default().bg(Color::DarkGray).fg(Color::White)).alignment(Alignment::Center);
      let power_pad  = Paragraph::new("\nP").style(Style::default().bg(Color::DarkGray).fg(Color::LightRed)).alignment(Alignment::Center);

      f.render_widget(back_pad,   Rect::new(buttons_x_offset,                                            buttons_y_offset, button_pad_width, button_pad_height));
      f.render_widget(home_pad,   Rect::new(buttons_x_offset + button_pad_width + button_pad_margin,     buttons_y_offset, button_pad_width, button_pad_height));
      f.render_widget(power_pad,  Rect::new(buttons_x_offset + 2*(button_pad_width + button_pad_margin), buttons_y_offset, button_pad_width, button_pad_height));

      let replay_pad = Paragraph::new("\n↺").style(Style::default().bg(Color::DarkGray).fg(Color::White)).alignment(Alignment::Center);
      let star_pad   = Paragraph::new("\n*").style(Style::default().bg(Color::DarkGray).fg(Color::White)).alignment(Alignment::Center);
      let mute_pad   = Paragraph::new("\nM").style(Style::default().bg(Color::DarkGray).fg(Color::White)).alignment(Alignment::Center);

      f.render_widget(replay_pad, Rect::new(buttons_x_offset,                                            buttons_y_offset + button_pad_height + 1, button_pad_width, button_pad_height));
      f.render_widget(star_pad,   Rect::new(buttons_x_offset + button_pad_width + button_pad_margin,     buttons_y_offset + button_pad_height + 1, button_pad_width, button_pad_height));
      f.render_widget(mute_pad,   Rect::new(buttons_x_offset + 2*(button_pad_width + button_pad_margin), buttons_y_offset + button_pad_height + 1, button_pad_width, button_pad_height));
    })
    .expect("Failed to render")
  }

  fn send(&mut self, input: DeviceInput) {
    if let Some(device) = self.devices.get(self.selected_device_index) {
      device.send_input(input)
    }
  }

  async fn on_key(&mut self, key: Key) -> bool {
    match key {
      Key::Delete | Key::Backspace => self.send(RokuKey::Back.into()),
      Key::Esc => self.send(RokuKey::Home.into()),

      Key::Up => self.send(RokuKey::VolumeUp.into()),
      Key::Down => self.send(RokuKey::VolumeDown.into()),
      Key::Left => self.send(RokuKey::InstantReplay.into()),
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
        'p' | 'P'  => self.send(RokuKey::Power.into()),
        'h' | 'H'  => self.send(RokuKey::Home.into()),
        'm' | 'M'  => self.send(RokuKey::VolumeMute.into()),
        '*'        => self.send(RokuKey::Info.into()),

        // arrow pad keys
        'w' | 'W' => self.send(RokuKey::PadUp.into()),
        'a' | 'A' => self.send(RokuKey::PadLeft.into()),
        's' | 'S' => self.send(RokuKey::PadDown.into()),
        'd' | 'D' => self.send(RokuKey::PadRight.into()),
        ' '       => self.send(RokuKey::Ok.into()),

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

    false
  }

  /// Handles input and discovery events, refreshing the UI after eache event.
  /// Only redraws when an event has occurred, however does not perform any logic
  /// to determine if the UI actually needs to be re-rendered, so is maybe 
  /// slightly suboptimal depending on the cost of this logic.
	pub async fn listen(&mut self, rx: UnboundedReceiver<Device>) {
    let mut input = user_input();
    let mut press = Box::pin(input.next());

    let mut discovery = UnboundedReceiverStream::new(rx);
		let mut device = Box::pin(discovery.next());

		loop {
      self.render();
      
      match select(press, device).await {
        Either::Left((k, f)) => {
          if let Some(key) = k { if self.on_key(key).await { break; } } 
          else { break }

          press = Box::pin(input.next());
          device = f;
        },
        Either::Right((d, f)) => {
          if let Some(device) = d { self.devices.push(device); } 
          else { break }

          press = f;
          device = Box::pin(discovery.next())
        }
      }
		}
  }
}
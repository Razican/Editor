// #![forbid(missing_docs, warnings)]
#![deny(deprecated, drop_with_repr_extern, improper_ctypes,
        non_shorthand_field_patterns, overflowing_literals, plugin_as_library,
        private_no_mangle_fns, private_no_mangle_statics, stable_features, unconditional_recursion,
        unknown_lints, unused_allocation, unused_attributes,
        unused_comparisons, unused_features, unused_parens, while_true)]
#![warn(trivial_casts, trivial_numeric_casts, unused, unused_extern_crates,
        unused_import_braces, unused_qualifications, unused_results, variant_size_differences)]
#![allow(missing_docs)]

extern crate piston_window;
extern crate glutin;
extern crate fps_counter;
extern crate time;

mod backend;

use std::{io, fmt, u8};
use std::error::Error as StdErr;

use piston_window::*;
use piston_window::character::CharacterCache;
use glutin::MouseCursor;

use fps_counter::FPSCounter;

use backend::*;

const BACKGROUND_COLOR: [f32; 4] = [33 as f32 / u8::MAX as f32,
                                    37 as f32 / u8::MAX as f32,
                                    43 as f32 / u8::MAX as f32,
                                    255 as f32 / u8::MAX as f32];
const EDITOR_BG_COLOR: [f32; 4] = [40 as f32 / u8::MAX as f32,
                                   44 as f32 / u8::MAX as f32,
                                   52 as f32 / u8::MAX as f32,
                                   255 as f32 / u8::MAX as f32];
const BG_COLOR_LIGHT: [f32; 4] = [44 as f32 / u8::MAX as f32,
                                  50 as f32 / u8::MAX as f32,
                                  60 as f32 / u8::MAX as f32,
                                  255 as f32 / u8::MAX as f32];
const CURSOR_COLOR: [f32; 4] = [82 as f32 / u8::MAX as f32,
                                139 as f32 / u8::MAX as f32,
                                255 as f32 / u8::MAX as f32,
                                255 as f32 / u8::MAX as f32];

const EM: u32 = 32;
const MENU_WIDTH: f64 = 250.0;

const SOFT_TABS: &'static str = "    ";
const TAB_FILL: &'static str = SOFT_TABS;

fn main() {
    // TODO read config

    let mut buf = TextBuffer::new(Some("test.txt")).unwrap();
    buf.load(|_, _| {}).unwrap();
    buf.set_cursors(vec![Default::default()]);

    let mut window: PistonWindow = WindowSettings::new("main.rs", [1920, 1080])
        .vsync(true)
        .build()
        .unwrap();

    let factory = window.factory.clone();
    let mut glyphs = Glyphs::new("fonts/cnr.otf", factory).unwrap();

    let mut fps_counter = FPSCounter::new();
    let mut events = window.events();
    while let Some(e) = events.next(&mut window) {
        match e {
            Event::Render(_) => {
                let draw_size = window.draw_size();

                let _ = window.draw_2d(&e, |c, g| {
                    clear(BACKGROUND_COLOR, g);
                    println!("Context: {{viewport: {{rect: {:?}, draw_size: {:?}, window_size: \
                              {:?}}}, view: {:?}, transform: {:?}, draw_state: {:?}}}",
                             c.viewport.unwrap().rect,
                             c.viewport.unwrap().draw_size,
                             c.viewport.unwrap().window_size,
                             c.view,
                             c.transform,
                             c.draw_state);

                    let transform = c.transform.trans(10.0, 100.0);
                    Text::new_color([1.0; 4], (EM as f32 * 0.7) as u32)
                        .draw(&format!("FPS: {}", fps_counter.tick()),
                              &mut glyphs,
                              &c.draw_state,
                              transform,
                              g);

                    let transform = c.transform.trans(MENU_WIDTH, 0.0);
                    rectangle(EDITOR_BG_COLOR,
                              [0.0,
                               0.0,
                               draw_size.width as f64 - MENU_WIDTH,
                               draw_size.height as f64],
                              transform,
                              g);

                    for cursor in buf.get_cursors() {
                        rectangle(BG_COLOR_LIGHT,
                                  [0.0,
                                   10.0 + EM as f64 * cursor.start_line as f64 * 1.1,
                                   draw_size.width as f64 - MENU_WIDTH,
                                   EM as f64 * (cursor.end_line - cursor.start_line + 1) as f64 *
                                   1.1],
                                  transform,
                                  g);
                        if cursor.is_atomic() {
                            let now_tick = (time::precise_time_ns() % 1_000_000_000) / 500_000_000;
                            if now_tick == 0 {
                                let c_transform = transform.trans(cursor.start_character as f64 *
                                           glyphs.character((EM as f32 * 0.7) as u32, ' ').width(),
                                           10.0 + cursor.start_line as f64 * EM as f64 * 1.1);
                                line(CURSOR_COLOR,
                                     EM as f64 / 15.0,
                                     [0.0, 0.0, 0.0, EM as f64],
                                     c_transform,
                                     g);
                            }
                        }
                    }

                    for (i, line) in buf.lines().enumerate() {
                        let transform = transform.trans(0.0, EM as f64 * 1.1 * (i + 1) as f64);
                        let line = if line.chars().rev().next() == Some('\n') {
                            &line[..line.len() - 1]
                        } else {
                            &line
                        };
                        Text::new_color([1.0; 4], (EM as f32 * 0.7) as u32)
                            .draw(&line, &mut glyphs, &c.draw_state, transform, g);
                    }
                });
            }
            Event::Input(Input::Text(ref s)) => {
                for c in s.chars() {
                    buf.write_character(c);
                }
            }
            Event::Input(Input::Press(Button::Keyboard(Key::Return))) => {
                buf.write_character('\n');
            }
            Event::Input(Input::Press(Button::Keyboard(Key::Tab))) => {
                // TODO optimize with write_str
                for c in TAB_FILL.chars() {
                    buf.write_character(c);
                }
            }
            Event::Input(Input::Press(Button::Keyboard(Key::Backspace))) => {
                buf.write_character(BACKSPACE);
            }
            Event::Input(Input::Press(Button::Keyboard(Key::Delete))) => {
                buf.write_character(DEL);
            }
            Event::Input(Input::Press(Button::Keyboard(Key::Left))) => {
                buf.move_cursors(Move::Left);
            }
            Event::Input(Input::Press(Button::Keyboard(Key::Right))) => {
                buf.move_cursors(Move::Right);
            }
            Event::Input(Input::Press(Button::Keyboard(Key::Up))) => {
                buf.move_cursors(Move::Up);
            }
            Event::Input(Input::Press(Button::Keyboard(Key::Down))) => {
                buf.move_cursors(Move::Down);
            }
            Event::Input(Input::Move(Motion::MouseCursor(x, _y))) => {
                if x > MENU_WIDTH {
                    window.window.window.set_cursor(MouseCursor::Text);
                } else {
                    window.window.window.set_cursor(MouseCursor::Default);
                }
            }
            Event::Input(Input::Move(Motion::MouseScroll(_x, _y))) => {}
            Event::Input(Input::Focus(false)) => buf.save(|_, _| {}).unwrap(),
            _ => {}
        }
        let _ = e.update(|_| {});
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    BigFileSize,
    IO(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IO(err)
    }
}

impl StdErr for Error {
    fn description(&self) -> &str {
        match self {
            &Error::BigFileSize => "file is too big",
            &Error::IO(ref e) => e.description(),
        }
    }

    fn cause(&self) -> Option<&StdErr> {
        match self {
            &Error::BigFileSize => None,
            &Error::IO(ref e) => Some(e),
        }
    }
}

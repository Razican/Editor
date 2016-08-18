use std::{fs, usize};
use std::io::{Write, BufRead, BufReader};
use std::path::Path;
use std::slice::Iter;

use super::Result;

pub const BACKSPACE: char = '\u{0008}';
pub const DEL: char = '\u{007F}';
pub const ALLOWED_CONTROL: [char; 4] = ['\t', '\n', BACKSPACE, DEL];

#[derive(Clone, Copy, Debug)]
pub struct Cursor {
    pub start_line: usize,
    pub start_byte: usize,
    pub start_character: usize,
    pub end_line: usize,
    pub end_byte: usize,
    pub end_character: usize,
}

impl Cursor {
    fn atomize(&mut self) {
        self.end_line = self.start_line;
        self.end_byte = self.start_byte;
        self.end_character = self.start_character;
    }

    pub fn is_atomic(&self) -> bool {
        self.start_line == self.end_line && self.start_byte == self.end_byte
    }
}

impl Default for Cursor {
    fn default() -> Cursor {
        Cursor {
            start_line: 0,
            start_byte: 0,
            start_character: 0,
            end_line: 0,
            end_byte: 0,
            end_character: 0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextBuffer {
    path: Option<String>,
    size: usize,
    lines: Vec<String>,
    saved: bool,
    loaded: bool,
    cursors: Vec<Cursor>,
}

impl TextBuffer {
    pub fn new<P: AsRef<Path>>(path: Option<P>) -> Result<TextBuffer> {
        let size = if path.is_some() {
            try!(path.as_ref().unwrap().as_ref().metadata()).len() as usize
        } else {
            0
        };
        let owned_path = match path.as_ref() {
            Some(p) => Some(p.as_ref().to_string_lossy().into_owned()),
            None => None,
        };

        let text_buffer = TextBuffer {
            path: owned_path,
            size: size,
            lines: if path.is_some() {
                Vec::new()
            } else {
                vec![String::new()]
            },
            saved: path.is_some(),
            loaded: false,
            cursors: Vec::new(),
        };

        Ok(text_buffer)
    }

    pub fn load<F>(&mut self, callback: F) -> Result<()>
        where F: Fn(usize, usize)
    {
        let f = try!(fs::File::open(self.path.as_ref().unwrap()));
        let reader = BufReader::new(f);
        let mut read_bytes = 0usize;
        for line in reader.lines() {
            let line = try!(line);
            read_bytes += line.as_bytes().len();
            self.lines.push(line + "\n");
            callback(read_bytes, self.size);
        }
        self.loaded = true;

        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.loaded
    }

    pub fn file_size(&self) -> usize {
        self.size
    }

    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    pub fn get_path(&self) -> Option<&Path> {
        match self.path.as_ref() {
            Some(p) => Some(Path::new(p)),
            None => None,
        }
    }

    pub fn set_path<P: AsRef<Path>>(&mut self, path: P) {
        self.path = Some(path.as_ref().to_string_lossy().into_owned());
    }

    pub fn remove_path(&mut self) {
        self.saved = false;
        self.path = None
    }

    pub fn save<F>(&mut self, callback: F) -> Result<()>
        where F: Fn(usize, usize)
    {
        if !self.saved {
            let path = Path::new(self.path.as_ref().unwrap());

            let mut f = if path.exists() {
                try!(fs::OpenOptions::new().write(true).truncate(true).open(path))
            } else {
                try!(fs::File::create(path))
            };

            let mut wrote_bytes = 0usize;
            let total_bytes = self.lines.iter().fold(0, |acc, x| acc + x.as_bytes().len());
            for line in &self.lines {
                let bytes = line.as_bytes();
                try!(f.write_all(bytes));
                wrote_bytes += bytes.len();
                callback(wrote_bytes, total_bytes);
            }

            self.saved = true;
        }

        Ok(())
    }

    pub fn get_cursors(&self) -> &[Cursor] {
        &self.cursors
    }

    pub fn set_cursors(&mut self, cursors: Vec<Cursor>) {
        // TODO check that cursors are valid with asserts
        self.cursors = cursors;
    }

    pub fn move_cursors(&mut self, movement: Move) {
        for cursor in self.cursors.iter_mut() {
            match movement {
                Move::Up => {
                    if cursor.start_line != 0 {
                        cursor.start_line -= 1;
                        let line_chars = self.lines[cursor.start_line].chars().count() - 1;
                        if line_chars > cursor.start_character {
                            cursor.start_byte = self.lines[cursor.start_line]
                                .char_indices()
                                .fold(0, |acc, (i, c)| if i < cursor.start_character {
                                    acc + c.len_utf8()
                                } else {
                                    acc
                                })
                        } else {
                            cursor.start_byte = self.lines[cursor.start_line].len() - 1;
                            cursor.start_character = line_chars;
                        }
                    } else {
                        cursor.start_byte = 0;
                        cursor.start_character = 0;
                    }
                    cursor.atomize();
                }
                Move::Down => {
                    if cursor.end_line == self.lines.len() - 1 {
                        cursor.start_character = self.lines[cursor.end_line].chars().count() - 1;
                        cursor.start_byte = if cursor.start_character == 0 {
                            0
                        } else {
                            self.lines[cursor.end_line].len() -
                            self.lines[cursor.end_line].chars().rev().next().unwrap().len_utf8()
                        };
                    } else {
                        cursor.start_line += 1;
                        let line_chars = self.lines[cursor.start_line].chars().count() - 1;
                        if line_chars > cursor.start_character {
                            cursor.start_byte = self.lines[cursor.start_line]
                                .char_indices()
                                .fold(0, |acc, (i, c)| if i < cursor.start_character {
                                    acc + c.len_utf8()
                                } else {
                                    acc
                                })
                        } else {
                            cursor.start_byte = self.lines[cursor.start_line].len() - 1;
                            cursor.start_character = line_chars;
                        }
                    }
                    cursor.atomize();
                }
                Move::Left => {
                    if cursor.is_atomic() {
                        if cursor.start_character != 0 {
                            cursor.start_byte -= self.lines[cursor.start_line][cursor.start_byte -
                                                                               1..]
                                .chars()
                                .next()
                                .unwrap()
                                .len_utf8();
                            cursor.start_character -= 1;
                        } else if cursor.start_line != 0 {
                            cursor.start_line -= 1;
                            cursor.start_byte = self.lines[cursor.start_line].len() - 1;
                            cursor.start_character =
                                self.lines[cursor.start_line].chars().count() - 1;
                        }
                    }
                    cursor.atomize();
                }
                Move::Right => {
                    if !cursor.is_atomic() {
                        cursor.start_line = cursor.end_line;
                        cursor.start_byte = cursor.end_byte;
                        cursor.start_character = cursor.end_character;
                    } else {
                        if cursor.start_line != self.lines.len() - 1 ||
                           cursor.start_character !=
                           self.lines[self.lines.len() - 1].chars().count() {
                            let next_char = self.lines[cursor.start_line][cursor.start_character..]
                                .chars()
                                .next();
                            if next_char != Some('\n') {
                                cursor.start_byte += next_char.unwrap().len_utf8();
                                cursor.start_character += 1;
                            } else {
                                cursor.start_line += 1;
                                cursor.start_byte = 0;
                                cursor.start_character = 0;
                            }
                            cursor.atomize()
                        }
                    }
                }
            }
        }
    }

    pub fn lines(&self) -> Iter<String> {
        self.lines.iter()
    }

    pub fn write_character(&mut self, c: char) {
        assert!(!c.is_control() || ALLOWED_CONTROL.contains(&c));

        for cursor in self.cursors.iter_mut() {
            if cursor.end_line > cursor.start_line + 1 {
                for line in cursor.start_line + 1..cursor.end_line {
                    self.saved = false;
                    let _ = self.lines.remove(line);
                }
            }
            match c {
                '\n' => {
                    self.saved = false;
                    if cursor.is_atomic() {
                        let (first_line, second_line) = {
                            let (first_line, second_line) = self.lines[cursor.start_line]
                                .split_at(cursor.start_byte);
                            (String::from(first_line), String::from(second_line))
                        };

                        self.lines[cursor.start_line] = first_line + "\n";
                        self.lines.insert(cursor.start_line + 1, second_line);

                        cursor.start_line = cursor.start_line + 1;
                        cursor.start_byte = 0;
                        cursor.start_character = 0;
                        cursor.atomize();
                    } else {
                        unimplemented!()
                    }
                }
                BACKSPACE => {
                    if cursor.is_atomic() {
                        if cursor.start_byte == 0 && cursor.start_line != 0 {
                            self.saved = false;
                            let _ = self.lines[cursor.start_line - 1].pop();
                            let new_index = self.lines[cursor.start_line - 1].len();
                            let new_char_index = self.lines[cursor.start_line - 1].chars().count();
                            let new_line = String::from(self.lines[cursor.start_line - 1]
                                .as_str()) +
                                           &self.lines.remove(cursor.start_line);
                            self.lines[cursor.start_line - 1] = new_line;

                            cursor.start_line -= 1;
                            cursor.start_byte = new_index;
                            cursor.start_character = new_char_index;
                            cursor.atomize();
                        } else if cursor.start_line != 0 || cursor.start_byte != 0 {
                            self.saved = false;
                            let mut index = cursor.start_byte - 1;
                            {
                                while !self.lines[cursor.start_line].is_char_boundary(index) {
                                    index -= 1;
                                }
                            }
                            let _ = self.lines[cursor.start_line].remove(index);

                            // Update cursor
                            cursor.start_byte = index;
                            cursor.start_character -= 1;
                            cursor.atomize();
                        }
                    } else {
                        unimplemented!()
                    }
                }
                DEL => unimplemented!(),
                _ => {
                    self.saved = false;
                    if cursor.is_atomic() {
                        self.lines[cursor.start_line].insert(cursor.start_byte, c);

                        // Update cursor
                        let new_cursor_char = cursor.start_byte + c.len_utf8();
                        cursor.start_byte = new_cursor_char;
                        cursor.start_character += 1;
                        cursor.atomize();
                    } else if cursor.start_line == cursor.end_line {
                        let pattern = String::from(
                            &self.lines[cursor.start_line]
                                [cursor.start_byte..cursor.end_byte]);
                        let new_line = self.lines[cursor.start_line]
                            .replace(&pattern, &c.escape_unicode().collect::<String>());
                        self.lines[cursor.start_line] = new_line;

                        // Update cursor
                        let new_cursor_char = cursor.start_byte + c.len_utf8();
                        cursor.start_byte = new_cursor_char;
                        cursor.start_character += 1;
                        cursor.atomize();
                    } else {
                        let second_line = self.lines.remove(cursor.end_line);
                        let second_line_preserve = &second_line[cursor.end_byte..];
                        let first_line_preserve =
                            String::from(&self.lines[cursor.start_line][cursor.start_byte..]);
                        self.lines[cursor.start_line] = first_line_preserve;
                        self.lines[cursor.start_line].push_str(second_line_preserve);

                        // Update cursor
                        let new_cursor_char = cursor.start_byte + c.len_utf8();
                        cursor.start_byte = new_cursor_char;
                        cursor.start_character += 1;
                        cursor.atomize();
                    }
                }
            }
        }
    }

    pub fn write_str<S: AsRef<str>>(&mut self, string: S) {
        for c in string.as_ref().chars() {
            assert!(!c.is_control() || ALLOWED_CONTROL.contains(&c));
        }
        for cursor in self.cursors.iter_mut() {
            // TODO
        }
        unimplemented!();
    }
}

pub enum Move {
    Up,
    Down,
    Left,
    Right,
}

use regex::Regex;
use std::{collections::HashMap, mem};

const CHORD_REGEX: &str = "^(C|D|E|F|G|A|B)(b|#)?(m|M|min|maj|dim|Δ|°|ø|Ø)?((sus|add)?(b|#)?\
                            (2|4|5|6|7|9|10|11|13)?)*(\\+|aug|alt)?(/(C|D|E|F|G|A|B)(b|#)?)?$";

#[derive(Default, Debug, Eq, PartialEq)]
pub struct SongChord {
    name: String,
    pos: u32,
}

impl Ord for SongChord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.pos.cmp(&other.pos)
    }
}

impl PartialOrd for SongChord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.pos.cmp(&other.pos))
    }
}

#[derive(Default, Debug)]
pub struct SongLine {
    text: String,
    chords: Vec<SongChord>,
}

impl SongLine {
    pub fn to_latex(&mut self) -> String {
        if self.chords.is_empty() {
            return format!("{}\n", self.text);
        }

        let mut out = self.text.clone();

        self.chords.sort_unstable();
        self.chords.reverse();

        let diff = self.chords[0].pos as i32 - out.len() as i32;
        if diff > 0 {
            out.push_str(&" ".repeat(diff as usize));
        }

        for chord in self.chords.iter() {
            out.insert_str(chord.pos as usize, &format!("\\[{}]", chord.name));
        }

        if self.text.is_empty() {
            return format!("\\nolyrics{{{}}}\n", out);
        }

        format!("{}\n", out)
    }

    fn to_html(&self) -> String {
        let mut out = String::new();

        if !self.chords.is_empty() {
            out.push_str("<b>");
            let mut last = 0;
            for chord in self.chords.iter() {
                out.push_str(&" ".repeat((chord.pos - last) as usize));
                out.push_str(&chord.name);
                last = chord.pos + chord.name.len() as u32;
            }
            out.push_str("</b>\n");
        }

        if !self.text.is_empty() {
            out.push_str(&self.text);
            out.push_str("\n");
        }

        out
    }
}

#[derive(Default, Debug)]
pub struct SongPart {
    name: String,
    lines: Vec<SongLine>,
}

impl SongPart {
    fn is_empty(&self) -> bool {
        self.name.is_empty() && self.lines.is_empty()
    }

    pub fn to_latex(&mut self) -> String {
        let mut out = String::new();

        lazy_static! {
            static ref RE: Regex = Regex::new(r"^verse (\d)+$").unwrap();
        };

        let lower_name = self.name.to_lowercase();

        let end = match lower_name.as_ref() {
            "chorus" => {
                out.push_str("\\beginchorus\n");
                "\\endchorus\n"
            }
            _ => {
                if RE.is_match(lower_name.as_ref()) {
                    out.push_str("\\beginverse\n");
                } else {
                    out.push_str("\\beginverse*\n");
                    out.push_str(&format!("\t\\textbf{{{}:}}\n", self.name));
                }
                "\\endverse\n"
            }
        };

        for line in self.lines.iter_mut() {
            out.push_str("\t");
            out.push_str(&line.to_latex());
        }

        out.push_str(end);
        out
    }

    fn to_html(&self) -> String {
        let mut out = String::new();

        // Add title
        out.push_str(&format!("<em>{}:</em>\n", self.name));

        for line in self.lines.iter() {
            out.push_str(&line.to_html());
        }

        out
    }
}

#[derive(Default, Debug)]
pub struct Song {
    title: String,
    metadata: HashMap<String, String>,
    parts: Vec<SongPart>,
}

impl Song {
    pub fn to_latex(&mut self) -> String {
        let mut out = String::new();

        // Begin the song
        out.push_str(&format!("\\beginsong{{{}}}", self.title));

        // Insert an optional artist
        if let Some(artist) = self.metadata.get("artist") {
            out.push_str(&format!("[by={{{}}}", artist));
        }
        out.push_str("\n\n");

        // Insert parts
        for part in self.parts.iter_mut() {
            out.push_str(&part.to_latex());
            out.push_str("\n");
        }

        // End the song
        out.push_str("\\endsong\n");

        out
    }

    pub fn to_html(&mut self) -> String {
        let mut out = String::new();

        // Surround with pre tag
        out.push_str("<pre>");

        // Add the title
        out.push_str(&format!("<h1>{}</h1>\n", self.title));

        // Add metadata
        for (k, v) in self.metadata.iter() {
            out.push_str(&format!("{}: {}\n", k, v));
        }
        out.push_str("\n\n");

        // Add parts
        for part in self.parts.iter() {
            out.push_str(&part.to_html());
            out.push_str("\n\n");
        }

        // Close pre tag
        out.push_str("</pre>");

        out
    }
}

enum SongParserState {
    START,
    DEFINITION,
    BODY,
}

impl Default for SongParserState {
    fn default() -> SongParserState {
        SongParserState::START
    }
}

#[derive(Default)]
pub struct SongParser {
    song: Song,
    state: SongParserState,
    last_chords: Vec<SongChord>,
    part: SongPart,
}

impl SongParser {
    pub fn parse(content: &str) -> Song {
        let mut parser = SongParser::default();

        for line in content.lines() {
            parser.parse_line(line);
        }
        parser.parse_line("");

        parser.song
    }

    fn parse_line(&mut self, line: &str) {
        let trimmed_line = line.trim();

        match self.state {
            SongParserState::START => {
                // Ignore any leading blank lines
                if trimmed_line.is_empty() {
                    return;
                }

                // The first line with content is the song title
                self.song.title = String::from(trimmed_line);
                self.state = SongParserState::DEFINITION;
            }
            SongParserState::DEFINITION => {
                // Ignore blank lines
                if trimmed_line.is_empty() {
                    return;
                }

                // Parse either metadata or the first song part
                if !self.parse_metadata(line) {
                    self.state = SongParserState::BODY;
                    self.parse_line(line);
                }
            }
            SongParserState::BODY => {
                // If there is a blank line, push any non empty part and reset it
                if trimmed_line.is_empty() {
                    if self.part.is_empty() {
                        return;
                    }

                    if !self.last_chords.is_empty() {
                        self.part.lines.push(SongLine {
                            text: String::new(),
                            chords: self.last_chords.drain(..).collect(),
                        });
                    }

                    self.song.parts.push(mem::take(&mut self.part));
                }

                // Otherwise parse the line to the current part
                self.parse_part(line);
            }
        }
    }

    fn parse_metadata(&mut self, line: &str) -> bool {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"^\s*(.*): (.*)\s*$").unwrap();
        };

        match RE.captures(line) {
            None => false,
            Some(groups) => {
                self.song
                    .metadata
                    .insert(String::from(&groups[1]), String::from(&groups[2]));
                true
            }
        }
    }

    fn parse_part(&mut self, line: &str) {
        // If the current part is empty, try to interpret the line as part name
        if self.part.is_empty() {
            lazy_static! {
                // A line with a part name has to end with a colon
                static ref RE: Regex = Regex::new(r"^\s*(.*):$").unwrap();
            };

            if let Some(groups) = RE.captures(line) {
                self.part.name = String::from(&groups[1]);
                return;
            }
        }

        // Try to interpret as a chord line
        if let Some(mut chords) = self.parse_chords(line) {
            // If the last line had chords, then put them on their own line
            mem::swap(&mut self.last_chords, &mut chords);
            if !chords.is_empty() {
                self.part.lines.push(SongLine {
                    text: String::new(),
                    chords,
                })
            }

            return;
        }

        // If the line is not a chord line, then save it with its chords
        self.part.lines.push(SongLine {
            text: String::from(line),
            chords: mem::take(&mut self.last_chords),
        })
    }

    fn parse_chords(&mut self, line: &str) -> Option<Vec<SongChord>> {
        let mut word = String::new();
        let mut chords = Vec::new();
        let mut pos = 0;
        let mut start = 0;

        for c in line.chars() {
            if c == ' ' {
                if !word.is_empty() {
                    if !SongParser::is_chord(&word) {
                        return None;
                    }

                    chords.push(SongChord {
                        name: String::clone(&word),
                        pos: start,
                    });
                    word.clear();
                }

                start = pos + 1;
            } else {
                word += &c.to_string();
            }

            pos += 1;
        }

        if !word.is_empty() {
            if !SongParser::is_chord(&word) {
                return None;
            }

            chords.push(SongChord {
                name: word,
                pos: start,
            });
        }

        return Some(chords);
    }

    fn is_chord(text: &str) -> bool {
        lazy_static! {
            static ref RE: Regex = Regex::new(CHORD_REGEX).unwrap();
        };

        RE.is_match(&text)
    }
}

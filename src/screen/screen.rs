use crossterm::{
    cursor,
    style::{Attributes, Color, ContentStyle, PrintStyledContent},
    terminal::{Clear, ClearType},
    QueueableCommand,
};
use std::io::Write;

pub struct Screen {}

impl Screen {
    pub fn new() -> Self {
        return Screen {};
    }
    pub fn clear(&self, write: &mut impl Write) {
        write.queue(Clear(ClearType::All));
    }

    /// move the cursor to x,y and clears the line.
    pub fn goto_clear(&self, write: &mut impl Write, x: u16, y: u16) {
        write.queue(cursor::MoveTo(x, y));
        write.queue(Clear(ClearType::UntilNewLine));
    }

    pub fn draw(&self, write: &mut impl Write, x: u16, y: u16, color: Color) {
        let cs = ContentStyle {
            foreground_color: Some(color),
            background_color: Some(Color::Black),
            attributes: Attributes::default(),
        };

        write.queue(cursor::MoveTo(x, y)).unwrap();
        write.queue(PrintStyledContent(cs.apply('█'))).unwrap();
    }
}

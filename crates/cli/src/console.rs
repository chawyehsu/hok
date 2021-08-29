use std::io::{self, Write};
use termcolor::{BufferWriter, Color, ColorChoice, ColorSpec, WriteColor};

pub fn warn<S: AsRef<str>>(text: S) -> io::Result<()> {
    let bufwtr = BufferWriter::stderr(ColorChoice::Always);
    let mut buffer = bufwtr.buffer();
    buffer.set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))?;
    write!(&mut buffer, "Warning: ")?;
    buffer.reset()?;
    writeln!(&mut buffer, "{}", text.as_ref())?;
    bufwtr.print(&buffer)
}

pub fn error<S: AsRef<str>>(text: S) -> io::Result<()> {
    let bufwtr = BufferWriter::stderr(ColorChoice::Always);
    let mut buffer = bufwtr.buffer();
    buffer.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    write!(&mut buffer, "Error: ")?;
    buffer.reset()?;
    writeln!(&mut buffer, "{}", text.as_ref())?;
    bufwtr.print(&buffer)
}

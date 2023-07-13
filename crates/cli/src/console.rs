#![allow(dead_code)]
use std::{
    fmt::Display,
    io::{self, Write},
};
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

pub fn error<T: Display>(input: &T) -> io::Result<()> {
    let bufwtr = BufferWriter::stderr(ColorChoice::Always);
    let mut buffer = bufwtr.buffer();
    buffer.set_color(ColorSpec::new().set_fg(Some(Color::Red)))?;
    write!(&mut buffer, "ERROR ")?;
    buffer.reset()?;
    writeln!(&mut buffer, "{}", input)?;
    bufwtr.print(&buffer)
}

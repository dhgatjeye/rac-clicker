use std::io::{self, Write};

pub trait BoxDrawing: Copy + Clone {
    const TOP_LEFT: &'static str;
    const TOP_RIGHT: &'static str;
    const BOTTOM_LEFT: &'static str;
    const BOTTOM_RIGHT: &'static str;
    const HORIZONTAL: &'static str;
    const VERTICAL: &'static str;
    const T_RIGHT: &'static str;
    const T_LEFT: &'static str;

    #[inline]
    fn top_border(width: usize) -> String {
        let inner_width = width.saturating_sub(2);
        format!(
            "{}{}{}",
            Self::TOP_LEFT,
            Self::HORIZONTAL.repeat(inner_width),
            Self::TOP_RIGHT
        )
    }

    #[inline]
    fn bottom_border(width: usize) -> String {
        let inner_width = width.saturating_sub(2);
        format!(
            "{}{}{}",
            Self::BOTTOM_LEFT,
            Self::HORIZONTAL.repeat(inner_width),
            Self::BOTTOM_RIGHT
        )
    }

    #[inline]
    fn divider(width: usize) -> String {
        let inner_width = width.saturating_sub(2);
        format!(
            "{}{}{}",
            Self::T_RIGHT,
            Self::HORIZONTAL.repeat(inner_width),
            Self::T_LEFT
        )
    }
}

#[derive(Debug, Copy, Clone)]
pub struct DoubleBox;

impl BoxDrawing for DoubleBox {
    const TOP_LEFT: &'static str = "╔";
    const TOP_RIGHT: &'static str = "╗";
    const BOTTOM_LEFT: &'static str = "╚";
    const BOTTOM_RIGHT: &'static str = "╝";
    const HORIZONTAL: &'static str = "═";
    const VERTICAL: &'static str = "║";
    const T_RIGHT: &'static str = "╠";
    const T_LEFT: &'static str = "╣";
}

#[derive(Debug, Copy, Clone)]
pub struct SingleBox;

impl BoxDrawing for SingleBox {
    const TOP_LEFT: &'static str = "┌";
    const TOP_RIGHT: &'static str = "┐";
    const BOTTOM_LEFT: &'static str = "└";
    const BOTTOM_RIGHT: &'static str = "┘";
    const HORIZONTAL: &'static str = "─";
    const VERTICAL: &'static str = "│";
    const T_RIGHT: &'static str = "├";
    const T_LEFT: &'static str = "┤";
}

#[derive(Debug, Copy, Clone)]
pub struct AsciiBox;

impl BoxDrawing for AsciiBox {
    const TOP_LEFT: &'static str = "+";
    const TOP_RIGHT: &'static str = "+";
    const BOTTOM_LEFT: &'static str = "+";
    const BOTTOM_RIGHT: &'static str = "+";
    const HORIZONTAL: &'static str = "-";
    const VERTICAL: &'static str = "|";
    const T_RIGHT: &'static str = "+";
    const T_LEFT: &'static str = "+";
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[inline]
fn display_width(s: &str) -> usize {
    s.chars().count()
}

fn pad_text(text: &str, width: usize, align: Align) -> String {
    let text_width = display_width(text);

    if text_width >= width {
        return text.to_string();
    }

    let padding = width - text_width;
    let mut result = String::with_capacity(width);

    match align {
        Align::Left => {
            result.push_str(text);
            result.push_str(&" ".repeat(padding));
        }
        Align::Right => {
            result.push_str(&" ".repeat(padding));
            result.push_str(text);
        }
        Align::Center => {
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;
            result.push_str(&" ".repeat(left_pad));
            result.push_str(text);
            result.push_str(&" ".repeat(right_pad));
        }
    }

    result
}

pub struct LayoutEngine<B: BoxDrawing> {
    width: usize,
    _box_style: std::marker::PhantomData<B>,
}

impl<B: BoxDrawing> LayoutEngine<B> {
    #[inline]
    pub const fn new(width: usize) -> Self {
        let width = if width < 10 { 10 } else { width };
        Self {
            width,
            _box_style: std::marker::PhantomData,
        }
    }

    pub fn render_header<W: Write>(
        &self,
        writer: &mut W,
        title: &str,
        align: Align,
    ) -> io::Result<()> {
        writeln!(writer, "{}", B::top_border(self.width))?;
        self.render_line(writer, title, align)?;
        writeln!(writer, "{}", B::bottom_border(self.width))?;
        Ok(())
    }

    pub fn render_line<W: Write>(
        &self,
        writer: &mut W,
        content: &str,
        align: Align,
    ) -> io::Result<()> {
        let inner_width = self.width.saturating_sub(2);
        let padded = pad_text(content, inner_width, align);
        writeln!(writer, "{}{}{}", B::VERTICAL, padded, B::VERTICAL)?;
        Ok(())
    }

    pub fn render_divider<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "{}", B::divider(self.width))?;
        Ok(())
    }

    pub fn render_box_top<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "{}", B::top_border(self.width))?;
        Ok(())
    }

    pub fn render_box_bottom<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "{}", B::bottom_border(self.width))?;
        Ok(())
    }

    pub fn render_plain<W: Write>(&self, writer: &mut W, content: &str) -> io::Result<()> {
        writeln!(writer, "{}", content)?;
        Ok(())
    }

    pub fn render_blank<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer)?;
        Ok(())
    }
}

pub struct MenuBuilder<B: BoxDrawing> {
    engine: LayoutEngine<B>,
    buffer: String,
}

impl<B: BoxDrawing> MenuBuilder<B> {
    pub fn new(width: usize) -> Self {
        Self {
            engine: LayoutEngine::new(width),
            buffer: String::with_capacity(2048),
        }
    }

    pub fn header(mut self, title: &str, align: Align) -> io::Result<Self> {
        let mut temp = Vec::new();
        self.engine.render_header(&mut temp, title, align)?;
        self.buffer.push_str(&String::from_utf8_lossy(&temp));
        Ok(self)
    }

    pub fn box_start(mut self) -> io::Result<Self> {
        let mut temp = Vec::new();
        self.engine.render_box_top(&mut temp)?;
        self.buffer.push_str(&String::from_utf8_lossy(&temp));
        Ok(self)
    }

    pub fn box_end(mut self) -> io::Result<Self> {
        let mut temp = Vec::new();
        self.engine.render_box_bottom(&mut temp)?;
        self.buffer.push_str(&String::from_utf8_lossy(&temp));
        Ok(self)
    }

    pub fn line(mut self, content: &str, align: Align) -> io::Result<Self> {
        let mut temp = Vec::new();
        self.engine.render_line(&mut temp, content, align)?;
        self.buffer.push_str(&String::from_utf8_lossy(&temp));
        Ok(self)
    }

    pub fn divider(mut self) -> io::Result<Self> {
        let mut temp = Vec::new();
        self.engine.render_divider(&mut temp)?;
        self.buffer.push_str(&String::from_utf8_lossy(&temp));
        Ok(self)
    }

    pub fn plain(mut self, content: &str) -> io::Result<Self> {
        let mut temp = Vec::new();
        self.engine.render_plain(&mut temp, content)?;
        self.buffer.push_str(&String::from_utf8_lossy(&temp));
        Ok(self)
    }

    pub fn blank(mut self) -> io::Result<Self> {
        let mut temp = Vec::new();
        self.engine.render_blank(&mut temp)?;
        self.buffer.push_str(&String::from_utf8_lossy(&temp));
        Ok(self)
    }

    pub fn finish<W: Write>(self, writer: &mut W) -> io::Result<()> {
        writer.write_all(self.buffer.as_bytes())?;
        writer.flush()?;
        Ok(())
    }
}

pub type DoubleBoxLayout = LayoutEngine<DoubleBox>;
pub type SingleBoxLayout = LayoutEngine<SingleBox>;
pub type AsciiBoxLayout = LayoutEngine<AsciiBox>;

pub type DoubleMenu = MenuBuilder<DoubleBox>;
pub type SingleMenu = MenuBuilder<SingleBox>;
pub type AsciiMenu = MenuBuilder<AsciiBox>;

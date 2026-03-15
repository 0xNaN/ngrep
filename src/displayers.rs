use std::io::Write;

use colored::{Color, ColoredString, Colorize};

macro_rules! try_write {
    ($handle:expr, $($arg:tt)*) => {
        write!($handle, $($arg)*)
    };
}

pub struct MatchDisplayerOptions {
    colors: Option<Vec<Color>>,
    line_number: bool,
    only_matching: bool,
    file_name: bool,
}

impl MatchDisplayerOptions {
    const RED: Color = Color::TrueColor {
        r: 255,
        g: 123,
        b: 123,
    };

    pub fn default() -> Self {
        MatchDisplayerOptions {
            colors: Some(vec![Self::RED]),
            line_number: false,
            only_matching: false,
            file_name: false,
        }
    }

    #[allow(dead_code)]
    pub fn with_colors(mut self, colors: Option<Vec<Color>>) -> Self {
        self.colors = colors;
        self
    }

    pub fn with_line_number(mut self, line_number: bool) -> Self {
        self.line_number = line_number;
        self
    }

    pub fn with_only_matching(mut self, only_matching: bool) -> Self {
        self.only_matching = only_matching;
        self
    }

    pub fn with_file_name(mut self, file_name: bool) -> Self {
        self.file_name = file_name;
        self
    }
}

pub struct MatchDisplayer {
    opts: MatchDisplayerOptions,
}

impl MatchDisplayer {
    pub fn new(options: MatchDisplayerOptions) -> Self {
        MatchDisplayer { opts: options }
    }

    pub fn display_line<W: Write>(
        &self,
        writer: &mut W,
        name: &str,
        inx: usize,
        line: &str,
        matches: &[(usize, usize)],
    ) -> std::io::Result<()> {
        use std::fmt::Write as _;

        let mut line_prefix = String::new();
        if self.opts.file_name {
            let _ = write!(line_prefix, "{name}:");
        }
        if self.opts.line_number {
            let inx = inx + 1;
            let _ = write!(line_prefix, "{inx}:");
        }

        let mut cursor = 0;
        for (i, match_span) in matches.iter().enumerate() {
            let (start, end) = *match_span;
            let mut match_ = ColoredString::from(&line[start..end]);

            if let Some(colors) = &self.opts.colors {
                match_.fgcolor = Some(colors[i % colors.len()]);
                match_ = match_.bold();
            }

            if self.opts.only_matching {
                try_write!(writer, "{}{}\n", line_prefix, match_)?;
            } else {
                if i == 0 {
                    try_write!(writer, "{}", line_prefix)?;
                }
                try_write!(writer, "{}{}", &line[cursor..start], match_)?;
            }

            cursor = end;
        }
        if !self.opts.only_matching {
            try_write!(writer, "{}\n", &line[cursor..])?;
        }

        writer.flush().expect("Error flushing output");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_displayer(opts: MatchDisplayerOptions) -> (Vec<u8>, MatchDisplayer) {
        let out = Vec::new();
        let displayer = MatchDisplayer::new(opts);

        (out, displayer)
    }

    #[test]
    fn test_display_line_number_starts_from_one() {
        let (mut out, displayer) =
            setup_displayer(MatchDisplayerOptions::default().with_line_number(true));

        displayer
            .display_line(&mut out, "-", 0, "hello world", &[(0, 5)])
            .unwrap();
        assert!(String::from_utf8(out).unwrap().starts_with("1:"));
    }

    #[test]
    fn test_only_match_show_just_span() {
        let (mut out, displayer) = setup_displayer(
            MatchDisplayerOptions::default()
                .with_colors(None)
                .with_only_matching(true),
        );

        displayer
            .display_line(&mut out, "-", 0, "hello world", &[(0, 5)])
            .unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "hello\n");
    }
}

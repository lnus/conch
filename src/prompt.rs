use nu_ansi_term::Style;

pub struct Segment {
    pub text: String,
    pub style: Style,
}

impl Segment {
    pub fn build(&self) -> String {
        self.style.paint(&self.text).to_string()
    }
}

pub type Part = Vec<Segment>;

pub trait PartExt {
    fn build(&self) -> String;
}

impl PartExt for Part {
    fn build(&self) -> String {
        self.iter().map(Segment::build).collect()
    }
}

pub struct Prompt {
    segments: Part,
    separator: Option<String>,
    prefix: Option<String>,
    suffix: Option<String>,
    style: Style,
}

impl Prompt {
    pub fn new() -> Self {
        Self {
            segments: Part::default(),
            separator: None,
            prefix: None,
            suffix: None,
            style: Style::default(),
        }
    }

    pub fn with_separator(mut self, separator: impl Into<String>) -> Self {
        self.separator = Some(separator.into());
        self
    }

    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub const fn with_style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }

    pub fn push(&mut self, text: impl Into<String>, style: Style) {
        self.segments.push(Segment {
            text: text.into(),
            style,
        });
    }

    // TODO do something with these or remove
    // I thought I would need them
    pub fn push_segment(&mut self, segment: Segment) {
        self.segments.push(segment);
    }

    pub fn push_segments(&mut self, segments: Vec<Segment>) {
        self.segments.extend(segments);
    }

    pub fn push_if(&mut self, text: Option<String>, style: Style) {
        if let Some(text) = text {
            self.push(text, style);
        }
    }

    pub fn push_if_segment(&mut self, segment: Option<Segment>) {
        if let Some(segment) = segment {
            self.push_segment(segment);
        }
    }

    pub fn push_if_segments(&mut self, segments: Option<Vec<Segment>>) {
        if let Some(segments) = segments {
            self.push_segments(segments);
        }
    }

    pub fn print(&self) {
        if let Some(prefix) = &self.prefix {
            print!("{}", self.style.paint(prefix));
        }

        let separator = self.separator.as_deref().unwrap_or(" ");

        self.segments.iter().enumerate().for_each(|(i, seg)| {
            if i > 0 {
                print!("{}", self.style.paint(separator));
            }
            print!("{}", seg.style.paint(&seg.text));
        });

        if let Some(suffix) = &self.suffix {
            print!("{}", self.style.paint(suffix));
        }
    }
}

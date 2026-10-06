pub(crate) fn utf16_len(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

pub(crate) struct Utf16ByteCursor<'a> {
    remaining: std::str::Chars<'a>,
    utf16_position: usize,
    byte_position: usize,
}

impl<'a> Utf16ByteCursor<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        Self {
            remaining: text.chars(),
            utf16_position: 0,
            byte_position: 0,
        }
    }

    pub(crate) fn byte_offset(&mut self, utf16_offset: usize) -> usize {
        while self.utf16_position < utf16_offset {
            let Some(character) = self.remaining.next() else {
                break;
            };
            self.utf16_position += character.len_utf16();
            self.byte_position += character.len_utf8();
        }
        self.byte_position
    }
}

#[cfg(test)]
#[path = "utf16-offsets-tests.rs"]
mod tests;

#[derive(Debug)]
pub struct Heading {
    pub depth: usize,
    pub content: String,
}

impl Heading {
    pub fn try_parse(line: &str) -> Option<Self> {
        let mut depth = 0;

        for ch in line.chars() {
            match ch {
                '#' => depth += 1,
                _ => break,
            }
        }

        if depth == 0 {
            return None;
        }

        Some(Self {
            depth,
            content: line[depth..].trim().to_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_parse_headings() {
        // When
        let value = Heading::try_parse("### The quick ## brown fox #");

        // Then
        let heading = value.unwrap();
        assert_eq!(heading.depth, 3);
        assert_eq!(heading.content, "The quick ## brown fox #");
    }

    #[test]
    fn should_parse_non_headings() {
        // When
        let value = Heading::try_parse("T#he quick brown fox ## jumped over the lazy dog");

        // Then
        assert!(value.is_none());
    }
}

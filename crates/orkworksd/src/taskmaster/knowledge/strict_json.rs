use serde::de::DeserializeOwned;
use std::collections::HashSet;

const MAX_DEPTH: usize = 32;

pub(super) fn parse<T: DeserializeOwned>(bytes: &[u8], max_bytes: usize) -> Result<T, String> {
    if bytes.len() > max_bytes {
        return Err(format!("JSON exceeds {max_bytes} byte limit"));
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err("JSON must not start with a UTF-8 BOM".into());
    }
    let source = std::str::from_utf8(bytes).map_err(|_| "JSON is not valid UTF-8")?;
    let mut scanner = Scanner { source, index: 0 };
    scanner.whitespace();
    scanner.value(0)?;
    scanner.whitespace();
    if scanner.index != source.len() {
        return Err("strict JSON has trailing data".into());
    }
    serde_json::from_slice(bytes).map_err(|error| format!("invalid typed JSON: {error}"))
}

struct Scanner<'a> {
    source: &'a str,
    index: usize,
}

impl Scanner<'_> {
    fn value(&mut self, depth: usize) -> Result<(), String> {
        self.whitespace();
        match self.byte() {
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => self.string().map(|_| ()),
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'n') => self.literal(b"null"),
            Some(b'0'..=b'9') => self.number(),
            _ => Err(self.error("invalid value")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<(), String> {
        self.check_depth(depth)?;
        self.index += 1;
        self.whitespace();
        if self.take(b'}') {
            return Ok(());
        }
        let mut keys = HashSet::new();
        loop {
            self.whitespace();
            if self.byte() != Some(b'"') {
                return Err(self.error("object key must be a string"));
            }
            let key = self.string()?;
            if !keys.insert(key.clone()) {
                return Err(self.error(&format!("duplicate object key {key:?}")));
            }
            self.whitespace();
            if !self.take(b':') {
                return Err(self.error("expected colon after object key"));
            }
            self.value(depth)?;
            self.whitespace();
            if self.take(b'}') {
                return Ok(());
            }
            if !self.take(b',') {
                return Err(self.error("expected comma between object fields"));
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<(), String> {
        self.check_depth(depth)?;
        self.index += 1;
        self.whitespace();
        if self.take(b']') {
            return Ok(());
        }
        loop {
            self.value(depth)?;
            self.whitespace();
            if self.take(b']') {
                return Ok(());
            }
            if !self.take(b',') {
                return Err(self.error("expected comma between array items"));
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        let start = self.index;
        self.index += 1;
        while let Some(byte) = self.byte() {
            if byte == b'"' {
                self.index += 1;
                return serde_json::from_str(&self.source[start..self.index])
                    .map_err(|_| self.error("invalid JSON string"));
            }
            if byte < 0x20 {
                return Err(self.error("unescaped control character in string"));
            }
            if byte != b'\\' {
                self.index += 1;
                continue;
            }
            self.index += 1;
            match self.byte() {
                Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => self.index += 1,
                Some(b'u') => {
                    let high = self.code_unit()?;
                    if (0xd800..=0xdbff).contains(&high) {
                        if self.source.as_bytes().get(self.index..self.index + 2) != Some(b"\\u") {
                            return Err(self.error("unpaired high surrogate"));
                        }
                        self.index += 1;
                        let low = self.code_unit()?;
                        if !(0xdc00..=0xdfff).contains(&low) {
                            return Err(self.error("unpaired high surrogate"));
                        }
                    } else if (0xdc00..=0xdfff).contains(&high) {
                        return Err(self.error("unpaired low surrogate"));
                    }
                }
                _ => return Err(self.error("invalid string escape")),
            }
        }
        Err(self.error("unterminated string"))
    }

    fn code_unit(&mut self) -> Result<u16, String> {
        self.index += 1;
        let end = self.index + 4;
        let hex = self
            .source
            .as_bytes()
            .get(self.index..end)
            .ok_or_else(|| self.error("invalid Unicode escape"))?;
        if !hex.iter().all(u8::is_ascii_hexdigit) {
            return Err(self.error("invalid Unicode escape"));
        }
        let text = &self.source[self.index..end];
        let value =
            u16::from_str_radix(text, 16).map_err(|_| self.error("invalid Unicode escape"))?;
        self.index = end;
        Ok(value)
    }

    fn number(&mut self) -> Result<(), String> {
        let start = self.index;
        let bytes = self.source.as_bytes();
        if bytes[self.index] == b'0' {
            self.index += 1;
        } else {
            while self.byte().is_some_and(|c| c.is_ascii_digit()) {
                self.index += 1;
            }
        }
        if self
            .byte()
            .is_some_and(|c| !matches!(c, b'\t' | b'\n' | b'\r' | b' ' | b',' | b']' | b'}'))
        {
            return Err(self.error("numbers must use canonical nonnegative integer syntax"));
        }
        let token = &self.source[start..self.index];
        if token.len() > 1 && token.starts_with('0') {
            return Err(self.error("numbers must use canonical nonnegative integer syntax"));
        }
        let value = token
            .parse::<u64>()
            .map_err(|_| self.error("integer exceeds safe range"))?;
        if value > 9_007_199_254_740_991 {
            return Err(self.error("integer exceeds safe range"));
        }
        Ok(())
    }

    fn literal(&mut self, value: &[u8]) -> Result<(), String> {
        if self
            .source
            .as_bytes()
            .get(self.index..self.index + value.len())
            == Some(value)
        {
            self.index += value.len();
            Ok(())
        } else {
            Err(self.error("invalid literal"))
        }
    }

    fn whitespace(&mut self) {
        while self
            .byte()
            .is_some_and(|c| matches!(c, b'\t' | b'\n' | b'\r' | b' '))
        {
            self.index += 1;
        }
    }

    fn check_depth(&self, depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            Err(self.error("JSON nesting exceeds 32 levels"))
        } else {
            Ok(())
        }
    }

    fn take(&mut self, byte: u8) -> bool {
        if self.byte() == Some(byte) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn byte(&self) -> Option<u8> {
        self.source.as_bytes().get(self.index).copied()
    }
    fn error(&self, message: &str) -> String {
        format!(
            "invalid strict JSON at byte offset {}: {message}",
            self.index
        )
    }
}

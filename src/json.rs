//! Small bounded JSON codec for the dependency-free, read-only POC.
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Self::Object(map) => map.get(key),
            _ => None,
        }
    }
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn encode(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Bool(b) => b.to_string(),
            Self::Number(n) => n.clone(),
            Self::String(s) => quote(s),
            Self::Array(a) => format!(
                "[{}]",
                a.iter().map(Self::encode).collect::<Vec<_>>().join(",")
            ),
            Self::Object(o) => format!(
                "{{{}}}",
                o.iter()
                    .map(|(k, v)| format!("{}:{}", quote(k), v.encode()))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

pub fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

pub fn parse(input: &str) -> Result<Value, &'static str> {
    let mut p = Parser {
        bytes: input.as_bytes(),
        pos: 0,
    };
    let value = p.value(0)?;
    p.ws();
    if p.pos != p.bytes.len() {
        return Err("Trailing JSON");
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}
impl Parser<'_> {
    fn ws(&mut self) {
        while self
            .bytes
            .get(self.pos)
            .is_some_and(|b| matches!(b, b' ' | b'\n' | b'\r' | b'\t'))
        {
            self.pos += 1;
        }
    }
    fn take(&mut self, byte: u8) -> bool {
        if self.bytes.get(self.pos) == Some(&byte) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn value(&mut self, depth: usize) -> Result<Value, &'static str> {
        if depth > 24 {
            return Err("JSON nesting limit");
        }
        self.ws();
        match self.bytes.get(self.pos).copied() {
            Some(b'"') => Ok(Value::String(self.string()?)),
            Some(b'{') => {
                self.pos += 1;
                self.ws();
                let mut map = BTreeMap::new();
                if self.take(b'}') {
                    return Ok(Value::Object(map));
                }
                loop {
                    self.ws();
                    let key = self.string()?;
                    self.ws();
                    if !self.take(b':') {
                        return Err("Expected colon");
                    }
                    let value = self.value(depth + 1)?;
                    if map.insert(key, value).is_some() {
                        return Err("Duplicate key");
                    }
                    self.ws();
                    if self.take(b'}') {
                        break;
                    }
                    if !self.take(b',') {
                        return Err("Expected comma");
                    }
                }
                Ok(Value::Object(map))
            }
            Some(b'[') => {
                self.pos += 1;
                self.ws();
                let mut list = Vec::new();
                if self.take(b']') {
                    return Ok(Value::Array(list));
                }
                loop {
                    list.push(self.value(depth + 1)?);
                    self.ws();
                    if self.take(b']') {
                        break;
                    }
                    if !self.take(b',') {
                        return Err("Expected comma");
                    }
                }
                Ok(Value::Array(list))
            }
            Some(b'n') => self.literal(b"null", Value::Null),
            Some(b't') => self.literal(b"true", Value::Bool(true)),
            Some(b'f') => self.literal(b"false", Value::Bool(false)),
            Some(b'-' | b'0'..=b'9') => {
                let start = self.pos;
                self.take(b'-');
                if !self.take(b'0') {
                    self.digits()?;
                }
                if self.take(b'.') {
                    self.digits()?;
                }
                if self.take(b'e') || self.take(b'E') {
                    if !self.take(b'+') {
                        self.take(b'-');
                    }
                    self.digits()?;
                }
                Ok(Value::Number(
                    String::from_utf8(self.bytes[start..self.pos].to_vec())
                        .map_err(|_| "Invalid number")?,
                ))
            }
            _ => Err("Invalid JSON"),
        }
    }
    fn digits(&mut self) -> Result<(), &'static str> {
        let start = self.pos;
        while self.bytes.get(self.pos).is_some_and(u8::is_ascii_digit) {
            self.pos += 1;
        }
        if start == self.pos {
            Err("Expected digit")
        } else {
            Ok(())
        }
    }
    fn literal(&mut self, literal: &[u8], value: Value) -> Result<Value, &'static str> {
        if self.bytes.get(self.pos..self.pos + literal.len()) != Some(literal) {
            return Err("Invalid literal");
        }
        self.pos += literal.len();
        Ok(value)
    }
    fn hex(&mut self) -> Result<u16, &'static str> {
        let end = self.pos + 4;
        let s = std::str::from_utf8(self.bytes.get(self.pos..end).ok_or("Invalid unicode")?)
            .map_err(|_| "Invalid unicode")?;
        let n = u16::from_str_radix(s, 16).map_err(|_| "Invalid unicode")?;
        self.pos = end;
        Ok(n)
    }
    fn string(&mut self) -> Result<String, &'static str> {
        if !self.take(b'"') {
            return Err("Expected string");
        }
        let mut out = Vec::new();
        loop {
            let b = *self.bytes.get(self.pos).ok_or("Unterminated string")?;
            self.pos += 1;
            match b {
                b'"' => return String::from_utf8(out).map_err(|_| "Invalid UTF-8"),
                b'\\' => {
                    let b = *self.bytes.get(self.pos).ok_or("Invalid escape")?;
                    self.pos += 1;
                    match b {
                        b'"' | b'\\' | b'/' => out.push(b),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let first = self.hex()?;
                            let code = if (0xd800..=0xdbff).contains(&first) {
                                if !self.take(b'\\') || !self.take(b'u') {
                                    return Err("Missing surrogate");
                                }
                                let second = self.hex()?;
                                if !(0xdc00..=0xdfff).contains(&second) {
                                    return Err("Invalid surrogate");
                                }
                                0x10000 + ((first as u32 - 0xd800) << 10) + (second as u32 - 0xdc00)
                            } else {
                                first as u32
                            };
                            let c = char::from_u32(code).ok_or("Invalid unicode")?;
                            let mut buf = [0; 4];
                            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return Err("Invalid escape"),
                    }
                }
                0..=31 => return Err("Control character"),
                _ => out.push(b),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_unicode() {
        let v = parse(r#"{"text":"Élise \ud83d\ude00","n":-1.25e+2,"a":[true,null]}"#).unwrap();
        assert_eq!(parse(&v.encode()).unwrap(), v);
    }
    #[test]
    fn rejects_ambiguous_and_malformed() {
        for s in [
            r#"{"id":1,"id":2}"#,
            "01",
            "1.",
            "[1,]",
            r#""\ud800""#,
            "true false",
        ] {
            assert!(parse(s).is_err(), "{s}");
        }
    }
    #[test]
    fn bounded_nesting() {
        assert!(parse(&format!("{}0{}", "[".repeat(30), "]".repeat(30))).is_err());
    }
}

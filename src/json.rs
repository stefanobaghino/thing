//! Hand-rolled JSON for the json_str/json_parse builtins. ting's data
//! model maps cleanly: map<->object, list<->array, string, int/float
//! <->number, bool, nil<->null. Functions cannot be encoded.

use crate::value::Value;
use std::collections::BTreeMap;

pub fn encode(v: &Value) -> Result<String, String> {
    let mut out = String::new();
    encode_into(v, &mut out)?;
    Ok(out)
}

fn encode_into(v: &Value, out: &mut String) -> Result<(), String> {
    match v {
        Value::Nil => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Int(n) => out.push_str(&n.to_string()),
        Value::Float(x) => {
            if !x.is_finite() {
                return Err("json_str cannot encode a non-finite float".to_string());
            }
            // Keep the float-ness visible so the value round-trips as
            // a float (matches how ting prints floats).
            if x.fract() == 0.0 {
                out.push_str(&format!("{x:.1}"));
            } else {
                out.push_str(&x.to_string());
            }
        }
        Value::Str(s) => encode_string(s, out),
        Value::List(items) => {
            out.push('[');
            for (i, it) in items.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode_into(it, out)?;
            }
            out.push(']');
        }
        Value::Map(entries) => {
            out.push('{');
            for (i, (k, v)) in entries.borrow().iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode_string(k, out);
                out.push(':');
                encode_into(v, out)?;
            }
            out.push('}');
        }
        Value::Fn(_) | Value::Builtin(_) => {
            return Err("json_str cannot encode a function".to_string());
        }
    }
    Ok(())
}

fn encode_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

pub fn decode(s: &str) -> Result<Value, String> {
    let bytes = s.as_bytes();
    let mut p = Parser { bytes, pos: 0 };
    p.skip_ws();
    let v = p.value()?;
    p.skip_ws();
    if p.pos != bytes.len() {
        return Err(p.err("trailing characters after JSON value"));
    }
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, msg: &str) -> String {
        format!("json_parse: {msg} at offset {}", self.pos)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, lit: &str) -> Result<(), String> {
        if self.bytes[self.pos..].starts_with(lit.as_bytes()) {
            self.pos += lit.len();
            Ok(())
        } else {
            Err(self.err(&format!("expected '{lit}'")))
        }
    }

    fn value(&mut self) -> Result<Value, String> {
        match self.peek() {
            Some(b'n') => self.expect("null").map(|_| Value::Nil),
            Some(b't') => self.expect("true").map(|_| Value::Bool(true)),
            Some(b'f') => self.expect("false").map(|_| Value::Bool(false)),
            Some(b'"') => self.string().map(Value::Str),
            Some(b'[') => self.array(),
            Some(b'{') => self.object(),
            Some(c) if c == b'-' || c.is_ascii_digit() => self.number(),
            Some(_) => Err(self.err("unexpected character")),
            None => Err(self.err("unexpected end of input")),
        }
    }

    fn array(&mut self) -> Result<Value, String> {
        self.pos += 1; // [
        let mut items = Vec::new();
        self.skip_ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::list(items));
        }
        loop {
            self.skip_ws();
            items.push(self.value()?);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::list(items));
                }
                _ => return Err(self.err("expected ',' or ']'")),
            }
        }
    }

    fn object(&mut self) -> Result<Value, String> {
        self.pos += 1; // {
        let mut entries = BTreeMap::new();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::map(entries));
        }
        loop {
            self.skip_ws();
            if self.peek() != Some(b'"') {
                return Err(self.err("expected a string key"));
            }
            let key = self.string()?;
            self.skip_ws();
            if self.peek() != Some(b':') {
                return Err(self.err("expected ':'"));
            }
            self.pos += 1;
            self.skip_ws();
            let val = self.value()?;
            entries.insert(key, val);
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::map(entries));
                }
                _ => return Err(self.err("expected ',' or '}'")),
            }
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.pos += 1; // "
        let mut out = String::new();
        loop {
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c == b'"' || c == b'\\' {
                    break;
                }
                if c < 0x20 {
                    return Err(self.err("unescaped control character in string"));
                }
                self.pos += 1;
            }
            out.push_str(
                std::str::from_utf8(&self.bytes[start..self.pos])
                    .map_err(|_| self.err("invalid UTF-8"))?,
            );
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    self.pos += 1;
                    let esc = self.peek().ok_or_else(|| self.err("unfinished escape"))?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{0008}'),
                        b'f' => out.push('\u{000C}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) {
                                // Surrogate pair.
                                self.expect("\\u")
                                    .map_err(|_| self.err("missing low surrogate"))?;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err(self.err("invalid low surrogate"));
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            out.push(
                                char::from_u32(code)
                                    .ok_or_else(|| self.err("invalid unicode escape"))?,
                            );
                        }
                        _ => return Err(self.err("unknown escape")),
                    }
                }
                _ => return Err(self.err("unterminated string")),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32, String> {
        let end = self.pos + 4;
        let hex = self
            .bytes
            .get(self.pos..end)
            .ok_or_else(|| self.err("unfinished unicode escape"))?;
        let s = std::str::from_utf8(hex).map_err(|_| self.err("invalid unicode escape"))?;
        let v = u32::from_str_radix(s, 16).map_err(|_| self.err("invalid unicode escape"))?;
        self.pos = end;
        Ok(v)
    }

    fn number(&mut self) -> Result<Value, String> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        let mut is_float = false;
        while let Some(c) = self.peek() {
            match c {
                b'0'..=b'9' => self.pos += 1,
                b'.' | b'e' | b'E' | b'+' | b'-' => {
                    is_float = true;
                    self.pos += 1;
                }
                _ => break,
            }
        }
        let text = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap();
        if is_float {
            text.parse::<f64>()
                .map(Value::Float)
                .map_err(|_| self.err("invalid number"))
        } else {
            // Integer syntax; fall back to float if it exceeds i64.
            text.parse::<i64>().map(Value::Int).or_else(|_| {
                text.parse::<f64>()
                    .map(Value::Float)
                    .map_err(|_| self.err("invalid number"))
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(json: &str) -> String {
        encode(&decode(json).unwrap()).unwrap()
    }

    #[test]
    fn scalars() {
        assert_eq!(decode("null").unwrap(), Value::Nil);
        assert_eq!(decode("true").unwrap(), Value::Bool(true));
        assert_eq!(decode(" -42 ").unwrap(), Value::Int(-42));
        assert_eq!(decode("2.5").unwrap(), Value::Float(2.5));
        assert_eq!(decode("1e3").unwrap(), Value::Float(1000.0));
        assert_eq!(
            decode("\"h\\u00e9llo\\n\"").unwrap(),
            Value::Str("héllo\n".into())
        );
    }

    #[test]
    fn structures_roundtrip() {
        assert_eq!(
            roundtrip("[1,2.5,\"x\",null,true]"),
            "[1,2.5,\"x\",null,true]"
        );
        assert_eq!(
            roundtrip("{\"b\":[1,{\"a\":null}],\"a\":\"z\"}"),
            "{\"a\":\"z\",\"b\":[1,{\"a\":null}]}"
        );
        // Integral floats keep their float-ness.
        assert_eq!(roundtrip("[1.0]"), "[1.0]");
    }

    #[test]
    fn surrogate_pairs() {
        assert_eq!(
            decode("\"\\ud83d\\ude00\"").unwrap(),
            Value::Str("😀".into())
        );
    }

    #[test]
    fn errors() {
        assert!(decode("[1,]").is_err());
        assert!(decode("{\"a\":}").is_err());
        assert!(decode("nul").is_err());
        assert!(decode("1 2").unwrap_err().contains("trailing"));
        assert!(decode("\"abc").unwrap_err().contains("unterminated"));
        assert!(encode(&Value::Float(f64::INFINITY)).is_err());
        assert!(encode(&Value::Builtin(crate::value::Builtin::Print)).is_err());
    }

    #[test]
    fn huge_integers_become_floats() {
        assert_eq!(decode("99999999999999999999").unwrap(), Value::Float(1e20));
    }
}

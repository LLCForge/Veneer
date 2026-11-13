//! Short shareable profile ids: constraints + seed (+ batch position) in a compact, checksummed,
//! URL-safe string. Decoding never trusts the id: it is length-limited, checksummed and the
//! constraints go through the same validation as any other input.

use serde_json::{Map, Number, Value};

use crate::model::Constraints;
use crate::Error;

const FORMAT: u8 = 1;
/// Longest id accepted for decoding (an id with every string field at its limit stays below this).
pub(crate) const MAX_ID: usize = 2048;

/// Field order is part of the id format: append new fields, never reorder.
const FIELDS: [&str; 18] = [
    "os",
    "browser",
    "kind",
    "device",
    "engine",
    "gpuVendor",
    "minVersion",
    "maxVersion",
    "locale",
    "country",
    "timezone",
    "seed",
    "minWidth",
    "maxWidth",
    "minHeight",
    "maxHeight",
    "minDpr",
    "maxDpr",
];

const T_STR: u8 = 0;
const T_UINT: u8 = 1;
const T_F64: u8 = 2;
/// Lowercase hex string stored as bytes (random seeds are 16 hex digits).
const T_HEX: u8 = 3;

/// Where a profile sits in a batch: `seed` is then the batch's base seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BatchRef {
    pub index: u32,
    pub unique: bool,
}

pub(crate) fn encode(c: &Constraints, batch: Option<BatchRef>) -> Result<String, Error> {
    let value = serde_json::to_value(c)?;
    let object = value
        .as_object()
        .ok_or_else(|| Error::InvalidConstraint("constraints must be an object".into()))?;
    let mut out = vec![FORMAT];
    match batch {
        None => out.push(0),
        Some(b) => {
            out.push(1 | if b.unique { 2 } else { 0 });
            varint(&mut out, u64::from(b.index));
        }
    }
    for (index, name) in FIELDS.iter().enumerate() {
        let Some(v) = object.get(*name) else { continue };
        let index = index as u8;
        match v {
            Value::String(s) if is_hex(s) => {
                out.push(index | T_HEX << 5);
                varint(&mut out, s.len() as u64 / 2);
                for pair in s.as_bytes().chunks(2) {
                    out.push(hex_digit(pair[0]) << 4 | hex_digit(pair[1]));
                }
            }
            Value::String(s) => {
                out.push(index | T_STR << 5);
                varint(&mut out, s.len() as u64);
                out.extend_from_slice(s.as_bytes());
            }
            Value::Number(n) if n.is_u64() => {
                out.push(index | T_UINT << 5);
                varint(&mut out, n.as_u64().unwrap_or_default());
            }
            Value::Number(n) => {
                out.push(index | T_F64 << 5);
                out.extend_from_slice(&n.as_f64().unwrap_or_default().to_le_bytes());
            }
            Value::Null => {}
            _ => {
                return Err(Error::InvalidConstraint(format!(
                    "`{name}` cannot be encoded in a profile id"
                )))
            }
        }
    }
    if let Some(unknown) = object.keys().find(|k| !FIELDS.contains(&k.as_str())) {
        return Err(Error::InvalidConstraint(format!(
            "`{unknown}` cannot be encoded in a profile id"
        )));
    }
    let sum = checksum(&out);
    out.extend_from_slice(&sum);
    Ok(base64url(&out))
}

pub(crate) fn decode(id: &str) -> Result<(Constraints, Option<BatchRef>), Error> {
    let bad = |why: &str| Error::InvalidConstraint(format!("invalid profile id: {why}"));
    if id.is_empty() || id.len() > MAX_ID {
        return Err(bad(&format!("must be 1-{MAX_ID} characters")));
    }
    let bytes = unbase64url(id).ok_or_else(|| bad("not base64url"))?;
    if bytes.len() < 4 {
        return Err(bad("too short"));
    }
    let (body, sum) = bytes.split_at(bytes.len() - 2);
    if checksum(body) != sum {
        return Err(bad("checksum mismatch (mistyped or truncated)"));
    }
    let mut r = Reader { bytes: body, at: 0 };
    if r.byte()? != FORMAT {
        return Err(bad("unsupported format version"));
    }
    let flags = r.byte()?;
    if flags & !3 != 0 || flags == 2 {
        return Err(bad("unknown flags"));
    }
    let batch = if flags & 1 == 1 {
        let index = u32::try_from(r.varint()?).map_err(|_| bad("batch index too large"))?;
        Some(BatchRef {
            index,
            unique: flags & 2 == 2,
        })
    } else {
        None
    };
    let mut object = Map::new();
    while r.at < body.len() {
        let tag = r.byte()?;
        let name = *FIELDS
            .get(usize::from(tag & 31))
            .ok_or_else(|| bad("unknown field"))?;
        if object.contains_key(name) {
            return Err(bad("repeated field"));
        }
        let value = match tag >> 5 {
            T_STR => {
                let len = usize::try_from(r.varint()?).map_err(|_| bad("string too long"))?;
                let raw = r.take(len)?;
                Value::String(
                    String::from_utf8(raw.to_vec()).map_err(|_| bad("string is not UTF-8"))?,
                )
            }
            T_HEX => {
                let len = usize::try_from(r.varint()?).map_err(|_| bad("string too long"))?;
                let raw = r.take(len)?;
                Value::String(raw.iter().map(|b| format!("{b:02x}")).collect())
            }
            T_UINT => Value::Number(Number::from(r.varint()?)),
            T_F64 => {
                let raw: [u8; 8] = r.take(8)?.try_into().map_err(|_| bad("truncated"))?;
                Value::Number(
                    Number::from_f64(f64::from_le_bytes(raw)).ok_or_else(|| bad("not a number"))?,
                )
            }
            _ => return Err(bad("unknown field type")),
        };
        object.insert(name.to_owned(), value);
    }
    let constraints: Constraints =
        serde_json::from_value(Value::Object(object)).map_err(|e| bad(&e.to_string()))?;
    Ok((constraints, batch))
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Reader<'_> {
    fn take(&mut self, len: usize) -> Result<&[u8], Error> {
        let end = self.at.checked_add(len).filter(|e| *e <= self.bytes.len());
        let end =
            end.ok_or_else(|| Error::InvalidConstraint("invalid profile id: truncated".into()))?;
        let out = &self.bytes[self.at..end];
        self.at = end;
        Ok(out)
    }

    fn byte(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }

    fn varint(&mut self) -> Result<u64, Error> {
        let mut value = 0u64;
        for shift in (0..64).step_by(7) {
            let b = self.byte()?;
            value |= u64::from(b & 0x7f) << shift;
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(Error::InvalidConstraint(
            "invalid profile id: number too long".into(),
        ))
    }
}

fn is_hex(s: &str) -> bool {
    !s.is_empty() && s.len() % 2 == 0 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn hex_digit(b: u8) -> u8 {
    match b {
        b'0'..=b'9' => b - b'0',
        _ => b - b'a' + 10,
    }
}

fn varint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push((v as u8 & 0x7f) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

fn checksum(bytes: &[u8]) -> [u8; 2] {
    let h = super::fnv1a(bytes);
    [(h ^ (h >> 16)) as u8, ((h >> 8) ^ (h >> 24)) as u8]
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn base64url(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |acc, (i, b)| acc | u32::from(*b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(char::from(ALPHABET[(n >> (18 - 6 * i) & 63) as usize]));
        }
    }
    out
}

fn unbase64url(text: &str) -> Option<Vec<u8>> {
    if text.len() % 4 == 1 {
        return None;
    }
    let mut out = Vec::with_capacity(text.len() * 3 / 4);
    for chunk in text.as_bytes().chunks(4) {
        let mut n = 0u32;
        for (i, c) in chunk.iter().enumerate() {
            let v = ALPHABET.iter().position(|a| a == c)? as u32;
            n |= v << (18 - 6 * i);
        }
        for i in 0..chunk.len() - 1 {
            out.push((n >> (16 - 8 * i)) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Engine, Kind};

    #[test]
    fn round_trips_every_field() {
        let c = Constraints {
            os: Some("android".into()),
            browser: Some("chrome".into()),
            kind: Some(Kind::Phone),
            device: Some("pixel-8".into()),
            engine: Some(Engine::Chromium),
            gpu_vendor: Some("adreno".into()),
            min_version: Some(120),
            max_version: Some(140),
            locale: Some("tr,en".into()),
            country: Some("TR".into()),
            timezone: Some("Europe/Istanbul".into()),
            seed: Some("şeed-ü-42".into()),
            min_width: Some(360),
            max_width: Some(430),
            min_height: Some(0),
            max_height: Some(1000),
            min_dpr: Some(2.625),
            max_dpr: Some(3.0),
        };
        for batch in [
            None,
            Some(BatchRef {
                index: 0,
                unique: false,
            }),
            Some(BatchRef {
                index: 999,
                unique: true,
            }),
        ] {
            let id = encode(&c, batch).unwrap();
            assert!(id.bytes().all(|b| ALPHABET.contains(&b)), "{id}");
            assert_eq!(decode(&id).unwrap(), (c.clone(), batch));
        }
    }

    #[test]
    fn hex_and_plain_strings_round_trip() {
        for seed in ["00ff10ab", "0", "abc", "ABCD", "0123456789abcdef", "g0"] {
            let c = Constraints {
                seed: Some(seed.into()),
                ..Default::default()
            };
            assert_eq!(decode(&encode(&c, None).unwrap()).unwrap().0, c);
        }
    }

    #[test]
    fn ids_are_short() {
        let c = Constraints {
            os: Some("windows".into()),
            seed: Some("0123456789abcdef".into()),
            ..Default::default()
        };
        let id = encode(&c, None).unwrap();
        assert!(id.len() <= 32, "{id}");
    }

    #[test]
    fn corrupted_ids_are_rejected() {
        let id = encode(
            &Constraints {
                seed: Some("x".into()),
                ..Default::default()
            },
            None,
        )
        .unwrap();
        let mut flipped = id.clone().into_bytes();
        flipped[3] = if flipped[3] == b'A' { b'B' } else { b'A' };
        let flipped = String::from_utf8(flipped).unwrap();
        for bad in [
            "",
            "!!!!",
            "A",
            &flipped,
            &id[..id.len() - 1],
            &"A".repeat(MAX_ID + 1),
        ] {
            assert!(decode(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn base64url_round_trip() {
        for len in 0..20u8 {
            let bytes: Vec<u8> = (0..len).map(|i| i.wrapping_mul(37) ^ 0xa5).collect();
            assert_eq!(unbase64url(&base64url(&bytes)).unwrap(), bytes);
        }
    }
}

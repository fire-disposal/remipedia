use anyhow::Result;
use chrono::Local;
use rmp_serde::from_read_ref;
use serde::Deserialize;
use serde_json::{Map as JsonMap, Value as JsonValue};

#[derive(Debug, Deserialize)]
struct BedEntry {
    #[serde(rename = "ma")] pub ma: Option<String>,
    #[serde(rename = "mo")] pub mo: Option<String>,
    #[serde(rename = "v")] pub v: Option<u8>,
    #[serde(rename = "sn")] pub sn: Option<String>,
    #[serde(rename = "d")] pub d: Option<BedD>,
}

#[derive(Debug, Deserialize)]
struct BedD {
    #[serde(rename = "st")] pub st: Option<String>,
    #[serde(rename = "hb")] pub hb: Option<i32>,
    #[serde(rename = "br")] pub br: Option<i32>,
    #[serde(rename = "wt")] pub wt: Option<bool>,
    #[serde(rename = "od")] pub od: Option<i32>,
    #[serde(rename = "we")] pub we: Option<i32>,
    #[serde(rename = "p")] pub p: Option<Vec<i32>>,
    #[serde(rename = "fv")] pub fv: Option<i32>,
}

/// Attempt to decode from the provided buffer. Returns (consumed_bytes, Option<serde_json::Value>)
pub fn decode_buffer(buf: &[u8]) -> Option<(usize, Option<JsonValue>)> {
    if buf.len() < 4 {
        return None;
    }
    // check for magic 0xAB 0xCD
    if buf[0] == 0xAB && buf[1] == 0xCD {
        let len = buf[2] as usize;
        let total = 4 + len;
        if buf.len() < total {
            return None;
        }
        let data = &buf[4..total];
        // try decode MessagePack
        match from_read_ref::<_, BedEntry>(data) {
            Ok(entry) => {
                let md = map_bedentry(entry);
                return Some((total, Some(md)));
            }
            Err(e) => {
                log::warn!("msgpack decode failed: {}", e);
                return Some((total, None));
            }
        }
    }
    // fallback: simple TLV-like parse (draft)
    // try to find a full TLV message by checking a length at pos 2
    let len = buf[2] as usize;
    let total = 4 + len;
    if buf.len() < total {
        return None;
    }
    let payload = &buf[8..total];
    let md = parse_tlv(payload);
    return Some((total, md));
}

fn map_bedentry(entry: BedEntry) -> JsonValue {
    let mut map = JsonMap::new();
    if let Some(sn) = entry.sn { map.insert("sn".to_string(), JsonValue::String(sn)); }
    if let Some(d) = entry.d {
        if let Some(st) = d.st { map.insert("st".to_string(), JsonValue::String(st)); }
        if let Some(hb) = d.hb { map.insert("hb".to_string(), JsonValue::Number(serde_json::Number::from(hb))); }
        if let Some(br) = d.br { map.insert("br".to_string(), JsonValue::Number(serde_json::Number::from(br))); }
        if let Some(wt) = d.wt { map.insert("wt".to_string(), JsonValue::Bool(wt)); }
        if let Some(od) = d.od { map.insert("od".to_string(), JsonValue::Number(serde_json::Number::from(od))); }
        if let Some(we) = d.we { map.insert("we".to_string(), JsonValue::Number(serde_json::Number::from(we))); }
        if let Some(p) = d.p { if p.len() >= 2 { map.insert("p".to_string(), JsonValue::String(format!("[{},{}]", p[0], p[1]))); } }
        if let Some(fv) = d.fv { map.insert("fv".to_string(), JsonValue::Number(serde_json::Number::from(fv))); }
    }
    map.insert("time".to_string(), JsonValue::String(Local::now().format("%Y-%m-%d %H:%M:%S").to_string()));
    JsonValue::Object(map)
}

fn parse_tlv(payload: &[u8]) -> Option<JsonValue> {
    let mut idx = 0usize;
    let mut map = JsonMap::new();
    while idx < payload.len() {
        let b = payload[idx];
        idx += 1;
        if b == 0x92 {
            if idx + 1 >= payload.len() { break; }
            let a = payload[idx] as i32;
            let c = payload[idx + 1] as i32;
            map.insert("p".to_string(), JsonValue::String(format!("[{},{}]", a, c)));
            idx += 2;
            continue;
        }
        if b >= 0xA1 && b <= 0xA7 {
            let l = (b - 0xA0) as usize;
            if idx + l > payload.len() { break; }
            let key = String::from_utf8_lossy(&payload[idx..idx + l]).to_string();
            idx += l;
            if idx >= payload.len() { break; }
            let v = payload[idx];
            idx += 1;
            match key.as_str() {
                "hb" => { map.insert("hb".to_string(), JsonValue::Number(serde_json::Number::from(v as i32))); }
                "br" => { map.insert("br".to_string(), JsonValue::Number(serde_json::Number::from(v as i32))); }
                "od" => { map.insert("od".to_string(), JsonValue::Number(serde_json::Number::from(if v == 255 { -1i32 } else { v as i32 }))); }
                "p" => { map.insert("p".to_string(), JsonValue::String(format!("[{},{}]", v, payload.get(idx).cloned().unwrap_or(0) as u8))); }
                "st" => { map.insert("st".to_string(), JsonValue::String(String::from_utf8_lossy(&[v]).to_string())); }
                "we" => { map.insert("we".to_string(), JsonValue::Number(serde_json::Number::from(if v == 255 { -1i32 } else { v as i32 }))); }
                "wt" => { map.insert("wt".to_string(), JsonValue::String(if v == 195 { "1".to_string() } else { "0".to_string() })); }
                "sn" => { map.insert("sn".to_string(), JsonValue::String(String::from_utf8_lossy(&[v]).to_string())); }
                _ => {}
            }
        }
    }
    map.insert("time".to_string(), JsonValue::String(Local::now().format("%Y-%m-%d %H:%M:%S").to_string()));
    Some(JsonValue::Object(map))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmp_serde::to_vec;
    use serde_json::json;

    #[test]
    fn test_decode_msgpack() {
        // build a simple msgpack map matching expected BedEntry shape
        let d = json!({
            "sn": "Z57566",
            "d": {
                "st": "on",
                "hb": 75,
                "br": 14,
                "we": 20,
                "p": [4,4],
                "wt": false
            }
        });
        let data = to_vec(&d).expect("encode");
        let len = data.len() as u8;
        let mut buf = vec![0xABu8, 0xCDu8, len, 0u8];
        buf.extend_from_slice(&data);

        let res = decode_buffer(&buf).expect("decoded");
        assert_eq!(res.0, 4 + data.len());
        let md = res.1.expect("have data");
        assert_eq!(md["sn"].as_str().unwrap(), "Z57566");
        assert_eq!(md["st"].as_str().unwrap(), "on");
        assert_eq!(md["hb"].as_i64().unwrap(), 75);
        assert_eq!(md["br"].as_i64().unwrap(), 14);
        assert_eq!(md["p"].as_str().unwrap(), "[4,4]");
    }

    #[test]
    fn test_decode_tlv_fallback() {
        // construct payload: 0xA2 'h' 'b' <75> 0x92 <4><4>
        let mut payload: Vec<u8> = Vec::new();
        payload.push(0xA2);
        payload.extend_from_slice(b"hb");
        payload.push(75u8);
        payload.push(0x92);
        payload.push(4u8);
        payload.push(4u8);

        let len = (payload.len() + 4) as u8; // TLV parse expects payload starts at offset 8 in buffer; we emulate len accordingly
        // build full buffer: header 4 bytes + 4 filler bytes then payload
        let mut buf = vec![0u8; 4 + 4];
        buf[2] = len; // set len
        buf.extend_from_slice(&payload);

        let res = decode_buffer(&buf).expect("decoded tlv");
        let md = res.1.expect("have tlv data");
        assert_eq!(md["hb"].as_i64().unwrap(), 75);
        assert_eq!(md["p"].as_str().unwrap(), "[4,4]");
    }
}

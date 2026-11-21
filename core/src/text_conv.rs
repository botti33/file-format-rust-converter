//! Text, markup and structured-data conversions.
//!
//! Structured formats all funnel through `serde_json::Value`, which means
//! adding a new data format only requires one reader and one writer.

use std::collections::BTreeMap;
use std::io::Cursor;

use configparser::ini::Ini;
use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{Map, Value};

use crate::engine::ConvertError;
use crate::model;

pub fn is_structured(ext: &str) -> bool {
    let e = model::canonical(ext);
    model::DATA_FORMATS.contains(&e)
}

pub fn is_text(ext: &str) -> bool {
    let e = model::canonical(ext);
    model::TEXT_FORMATS.contains(&e)
}

fn utf8(data: &[u8], what: &str) -> Result<String, ConvertError> {
    std::str::from_utf8(data)
        .map(str::to_owned)
        .map_err(|e| ConvertError::InvalidData(what.into(), e.to_string()))
}

fn bad(what: &str, e: impl std::fmt::Display) -> ConvertError {
    ConvertError::InvalidData(what.into(), e.to_string())
}

// ---------------------------------------------------------------------------
// Structured data
// ---------------------------------------------------------------------------

pub fn convert_structured(data: &[u8], from: &str, to: &str) -> Result<Vec<u8>, ConvertError> {
    let value = read_structured(data, from)?;
    write_structured(&value, to)
}

fn read_structured(data: &[u8], from: &str) -> Result<Value, ConvertError> {
    match from {
        "json" => serde_json::from_slice(data).map_err(|e| bad("json", e)),
        "yaml" => serde_yaml::from_slice(data).map_err(|e| bad("yaml", e)),
        "toml" => toml::from_str(&utf8(data, "toml")?).map_err(|e| bad("toml", e)),
        "csv" => csv_to_value(data),
        "xml" => xml_to_value(data),
        "ron" => ron::from_str(&utf8(data, "ron")?).map_err(|e| bad("ron", e)),
        "json5" => json5::from_str(&utf8(data, "json5")?).map_err(|e| bad("json5", e)),
        "ini" => ini_to_value(&utf8(data, "ini")?),
        other => Err(ConvertError::Other(format!("no data reader for .{other}"))),
    }
}

fn write_structured(value: &Value, to: &str) -> Result<Vec<u8>, ConvertError> {
    match to {
        "json" | "json5" => serde_json::to_vec_pretty(value).map_err(|e| bad("json", e)),
        "yaml" => serde_yaml::to_string(value)
            .map(String::into_bytes)
            .map_err(|e| bad("yaml", e)),
        "toml" => {
            let normalized = for_toml(value);
            toml::to_string_pretty(&normalized)
                .map(String::into_bytes)
                .map_err(|e| bad("toml", e))
        }
        "csv" => value_to_csv(value),
        "xml" => Ok(value_to_xml(value)),
        "ron" => ron::ser::to_string_pretty(value, ron::ser::PrettyConfig::default())
            .map(String::into_bytes)
            .map_err(|e| bad("ron", e)),
        "ini" => value_to_ini(value),
        other => Err(ConvertError::Other(format!("no data writer for .{other}"))),
    }
}

/// TOML requires a table at the root and has no null value.
fn for_toml(value: &Value) -> Value {
    fn clean(v: &Value) -> Value {
        match v {
            Value::Object(m) => {
                let mut out = Map::new();
                for (k, val) in m {
                    if !val.is_null() {
                        out.insert(k.clone(), clean(val));
                    }
                }
                Value::Object(out)
            }
            Value::Array(a) => Value::Array(a.iter().map(clean).collect()),
            other => other.clone(),
        }
    }
    let cleaned = clean(value);
    if cleaned.is_object() {
        cleaned
    } else {
        let mut root = Map::new();
        root.insert("value".into(), cleaned);
        Value::Object(root)
    }
}

// ---- CSV ------------------------------------------------------------------

fn cell_value(s: &str) -> Value {
    let t = s.trim();
    if t.is_empty() {
        return Value::Null;
    }
    if t.eq_ignore_ascii_case("true") {
        return Value::Bool(true);
    }
    if t.eq_ignore_ascii_case("false") {
        return Value::Bool(false);
    }
    if let Ok(i) = t.parse::<i64>() {
        return Value::Number(i.into());
    }
    if let Ok(f) = t.parse::<f64>() {
        if let Some(n) = serde_json::Number::from_f64(f) {
            return Value::Number(n);
        }
    }
    Value::String(s.to_string())
}

fn csv_to_value(data: &[u8]) -> Result<Value, ConvertError> {
    let mut reader = csv::ReaderBuilder::new().has_headers(true).from_reader(data);
    let headers: Vec<String> = reader
        .headers()
        .map_err(|e| bad("csv", e))?
        .iter()
        .map(str::to_string)
        .collect();

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.map_err(|e| bad("csv", e))?;
        let mut object = Map::new();
        for (i, header) in headers.iter().enumerate() {
            object.insert(header.clone(), cell_value(record.get(i).unwrap_or("")));
        }
        rows.push(Value::Object(object));
    }
    Ok(Value::Array(rows))
}

fn cell_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        other => serde_json::to_string(other).unwrap_or_default(),
    }
}

fn value_to_csv(value: &Value) -> Result<Vec<u8>, ConvertError> {
    let mut writer = csv::Writer::from_writer(Vec::new());
    match value {
        Value::Array(items) => {
            let all_objects = items.iter().all(|v| v.is_object());
            if all_objects && !items.is_empty() {
                let mut headers: Vec<String> = Vec::new();
                for item in items {
                    if let Value::Object(map) = item {
                        for key in map.keys() {
                            if !headers.contains(key) {
                                headers.push(key.clone());
                            }
                        }
                    }
                }
                writer.write_record(&headers).map_err(|e| bad("csv", e))?;
                for item in items {
                    if let Value::Object(map) = item {
                        let row: Vec<String> = headers
                            .iter()
                            .map(|h| map.get(h).map(cell_string).unwrap_or_default())
                            .collect();
                        writer.write_record(row).map_err(|e| bad("csv", e))?;
                    }
                }
            } else {
                for item in items {
                    let row: Vec<String> = match item {
                        Value::Array(cells) => cells.iter().map(cell_string).collect(),
                        other => vec![cell_string(other)],
                    };
                    writer.write_record(row).map_err(|e| bad("csv", e))?;
                }
            }
        }
        Value::Object(map) => {
            let headers: Vec<String> = map.keys().cloned().collect();
            let row: Vec<String> = map.values().map(cell_string).collect();
            writer.write_record(headers).map_err(|e| bad("csv", e))?;
            writer.write_record(row).map_err(|e| bad("csv", e))?;
        }
        other => writer.write_record([cell_string(other)]).map_err(|e| bad("csv", e))?,
    }
    writer.into_inner().map_err(|e| bad("csv", e))
}

// ---- XML ------------------------------------------------------------------

#[derive(Default)]
struct XmlNode {
    name: String,
    attrs: Vec<(String, String)>,
    text: String,
    children: Vec<XmlNode>,
}

fn node_from_start(start: &quick_xml::events::BytesStart<'_>) -> Result<XmlNode, ConvertError> {
    let mut node = XmlNode {
        name: start.local_name().into_inner().to_string(),
        ..Default::default()
    };
    for attr in start.attributes() {
        let attr = attr.map_err(|e| bad("xml", e))?;
        let key = attr.key.as_ref().to_string();
        let value = attr
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|e| bad("xml", e))?
            .into_owned();
        node.attrs.push((key, value));
    }
    Ok(node)
}

fn attach(stack: &mut Vec<XmlNode>, root: &mut Option<XmlNode>, node: XmlNode) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else if root.is_none() {
        *root = Some(node);
    }
}

fn xml_to_value(data: &[u8]) -> Result<Value, ConvertError> {
    let mut reader = Reader::from_reader(data);
    reader.config_mut().trim_text(true);

    let mut stack: Vec<XmlNode> = Vec::new();
    let mut root: Option<XmlNode> = None;

    loop {
        match reader.read_event().map_err(|e| bad("xml", e))? {
            Event::Start(start) => {
                stack.push(node_from_start(&start)?);
            }
            Event::Empty(start) => {
                let node = node_from_start(&start)?;
                attach(&mut stack, &mut root, node);
            }
            Event::End(_) => {
                if let Some(node) = stack.pop() {
                    attach(&mut stack, &mut root, node);
                }
            }
            Event::Text(text) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(
                        &quick_xml::escape::unescape(text.as_ref()).map_err(|e| bad("xml", e))?,
                    );
                }
            }
            Event::CData(text) => {
                if let Some(node) = stack.last_mut() {
                    node.text.push_str(text.as_ref());
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    match root {
        Some(node) => Ok(node_to_value(&node)),
        None => Err(ConvertError::InvalidData("xml".into(), "no root element found".into())),
    }
}

fn scalar(text: &str) -> Value {
    let t = text.trim();
    if t.eq_ignore_ascii_case("true") {
        Value::Bool(true)
    } else if t.eq_ignore_ascii_case("false") {
        Value::Bool(false)
    } else if t.eq_ignore_ascii_case("null") {
        Value::Null
    } else if let Ok(i) = t.parse::<i64>() {
        Value::Number(i.into())
    } else if let Ok(f) = t.parse::<f64>() {
        serde_json::Number::from_f64(f).map(Value::Number).unwrap_or_else(|| Value::String(t.to_string()))
    } else {
        Value::String(text.to_string())
    }
}

fn node_to_value(node: &XmlNode) -> Value {
    let mut map = Map::new();
    for (key, value) in &node.attrs {
        map.insert(format!("@{key}"), Value::String(value.clone()));
    }

    let mut groups: BTreeMap<String, Vec<&XmlNode>> = BTreeMap::new();
    for child in &node.children {
        groups.entry(child.name.clone()).or_default().push(child);
    }
    for (name, children) in groups {
        let values: Vec<Value> = children.iter().map(|c| node_to_value(c)).collect();
        if values.len() == 1 {
            map.insert(name, values.into_iter().next().unwrap());
        } else {
            map.insert(name, Value::Array(values));
        }
    }

    let text = node.text.trim();
    if !text.is_empty() {
        if map.is_empty() {
            return scalar(text);
        }
        map.insert("#text".into(), scalar(text));
    }

    if map.is_empty() {
        Value::Null
    } else {
        Value::Object(map)
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

fn xml_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() || out.as_bytes()[0].is_ascii_digit() {
        out.insert(0, '_');
    }
    out
}

fn scalar_string(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        other => cell_string(other),
    }
}

fn emit_element(out: &mut String, name: &str, value: &Value) {
    let name = xml_name(name);
    match value {
        Value::Array(items) => {
            for item in items {
                emit_element(out, &name, item);
            }
        }
        Value::Object(map) => {
            out.push('<');
            out.push_str(&name);
            out.push('>');
            for (key, val) in map {
                emit_element(out, key, val);
            }
            out.push_str("</");
            out.push_str(&name);
            out.push('>');
        }
        Value::Null => {
            out.push('<');
            out.push_str(&name);
            out.push_str(" />");
        }
        other => {
            out.push('<');
            out.push_str(&name);
            out.push('>');
            out.push_str(&escape_xml(&scalar_string(other)));
            out.push_str("</");
            out.push_str(&name);
            out.push('>');
        }
    }
}

fn value_to_xml(value: &Value) -> Vec<u8> {
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<root>\n");
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                emit_element(&mut out, key, val);
            }
        }
        Value::Array(items) => {
            for item in items {
                emit_element(&mut out, "item", item);
            }
        }
        Value::Null => {}
        other => emit_element(&mut out, "value", other),
    }
    out.push_str("\n</root>\n");
    out.into_bytes()
}

// ---- INI ------------------------------------------------------------------

fn ini_to_value(text: &str) -> Result<Value, ConvertError> {
    let mut ini = Ini::new();
    ini.read(text.to_string()).map_err(|e| bad("ini", e))?;
    let mut root = Map::new();
    for (section, props) in ini.get_map_ref().iter() {
        let mut object = Map::new();
        for (key, value) in props.iter() {
            if let Some(value) = value {
                object.insert(key.clone(), scalar(value));
            }
        }
        if !object.is_empty() {
            root.insert(section.clone(), Value::Object(object));
        }
    }
    Ok(Value::Object(root))
}

fn value_to_ini(value: &Value) -> Result<Vec<u8>, ConvertError> {
    let mut ini = Ini::new();
    match value {
        Value::Object(map) => {
            for (key, val) in map {
                if let Value::Object(props) = val {
                    for (k, v) in props {
                        ini.set(key, k, Some(scalar_string(v)));
                    }
                } else {
                    ini.set("default", key, Some(scalar_string(val)));
                }
            }
        }
        other => {
            ini.set("default", "value", Some(scalar_string(other)));
        }
    }
    Ok(ini.writes().into_bytes())
}

// ---------------------------------------------------------------------------
// Text / markup
// ---------------------------------------------------------------------------

pub fn convert_text(data: &[u8], from: &str, to: &str) -> Result<Vec<u8>, ConvertError> {
    let text = String::from_utf8_lossy(data).into_owned();
    let output = match (from, to) {
        ("md", "html") => markdown_to_html(&text),
        ("md", "txt") => html_to_plain(&markdown_to_html(&text)),
        ("html", "txt") => html_to_plain(&text),
        ("html", "md") => quick_html2md::html_to_markdown(&text),
        ("txt", "html") => format!(
            "<!doctype html>\n<html><head><meta charset=\"utf-8\"><title>Converted text</title></head>\n<body><pre>{}</pre></body></html>\n",
            escape_html(&text)
        ),
        ("txt", "md") => format!("```text\n{text}\n```\n"),
        _ => return Err(ConvertError::Unsupported(from.into(), to.into())),
    };
    Ok(output.into_bytes())
}

fn markdown_to_html(text: &str) -> String {
    let mut options = pulldown_cmark::Options::empty();
    options.insert(pulldown_cmark::Options::ENABLE_TABLES);
    options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
    options.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
    let parser = pulldown_cmark::Parser::new_ext(text, options);
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, parser);
    html
}

fn html_to_plain(html: &str) -> String {
    let mut text = html2text::from_read(Cursor::new(html.as_bytes()), 100)
        .unwrap_or_else(|_| html.to_string());
    while text.ends_with('\n') {
        text.pop();
    }
    text.push('\n');
    text
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

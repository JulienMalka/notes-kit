#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Scalar(String),
    List(Vec<String>),
}

impl Value {
    pub fn text(&self) -> String {
        match self {
            Value::Scalar(s) => s.clone(),
            Value::List(items) => items.join(" "),
        }
    }

    pub fn items(&self) -> Vec<String> {
        match self {
            Value::List(items) => items.clone(),
            Value::Scalar(s) => s
                .split([':', ' ', ','])
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_string)
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrontMatter {
    pub fields: Vec<(String, Value)>,
    pub body_start: usize,
}

impl FrontMatter {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.fields
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v)
    }

    pub fn scalar(&self, key: &str) -> Option<String> {
        self.get(key).map(Value::text).filter(|v| !v.is_empty())
    }

    pub fn body<'a>(&self, content: &'a str) -> &'a str {
        &content[self.body_start.min(content.len())..]
    }
}

pub const META_KEYS: [&str; 3] = ["title", "tags", "access"];

pub fn is_meta_key(key: &str) -> bool {
    META_KEYS.iter().any(|m| key.eq_ignore_ascii_case(m))
}

fn unquote(raw: &str) -> String {
    let raw = raw.trim();
    if raw.len() >= 2 && raw.starts_with('"') && raw.ends_with('"') {
        let mut out = String::new();
        let mut chars = raw[1..raw.len() - 1].chars();
        while let Some(c) = chars.next() {
            if c == '\\' {
                match chars.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some(other) => out.push(other),
                    None => {}
                }
            } else {
                out.push(c);
            }
        }
        return out;
    }
    if raw.len() >= 2 && raw.starts_with('\'') && raw.ends_with('\'') {
        return raw[1..raw.len() - 1].replace("''", "'");
    }
    raw.to_string()
}

fn flow_list(raw: &str) -> Option<Vec<String>> {
    let inner = raw.trim().strip_prefix('[')?.strip_suffix(']')?;
    Some(inner.split(',').map(unquote).filter(|s| !s.is_empty()).collect())
}

pub fn parse(content: &str) -> FrontMatter {
    let mut fields: Vec<(String, Value)> = Vec::new();
    let first = content.lines().next().unwrap_or("").trim_end();
    if first != "---" {
        return FrontMatter { fields, body_start: 0 };
    }
    let mut offset = first.len() + 1;
    let mut lines = content[offset.min(content.len())..].split_inclusive('\n').peekable();
    let mut body_start = content.len();
    while let Some(line) = lines.next() {
        offset += line.len();
        let trimmed = line.trim_end();
        if trimmed == "---" || trimmed == "..." {
            body_start = offset;
            break;
        }
        if trimmed.trim().is_empty() || trimmed.trim_start().starts_with('#') {
            continue;
        }
        if line.starts_with([' ', '\t']) {
            continue;
        }
        let Some((key, rest)) = trimmed.split_once(':') else { continue };
        let key = key.trim().to_string();
        let rest = rest.trim();
        if rest.is_empty() || rest == "|" || rest == ">" {
            let block_scalar = rest == "|" || rest == ">";
            let mut items: Vec<String> = Vec::new();
            let mut block: Vec<String> = Vec::new();
            while let Some(next) = lines.peek() {
                let n = next.trim_end();
                let blank = n.trim().is_empty();
                if blank && block_scalar {
                    offset += next.len();
                    lines.next();
                    block.push(String::new());
                    continue;
                }
                if blank || !next.starts_with([' ', '\t']) {
                    break;
                }
                offset += next.len();
                lines.next();
                match n.trim().strip_prefix("- ") {
                    Some(item) if !block_scalar => items.push(unquote(item)),
                    _ => block.push(n.trim().to_string()),
                }
            }
            let value = if !items.is_empty() {
                Value::List(items)
            } else if rest == "|" {
                Value::Scalar(block.join("\n").trim().to_string())
            } else {
                Value::Scalar(block.join(" ").trim().to_string())
            };
            fields.push((key, value));
            continue;
        }
        let value = match flow_list(rest) {
            Some(items) => Value::List(items),
            None => Value::Scalar(unquote(rest)),
        };
        fields.push((key, value));
    }
    FrontMatter { fields, body_start: body_start.min(content.len()) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields() {
        let c = "---\ntitle: Rank-one edits\ntags: [editing, memory]\nid: EXP-07\ntests: \"[[h-02]]\"\nowner: \"@mara\"\nsource:\n  - https://a\n  - https://b\nnote: >\n  folded\n  text\nempty:\n---\n\nBody [[h-02]].\n";
        let fm = parse(c);
        assert_eq!(fm.scalar("title").as_deref(), Some("Rank-one edits"));
        assert_eq!(fm.get("tags").unwrap().items(), vec!["editing", "memory"]);
        assert_eq!(fm.scalar("tests").as_deref(), Some("[[h-02]]"));
        assert_eq!(fm.scalar("owner").as_deref(), Some("@mara"));
        assert_eq!(fm.scalar("source").as_deref(), Some("https://a https://b"));
        assert_eq!(fm.scalar("note").as_deref(), Some("folded text"));
        assert_eq!(fm.get("empty").unwrap().text(), "");
        assert!(fm.body(c).starts_with("\nBody"));
    }

    #[test]
    fn no_front_matter() {
        let fm = parse("# Hello\n\ntext\n");
        assert!(fm.fields.is_empty());
        assert_eq!(fm.body_start, 0);
    }
}

/// LRC 歌词解析与时间同步

pub struct LrcLine {
    pub time_secs: u32,
    pub text: String,
}

pub enum ParsedLyrics {
    Synced(Vec<LrcLine>),
    Plain(Vec<String>),
    Empty,
}

/// 解析 LRC 格式歌词，支持 `[mm:ss.xx]text` 和一行多时间标签
pub fn parse(raw: &str) -> ParsedLyrics {
    let raw = raw.trim();
    if raw.is_empty() {
        return ParsedLyrics::Empty;
    }

    let mut lines: Vec<LrcLine> = Vec::new();

    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let mut times: Vec<u32> = Vec::new();
        let mut rest = line;

        // 提取所有 [mm:ss.xx] 时间标签
        while rest.starts_with('[') {
            let Some(close) = rest.find(']') else { break };
            let tag = &rest[1..close];
            if let Some(secs) = parse_timestamp(tag) {
                times.push(secs);
                rest = &rest[close + 1..];
            } else {
                // 非时间标签（如 [ti:xxx]），跳过整个标签
                rest = &rest[close + 1..];
            }
        }

        let text = rest.trim().to_string();
        if times.is_empty() {
            continue;
        }

        for t in times {
            lines.push(LrcLine { time_secs: t, text: text.clone() });
        }
    }

    if lines.is_empty() {
        // 没有解析到时间标签，回退为纯文本
        let plain: Vec<String> = raw.lines().map(|l| l.to_string()).collect();
        return ParsedLyrics::Plain(plain);
    }

    lines.sort_by_key(|l| l.time_secs);
    ParsedLyrics::Synced(lines)
}

/// 解析时间戳 `mm:ss` 或 `mm:ss.xx`，返回秒数
fn parse_timestamp(tag: &str) -> Option<u32> {
    let (min_str, rest) = tag.split_once(':')?;
    let min: u32 = min_str.parse().ok()?;

    // rest 可能是 "ss" 或 "ss.xx"
    let sec: u32 = if let Some((sec_str, _frac)) = rest.split_once('.') {
        sec_str.parse().ok()?
    } else {
        rest.parse().ok()?
    };

    Some(min * 60 + sec)
}

impl ParsedLyrics {
    /// 二分查找 `time_secs <= current_secs` 的最后一行索引
    #[allow(dead_code)] // TODO: 歌词高亮定位
    pub fn current_index(&self, current_secs: u32) -> Option<usize> {
        let ParsedLyrics::Synced(lines) = self else { return None };
        if lines.is_empty() {
            return None;
        }
        // 找最后一个 time_secs <= current_secs
        let idx = lines.partition_point(|l| l.time_secs <= current_secs);
        if idx == 0 { None } else { Some(idx - 1) }
    }
}

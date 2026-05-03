use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SmpsFile {
    pub song_label: String,
    pub voice_ref: VoiceRef,
    pub fm_count: u8,
    pub psg_count: u8,
    pub tempo_divider: u8,
    pub tempo_modifier: u8,
    pub channels: Vec<SmpsChannel>,
    pub voices: Vec<[u8; 25]>,
}

#[derive(Debug, Clone)]
pub enum VoiceRef {
    Inline(String),
    Uvb,
    External(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpsChannelKind {
    Dac,
    Fm,
    Psg,
}

#[derive(Debug, Clone)]
pub struct SmpsChannel {
    pub kind: SmpsChannelKind,
    pub label: String,
    pub initial_pitch: i8,
    pub initial_volume: u8,
    pub psg_envelope: Option<u8>,
    pub events: Vec<SmpsEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SmpsEvent {
    Note { pitch: u8, duration: u8 },
    Rest { duration: u8 },
    SetVoice(u8),
    SetPan(u8),
    Transpose(i8),
    Detune(i8),
    VolumeChange(i8),
    SetModulation { wait: u8, speed: u8, delta: u8, steps: u8 },
    ModOff,
    Tie,
    Stop,
    Unsupported { name: String },
}

struct Tables {
    notes: HashMap<String, u8>,
    dac: HashMap<String, u8>,
    pan: HashMap<String, u8>,
}

struct ChannelHeader {
    kind: SmpsChannelKind,
    label: String,
    initial_pitch: i8,
    initial_volume: u8,
    psg_envelope: Option<u8>,
}

struct ParsedHeader {
    song_label: String,
    voice_ref: VoiceRef,
    fm_count: u8,
    psg_count: u8,
    tempo_divider: u8,
    tempo_modifier: u8,
    channel_headers: Vec<ChannelHeader>,
}

fn strip_comment(line: &str) -> &str {
    if let Some(pos) = line.find(';') {
        line[..pos].trim_end()
    } else {
        line
    }
}

fn parse_hex(s: &str) -> Option<u8> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('$') {
        u8::from_str_radix(hex, 16).ok()
    } else {
        s.parse::<u8>().ok()
    }
}

fn parse_hex_i8(s: &str) -> Option<i8> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('$') {
        u8::from_str_radix(hex, 16).ok().map(|v| v as i8)
    } else {
        s.parse::<i8>().ok()
    }
}

fn parse_psg_envelope(s: &str) -> Option<u8> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("sTone_") {
        u8::from_str_radix(hex, 16).ok()
    } else {
        parse_hex(s)
    }
}

fn extract_macro_args(line: &str, macro_name: &str) -> Vec<String> {
    let rest = line.trim().strip_prefix(macro_name).unwrap_or("").trim();
    if rest.is_empty() {
        vec![]
    } else {
        rest.split(',').map(|x| x.trim().to_string()).collect()
    }
}

fn try_extract_label(trimmed: &str) -> Option<&str> {
    let colon = trimmed.find(':')?;
    let label = trimmed[..colon].trim();
    if !label.is_empty() && !label.contains(|c: char| c.is_whitespace()) {
        Some(label)
    } else {
        None
    }
}

fn collect_labels(lines: &[&str]) -> HashMap<String, usize> {
    let mut labels = HashMap::new();
    for (i, line) in lines.iter().enumerate() {
        let trimmed = strip_comment(line).trim();
        if let Some(label) = try_extract_label(trimmed) {
            labels.insert(label.to_string(), i);
        }
    }
    labels
}

fn parse_header(lines: &[&str]) -> Result<ParsedHeader, String> {
    let mut song_label = String::new();
    let mut voice_ref = VoiceRef::External(String::new());
    let mut fm_count = 0u8;
    let mut psg_count = 0u8;
    let mut tempo_divider = 1u8;
    let mut tempo_modifier = 0u8;
    let mut channel_headers = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = strip_comment(line).trim();

        if trimmed.starts_with("smpsHeaderStartSong") && song_label.is_empty() {
            for j in (0..i).rev() {
                let prev = strip_comment(lines[j]).trim();
                if let Some(label) = try_extract_label(prev) {
                    song_label = label.to_string();
                    break;
                }
            }
        }

        if trimmed.starts_with("smpsHeaderVoiceUVB") {
            voice_ref = VoiceRef::Uvb;
        } else if trimmed.starts_with("smpsHeaderVoice") {
            let args = extract_macro_args(trimmed, "smpsHeaderVoice");
            if let Some(label) = args.first() {
                voice_ref = VoiceRef::Inline(label.clone());
            }
        }

        if trimmed.starts_with("smpsHeaderChan") {
            let args = extract_macro_args(trimmed, "smpsHeaderChan");
            if args.len() >= 2 {
                fm_count = parse_hex(&args[0]).unwrap_or(0);
                psg_count = parse_hex(&args[1]).unwrap_or(0);
            }
        }

        if trimmed.starts_with("smpsHeaderTempo") {
            let args = extract_macro_args(trimmed, "smpsHeaderTempo");
            if args.len() >= 2 {
                tempo_divider = parse_hex(&args[0]).unwrap_or(1);
                tempo_modifier = parse_hex(&args[1]).unwrap_or(0);
            }
        }

        if trimmed.starts_with("smpsHeaderDAC") {
            let args = extract_macro_args(trimmed, "smpsHeaderDAC");
            if let Some(label) = args.first() {
                channel_headers.push(ChannelHeader {
                    kind: SmpsChannelKind::Dac,
                    label: label.clone(),
                    initial_pitch: 0,
                    initial_volume: 0,
                    psg_envelope: None,
                });
            }
        }

        if trimmed.starts_with("smpsHeaderFM") && !trimmed.starts_with("smpsHeaderFM3") {
            let args = extract_macro_args(trimmed, "smpsHeaderFM");
            if args.len() >= 3 {
                channel_headers.push(ChannelHeader {
                    kind: SmpsChannelKind::Fm,
                    label: args[0].clone(),
                    initial_pitch: parse_hex_i8(&args[1]).unwrap_or(0),
                    initial_volume: parse_hex(&args[2]).unwrap_or(0),
                    psg_envelope: None,
                });
            }
        }

        if trimmed.starts_with("smpsHeaderPSG") {
            let args = extract_macro_args(trimmed, "smpsHeaderPSG");
            if args.len() >= 4 {
                let envelope = if args.len() >= 5 {
                    parse_psg_envelope(&args[4])
                } else {
                    parse_psg_envelope(&args[3])
                };
                channel_headers.push(ChannelHeader {
                    kind: SmpsChannelKind::Psg,
                    label: args[0].clone(),
                    initial_pitch: parse_hex_i8(&args[1]).unwrap_or(0),
                    initial_volume: parse_hex(&args[2]).unwrap_or(0),
                    psg_envelope: envelope,
                });
            }
        }
    }

    Ok(ParsedHeader {
        song_label,
        voice_ref,
        fm_count,
        psg_count,
        tempo_divider,
        tempo_modifier,
        channel_headers,
    })
}

fn parse_macro_line(trimmed: &str, tables: &Tables) -> Option<Vec<SmpsEvent>> {
    if trimmed.starts_with("smpsSetvoice") || trimmed.starts_with("smpsFMvoice") {
        let name = if trimmed.starts_with("smpsSetvoice") {
            "smpsSetvoice"
        } else {
            "smpsFMvoice"
        };
        let args = extract_macro_args(trimmed, name);
        if let Some(v) = args.first().and_then(|a| parse_hex(a)) {
            return Some(vec![SmpsEvent::SetVoice(v)]);
        }
        return Some(vec![]);
    }

    if trimmed.starts_with("smpsPan") {
        let args = extract_macro_args(trimmed, "smpsPan");
        if let Some(direction) = args.first() {
            let pan = tables
                .pan
                .get(direction.as_str())
                .copied()
                .or_else(|| parse_hex(direction));
            if let Some(p) = pan {
                return Some(vec![SmpsEvent::SetPan(p)]);
            }
        }
        return Some(vec![]);
    }

    if trimmed.starts_with("smpsChangeTransposition") {
        let args = extract_macro_args(trimmed, "smpsChangeTransposition");
        if let Some(v) = args.first().and_then(|a| parse_hex_i8(a)) {
            return Some(vec![SmpsEvent::Transpose(v)]);
        }
        return Some(vec![]);
    }

    if trimmed.starts_with("smpsAlterNote") {
        let args = extract_macro_args(trimmed, "smpsAlterNote");
        if let Some(v) = args.first().and_then(|a| parse_hex_i8(a)) {
            return Some(vec![SmpsEvent::Detune(v)]);
        }
        return Some(vec![]);
    }

    if trimmed.starts_with("smpsModSet") {
        let args = extract_macro_args(trimmed, "smpsModSet");
        if args.len() >= 4 {
            let wait = args[0].parse::<u8>().or_else(|_| parse_hex(&args[0]).ok_or(())).unwrap_or(0);
            let speed = args[1].parse::<u8>().or_else(|_| parse_hex(&args[1]).ok_or(())).unwrap_or(1);
            let delta = args[2].parse::<u8>().or_else(|_| parse_hex(&args[2]).ok_or(())).unwrap_or(0);
            let steps = args[3].parse::<u8>().or_else(|_| parse_hex(&args[3]).ok_or(())).unwrap_or(0);
            return Some(vec![SmpsEvent::SetModulation { wait, speed, delta, steps }]);
        }
        return Some(vec![]);
    }

    if trimmed.starts_with("smpsModOff") {
        return Some(vec![SmpsEvent::ModOff]);
    }

    if trimmed.starts_with("smpsFMAlterVol")
        || trimmed.starts_with("smpsAlterVol")
        || trimmed.starts_with("smpsPSGAlterVol")
    {
        let name = if trimmed.starts_with("smpsFMAlterVol") {
            "smpsFMAlterVol"
        } else if trimmed.starts_with("smpsPSGAlterVol") {
            "smpsPSGAlterVol"
        } else {
            "smpsAlterVol"
        };
        let args = extract_macro_args(trimmed, name);
        if let Some(v) = args.first().and_then(|a| parse_hex_i8(a)) {
            return Some(vec![SmpsEvent::VolumeChange(v)]);
        }
        return Some(vec![]);
    }

    if trimmed.starts_with("smpsPSGvoice") {
        let args = extract_macro_args(trimmed, "smpsPSGvoice");
        if let Some(v) = args.first().and_then(|a| parse_psg_envelope(a)) {
            return Some(vec![SmpsEvent::SetVoice(v)]);
        }
        return Some(vec![]);
    }

    let known_skips: &[&str] = &[
        "smpsModChange",
        "smpsModOn",
        "smpsAlterPitch",
        "smpsDetune",
        "smpsNoteFill",
        "smpsSetTempoMod",
        "smpsSetTempoDiv",
        "smpsSSGEG",
        "smpsPlayDACSample",
        "smpsPSGform",
        "smpsSetNote",
        "smpsFMICommand",
        "smpsSetVol",
        "smpsNop",
        "smpsFade",
        "smpsStopFM",
        "smpsSpindashRev",
        "smpsConditionalJump",
        "smpsAlternateSMPS",
        "smpsFM3SpecialMode",
        "smpsChanTempoDiv",
        "smpsPitchSlide",
        "smpsSetLFO",
    ];

    for &name in known_skips {
        if trimmed.starts_with(name) {
            return Some(vec![SmpsEvent::Unsupported {
                name: name.to_string(),
            }]);
        }
    }

    None
}

fn parse_dcb_tokens(
    data: &str,
    events: &mut Vec<SmpsEvent>,
    current_duration: &mut Option<u8>,
    _kind: SmpsChannelKind,
    tables: &Tables,
) {
    let tokens: Vec<&str> = data.split(',').map(|t| t.trim()).collect();
    let mut i = 0;

    while i < tokens.len() {
        let token = tokens[i];
        if token.is_empty() {
            i += 1;
            continue;
        }

        if let Some(&byte) = tables.notes.get(token) {
            let (dur, consumed) = peek_duration(&tokens, i + 1, current_duration);
            if byte == 0x80 {
                events.push(SmpsEvent::Rest { duration: dur });
            } else {
                events.push(SmpsEvent::Note {
                    pitch: byte,
                    duration: dur,
                });
            }
            i += 1 + consumed;
            continue;
        }

        if let Some(&byte) = tables.dac.get(token) {
            let (dur, consumed) = peek_duration(&tokens, i + 1, current_duration);
            events.push(SmpsEvent::Note {
                pitch: byte,
                duration: dur,
            });
            i += 1 + consumed;
            continue;
        }

        if token == "smpsNoAttack" {
            events.push(SmpsEvent::Tie);
            i += 1;
            continue;
        }

        if let Some(byte) = parse_hex(token) {
            if byte >= 0x80 {
                let (dur, consumed) = peek_duration(&tokens, i + 1, current_duration);
                if byte == 0x80 {
                    events.push(SmpsEvent::Rest { duration: dur });
                } else {
                    events.push(SmpsEvent::Note {
                        pitch: byte,
                        duration: dur,
                    });
                }
                i += 1 + consumed;
            } else {
                *current_duration = Some(byte);
                i += 1;
            }
            continue;
        }

        i += 1;
    }
}

fn peek_duration(tokens: &[&str], next_idx: usize, current: &mut Option<u8>) -> (u8, usize) {
    if next_idx < tokens.len() {
        let next = tokens[next_idx].trim();
        if !next.starts_with('n')
            && !next.starts_with('d')
            && !next.starts_with("smps")
            && !next.starts_with("sTone")
        {
            if let Some(val) = parse_hex(next) {
                if val < 0x80 {
                    *current = Some(val);
                    return (val, 1);
                }
            }
        }
    }
    (current.unwrap_or(1), 0)
}

fn parse_channel_events(
    lines: &[&str],
    all_labels: &HashMap<String, usize>,
    start_line: usize,
    kind: SmpsChannelKind,
    tables: &Tables,
) -> Result<Vec<SmpsEvent>, String> {
    let mut events = Vec::new();
    let mut current_duration: Option<u8> = None;
    let mut label_positions: HashMap<String, usize> = HashMap::new();
    let mut line_idx = start_line;
    let mut call_stack: Vec<usize> = Vec::new();

    while line_idx < lines.len() {
        let raw = strip_comment(lines[line_idx]);
        let trimmed = raw.trim();

        if trimmed.is_empty() {
            line_idx += 1;
            continue;
        }

        if trimmed.starts_with("if ") || trimmed == "if" {
            line_idx += 1;
            continue;
        }
        if trimmed == "else" {
            let mut depth = 1u32;
            line_idx += 1;
            while line_idx < lines.len() && depth > 0 {
                let inner = strip_comment(lines[line_idx]).trim().to_string();
                if inner.starts_with("if ") || inner == "if" {
                    depth += 1;
                } else if inner == "endif" || inner == "endc" {
                    depth -= 1;
                }
                line_idx += 1;
            }
            continue;
        }
        if trimmed == "endif" || trimmed == "endc" {
            line_idx += 1;
            continue;
        }

        if let Some(label) = try_extract_label(trimmed) {
            label_positions.insert(label.to_string(), events.len());
            line_idx += 1;
            continue;
        }

        if trimmed.starts_with("smpsStop") {
            events.push(SmpsEvent::Stop);
            break;
        }

        if trimmed.starts_with("smpsReturn") {
            if let Some(ret) = call_stack.pop() {
                line_idx = ret;
                continue;
            }
            break;
        }

        if trimmed.starts_with("smpsJump") || trimmed.starts_with("smpsContinuousLoop") {
            let name = if trimmed.starts_with("smpsContinuousLoop") {
                "smpsContinuousLoop"
            } else {
                "smpsJump"
            };
            let args = extract_macro_args(trimmed, name);
            if let Some(target) = args.first() {
                if let Some(&target_line) = all_labels.get(target.as_str()) {
                    if target_line <= line_idx {
                        break;
                    }
                    line_idx = target_line;
                    continue;
                }
            }
            break;
        }

        if trimmed.starts_with("smpsLoop") {
            let args = extract_macro_args(trimmed, "smpsLoop");
            if args.len() >= 3 {
                let count = parse_hex(&args[1]).unwrap_or(1) as usize;
                let target = &args[2];
                if let Some(&body_start) = label_positions.get(target.as_str()) {
                    let body: Vec<SmpsEvent> = events[body_start..].to_vec();
                    for _ in 1..count {
                        events.extend(body.iter().cloned());
                    }
                }
            }
            line_idx += 1;
            continue;
        }

        if trimmed.starts_with("smpsCall") {
            let args = extract_macro_args(trimmed, "smpsCall");
            if let Some(target) = args.first() {
                if let Some(&target_line) = all_labels.get(target.as_str()) {
                    call_stack.push(line_idx + 1);
                    line_idx = target_line;
                    continue;
                }
            }
            line_idx += 1;
            continue;
        }

        if trimmed.starts_with("dc.b") {
            let mut merged = trimmed.strip_prefix("dc.b").unwrap_or("").trim().to_string();
            line_idx += 1;
            while line_idx < lines.len() {
                let next_raw = strip_comment(lines[line_idx]);
                let next_trimmed = next_raw.trim();
                if next_trimmed.starts_with("dc.b") {
                    merged.push_str(", ");
                    merged.push_str(next_trimmed.strip_prefix("dc.b").unwrap_or("").trim());
                    line_idx += 1;
                } else {
                    break;
                }
            }
            parse_dcb_tokens(&merged, &mut events, &mut current_duration, kind, tables);
            continue;
        }

        if let Some(new_events) = parse_macro_line(trimmed, tables) {
            events.extend(new_events);
            line_idx += 1;
            continue;
        }

        line_idx += 1;
    }

    Ok(events)
}

fn build_voice_bytes(
    alg: u8,
    fb: u8,
    detune: &[u8; 4],
    mul: &[u8; 4],
    rs: &[u8; 4],
    ar: &[u8; 4],
    am: &[u8; 4],
    d1r: &[u8; 4],
    d2r: &[u8; 4],
    sl: &[u8; 4],
    rr: &[u8; 4],
    tl: &[u8; 4],
) -> [u8; 25] {
    let mut bytes = [0u8; 25];
    let op_order = [0usize, 1, 2, 3];
    for (pos, &idx) in op_order.iter().enumerate() {
        bytes[pos] = (detune[idx] << 4) | (mul[idx] & 0x0F);
        bytes[4 + pos] = (rs[idx] << 6) | (ar[idx] & 0x1F);
        bytes[8 + pos] = (am[idx] << 7) | (d1r[idx] & 0x1F);
        bytes[12 + pos] = d2r[idx] & 0x1F;
        bytes[16 + pos] = (sl[idx] << 4) | (rr[idx] & 0x0F);
        bytes[20 + pos] = tl[idx];
    }
    bytes[24] = (fb << 3) | (alg & 0x07);
    bytes
}

fn parse_4_args(trimmed: &str, name: &str) -> [u8; 4] {
    let args = extract_macro_args(trimmed, name);
    let mut vals = [0u8; 4];
    for (i, a) in args.iter().take(4).enumerate() {
        vals[i] = parse_hex(a).unwrap_or(0);
    }
    vals
}

fn parse_voices(
    lines: &[&str],
    labels: &HashMap<String, usize>,
    voice_label: &str,
) -> Result<Vec<[u8; 25]>, String> {
    let start = labels
        .get(voice_label)
        .ok_or(format!("voice label not found: {voice_label}"))?;

    let mut voices = Vec::new();
    let mut line_idx = *start + 1;

    let mut alg: u8 = 0;
    let mut fb: u8 = 0;
    let mut detune = [0u8; 4];
    let mut mul = [0u8; 4];
    let mut rs = [0u8; 4];
    let mut ar = [0u8; 4];
    let mut am = [0u8; 4];
    let mut d1r = [0u8; 4];
    let mut d2r = [0u8; 4];
    let mut sl = [0u8; 4];
    let mut rr = [0u8; 4];
    let mut tl = [0u8; 4];
    let mut in_voice = false;

    while line_idx < lines.len() {
        let trimmed = strip_comment(lines[line_idx]).trim();

        if trimmed.is_empty() {
            line_idx += 1;
            continue;
        }

        if trimmed.starts_with("smpsVcAlgorithm") {
            if in_voice {
                voices.push(build_voice_bytes(
                    alg, fb, &detune, &mul, &rs, &ar, &am, &d1r, &d2r, &sl, &rr, &tl,
                ));
            }
            in_voice = true;
            detune = [0; 4];
            mul = [0; 4];
            rs = [0; 4];
            ar = [0; 4];
            am = [0; 4];
            d1r = [0; 4];
            d2r = [0; 4];
            sl = [0; 4];
            rr = [0; 4];
            tl = [0; 4];
            fb = 0;
            let args = extract_macro_args(trimmed, "smpsVcAlgorithm");
            alg = args.first().and_then(|a| parse_hex(a)).unwrap_or(0);
        } else if trimmed.starts_with("smpsVcFeedback") {
            let args = extract_macro_args(trimmed, "smpsVcFeedback");
            fb = args.first().and_then(|a| parse_hex(a)).unwrap_or(0);
        } else if trimmed.starts_with("smpsVcUnusedBits") {
            // skip
        } else if trimmed.starts_with("smpsVcDetune") {
            detune = parse_4_args(trimmed, "smpsVcDetune");
        } else if trimmed.starts_with("smpsVcCoarseFreq") {
            mul = parse_4_args(trimmed, "smpsVcCoarseFreq");
        } else if trimmed.starts_with("smpsVcRateScale") {
            rs = parse_4_args(trimmed, "smpsVcRateScale");
        } else if trimmed.starts_with("smpsVcAttackRate") {
            ar = parse_4_args(trimmed, "smpsVcAttackRate");
        } else if trimmed.starts_with("smpsVcAmpMod") {
            am = parse_4_args(trimmed, "smpsVcAmpMod");
        } else if trimmed.starts_with("smpsVcDecayRate1") {
            d1r = parse_4_args(trimmed, "smpsVcDecayRate1");
        } else if trimmed.starts_with("smpsVcDecayRate2") {
            d2r = parse_4_args(trimmed, "smpsVcDecayRate2");
        } else if trimmed.starts_with("smpsVcDecayLevel") {
            sl = parse_4_args(trimmed, "smpsVcDecayLevel");
        } else if trimmed.starts_with("smpsVcReleaseRate") {
            rr = parse_4_args(trimmed, "smpsVcReleaseRate");
        } else if trimmed.starts_with("smpsVcTotalLevel") {
            tl = parse_4_args(trimmed, "smpsVcTotalLevel");
        } else if !trimmed.starts_with("smpsVc") {
            break;
        }

        line_idx += 1;
    }

    if in_voice {
        voices.push(build_voice_bytes(
            alg, fb, &detune, &mul, &rs, &ar, &am, &d1r, &d2r, &sl, &rr, &tl,
        ));
    }

    Ok(voices)
}

pub fn parse_smps(source: &str) -> Result<SmpsFile, String> {
    let tables = Tables {
        notes: build_note_table(),
        dac: build_dac_table(),
        pan: build_pan_table(),
    };

    let lines: Vec<&str> = source.lines().collect();
    let all_labels = collect_labels(&lines);
    let header = parse_header(&lines)?;

    let mut channels = Vec::new();
    for ch in &header.channel_headers {
        let start_line = all_labels
            .get(&ch.label)
            .ok_or(format!("channel label not found: {}", ch.label))?;

        let events =
            parse_channel_events(&lines, &all_labels, *start_line, ch.kind, &tables)?;

        channels.push(SmpsChannel {
            kind: ch.kind,
            label: ch.label.clone(),
            initial_pitch: ch.initial_pitch,
            initial_volume: ch.initial_volume,
            psg_envelope: ch.psg_envelope,
            events,
        });
    }

    let voices = if let VoiceRef::Inline(ref label) = header.voice_ref {
        parse_voices(&lines, &all_labels, label)?
    } else {
        vec![]
    };

    Ok(SmpsFile {
        song_label: header.song_label,
        voice_ref: header.voice_ref,
        fm_count: header.fm_count,
        psg_count: header.psg_count,
        tempo_divider: header.tempo_divider,
        tempo_modifier: header.tempo_modifier,
        channels,
        voices,
    })
}

pub fn parse_voice_bank(source: &str) -> Result<Vec<[u8; 25]>, String> {
    let lines: Vec<&str> = source.lines().collect();
    let mut voices = Vec::new();

    let mut alg: u8 = 0;
    let mut fb: u8 = 0;
    let mut detune = [0u8; 4];
    let mut mul = [0u8; 4];
    let mut rs = [0u8; 4];
    let mut ar = [0u8; 4];
    let mut am = [0u8; 4];
    let mut d1r = [0u8; 4];
    let mut d2r = [0u8; 4];
    let mut sl = [0u8; 4];
    let mut rr = [0u8; 4];
    let mut tl = [0u8; 4];
    let mut in_voice = false;

    for line in &lines {
        let trimmed = strip_comment(line).trim();
        if trimmed.is_empty() { continue; }

        if trimmed.starts_with("smpsVcAlgorithm") {
            if in_voice {
                voices.push(build_voice_bytes(alg, fb, &detune, &mul, &rs, &ar, &am, &d1r, &d2r, &sl, &rr, &tl));
            }
            in_voice = true;
            detune = [0; 4]; mul = [0; 4]; rs = [0; 4]; ar = [0; 4];
            am = [0; 4]; d1r = [0; 4]; d2r = [0; 4]; sl = [0; 4];
            rr = [0; 4]; tl = [0; 4]; fb = 0;
            let args = extract_macro_args(trimmed, "smpsVcAlgorithm");
            alg = args.first().and_then(|a| parse_hex(a)).unwrap_or(0);
        } else if trimmed.starts_with("smpsVcFeedback") {
            let args = extract_macro_args(trimmed, "smpsVcFeedback");
            fb = args.first().and_then(|a| parse_hex(a)).unwrap_or(0);
        } else if trimmed.starts_with("smpsVcUnusedBits") {
        } else if trimmed.starts_with("smpsVcDetune") {
            detune = parse_4_args(trimmed, "smpsVcDetune");
        } else if trimmed.starts_with("smpsVcCoarseFreq") {
            mul = parse_4_args(trimmed, "smpsVcCoarseFreq");
        } else if trimmed.starts_with("smpsVcRateScale") {
            rs = parse_4_args(trimmed, "smpsVcRateScale");
        } else if trimmed.starts_with("smpsVcAttackRate") {
            ar = parse_4_args(trimmed, "smpsVcAttackRate");
        } else if trimmed.starts_with("smpsVcAmpMod") {
            am = parse_4_args(trimmed, "smpsVcAmpMod");
        } else if trimmed.starts_with("smpsVcDecayRate1") {
            d1r = parse_4_args(trimmed, "smpsVcDecayRate1");
        } else if trimmed.starts_with("smpsVcDecayRate2") {
            d2r = parse_4_args(trimmed, "smpsVcDecayRate2");
        } else if trimmed.starts_with("smpsVcDecayLevel") {
            sl = parse_4_args(trimmed, "smpsVcDecayLevel");
        } else if trimmed.starts_with("smpsVcReleaseRate") {
            rr = parse_4_args(trimmed, "smpsVcReleaseRate");
        } else if trimmed.starts_with("smpsVcTotalLevel") {
            tl = parse_4_args(trimmed, "smpsVcTotalLevel");
        }
    }

    if in_voice {
        voices.push(build_voice_bytes(alg, fb, &detune, &mul, &rs, &ar, &am, &d1r, &d2r, &sl, &rr, &tl));
    }

    Ok(voices)
}

pub fn build_note_table() -> HashMap<String, u8> {
    let mut map = HashMap::new();
    map.insert("nRst".into(), 0x80);
    let nbb6 = 0x81u8 + 6 * 12 + 10; // nBb6 = $D3
    map.insert("nMaxPSG".into(), nbb6 - 12);
    map.insert("nMaxPSG1".into(), nbb6);
    map.insert("nMaxPSG2".into(), nbb6 + 1); // nB6
    let note_names = [
        "C", "Cs", "D", "Eb", "E", "F", "Fs", "G", "Ab", "A", "Bb", "B",
    ];
    let aliases = [
        ("Db", "Cs"),
        ("Ds", "Eb"),
        ("Es", "F"),
        ("Fb", "E"),
        ("Gb", "Fs"),
        ("Gs", "Ab"),
        ("As", "Bb"),
        ("Bs", "C"),
        ("Cb", "B"),
    ];
    for octave in 0..=7u8 {
        for (semi, name) in note_names.iter().enumerate() {
            let byte = 0x81u8.wrapping_add(octave * 12 + semi as u8);
            if byte > 0x80 {
                map.insert(format!("n{name}{octave}"), byte);
            }
        }
    }
    for octave in 0..=7u8 {
        for &(alias, canonical) in &aliases {
            if alias == "Bs" {
                let target_octave = octave + 1;
                if let Some(&val) = map.get(&format!("n{canonical}{target_octave}")) {
                    map.insert(format!("n{alias}{octave}"), val);
                }
            } else if alias == "Cb" {
                if octave > 0 {
                    if let Some(&val) = map.get(&format!("n{canonical}{}", octave - 1)) {
                        map.insert(format!("n{alias}{octave}"), val);
                    }
                }
            } else if let Some(&val) = map.get(&format!("n{canonical}{octave}")) {
                map.insert(format!("n{alias}{octave}"), val);
            }
        }
    }
    map
}

pub fn build_dac_table() -> HashMap<String, u8> {
    let mut map = HashMap::new();
    let dac_names: &[(&str, u8)] = &[
        ("dSnareS3", 0x81),
        ("dHighTom", 0x82),
        ("dMidTomS3", 0x83),
        ("dLowTomS3", 0x84),
        ("dFloorTomS3", 0x85),
        ("dKickS3", 0x86),
        ("dMuffledSnare", 0x87),
        ("dCrashCymbal", 0x88),
        ("dRideCymbal", 0x89),
        ("dLowMetalHit", 0x8A),
        ("dMetalHit", 0x8B),
        ("dHighMetalHit", 0x8C),
        ("dHigherMetalHit", 0x8D),
        ("dMidMetalHit", 0x8E),
        ("dClapS3", 0x8F),
        ("dElectricHighTom", 0x90),
        ("dElectricMidTom", 0x91),
        ("dElectricLowTom", 0x92),
        ("dElectricFloorTom", 0x93),
        ("dTightSnare", 0x94),
        ("dMidpitchSnare", 0x95),
        ("dLooseSnare", 0x96),
        ("dLooserSnare", 0x97),
        ("dHiTimpaniS3", 0x98),
        ("dLowTimpaniS3", 0x99),
        ("dMidTimpaniS3", 0x9A),
        ("dQuickLooseSnare", 0x9B),
        ("dClick", 0x9C),
        ("dPowerKick", 0x9D),
        ("dQuickGlassCrash", 0x9E),
        ("dGlassCrashSnare", 0x9F),
        ("dGlassCrash", 0xA0),
        ("dGlassCrashKick", 0xA1),
        ("dQuietGlassCrash", 0xA2),
        ("dOddSnareKick", 0xA3),
        ("dKickExtraBass", 0xA4),
        ("dComeOn", 0xA5),
        ("dDanceSnare", 0xA6),
        ("dLooseKick", 0xA7),
        ("dModLooseKick", 0xA8),
        ("dWoo", 0xA9),
        ("dGo", 0xAA),
        ("dSnareGo", 0xAB),
        ("dPowerTom", 0xAC),
        ("dHiWoodBlock", 0xAD),
        ("dLowWoodBlock", 0xAE),
        ("dHiHitDrum", 0xAF),
        ("dLowHitDrum", 0xB0),
        ("dMetalCrashHit", 0xB1),
        ("dEchoedClapHit_S3", 0xB2),
        ("dLowerEchoedClapHit_S3", 0xB3),
        ("dHipHopHitKick", 0xB4),
        ("dHipHopHitPowerKick", 0xB5),
        ("dBassHey", 0xB6),
        ("dDanceStyleKick", 0xB7),
        ("dHipHopHitKick2", 0xB8),
        ("dHipHopHitKick3", 0xB9),
        ("dReverseFadingWind", 0xBA),
        ("dScratchS3", 0xBB),
        ("dLooseSnareNoise", 0xBC),
        ("dPowerKick2", 0xBD),
        ("dCrashingNoiseWoo", 0xBE),
        ("dQuickHit", 0xBF),
        ("dKickHey", 0xC0),
        ("dPowerKickHit", 0xC1),
        ("dLowPowerKickHit", 0xC2),
        ("dLowerPowerKickHit", 0xC3),
        ("dLowestPowerKickHit", 0xC4),
    ];
    for &(name, val) in dac_names {
        map.insert(name.into(), val);
    }
    map
}

pub fn build_pan_table() -> HashMap<String, u8> {
    let mut map = HashMap::new();
    map.insert("panNone".into(), 0x00);
    map.insert("panRight".into(), 0x40);
    map.insert("panLeft".into(), 0x80);
    map.insert("panCenter".into(), 0xC0);
    map.insert("panCentre".into(), 0xC0);
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_table_nrst() {
        let table = build_note_table();
        assert_eq!(table["nRst"], 0x80);
    }

    #[test]
    fn test_note_table_nc0_is_81() {
        let table = build_note_table();
        assert_eq!(table["nC0"], 0x81);
    }

    #[test]
    fn test_note_table_aliases() {
        let table = build_note_table();
        assert_eq!(table["nDb0"], table["nCs0"]);
        assert_eq!(table["nEb4"], table["nDs4"]);
        assert_eq!(table["nFs3"], table["nGb3"]);
    }

    #[test]
    fn test_dac_table() {
        let table = build_dac_table();
        assert_eq!(table["dKickS3"], 0x86);
        assert_eq!(table["dSnareS3"], 0x81);
    }

    #[test]
    fn test_pan_table() {
        let table = build_pan_table();
        assert_eq!(table["panCenter"], 0xC0);
        assert_eq!(table["panLeft"], 0x80);
        assert_eq!(table["panRight"], 0x40);
    }

    #[test]
    fn test_parse_dez1_header() {
        let source = include_str!("../../test_data/Mus - DEZ1.asm");
        let result = parse_smps(source).unwrap();
        assert_eq!(result.fm_count, 6);
        assert_eq!(result.psg_count, 3);
        assert_eq!(result.tempo_divider, 1);
        assert_eq!(result.tempo_modifier, 0x08);
        assert_eq!(result.channels.len(), 9); // 1 DAC + 5 FM + 3 PSG
    }

    #[test]
    fn test_parse_dez1_voices() {
        let source = include_str!("../../test_data/Mus - DEZ1.asm");
        let result = parse_smps(source).unwrap();
        assert_eq!(result.voices.len(), 4);
        assert!(matches!(result.voice_ref, VoiceRef::Inline(_)));
    }

    #[test]
    fn test_parse_dez1_fm1_has_notes() {
        let source = include_str!("../../test_data/Mus - DEZ1.asm");
        let result = parse_smps(source).unwrap();
        let fm1 = &result.channels[1]; // 0=DAC, 1=FM1
        assert_eq!(fm1.kind, SmpsChannelKind::Fm);
        assert!(!fm1.events.is_empty());
        assert!(fm1.events.iter().any(|e| matches!(e, SmpsEvent::SetVoice(0))));
    }

    #[test]
    fn test_parse_uvb_song() {
        let source = include_str!("../../test_data/AIZ1.asm");
        let result = parse_smps(source).unwrap();
        assert!(matches!(result.voice_ref, VoiceRef::Uvb));
        assert_eq!(result.voices.len(), 0);
    }

    #[test]
    fn test_parse_loops_are_flattened() {
        let source = include_str!("../../test_data/Mus - DEZ1.asm");
        let result = parse_smps(source).unwrap();
        for ch in &result.channels {
            for ev in &ch.events {
                match ev {
                    SmpsEvent::Note { .. }
                    | SmpsEvent::Rest { .. }
                    | SmpsEvent::SetVoice(_)
                    | SmpsEvent::SetPan(_)
                    | SmpsEvent::Transpose(_)
                    | SmpsEvent::Detune(_)
                    | SmpsEvent::Tie
                    | SmpsEvent::Stop
                    | SmpsEvent::SetModulation { .. }
                    | SmpsEvent::ModOff
                    | SmpsEvent::VolumeChange(_)
                    | SmpsEvent::Unsupported { .. } => {}
                }
            }
        }
    }

    #[test]
    fn test_parse_psg_channels_stop() {
        let source = include_str!("../../test_data/Mus - DEZ1.asm");
        let result = parse_smps(source).unwrap();
        for ch in &result.channels {
            if ch.kind == SmpsChannelKind::Psg {
                assert!(ch.events.iter().any(|e| matches!(e, SmpsEvent::Stop)));
            }
        }
    }

    #[test]
    fn test_parse_dez1_voice_bytes() {
        let source = include_str!("../../test_data/Mus - DEZ1.asm");
        let result = parse_smps(source).unwrap();
        let v0 = &result.voices[0];
        assert_eq!(v0[24], (2 << 3) | 0); // fb=2, alg=0
        assert_eq!(v0[0], (4 << 4) | 1); // op1: dt=4, mul=1
        assert_eq!(v0[1], (6 << 4) | 4); // op2: dt=6, mul=4
        assert_eq!(v0[2], (5 << 4) | 0); // op3: dt=5, mul=0
        assert_eq!(v0[3], (4 << 4) | 5); // op4: dt=4, mul=5
    }

    #[test]
    fn test_parse_aiz1_has_calls_inlined() {
        let source = include_str!("../../test_data/AIZ1.asm");
        let result = parse_smps(source).unwrap();
        let dac = &result.channels[0];
        assert_eq!(dac.kind, SmpsChannelKind::Dac);
        assert!(dac.events.len() > 10);
    }

    #[test]
    fn test_aiz1_alter_note_parsed_as_detune() {
        let source = include_str!("../../test_data/AIZ1.asm");
        let result = parse_smps(&source).unwrap();
        let fm2 = &result.channels[2];
        assert_eq!(fm2.kind, SmpsChannelKind::Fm);
        let detune_events: Vec<_> = fm2.events.iter()
            .filter_map(|e| match e {
                SmpsEvent::Detune(v) => Some(*v),
                _ => None,
            })
            .collect();
        assert!(!detune_events.is_empty(),
            "FM2 should have Detune events from smpsAlterNote");
        assert!(detune_events.contains(&-5i8),
            "FM2 should have smpsAlterNote $FB = -5, got {:?}", detune_events);
    }

    #[test]
    fn test_aiz1_all_fm_channels_start_at_tick_zero() {
        let source = include_str!("../../test_data/AIZ1.asm");
        let result = parse_smps(&source).unwrap();
        for ch in &result.channels {
            if ch.kind != SmpsChannelKind::Fm { continue; }
            let first_note = ch.events.iter().find_map(|e| match e {
                SmpsEvent::Note { .. } => Some(true),
                SmpsEvent::Rest { .. } => Some(false),
                _ => None,
            });
            assert_eq!(first_note, Some(true),
                "{}: first timed event should be a note (not rest)", ch.label);
        }
    }
}

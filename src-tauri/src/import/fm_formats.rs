use uuid::Uuid;
use crate::model::instrument::*;

// Seraph operators[0..3] map to hardware via PACKED_OP_SLOTS = [0x0C, 0x04, 0x08, 0x00]
// = Yamaha [Op4, Op3, Op2, Op1] (reverse of Yamaha documentation order).
//
// TFI/VGI/Y12 store operators in Yamaha logical order: Op1, Op2, Op3, Op4
// GYB stores operators in hardware register order: Op1, Op3, Op2, Op4

const YAMAHA_TO_DAW: [usize; 4] = [3, 2, 1, 0];
const HW_SLOT_TO_DAW: [usize; 4] = [3, 1, 2, 0];

pub struct FmFileImportResult {
    pub instruments: Vec<FmInstrument>,
    pub format: String,
}

pub fn import_fm_file(data: &[u8], filename: &str) -> Result<FmFileImportResult, String> {
    let ext = filename.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "tfi" => parse_tfi(data, filename),
        "vgi" => parse_vgi(data, filename),
        "y12" => parse_y12(data, filename),
        "gyb" => parse_gyb(data, filename),
        _ => Err(format!("unsupported FM instrument format: .{ext}")),
    }
}

fn stem_name(filename: &str) -> String {
    std::path::Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("FM Import")
        .to_string()
}

fn parse_tfi(data: &[u8], filename: &str) -> Result<FmFileImportResult, String> {
    if data.len() < 42 {
        return Err(format!("TFI file too short: {} bytes (expected 42)", data.len()));
    }

    let algorithm = data[0] & 0x07;
    let feedback = data[1] & 0x07;
    let mut operators = [FmOperator::default(); 4];

    for i in 0..4 {
        let base = 2 + i * 10;
        let dst = YAMAHA_TO_DAW[i];
        operators[dst].multiple = data[base] & 0x0F;
        operators[dst].detune = data[base + 1] & 0x07;
        operators[dst].total_level = data[base + 2] & 0x7F;
        operators[dst].rate_scale = data[base + 3] & 0x03;
        operators[dst].attack_rate = data[base + 4] & 0x1F;
        operators[dst].d1r = data[base + 5] & 0x1F;
        operators[dst].d2r = data[base + 6] & 0x1F;
        operators[dst].sustain_level = data[base + 7] & 0x0F;
        operators[dst].release_rate = data[base + 8] & 0x0F;
        operators[dst].ssg_eg = data[base + 9] & 0x0F;
    }

    Ok(FmFileImportResult {
        instruments: vec![FmInstrument {
            id: Uuid::new_v4(),
            name: stem_name(filename),
            algorithm,
            feedback,
            operators,
            metadata: InstrumentMetadata {
                category: "TFI Import".into(),
                ..InstrumentMetadata::default()
            },
        }],
        format: "TFI".into(),
    })
}

fn parse_vgi(data: &[u8], filename: &str) -> Result<FmFileImportResult, String> {
    if data.len() < 43 {
        return Err(format!("VGI file too short: {} bytes (expected 43)", data.len()));
    }

    let algorithm = data[0] & 0x07;
    let feedback = data[1] & 0x07;

    let mut operators = [FmOperator::default(); 4];
    for i in 0..4 {
        let base = 3 + i * 10;
        let dst = YAMAHA_TO_DAW[i];
        operators[dst].multiple = data[base] & 0x0F;
        operators[dst].detune = data[base + 1] & 0x07;
        operators[dst].total_level = data[base + 2] & 0x7F;
        operators[dst].rate_scale = data[base + 3] & 0x03;
        operators[dst].attack_rate = data[base + 4] & 0x1F;
        operators[dst].d1r = data[base + 5] & 0x1F;
        operators[dst].d2r = data[base + 6] & 0x1F;
        operators[dst].sustain_level = data[base + 7] & 0x0F;
        operators[dst].release_rate = data[base + 8] & 0x0F;
        operators[dst].ssg_eg = data[base + 9] & 0x0F;
    }

    Ok(FmFileImportResult {
        instruments: vec![FmInstrument {
            id: Uuid::new_v4(),
            name: stem_name(filename),
            algorithm,
            feedback,
            operators,
            metadata: InstrumentMetadata {
                category: "VGI Import".into(),
                ..InstrumentMetadata::default()
            },
        }],
        format: "VGI".into(),
    })
}

fn parse_y12(data: &[u8], filename: &str) -> Result<FmFileImportResult, String> {
    if data.len() < 128 {
        return Err(format!("Y12 file too short: {} bytes (expected 128)", data.len()));
    }

    let mut operators = [FmOperator::default(); 4];
    for i in 0..4 {
        let base = i * 16;
        let dst = YAMAHA_TO_DAW[i];
        operators[dst].multiple = data[base] & 0x0F;
        operators[dst].detune = data[base + 1] & 0x07;
        operators[dst].total_level = data[base + 2] & 0x7F;
        operators[dst].rate_scale = data[base + 3] & 0x03;
        operators[dst].attack_rate = data[base + 4] & 0x1F;
        operators[dst].d1r = data[base + 5] & 0x1F;
        operators[dst].d2r = data[base + 6] & 0x1F;
        operators[dst].sustain_level = data[base + 7] & 0x0F;
        operators[dst].release_rate = data[base + 8] & 0x0F;
        operators[dst].ssg_eg = data[base + 9] & 0x0F;
    }

    let algorithm = data[64] & 0x07;
    let feedback = data[65] & 0x07;

    Ok(FmFileImportResult {
        instruments: vec![FmInstrument {
            id: Uuid::new_v4(),
            name: stem_name(filename),
            algorithm,
            feedback,
            operators,
            metadata: InstrumentMetadata {
                category: "Y12 Import".into(),
                ..InstrumentMetadata::default()
            },
        }],
        format: "Y12".into(),
    })
}

fn parse_gyb(data: &[u8], _filename: &str) -> Result<FmFileImportResult, String> {
    if data.len() < 5 {
        return Err("GYB file too short".into());
    }
    if data[0] != 0x1A || data[1] != 0x0C {
        return Err("invalid GYB signature".into());
    }

    let version = data[2];
    if version < 1 || version > 2 {
        return Err(format!("unsupported GYB version {version}"));
    }

    let melody_count = data[3] as usize;
    let drum_count = data[4] as usize;
    let total = melody_count + drum_count;

    let inst_data_start = match version {
        1 => 5,
        2 => 262,
        _ => unreachable!(),
    };

    if inst_data_start > data.len() {
        return Err("GYB file truncated before instrument data".into());
    }

    let mut instruments = Vec::new();
    let mut offset = inst_data_start;

    for idx in 0..total {
        if offset + 32 > data.len() {
            break;
        }

        let reg = &data[offset..offset + 25];
        let mut operators = [FmOperator::default(); 4];

        // GYB register data: register-dump order with hardware slot ordering
        for i in 0..4 {
            let dst = HW_SLOT_TO_DAW[i];
            operators[dst].detune = reg[i] >> 4;
            operators[dst].multiple = reg[i] & 0x0F;
            operators[dst].total_level = reg[4 + i] & 0x7F;
            operators[dst].rate_scale = reg[8 + i] >> 6;
            operators[dst].attack_rate = reg[8 + i] & 0x1F;
            operators[dst].amp_mod = reg[12 + i] & 0x80 != 0;
            operators[dst].d1r = reg[12 + i] & 0x1F;
            operators[dst].d2r = reg[16 + i] & 0x1F;
            operators[dst].sustain_level = reg[20 + i] >> 4;
            operators[dst].release_rate = reg[20 + i] & 0x0F;
        }

        let algorithm = reg[24] & 0x07;
        let feedback = (reg[24] >> 3) & 0x07;

        offset += 32;

        let name = if version >= 2 && offset < data.len() {
            let name_len = data[offset] as usize;
            offset += 1;
            if offset + name_len <= data.len() {
                let s = String::from_utf8_lossy(&data[offset..offset + name_len]).to_string();
                offset += name_len;
                if s.is_empty() { format!("GYB {idx}") } else { s }
            } else {
                format!("GYB {idx}")
            }
        } else {
            format!("GYB {idx}")
        };

        let category = if idx < melody_count { "GYB Melody" } else { "GYB Drum" };

        instruments.push(FmInstrument {
            id: Uuid::new_v4(),
            name,
            algorithm,
            feedback,
            operators,
            metadata: InstrumentMetadata {
                category: category.into(),
                ..InstrumentMetadata::default()
            },
        });
    }

    if instruments.is_empty() {
        return Err("GYB file contains no instruments".into());
    }

    Ok(FmFileImportResult {
        instruments,
        format: format!("GYB v{version}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_tfi(alg: u8, fb: u8, ops: [[u8; 10]; 4]) -> Vec<u8> {
        let mut data = vec![alg, fb];
        for op in &ops {
            data.extend_from_slice(op);
        }
        data
    }

    #[test]
    fn test_tfi_parse_basic() {
        let ops = [
            [1, 3, 20, 0, 31, 5, 3, 2, 8, 0],
            [2, 5, 30, 1, 28, 7, 5, 4, 10, 0],
            [3, 0, 40, 2, 25, 9, 7, 6, 12, 0],
            [4, 7, 50, 3, 22, 11, 9, 8, 14, 0],
        ];
        let data = build_tfi(4, 5, ops);
        let result = import_fm_file(&data, "test.tfi").unwrap();
        assert_eq!(result.format, "TFI");
        assert_eq!(result.instruments.len(), 1);

        let inst = &result.instruments[0];
        assert_eq!(inst.algorithm, 4);
        assert_eq!(inst.feedback, 5);

        // TFI Op1 (Yamaha) → operators[3]
        assert_eq!(inst.operators[3].multiple, 1);
        assert_eq!(inst.operators[3].detune, 3);
        assert_eq!(inst.operators[3].total_level, 20);

        // TFI Op4 (Yamaha) → operators[0]
        assert_eq!(inst.operators[0].multiple, 4);
        assert_eq!(inst.operators[0].detune, 7);
        assert_eq!(inst.operators[0].total_level, 50);
    }

    #[test]
    fn test_tfi_too_short() {
        assert!(import_fm_file(&[0u8; 41], "test.tfi").is_err());
    }

    #[test]
    fn test_vgi_parse_basic() {
        let mut data = vec![2u8, 3, 0x00]; // alg=2, fb=3, fms/ams=0
        for i in 0..4 {
            data.extend_from_slice(&[i + 1, 0, 10 * (i + 1), 0, 31, 5, 3, 2, 8, 0]);
        }
        let result = import_fm_file(&data, "test.vgi").unwrap();
        assert_eq!(result.format, "VGI");
        assert_eq!(result.instruments[0].algorithm, 2);
        assert_eq!(result.instruments[0].feedback, 3);
    }

    #[test]
    fn test_y12_parse_basic() {
        let mut data = vec![0u8; 128];
        // Op1 at offset 0: MUL=2, DT=1
        data[0] = 2;
        data[1] = 1;
        data[2] = 25; // TL
        // Algorithm and feedback at offset 64-65
        data[64] = 5;
        data[65] = 6;

        let result = import_fm_file(&data, "test.y12").unwrap();
        assert_eq!(result.format, "Y12");
        let inst = &result.instruments[0];
        assert_eq!(inst.algorithm, 5);
        assert_eq!(inst.feedback, 6);
        // Y12 Op1 (Yamaha) → operators[3]
        assert_eq!(inst.operators[3].multiple, 2);
        assert_eq!(inst.operators[3].detune, 1);
        assert_eq!(inst.operators[3].total_level, 25);
    }

    #[test]
    fn test_gyb_v1_basic() {
        let mut data = vec![0x1A, 0x0C, 0x01]; // signature + v1
        data.push(1); // 1 melody
        data.push(0); // 0 drums

        // 32-byte instrument block
        let mut inst_block = vec![0u8; 32];
        // DT|MUL for 4 ops in hw slot order
        inst_block[0] = (3 << 4) | 1; // Op1: DT=3, MUL=1
        inst_block[1] = (0 << 4) | 4; // Op3: DT=0, MUL=4
        inst_block[2] = (5 << 4) | 2; // Op2: DT=5, MUL=2
        inst_block[3] = (7 << 4) | 8; // Op4: DT=7, MUL=8
        // TL
        inst_block[4] = 10; // Op1 TL
        inst_block[5] = 20; // Op3 TL
        inst_block[6] = 30; // Op2 TL
        inst_block[7] = 40; // Op4 TL
        // RS|AR
        inst_block[8] = (0 << 6) | 31;
        inst_block[9] = (0 << 6) | 31;
        inst_block[10] = (0 << 6) | 31;
        inst_block[11] = (0 << 6) | 31;
        // AM|D1R
        inst_block[12] = 5;
        inst_block[13] = 5;
        inst_block[14] = 5;
        inst_block[15] = 5;
        // D2R
        inst_block[16] = 3;
        inst_block[17] = 3;
        inst_block[18] = 3;
        inst_block[19] = 3;
        // SL|RR
        inst_block[20] = (2 << 4) | 8;
        inst_block[21] = (2 << 4) | 8;
        inst_block[22] = (2 << 4) | 8;
        inst_block[23] = (2 << 4) | 8;
        // FB|ALG
        inst_block[24] = (5 << 3) | 4;

        data.extend_from_slice(&inst_block);

        let result = import_fm_file(&data, "test.gyb").unwrap();
        assert_eq!(result.format, "GYB v1");
        assert_eq!(result.instruments.len(), 1);

        let inst = &result.instruments[0];
        assert_eq!(inst.algorithm, 4);
        assert_eq!(inst.feedback, 5);

        // HW slot 0 (Op1) → operators[3]
        assert_eq!(inst.operators[3].detune, 3);
        assert_eq!(inst.operators[3].multiple, 1);
        assert_eq!(inst.operators[3].total_level, 10);

        // HW slot 3 (Op4) → operators[0]
        assert_eq!(inst.operators[0].detune, 7);
        assert_eq!(inst.operators[0].multiple, 8);
        assert_eq!(inst.operators[0].total_level, 40);

        // HW slot 1 (Op3) → operators[1]
        assert_eq!(inst.operators[1].detune, 0);
        assert_eq!(inst.operators[1].multiple, 4);
        assert_eq!(inst.operators[1].total_level, 20);

        // HW slot 2 (Op2) → operators[2]
        assert_eq!(inst.operators[2].detune, 5);
        assert_eq!(inst.operators[2].multiple, 2);
        assert_eq!(inst.operators[2].total_level, 30);
    }

    #[test]
    fn test_gyb_invalid_signature() {
        assert!(import_fm_file(&[0x00, 0x00, 0x01, 1, 0], "test.gyb").is_err());
    }

    #[test]
    fn test_unsupported_format() {
        assert!(import_fm_file(&[0u8; 100], "test.xyz").is_err());
    }
}

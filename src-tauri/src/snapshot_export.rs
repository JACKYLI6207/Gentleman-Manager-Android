//! 與 PC `gentleman-manager.snapshot.v1` 匯出格式一致（`JSON.stringify(export, null, 2)`）。

use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::{
    snapshot_catalog::SnapshotMeta,
    types::ComicInSearch,
};

pub const SNAPSHOT_FORMAT_V1: &str = "gentleman-manager.snapshot.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotExportFileV1 {
    pub format: String,
    pub exported_at: String,
    pub snapshot: SnapshotExportSnapshotV1,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotExportSnapshotV1 {
    pub kind: String,
    pub meta: SnapshotMeta,
    #[serde(default)]
    pub comics: Vec<ComicInSearch>,
}

pub fn parse_snapshot_export_bytes(bytes: &[u8]) -> anyhow::Result<SnapshotExportFileV1> {
    let file: SnapshotExportFileV1 =
        serde_json::from_slice(bytes).context("解析快照 JSON 失敗（檔案可能損壞或格式不符）")?;
    if file.format != SNAPSHOT_FORMAT_V1 {
        anyhow::bail!("快照格式不支援：{}", file.format);
    }
    if file.snapshot.kind != "global" {
        anyhow::bail!("僅支援 global 分類快照");
    }
    Ok(file)
}

/// 完整寫入（與 PC 相同：2 空格縮排、camelCase、檔尾換行）。
pub fn write_global_snapshot_export(
    path: &Path,
    meta: &SnapshotMeta,
    comics: &[ComicInSearch],
    exported_at: &str,
) -> anyhow::Result<()> {
    let file = SnapshotExportFileV1 {
        format: SNAPSHOT_FORMAT_V1.to_string(),
        exported_at: exported_at.to_string(),
        snapshot: SnapshotExportSnapshotV1 {
            kind: "global".to_string(),
            meta: meta.clone(),
            comics: comics.to_vec(),
        },
    };
    let json =
        serde_json::to_string_pretty(&file).context("序列化快照 JSON 失敗")?;
    let mut content = json;
    content.push('\n');
    std::fs::write(path, content).with_context(|| format!("寫入快照`{}`失敗", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot_catalog::SnapshotMeta;

    #[test]
    fn pretty_export_roundtrip() {
        let meta = SnapshotMeta {
            id: "test-id".to_string(),
            saved_at: "2026-05-29T00:00:00.000Z".to_string(),
            total_count: 1,
            total_pages: 10,
            scan_completion_percent: 100,
            scan_direction: Some("tailToHead".to_string()),
            scan_completed_pages: 10,
            scan_completed_page_ranges: vec![crate::snapshot_catalog::SnapshotPageRange {
                start: 1,
                end: 10,
            }],
            scan_target_kind: Some("category".to_string()),
            scan_target_cate_id: Some(20),
            scan_target_label: Some("漢化".to_string()),
        };
        let dir = std::env::temp_dir().join("gm-snapshot-export-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.gm-snapshot.json");
        write_global_snapshot_export(&path, &meta, &[], "2026-05-29T00:00:00.000Z").unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let parsed = parse_snapshot_export_bytes(&bytes).unwrap();
        assert_eq!(parsed.format, SNAPSHOT_FORMAT_V1);
        assert_eq!(parsed.snapshot.meta.id, "test-id");
        let _: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}

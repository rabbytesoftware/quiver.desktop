use serde::Deserialize;

/// `GET` and `POST /v0/system/path`. `files` is always an array on a real daemon.
#[derive(Debug, Clone, Deserialize)]
pub struct PathStatusDto {
    pub bin_dir: String,
    #[serde(default)]
    pub on_path: Option<bool>,
    #[serde(default)]
    pub configured: Option<bool>,
    #[serde(default)]
    pub files: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathStatus {
    pub bin_dir: String,
    /// Whether the daemon's own `PATH` already has the directory.
    pub on_path: bool,
    /// Whether the directory is set to stay on `PATH` in new terminals.
    pub configured: bool,
    /// Where that setting is checked and written: shell files, or the user `Path` on Windows.
    pub files: Vec<String>,
}

pub fn to_path_status(dto: PathStatusDto) -> PathStatus {
    PathStatus {
        bin_dir: dto.bin_dir,
        on_path: dto.on_path == Some(true),
        configured: dto.configured == Some(true),
        files: dto.files.unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dto(json: serde_json::Value) -> PathStatusDto {
        serde_json::from_value(json).unwrap()
    }

    #[test]
    fn maps_the_wire_fields() {
        let s = to_path_status(dto(serde_json::json!(
            {"bin_dir": "/home/u/.quiver/bin", "on_path": true, "configured": true, "files": ["/home/u/.zshrc"]}
        )));
        assert_eq!(
            s,
            PathStatus {
                bin_dir: "/home/u/.quiver/bin".into(),
                on_path: true,
                configured: true,
                files: vec!["/home/u/.zshrc".into()]
            }
        );
    }

    #[test]
    fn reads_a_missing_files_list_as_empty() {
        assert!(to_path_status(dto(serde_json::json!({"bin_dir": "/b", "files": null}))).files.is_empty());
    }

    #[test]
    fn reads_anything_but_true_as_false() {
        let s = to_path_status(dto(serde_json::json!({"bin_dir": "/b"})));
        assert!(!s.on_path && !s.configured);
    }
}

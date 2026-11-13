use crate::domain::ConnectionConfig;

pub fn filter_connections(
    connections: &[ConnectionConfig],
    query: &str,
) -> Vec<ConnectionConfig> {
    let q = query.trim().to_lowercase();
    if q.is_empty() {
        return connections.to_vec();
    }
    connections
        .iter()
        .filter(|c| c.name.to_lowercase().contains(&q))
        .cloned()
        .collect()
}

/// Wraps `index` into `[0, length)`, matching arrow-key navigation over a circular list. `0` for an
/// empty list, since there is nothing to index.
pub fn clamp_index(
    index: i64,
    length: usize,
) -> usize {
    let Ok(len) = i64::try_from(length) else {
        return 0;
    };
    if len == 0 {
        return 0;
    }
    usize::try_from(index.rem_euclid(len)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{local_connection, ConnectionKind};

    fn remote(name: &str) -> ConnectionConfig {
        ConnectionConfig {
            id: name.into(),
            name: name.into(),
            kind: ConnectionKind::Remote,
            url: Some(format!("https://{name}")),
            api_version: "v0".into(),
        }
    }

    #[test]
    fn a_blank_query_keeps_everything_and_a_query_matches_names_case_insensitively() {
        let all = [
            local_connection(),
            remote("Home Server"),
            remote("Work box"),
        ];
        assert_eq!(filter_connections(&all, "   ").len(), 3);
        let found = filter_connections(&all, " SERVER ");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "Home Server");
        assert!(filter_connections(&all, "nope").is_empty());
    }

    #[test]
    fn arrow_navigation_wraps_in_both_directions() {
        assert_eq!(clamp_index(-1, 3), 2);
        assert_eq!(clamp_index(3, 3), 0);
        assert_eq!(clamp_index(1, 3), 1);
        assert_eq!(clamp_index(5, 0), 0);
        assert_eq!(clamp_index(-7, 3), 2);
    }
}

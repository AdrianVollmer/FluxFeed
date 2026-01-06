use crate::domain::models::{Feed, FlatTreeItem, Group, GroupNode};
use crate::infrastructure::repository;
use sqlx::SqlitePool;
use std::collections::HashMap;

/// Build a hierarchical tree from flat lists of groups and feeds
pub fn build_group_tree(groups: Vec<Group>, feeds: Vec<Feed>) -> (Vec<GroupNode>, Vec<Feed>) {
    // Group feeds by group_id
    let mut feeds_by_group: HashMap<Option<i64>, Vec<Feed>> = HashMap::new();
    for feed in feeds {
        feeds_by_group.entry(feed.group_id).or_default().push(feed);
    }

    // Create a map of group_id -> GroupNode (initially with empty children)
    let mut group_map: HashMap<i64, GroupNode> = HashMap::new();
    for group in &groups {
        group_map.insert(
            group.id,
            GroupNode {
                group: group.clone(),
                children: Vec::new(),
                feeds: feeds_by_group.remove(&Some(group.id)).unwrap_or_default(),
            },
        );
    }

    // Build tree by attaching children to parents (process in reverse to handle nested)
    // We need to collect child IDs first to avoid borrow issues
    let child_parent_pairs: Vec<(i64, i64)> = groups
        .iter()
        .filter_map(|g| g.parent_id.map(|pid| (g.id, pid)))
        .collect();

    for (child_id, parent_id) in child_parent_pairs {
        if let Some(child) = group_map.remove(&child_id) {
            if let Some(parent) = group_map.get_mut(&parent_id) {
                parent.children.push(child);
            } else {
                // Parent not found, put child back (orphan)
                group_map.insert(child_id, child);
            }
        }
    }

    // Collect root groups (those with no parent)
    let mut root_groups: Vec<GroupNode> = group_map.into_values().collect();

    // Sort root groups by position, then name
    root_groups.sort_by(|a, b| {
        a.group
            .position
            .cmp(&b.group.position)
            .then_with(|| a.group.name.cmp(&b.group.name))
    });

    // Sort children recursively
    fn sort_children(node: &mut GroupNode) {
        node.children.sort_by(|a, b| {
            a.group
                .position
                .cmp(&b.group.position)
                .then_with(|| a.group.name.cmp(&b.group.name))
        });
        node.feeds.sort_by(|a, b| a.title.cmp(&b.title));
        for child in &mut node.children {
            sort_children(child);
        }
    }

    for node in &mut root_groups {
        sort_children(node);
    }

    // Get ungrouped feeds (those with group_id = None)
    let mut ungrouped = feeds_by_group.remove(&None).unwrap_or_default();
    ungrouped.sort_by(|a, b| a.title.cmp(&b.title));

    (root_groups, ungrouped)
}

/// Flatten a group tree into a list of items with depth information
pub fn flatten_group_tree(tree: &[GroupNode]) -> Vec<FlatTreeItem> {
    let mut items = Vec::new();

    fn flatten_node(node: &GroupNode, depth: usize, items: &mut Vec<FlatTreeItem>) {
        // Add the group itself
        items.push(FlatTreeItem::Group {
            group: node.group.clone(),
            depth,
        });

        // Add feeds in this group
        for feed in &node.feeds {
            items.push(FlatTreeItem::Feed {
                feed: feed.clone(),
                depth: depth + 1,
            });
        }

        // Recurse into children
        for child in &node.children {
            flatten_node(child, depth + 1, items);
        }
    }

    for node in tree {
        flatten_node(node, 0, &mut items);
    }

    items
}

/// Resolve selected groups and feeds to a list of feed IDs
/// Groups are expanded recursively to include all descendant feeds
pub async fn resolve_selection_to_feed_ids(
    pool: &SqlitePool,
    selected_group_ids: &[i64],
    selected_feed_ids: &[i64],
) -> Result<Vec<i64>, sqlx::Error> {
    let mut all_feed_ids: Vec<i64> = selected_feed_ids.to_vec();

    // For each selected group, get all descendant feeds
    for &group_id in selected_group_ids {
        let feed_ids = repository::get_feed_ids_in_group_recursive(pool, group_id).await?;
        all_feed_ids.extend(feed_ids);
    }

    // Deduplicate
    all_feed_ids.sort();
    all_feed_ids.dedup();

    Ok(all_feed_ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_group(id: i64, name: &str, parent_id: Option<i64>, position: i64) -> Group {
        Group {
            id,
            name: name.to_string(),
            parent_id,
            position,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn make_feed(id: i64, title: &str, group_id: Option<i64>) -> Feed {
        Feed {
            id,
            url: format!("https://example.com/feed{}", id),
            title: title.to_string(),
            description: None,
            site_url: None,
            group_id,
            last_fetched_at: None,
            last_modified: None,
            etag: None,
            fetch_interval_minutes: 30,
            color: "#3B82F6".to_string(),
            fetch_frequency: "adaptive".to_string(),
            ttl_minutes: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_build_group_tree_empty() {
        let (tree, ungrouped) = build_group_tree(vec![], vec![]);
        assert!(tree.is_empty());
        assert!(ungrouped.is_empty());
    }

    #[test]
    fn test_build_group_tree_flat_groups() {
        let groups = vec![
            make_group(1, "Tech", None, 0),
            make_group(2, "News", None, 1),
        ];
        let feeds = vec![
            make_feed(1, "TechCrunch", Some(1)),
            make_feed(2, "BBC News", Some(2)),
        ];

        let (tree, ungrouped) = build_group_tree(groups, feeds);

        assert_eq!(tree.len(), 2);
        assert_eq!(tree[0].group.name, "Tech");
        assert_eq!(tree[0].feeds.len(), 1);
        assert_eq!(tree[0].feeds[0].title, "TechCrunch");
        assert_eq!(tree[1].group.name, "News");
        assert_eq!(tree[1].feeds.len(), 1);
        assert!(ungrouped.is_empty());
    }

    #[test]
    fn test_build_group_tree_nested_groups() {
        let groups = vec![
            make_group(1, "Tech", None, 0),
            make_group(2, "Tech News", Some(1), 0),
            make_group(3, "Tech Blogs", Some(1), 1),
        ];
        let feeds = vec![
            make_feed(1, "TechCrunch", Some(2)),
            make_feed(2, "Ars Technica", Some(3)),
        ];

        let (tree, ungrouped) = build_group_tree(groups, feeds);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].group.name, "Tech");
        assert_eq!(tree[0].children.len(), 2);
        assert_eq!(tree[0].children[0].group.name, "Tech News");
        assert_eq!(tree[0].children[0].feeds.len(), 1);
        assert_eq!(tree[0].children[1].group.name, "Tech Blogs");
        assert!(ungrouped.is_empty());
    }

    #[test]
    fn test_build_group_tree_ungrouped_feeds() {
        let groups = vec![make_group(1, "Tech", None, 0)];
        let feeds = vec![
            make_feed(1, "TechCrunch", Some(1)),
            make_feed(2, "Random Blog", None),
            make_feed(3, "Another Blog", None),
        ];

        let (tree, ungrouped) = build_group_tree(groups, feeds);

        assert_eq!(tree.len(), 1);
        assert_eq!(tree[0].feeds.len(), 1);
        assert_eq!(ungrouped.len(), 2);
        // Should be sorted by title
        assert_eq!(ungrouped[0].title, "Another Blog");
        assert_eq!(ungrouped[1].title, "Random Blog");
    }
}

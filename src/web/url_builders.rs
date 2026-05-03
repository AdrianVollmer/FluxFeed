use url::form_urlencoded;

#[derive(Clone)]
pub struct ArticleFilters {
    pub feed_ids: Vec<i64>,
    pub group_ids: Vec<i64>,
    pub tag_ids: Vec<i64>,
    pub is_read: Option<bool>,
    pub is_starred: Option<bool>,
    pub search_query: Option<String>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
}

#[derive(Clone)]
pub struct LogFilters {
    pub feed_id: Option<i64>,
    pub feed_name: Option<String>,
    pub log_type: Option<String>,
}

fn ids_to_csv(ids: &[i64]) -> String {
    ids.iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn url_encode(s: &str) -> String {
    form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn append_param(buf: &mut String, kv: &str) {
    if buf.is_empty() {
        buf.push_str(kv);
    } else {
        buf.push('&');
        buf.push_str(kv);
    }
}

impl ArticleFilters {
    fn filter_params(&self) -> String {
        let mut p = String::new();
        if !self.feed_ids.is_empty() {
            append_param(&mut p, &format!("feed_ids={}", ids_to_csv(&self.feed_ids)));
        }
        if !self.group_ids.is_empty() {
            append_param(&mut p, &format!("group_ids={}", ids_to_csv(&self.group_ids)));
        }
        if !self.tag_ids.is_empty() {
            append_param(&mut p, &format!("tag_ids={}", ids_to_csv(&self.tag_ids)));
        }
        if let Some(v) = self.is_read {
            append_param(&mut p, &format!("is_read={}", v));
        }
        if let Some(v) = self.is_starred {
            append_param(&mut p, &format!("is_starred={}", v));
        }
        if let Some(ref q) = self.search_query {
            append_param(&mut p, &format!("q={}", url_encode(q)));
        }
        if let Some(ref v) = self.date_from {
            append_param(&mut p, &format!("date_from={}", v));
        }
        if let Some(ref v) = self.date_to {
            append_param(&mut p, &format!("date_to={}", v));
        }
        p
    }

    pub fn articles_url(&self, offset: i64) -> String {
        let p = self.filter_params();
        if p.is_empty() {
            format!("/articles?offset={}", offset)
        } else {
            format!("/articles?offset={}&{}", offset, p)
        }
    }

    pub fn articles_fullscreen_url(&self, offset: i64) -> String {
        let p = self.filter_params();
        if p.is_empty() {
            format!("/articles?offset={}&view=fullscreen", offset)
        } else {
            format!("/articles?offset={}&view=fullscreen&{}", offset, p)
        }
    }

    pub fn mark_all_read_url(&self) -> String {
        if self.feed_ids.is_empty() {
            "/articles/mark-all-read".to_string()
        } else {
            format!("/articles/mark-all-read?feed_ids={}", ids_to_csv(&self.feed_ids))
        }
    }

    pub fn feed_filter_modal_url(&self) -> String {
        let mut url = format!(
            "/articles/filter-modal?feed_ids={}&group_ids={}",
            ids_to_csv(&self.feed_ids),
            ids_to_csv(&self.group_ids),
        );
        if let Some(v) = self.is_read {
            url.push_str(&format!("&is_read={}", v));
        }
        if let Some(v) = self.is_starred {
            url.push_str(&format!("&is_starred={}", v));
        }
        url
    }

    pub fn tag_filter_modal_url(&self) -> String {
        let mut url = format!(
            "/articles/tag-filter-modal?tag_ids={}&feed_ids={}&group_ids={}",
            ids_to_csv(&self.tag_ids),
            ids_to_csv(&self.feed_ids),
            ids_to_csv(&self.group_ids),
        );
        if let Some(v) = self.is_read {
            url.push_str(&format!("&is_read={}", v));
        }
        if let Some(v) = self.is_starred {
            url.push_str(&format!("&is_starred={}", v));
        }
        url
    }

    pub fn clear_feed_filter_url(&self) -> String {
        let mut p = String::new();
        if let Some(v) = self.is_read {
            append_param(&mut p, &format!("is_read={}", v));
        }
        if let Some(v) = self.is_starred {
            append_param(&mut p, &format!("is_starred={}", v));
        }
        if !self.tag_ids.is_empty() {
            append_param(&mut p, &format!("tag_ids={}", ids_to_csv(&self.tag_ids)));
        }
        if p.is_empty() {
            "/articles".to_string()
        } else {
            format!("/articles?{}", p)
        }
    }

    pub fn clear_tag_filter_url(&self) -> String {
        let mut p = String::new();
        if let Some(v) = self.is_read {
            append_param(&mut p, &format!("is_read={}", v));
        }
        if let Some(v) = self.is_starred {
            append_param(&mut p, &format!("is_starred={}", v));
        }
        if !self.feed_ids.is_empty() {
            append_param(&mut p, &format!("feed_ids={}", ids_to_csv(&self.feed_ids)));
        }
        if !self.group_ids.is_empty() {
            append_param(&mut p, &format!("group_ids={}", ids_to_csv(&self.group_ids)));
        }
        if p.is_empty() {
            "/articles".to_string()
        } else {
            format!("/articles?{}", p)
        }
    }
}

impl LogFilters {
    pub fn logs_url(&self, offset: i64) -> String {
        let mut url = format!("/logs?offset={}", offset);
        if let Some(id) = self.feed_id {
            url.push_str(&format!("&feed_id={}", id));
        }
        if let Some(ref name) = self.feed_name {
            url.push_str(&format!("&feed_name={}", url_encode(name)));
        }
        if let Some(ref t) = self.log_type {
            url.push_str(&format!("&log_type={}", t));
        }
        url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_filters() -> ArticleFilters {
        ArticleFilters {
            feed_ids: vec![],
            group_ids: vec![],
            tag_ids: vec![],
            is_read: None,
            is_starred: None,
            search_query: None,
            date_from: None,
            date_to: None,
        }
    }

    #[test]
    fn articles_url_no_filters() {
        let f = empty_filters();
        assert_eq!(f.articles_url(0), "/articles?offset=0");
    }

    #[test]
    fn articles_url_with_offset_and_feed_ids() {
        let f = ArticleFilters { feed_ids: vec![1, 2], ..empty_filters() };
        assert_eq!(f.articles_url(20), "/articles?offset=20&feed_ids=1,2");
    }

    #[test]
    fn articles_url_all_filters() {
        let f = ArticleFilters {
            feed_ids: vec![3],
            group_ids: vec![4],
            tag_ids: vec![5],
            is_read: Some(false),
            is_starred: Some(true),
            search_query: Some("hello world".to_string()),
            date_from: Some("2024-01-01".to_string()),
            date_to: Some("2024-12-31".to_string()),
        };
        let url = f.articles_url(0);
        assert!(url.contains("feed_ids=3"));
        assert!(url.contains("group_ids=4"));
        assert!(url.contains("tag_ids=5"));
        assert!(url.contains("is_read=false"));
        assert!(url.contains("is_starred=true"));
        assert!(url.contains("q=hello+world") || url.contains("q=hello%20world"));
        assert!(url.contains("date_from=2024-01-01"));
        assert!(url.contains("date_to=2024-12-31"));
    }

    #[test]
    fn articles_fullscreen_url_includes_view() {
        let f = ArticleFilters { feed_ids: vec![1], ..empty_filters() };
        let url = f.articles_fullscreen_url(40);
        assert!(url.contains("view=fullscreen"));
        assert!(url.contains("offset=40"));
        assert!(url.contains("feed_ids=1"));
    }

    #[test]
    fn mark_all_read_url_no_feed_filter() {
        let f = empty_filters();
        assert_eq!(f.mark_all_read_url(), "/articles/mark-all-read");
    }

    #[test]
    fn mark_all_read_url_with_feed_ids() {
        let f = ArticleFilters { feed_ids: vec![7, 8], ..empty_filters() };
        assert_eq!(f.mark_all_read_url(), "/articles/mark-all-read?feed_ids=7,8");
    }

    #[test]
    fn clear_feed_filter_url_keeps_read_and_tags() {
        let f = ArticleFilters {
            feed_ids: vec![1],
            group_ids: vec![2],
            tag_ids: vec![3],
            is_read: Some(false),
            is_starred: None,
            ..empty_filters()
        };
        let url = f.clear_feed_filter_url();
        assert!(!url.contains("feed_ids"));
        assert!(!url.contains("group_ids"));
        assert!(url.contains("is_read=false"));
        assert!(url.contains("tag_ids=3"));
    }

    #[test]
    fn clear_feed_filter_url_no_remaining_filters() {
        let f = ArticleFilters { feed_ids: vec![1], ..empty_filters() };
        assert_eq!(f.clear_feed_filter_url(), "/articles");
    }

    #[test]
    fn clear_tag_filter_url_keeps_feed_and_read() {
        let f = ArticleFilters {
            feed_ids: vec![1],
            group_ids: vec![2],
            tag_ids: vec![3],
            is_read: Some(true),
            is_starred: None,
            ..empty_filters()
        };
        let url = f.clear_tag_filter_url();
        assert!(!url.contains("tag_ids"));
        assert!(url.contains("feed_ids=1"));
        assert!(url.contains("group_ids=2"));
        assert!(url.contains("is_read=true"));
    }

    #[test]
    fn feed_filter_modal_url() {
        let f = ArticleFilters {
            feed_ids: vec![1],
            group_ids: vec![2],
            is_read: Some(false),
            ..empty_filters()
        };
        let url = f.feed_filter_modal_url();
        assert!(url.starts_with("/articles/filter-modal"));
        assert!(url.contains("feed_ids=1"));
        assert!(url.contains("group_ids=2"));
        assert!(url.contains("is_read=false"));
    }

    #[test]
    fn tag_filter_modal_url() {
        let f = ArticleFilters {
            feed_ids: vec![1],
            group_ids: vec![2],
            tag_ids: vec![3],
            is_starred: Some(true),
            ..empty_filters()
        };
        let url = f.tag_filter_modal_url();
        assert!(url.starts_with("/articles/tag-filter-modal"));
        assert!(url.contains("tag_ids=3"));
        assert!(url.contains("feed_ids=1"));
        assert!(url.contains("group_ids=2"));
        assert!(url.contains("is_starred=true"));
    }

    #[test]
    fn log_filters_url_no_filters() {
        let f = LogFilters { feed_id: None, feed_name: None, log_type: None };
        assert_eq!(f.logs_url(0), "/logs?offset=0");
    }

    #[test]
    fn log_filters_url_all_filters() {
        let f = LogFilters {
            feed_id: Some(42),
            feed_name: Some("My Feed".to_string()),
            log_type: Some("error".to_string()),
        };
        let url = f.logs_url(50);
        assert!(url.contains("offset=50"));
        assert!(url.contains("feed_id=42"));
        assert!(url.contains("feed_name=My+Feed") || url.contains("feed_name=My%20Feed"));
        assert!(url.contains("log_type=error"));
    }
}

#![cfg(target_os = "linux")]
//! Graft prefixes above home were never listed: they are no hits and no
//! `in:` scope. Home itself stays searchable.
use fsearch::{
    index::Index,
    live::Live,
    query::{Query, Searcher},
    walk,
};

fn live_at(home: &[u8]) -> Live {
    let ls = walk::graft_home_prefix(walk::scan(home, 4), home);
    Live::new(Index::build(ls, 0, 0, home))
}

fn paths(live: &Live, hits: &[fsearch::query::Hit]) -> Vec<String> {
    let mut b = Vec::new();
    hits.iter()
        .map(|h| {
            live.base.path(h.idx as usize, &mut b);
            String::from_utf8_lossy(&b).into_owned()
        })
        .collect()
}

#[test]
fn graft_prefix_is_no_hit_no_scope() {
    let home = b"/tmp/fsearch-graft-synth/h1";
    std::fs::create_dir_all("/tmp/fsearch-graft-synth/h1/proj").unwrap();
    std::fs::write("/tmp/fsearch-graft-synth/h1/proj/main.rs", b"hello").unwrap();
    let hs = "/tmp/fsearch-graft-synth/h1";
    let live = live_at(home);
    let s = Searcher { live: &live };
    // Middle graft component: purely synthetic, outside home.
    let hits = paths(&live, &s.search(&Query::parse("fsearch-graft-synth", hs).unwrap()));
    assert!(hits.iter().all(|p| p.starts_with(hs)), "synthetic hit leaked: {hits:?}");
    assert!(!hits.iter().any(|p| p == "/tmp/fsearch-graft-synth"), "unscanned prefix hit: {hits:?}");
    // Synthetic folders lend no folder matches either.
    let hits = paths(&live, &s.search(&Query::parse("fsearch-graft-synth main", hs).unwrap()));
    assert!(hits.is_empty(), "inherited synthetic folder match: {hits:?}");
    // `in:` above home is nothing; home itself still scopes.
    assert!(s.scope_range(&Query::parse("in:/tmp main", hs).unwrap()).is_none());
    assert!(s.search(&Query::parse("in:/tmp main", hs).unwrap()).is_empty());
    let scoped = paths(&live, &s.search(&Query::parse("in:/tmp/fsearch-graft-synth/h1 main", hs).unwrap()));
    assert!(!scoped.is_empty(), "in:HOME broke: {scoped:?}");
    std::fs::remove_dir_all("/tmp/fsearch-graft-synth").ok();
}

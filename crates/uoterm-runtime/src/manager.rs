use crate::config::{ConnectOptions, DEFAULT_MAX_SESSIONS};
use crate::error::{Result, RuntimeError};
use crate::session::{self, SessionHandle};
use crate::tools::{ToolCall, ToolResult};
use parking_lot::{Mutex, RwLock};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use uoterm_nav::{ClilocData, MulMap, MultiData};

/// A cache that has opened nothing yet.
const OPENS_NONE: u64 = 0;
/// One facet open.
const ONE_OPEN: u64 = 1;
/// A cached facet that no session holds any more.
const NO_HOLDERS: usize = 0;
/// The first session id a runtime hands out.
const FIRST_SESSION_NUMBER: u64 = 1;
/// One session added to the runtime.
const ONE_SESSION: u64 = 1;
/// The facet index the multi shapes of one client directory are cached under.
///
/// A house has the same shape on every facet, so the shapes are read once for
/// a client directory and shared by every session of it, whichever facet each
/// session stands on. The cache keys on a directory and an index, and this is
/// the index that means "no facet of its own".
const MULTI_SHAPES_INDEX: u8 = 0;

/// Names one map facet: the client directory it is read from, and its index.
///
/// Two shards installed side by side hold their own map files, so the
/// directory belongs in the key. Paths are compared as the operator wrote
/// them, which is how every session of one shard states its `uopath`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct FacetKey {
    uopath: PathBuf,
    map_index: u8,
}

/// Map facets shared by every session of one [`Runtime`].
///
/// A facet is read-only after it is built, so one instance serves all the
/// sessions that stand on it. Each entry is a [`Weak`] handle: the facet and
/// its file handles are freed as soon as the last session that holds it goes
/// away, and a session that dies without a clean disconnect leaves nothing
/// behind.
pub struct FacetCache<T> {
    entries: Mutex<HashMap<FacetKey, Weak<T>>>,
    opens: AtomicU64,
}

impl<T> Default for FacetCache<T> {
    fn default() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            opens: AtomicU64::new(OPENS_NONE),
        }
    }
}

impl<T> FacetCache<T> {
    /// Hands out the facet for `uopath` and `map_index`, calling `load` only
    /// when no session holds that facet.
    ///
    /// The lock is held across `load`, so two sessions that ask at the same
    /// time read the client files once and share the one instance.
    pub fn get_or_load<E>(
        &self,
        uopath: &Path,
        map_index: u8,
        load: impl FnOnce() -> std::result::Result<T, E>,
    ) -> std::result::Result<Arc<T>, E> {
        let key = FacetKey {
            uopath: uopath.to_path_buf(),
            map_index,
        };
        let mut entries = self.entries.lock();
        if let Some(shared) = entries.get(&key).and_then(Weak::upgrade) {
            return Ok(shared);
        }
        let facet = Arc::new(load()?);
        self.opens.fetch_add(ONE_OPEN, Ordering::Relaxed);
        entries.retain(|_, holder| holder.strong_count() > NO_HOLDERS);
        entries.insert(key, Arc::downgrade(&facet));
        Ok(facet)
    }

    /// How many facets this cache has opened since the runtime started.
    pub fn opens(&self) -> u64 {
        self.opens.load(Ordering::Relaxed)
    }

    /// How many cached facets a session still holds.
    pub fn live(&self) -> usize {
        self.entries
            .lock()
            .values()
            .filter(|holder| holder.strong_count() > NO_HOLDERS)
            .count()
    }
}

/// Where the multi shapes of one client directory are asked for, so every
/// session of that directory reads the client multi files once between them.
pub fn shared_multi_shapes(
    shapes: &FacetCache<MultiData>,
    uopath: &Path,
) -> std::result::Result<Arc<MultiData>, uoterm_nav::MapError> {
    shapes.get_or_load(uopath, MULTI_SHAPES_INDEX, || {
        tracing::info!(path = %uopath.display(), "opening the client multi files");
        MultiData::open(uopath)
    })
}

#[derive(Clone, Default)]
pub struct Runtime {
    sessions: Arc<RwLock<HashMap<String, SessionHandle>>>,
    facets: Arc<FacetCache<MulMap>>,
    /// The shapes of every house and boat the client files describe, shared
    /// with every session on the same client directory.
    multi_shapes: Arc<FacetCache<MultiData>>,
    /// The client text database, shared by every session of one directory.
    clilocs: Arc<FacetCache<ClilocData>>,
    next: Arc<AtomicU64>,
    max: usize,
}

impl Runtime {
    pub fn new(max: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            facets: Arc::new(FacetCache::default()),
            multi_shapes: Arc::new(FacetCache::default()),
            clilocs: Arc::new(FacetCache::default()),
            next: Arc::new(AtomicU64::new(FIRST_SESSION_NUMBER)),
            max: if max == 0 { DEFAULT_MAX_SESSIONS } else { max },
        }
    }

    pub async fn connect(&self, opts: ConnectOptions) -> Result<SessionHandle> {
        if self.sessions.read().len() >= self.max {
            return Err(RuntimeError::World("session cap reached".into()));
        }
        let id = format!("s{}", self.next.fetch_add(ONE_SESSION, Ordering::Relaxed));
        let handle = session::start(
            id.clone(),
            opts,
            self.facets.clone(),
            self.multi_shapes.clone(),
            self.clilocs.clone(),
        )
        .await?;
        self.sessions.write().insert(id, handle.clone());
        Ok(handle)
    }

    pub fn get(&self, id: &str) -> Option<SessionHandle> {
        self.sessions.read().get(id).cloned()
    }

    pub fn list(&self) -> Vec<String> {
        self.sessions.read().keys().cloned().collect()
    }

    pub fn first(&self) -> Option<SessionHandle> {
        self.sessions.read().values().next().cloned()
    }

    pub fn require(&self, id: Option<&str>) -> Result<SessionHandle> {
        if let Some(id) = id {
            self.get(id)
                .ok_or_else(|| RuntimeError::World(format!("no session {id}")))
        } else {
            self.first()
                .ok_or_else(|| RuntimeError::World("no active session".into()))
        }
    }

    pub async fn call(&self, id: Option<&str>, tool: ToolCall) -> Result<ToolResult> {
        Ok(self.require(id)?.call(tool).await)
    }

    pub async fn stop(&self, id: &str) -> Result<()> {
        let handle = self.sessions.write().remove(id);
        if let Some(s) = handle {
            s.shutdown().await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock::{MockServer, MOCK_CHAR, MOCK_X, MOCK_Y};
    use crate::tools::TOOL_CAN_WALK;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Barrier;
    use std::time::Duration;
    use uoterm_nav::client_data_dir_from_env;
    use uoterm_nav::MapError;
    use uoterm_protocol::types::{ClientVersion, Era};

    /// Stands in for a facet. These tests must pass on a machine that holds
    /// no client files, so they cache this instead of a `MulMap`.
    struct TestFacet;

    /// Two client directories of two shards installed side by side.
    const CLIENT_A: &str = "/uo/client-a";
    const CLIENT_B: &str = "/uo/client-b";
    const FACET_FELUCCA: u8 = 0;
    const FACET_TRAMMEL: u8 = 1;
    /// A counted loader starts here and steps by one for each real open.
    const LOADS_NONE: usize = 0;
    const ONE_LOAD: usize = 1;
    const TWO_LOADS: usize = 2;
    const TWO_OPENS: u64 = 2;
    /// One facet held in the cache.
    const ONE_LIVE_FACET: usize = 1;
    const NO_LIVE_FACETS: usize = 0;
    /// One key in the cache table.
    const ONE_ENTRY: usize = 1;
    /// How many sessions ask for the one facet.
    const SESSIONS_PER_FACET: usize = 8;
    const MAX_TEST_SESSIONS: usize = 4;
    const LOGIN_POLLS: usize = 25;
    const LOGIN_POLL_MS: u64 = 100;

    /// A loader that counts how often the cache really opens a facet.
    fn counted(
        loads: &AtomicUsize,
    ) -> impl FnOnce() -> std::result::Result<TestFacet, MapError> + '_ {
        move || {
            loads.fetch_add(ONE_LOAD, Ordering::Relaxed);
            Ok(TestFacet)
        }
    }

    fn mock_opts(server: &MockServer) -> ConnectOptions {
        ConnectOptions {
            host: server.addr.ip().to_string(),
            port: server.addr.port(),
            account: "test".into(),
            password: "test".into(),
            character: MOCK_CHAR.into(),
            version: ClientVersion::T2A,
            era: Era::T2a,
            stay_on_socket: true,
            ..ConnectOptions::default()
        }
    }

    async fn wait_for_login(handle: &SessionHandle) {
        for _ in 0..LOGIN_POLLS {
            if handle.logged_in() {
                return;
            }
            tokio::time::sleep(Duration::from_millis(LOGIN_POLL_MS)).await;
        }
        panic!("mock login must reach the world");
    }

    #[test]
    fn sessions_on_one_uopath_and_facet_share_one_instance() {
        let cache = FacetCache::<TestFacet>::default();
        let loads = AtomicUsize::new(LOADS_NONE);
        let first = cache
            .get_or_load(Path::new(CLIENT_A), FACET_TRAMMEL, counted(&loads))
            .unwrap();
        let second = cache
            .get_or_load(Path::new(CLIENT_A), FACET_TRAMMEL, counted(&loads))
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(loads.load(Ordering::Relaxed), ONE_LOAD);
        assert_eq!(cache.opens(), ONE_OPEN);
        assert_eq!(cache.live(), ONE_LIVE_FACET);
    }

    #[test]
    fn two_client_directories_do_not_collide() {
        let cache = FacetCache::<TestFacet>::default();
        let loads = AtomicUsize::new(LOADS_NONE);
        let from_a = cache
            .get_or_load(Path::new(CLIENT_A), FACET_TRAMMEL, counted(&loads))
            .unwrap();
        let from_b = cache
            .get_or_load(Path::new(CLIENT_B), FACET_TRAMMEL, counted(&loads))
            .unwrap();
        assert!(!Arc::ptr_eq(&from_a, &from_b));
        assert_eq!(loads.load(Ordering::Relaxed), TWO_LOADS);
        assert_eq!(cache.opens(), TWO_OPENS);
    }

    #[test]
    fn two_facets_of_one_client_do_not_collide() {
        let cache = FacetCache::<TestFacet>::default();
        let loads = AtomicUsize::new(LOADS_NONE);
        let felucca = cache
            .get_or_load(Path::new(CLIENT_A), FACET_FELUCCA, counted(&loads))
            .unwrap();
        let trammel = cache
            .get_or_load(Path::new(CLIENT_A), FACET_TRAMMEL, counted(&loads))
            .unwrap();
        assert!(!Arc::ptr_eq(&felucca, &trammel));
        assert_eq!(loads.load(Ordering::Relaxed), TWO_LOADS);
    }

    #[test]
    fn a_cached_facet_is_not_opened_again() {
        let cache = FacetCache::<TestFacet>::default();
        let loads = AtomicUsize::new(LOADS_NONE);
        let held: Vec<Arc<TestFacet>> = (0..SESSIONS_PER_FACET)
            .map(|_| {
                cache
                    .get_or_load(Path::new(CLIENT_A), FACET_TRAMMEL, counted(&loads))
                    .unwrap()
            })
            .collect();
        let (first, rest) = held.split_first().expect("every session holds a facet");
        assert!(rest.iter().all(|facet| Arc::ptr_eq(first, facet)));
        assert_eq!(loads.load(Ordering::Relaxed), ONE_LOAD);
        assert_eq!(cache.opens(), ONE_OPEN);
        assert_eq!(cache.live(), ONE_LIVE_FACET);
    }

    #[test]
    fn sessions_that_ask_at_the_same_time_open_the_facet_once() {
        let cache = Arc::new(FacetCache::<TestFacet>::default());
        let start = Arc::new(Barrier::new(SESSIONS_PER_FACET));
        let loads = Arc::new(AtomicUsize::new(LOADS_NONE));
        let mut asks = Vec::with_capacity(SESSIONS_PER_FACET);
        for _ in 0..SESSIONS_PER_FACET {
            let cache = cache.clone();
            let start = start.clone();
            let loads = loads.clone();
            asks.push(std::thread::spawn(move || {
                start.wait();
                cache
                    .get_or_load(Path::new(CLIENT_A), FACET_TRAMMEL, || {
                        loads.fetch_add(ONE_LOAD, Ordering::Relaxed);
                        Ok::<TestFacet, MapError>(TestFacet)
                    })
                    .unwrap()
            }));
        }
        let held: Vec<Arc<TestFacet>> = asks
            .into_iter()
            .map(|ask| ask.join().expect("a racing session panicked"))
            .collect();
        let (first, rest) = held.split_first().expect("every session holds a facet");
        assert!(rest.iter().all(|facet| Arc::ptr_eq(first, facet)));
        assert_eq!(loads.load(Ordering::Relaxed), ONE_LOAD);
        assert_eq!(cache.opens(), ONE_OPEN);
    }

    #[test]
    fn the_last_session_to_go_frees_the_facet() {
        let cache = FacetCache::<TestFacet>::default();
        let loads = AtomicUsize::new(LOADS_NONE);
        let held = cache
            .get_or_load(Path::new(CLIENT_A), FACET_TRAMMEL, counted(&loads))
            .unwrap();
        let watch = Arc::downgrade(&held);
        assert_eq!(cache.live(), ONE_LIVE_FACET);
        drop(held);
        assert!(watch.upgrade().is_none(), "the cache must not hold a facet");
        assert_eq!(cache.live(), NO_LIVE_FACETS);
        let next = cache
            .get_or_load(Path::new(CLIENT_A), FACET_TRAMMEL, counted(&loads))
            .unwrap();
        assert_eq!(loads.load(Ordering::Relaxed), TWO_LOADS);
        assert_eq!(cache.opens(), TWO_OPENS);
        assert_eq!(cache.live(), ONE_LIVE_FACET);
        drop(next);
    }

    #[test]
    fn a_dead_entry_does_not_stay_in_the_table() {
        let cache = FacetCache::<TestFacet>::default();
        let loads = AtomicUsize::new(LOADS_NONE);
        let gone = cache
            .get_or_load(Path::new(CLIENT_A), FACET_TRAMMEL, counted(&loads))
            .unwrap();
        drop(gone);
        let held = cache
            .get_or_load(Path::new(CLIENT_B), FACET_TRAMMEL, counted(&loads))
            .unwrap();
        assert_eq!(cache.entries.lock().len(), ONE_ENTRY);
        drop(held);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_session_without_uopath_keeps_its_own_mock_map() {
        let server = MockServer::start().await.unwrap();
        let rt = Runtime::new(MAX_TEST_SESSIONS);
        let mut opts = mock_opts(&server);
        opts.uopath = None;
        let handle = rt.connect(opts).await.unwrap();
        wait_for_login(&handle).await;
        assert_eq!(rt.facets.opens(), OPENS_NONE);
        assert_eq!(rt.facets.live(), NO_LIVE_FACETS);
        let walk = handle
            .call(ToolCall {
                name: TOOL_CAN_WALK.into(),
                args: serde_json::json!({ "x": MOCK_X, "y": MOCK_Y }),
            })
            .await;
        assert!(walk.ok, "{walk:?}");
        assert_eq!(
            walk.result.as_bool(),
            Some(true),
            "the mock map must answer when no uopath is set"
        );
        rt.stop(&handle.id).await.unwrap();
    }

    #[test]
    fn one_real_facet_serves_every_asker() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let cache = FacetCache::<MulMap>::default();
        let first = cache
            .get_or_load(&dir, FACET_TRAMMEL, || MulMap::open(&dir, FACET_TRAMMEL))
            .unwrap();
        let second = cache
            .get_or_load(&dir, FACET_TRAMMEL, || MulMap::open(&dir, FACET_TRAMMEL))
            .unwrap();
        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(cache.opens(), ONE_OPEN);
        drop(first);
        drop(second);
        assert_eq!(cache.live(), NO_LIVE_FACETS);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn two_sessions_on_one_client_directory_open_one_facet() {
        let Some(dir) = client_data_dir_from_env() else {
            return;
        };
        let server = MockServer::start().await.unwrap();
        let rt = Runtime::new(MAX_TEST_SESSIONS);
        let mut first = mock_opts(&server);
        first.uopath = Some(dir.clone());
        let mut second = mock_opts(&server);
        second.uopath = Some(dir);
        let one = rt.connect(first).await.unwrap();
        let two = rt.connect(second).await.unwrap();
        wait_for_login(&one).await;
        wait_for_login(&two).await;
        assert_eq!(rt.facets.opens(), ONE_OPEN);
        assert_eq!(rt.facets.live(), ONE_LIVE_FACET);
        rt.stop(&one.id).await.unwrap();
        rt.stop(&two.id).await.unwrap();
    }
}

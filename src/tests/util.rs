use crate::commands::{
    ArrayCommands, ClusterCommands, GenericCommands, JsonCommands, SearchCommands,
    SentinelCommands, ServerCommands, SortedSetCommands, StreamCommands, StringCommands,
    VectorSetCommands,
};
#[cfg(feature = "native-tls")]
use native_tls::Certificate;
use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};

pub(crate) fn log_try_init() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        let inner = env_logger::builder()
            .format_target(false)
            .format_timestamp(None)
            .filter_level(log::LevelFilter::Debug)
            .target(env_logger::Target::Stdout)
            .is_test(true)
            .parse_default_env()
            .build();
        log::set_max_level(inner.filter());
        // The recorder wraps `env_logger` rather than replacing it, so
        // [`LogCapture`] can read the levels without silencing the suite.
        let _ = log::set_boxed_logger(Box::new(TeeLogger { inner }));
    });
}

/// Builds a command without a connection, so that a command no test can safely
/// send to the shared servers can still have its wire form checked.
pub(crate) struct TestClient;
impl<'a> ArrayCommands<'a> for TestClient {}
impl<'a> StreamCommands<'a> for TestClient {}
impl<'a> VectorSetCommands<'a> for TestClient {}
impl<'a> ClusterCommands<'a> for TestClient {}
impl<'a> GenericCommands<'a> for TestClient {}
impl<'a> JsonCommands<'a> for TestClient {}
impl<'a> SearchCommands<'a> for TestClient {}
impl<'a> SentinelCommands<'a> for TestClient {}
impl<'a> ServerCommands<'a> for TestClient {}
impl<'a> SortedSetCommands<'a> for TestClient {}
impl<'a> StringCommands<'a> for TestClient {}

/// The crate logs through `tracing`, which forwards to `log` when no `tracing`
/// subscriber is installed — which is the case in the suite. A `log`
/// implementation therefore sees every event with its level, and is what lets a
/// test assert that a routine outcome is not reported as a warning.
// The logger writes here whatever the suite selected; `LogCapture`, which reads
// them back, lives with the tests that need a server.
pub(crate) static RECORDED: Mutex<Vec<(log::Level, String)>> = Mutex::new(Vec::new());
pub(crate) static RECORDING: AtomicBool = AtomicBool::new(false);

/// Records the crate's events while forwarding them to `env_logger`, so a test
/// that captures does not lose the output the rest of the suite prints.
struct TeeLogger {
    inner: env_logger::Logger,
}

impl log::Log for TeeLogger {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &log::Record<'_>) {
        if RECORDING.load(Ordering::SeqCst) && record.target().starts_with("rustis") {
            RECORDED
                .lock()
                .unwrap()
                .push((record.level(), record.args().to_string()));
        }
        self.inner.log(record);
    }

    fn flush(&self) {
        self.inner.flush();
    }
}

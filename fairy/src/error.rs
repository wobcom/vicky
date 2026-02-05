use hyper::http;
use snafu::Snafu;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)), context(suffix(Err)))]
pub enum Error {
    #[snafu(display("load config: {source}"))]
    Config {
        #[snafu(source(from(rocket::figment::Error, Box::new)))]
        source: Box<rocket::figment::Error>,
    },

    #[snafu(display("encode request: {source}"))]
    Serialize { source: serde_json::Error },

    #[snafu(display("build request: {source}"))]
    BuildRequest { source: http::Error },

    #[snafu(display("call api: {source}"))]
    Request { source: hyper::Error },

    #[snafu(display("api returned {status}"))]
    ApiStatus { status: hyper::StatusCode },

    #[snafu(display("read response: {source}"))]
    ReadBody { source: hyper::Error },

    #[snafu(display("decode response: {source}"))]
    DecodeResponse { source: serde_json::Error },

    #[snafu(display("spawn nix: {source}"))]
    SpawnNix { source: std::io::Error },

    #[snafu(display("stream logs: {source}"))]
    StreamLogs {
        source: tokio_util::codec::LinesCodecError,
    },

    #[snafu(display("wait for nix: {source}"))]
    WaitNix { source: std::io::Error },

    #[snafu(display("nix exited with {code:?}"))]
    TaskExit { code: Option<i32> },

    #[snafu(display("missing log pipe: {which}"))]
    MissingPipe { which: &'static str },

    #[snafu(display("the runner stopped sending heartbeats, timed out and got killed"))]
    Timeout,

    #[snafu(display("process was killed but process isn't exiting"))]
    NixZombie,
}

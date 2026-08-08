pub const BRIDGE_WS_PATH: &str = "/bridge/ws";
pub const BRIDGE_HEARTBEAT_INTERVAL_MS: u64 = 30_000;
pub const BRIDGE_HEARTBEAT_TIMEOUT_MS: u64 = 90_000;
pub const BRIDGE_MCP_REQUEST_TIMEOUT_MS: u64 = 30_000;
pub const BRIDGE_WS_MAX_MESSAGE_BYTES: usize = 16_777_216;
pub const FIGMA_MCP_ENDPOINT: &str = "http://127.0.0.1:3845/mcp";
pub const FIGMA_HEALTH_CHECK_INTERVAL_MS: u64 = 10_000;
pub const FIGMA_HEALTH_CHECK_TIMEOUT_MS: u64 = 3_000;

pub const RECONNECT_BASE_DELAY_MS: u64 = 1_000;
pub const RECONNECT_MAX_DELAY_MS: u64 = 60_000;

pub const DEFAULT_CLOUD_URL: &str = "https://ant.crosstoken.io";
pub const DEFAULT_LOCAL_HOST: &str = "http://127.0.0.1";
pub const DEFAULT_LOCAL_PORT: u16 = 4101;

/// How long a deep-link connect request may sit awaiting the user's approval.
/// A parked request holds a bearer token, so it expires rather than waiting
/// forever for a click that may come from a different session entirely.
pub const DEEPLINK_PENDING_TTL_SECS: u64 = 300;

/// How long a locally-minted pairing nonce stays redeemable. The window only
/// has to cover "open the browser, sign in, click connect".
pub const PAIRING_TTL_SECS: u64 = 300;

pub const KEYCHAIN_SERVICE: &str = "ant-desktop";
pub const KEYCHAIN_JWT_KEY: &str = "jwt";

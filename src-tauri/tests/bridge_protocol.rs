use ant_desktop_lib::bridge::protocol::*;

const FIXTURES: &str = include_str!("fixtures/bridge_messages.json");

fn fixtures() -> serde_json::Value {
    serde_json::from_str(FIXTURES).expect("fixtures parse")
}

fn round_trip(json: &serde_json::Value) -> serde_json::Value {
    let msg: BridgeMessage =
        serde_json::from_value(json.clone()).expect("deserialize BridgeMessage");
    serde_json::to_value(&msg).expect("serialize BridgeMessage")
}

fn assert_json_eq(a: &serde_json::Value, b: &serde_json::Value, context: &str) {
    assert_eq!(a, b, "JSON mismatch for {context}:\n  left:  {a}\n  right: {b}");
}

#[test]
fn register_round_trip() {
    let fix = fixtures();
    let json = &fix["register"];
    let result = round_trip(json);
    assert_json_eq(json, &result, "register");

    let msg: BridgeMessage = serde_json::from_value(json.clone()).unwrap();
    match msg {
        BridgeMessage::Register(r) => {
            assert_eq!(r.user_id, "user-123");
            assert_eq!(r.machine_id, "machine-abc-def");
            assert_eq!(r.capabilities, vec![BridgeCapability::FigmaMcp]);
        }
        _ => panic!("expected Register variant"),
    }
}

#[test]
fn heartbeat_round_trip() {
    let fix = fixtures();

    let json = &fix["heartbeat"];
    let result = round_trip(json);
    assert_json_eq(json, &result, "heartbeat");

    let msg: BridgeMessage = serde_json::from_value(json.clone()).unwrap();
    match msg {
        BridgeMessage::Heartbeat(h) => {
            assert_eq!(h.timestamp, 1711382400000);
            assert_eq!(h.figma_desktop_reachable, Some(true));
        }
        _ => panic!("expected Heartbeat variant"),
    }
}

#[test]
fn heartbeat_minimal_round_trip() {
    let fix = fixtures();
    let json = &fix["heartbeat_minimal"];
    let result = round_trip(json);
    assert_json_eq(json, &result, "heartbeat_minimal");
}

#[test]
fn disconnect_round_trip() {
    let fix = fixtures();
    let json = &fix["disconnect"];
    let result = round_trip(json);
    assert_json_eq(json, &result, "disconnect");
}

#[test]
fn disconnect_no_reason_round_trip() {
    let fix = fixtures();
    let json = &fix["disconnect_no_reason"];
    let result = round_trip(json);
    assert_json_eq(json, &result, "disconnect_no_reason");
}

#[test]
fn mcp_request_round_trip() {
    let fix = fixtures();
    let json = &fix["mcp_request"];
    let result = round_trip(json);
    assert_json_eq(json, &result, "mcp_request");

    let msg: BridgeMessage = serde_json::from_value(json.clone()).unwrap();
    match msg {
        BridgeMessage::McpRequest(r) => {
            assert_eq!(r.request_id, "req-uuid-001");
            assert_eq!(r.tool, "get_design_context");
            assert_eq!(r.args["fileKey"], "abc123");
            assert_eq!(r.args["nodeId"], "1:2");
        }
        _ => panic!("expected McpRequest variant"),
    }
}

#[test]
fn mcp_response_success_round_trip() {
    let fix = fixtures();
    let json = &fix["mcp_response_success"];
    let result = round_trip(json);
    assert_json_eq(json, &result, "mcp_response_success");
}

#[test]
fn mcp_response_error_round_trip() {
    let fix = fixtures();
    let json = &fix["mcp_response_error"];
    let result = round_trip(json);
    assert_json_eq(json, &result, "mcp_response_error");
}

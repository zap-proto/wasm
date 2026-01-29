//! WASM tests for zap-wasm
//!
//! Run with: wasm-pack test --headless --firefox

#![cfg(target_arch = "wasm32")]

use wasm_bindgen_test::*;
use zap_wasm::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn test_generate_id() {
    let id1 = generate_id();
    let id2 = generate_id();

    assert!(!id1.is_empty());
    assert!(!id2.is_empty());
    assert_ne!(id1, id2);
}

#[wasm_bindgen_test]
fn test_generate_client_id() {
    let id = generate_client_id();

    assert!(id.starts_with("zap-"));
    assert!(id.len() > 4);
}

#[wasm_bindgen_test]
fn test_protocol_encode_decode_binary() {
    use js_sys::Object;
    use wasm_bindgen::JsValue;

    let protocol = Protocol::new(Some(true));

    // Create test data
    let obj = Object::new();
    js_sys::Reflect::set(&obj, &"test".into(), &"value".into()).unwrap();

    // Encode
    let encoded = protocol.encode(MessageType::Request, obj.into()).unwrap();

    // Decode
    let decoded = protocol.decode(encoded).unwrap();

    let msg_type = js_sys::Reflect::get(&decoded, &"type".into())
        .unwrap()
        .as_f64()
        .unwrap() as u8;

    assert_eq!(msg_type, MessageType::Request as u8);
}

#[wasm_bindgen_test]
fn test_protocol_encode_decode_json() {
    use js_sys::Object;

    let protocol = Protocol::new(Some(false));

    // Create test data
    let obj = Object::new();
    js_sys::Reflect::set(&obj, &"hello".into(), &"world".into()).unwrap();

    // Encode
    let encoded = protocol.encode(MessageType::Ping, obj.into()).unwrap();

    // Should be a string
    assert!(encoded.is_string());

    // Decode
    let decoded = protocol.decode(encoded).unwrap();

    let msg_type = js_sys::Reflect::get(&decoded, &"type".into())
        .unwrap()
        .as_f64()
        .unwrap() as u8;

    assert_eq!(msg_type, MessageType::Ping as u8);
}

#[wasm_bindgen_test]
fn test_client_type_enum() {
    assert_eq!(ClientType::McpServer as u8, 0);
    assert_eq!(ClientType::McpClient as u8, 1);
    assert_eq!(ClientType::BrowserExtension as u8, 2);
    assert_eq!(ClientType::Agent as u8, 3);
}

#[wasm_bindgen_test]
fn test_browser_action_enum() {
    assert_eq!(BrowserAction::Navigate as u8, 1);
    assert_eq!(BrowserAction::Click as u8, 10);
    assert_eq!(BrowserAction::Evaluate as u8, 20);
    assert_eq!(BrowserAction::Screenshot as u8, 40);
    assert_eq!(BrowserAction::GetTabs as u8, 50);
}

#[wasm_bindgen_test]
fn test_zap_client_creation() {
    use js_sys::Object;

    let options = Object::new();
    js_sys::Reflect::set(&options, &"clientId".into(), &"test-client".into()).unwrap();

    let client = ZapClient::new(options.into()).unwrap();

    assert_eq!(client.client_id(), "test-client");
    assert!(!client.is_connected());
}

#[wasm_bindgen_test]
fn test_zap_client_default_options() {
    use js_sys::Object;

    let client = ZapClient::new(Object::new().into()).unwrap();

    // Should have auto-generated client ID
    assert!(client.client_id().starts_with("zap-"));
    assert!(!client.is_connected());
}

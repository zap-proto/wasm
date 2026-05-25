//! ZAP Protocol WebAssembly bindings
//!
//! This crate provides WebAssembly bindings for the ZAP protocol,
//! enabling browser extensions and web applications to communicate
//! directly with MCP servers using ZAP's zero-copy serialization.
//!
//! # Example
//!
//! ```javascript
//! import init, { ZapClient, Protocol, generateId } from '@zap-proto/wasm';
//!
//! await init();
//!
//! const protocol = new Protocol(true);
//! const encoded = protocol.encode(3, { id: '123', method: 'tools/list' });
//! ```

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use js_sys::{Array, Function, Object, Promise, Reflect, Uint8Array};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{console, BinaryType, CloseEvent, ErrorEvent, MessageEvent, WebSocket};

// ============================================================================
// Constants
// ============================================================================

const ZAP_MAGIC: [u8; 4] = [0x5A, 0x41, 0x50, 0x01]; // "ZAP\x01"

// ============================================================================
// Types
// ============================================================================

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum ClientType {
    Unknown = 0,
    BrowserExtension = 1,
    McpServer = 2,
    McpClient = 3,
    Agent = 4,
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum MessageType {
    Handshake = 1,
    HandshakeResponse = 2,
    Request = 3,
    Response = 4,
    Stream = 5,
    Ping = 6,
    Pong = 7,
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
pub enum BrowserAction {
    None = 0,
    Navigate = 1,
    Reload = 2,
    Back = 3,
    Forward = 4,
    Click = 10,
    Type = 11,
    Fill = 12,
    Select = 13,
    Scroll = 14,
    Hover = 15,
    Evaluate = 20,
    NewPage = 30,
    ClosePage = 31,
    ListPages = 32,
    GetActivePage = 33,
    Screenshot = 40,
    GetTabs = 50,
    SetActiveTab = 51,
    CloseTab = 52,
    NewTab = 53,
    GetCookies = 60,
    SetCookie = 61,
    ClearCookies = 62,
    GetStorage = 63,
    SetStorage = 64,
    Status = 70,
    Ping = 71,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZapRequest {
    pub id: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZapResponse {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ZapErrorData>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ZapErrorData {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Handshake {
    pub version: String,
    #[serde(rename = "clientType")]
    pub client_type: u8,
    #[serde(rename = "clientId")]
    pub client_id: String,
    pub capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HandshakeResponse {
    pub accepted: bool,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "serverVersion")]
    pub server_version: String,
    pub capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub content: Vec<ContentItem>,
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentItem {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

// ============================================================================
// Protocol Encoder/Decoder
// ============================================================================

/// Protocol handler for ZAP messages
#[wasm_bindgen]
pub struct Protocol {
    binary: bool,
}

#[wasm_bindgen]
impl Protocol {
    #[wasm_bindgen(constructor)]
    pub fn new(binary: Option<bool>) -> Protocol {
        Protocol {
            binary: binary.unwrap_or(true),
        }
    }

    /// Encode a message to binary or JSON format
    #[wasm_bindgen]
    pub fn encode(&self, msg_type: u8, data: JsValue) -> Result<JsValue, JsValue> {
        if !self.binary {
            let obj = Object::new();
            Reflect::set(&obj, &"t".into(), &JsValue::from(msg_type))?;
            Reflect::set(&obj, &"d".into(), &data)?;
            let json = js_sys::JSON::stringify(&obj)?;
            return Ok(json.into());
        }

        // Binary format: ZAP_MAGIC + type byte + JSON payload
        let json_str = js_sys::JSON::stringify(&data)?;
        let json_bytes = json_str.as_string().unwrap_or_default().into_bytes();

        let mut buffer = Vec::with_capacity(5 + json_bytes.len());
        buffer.extend_from_slice(&ZAP_MAGIC);
        buffer.push(msg_type);
        buffer.extend_from_slice(&json_bytes);

        let arr = Uint8Array::new_with_length(buffer.len() as u32);
        arr.copy_from(&buffer);
        Ok(arr.buffer().into())
    }

    /// Decode a binary or JSON message
    #[wasm_bindgen]
    pub fn decode(&self, data: JsValue) -> Result<JsValue, JsValue> {
        if let Some(s) = data.as_string() {
            // JSON format
            let parsed: serde_json::Value = serde_json::from_str(&s)
                .map_err(|e| JsValue::from_str(&e.to_string()))?;

            let obj = Object::new();
            if let Some(t) = parsed.get("t").and_then(|v| v.as_u64()) {
                Reflect::set(&obj, &"type".into(), &JsValue::from(t as u8))?;
            }
            if let Some(d) = parsed.get("d") {
                let payload = serde_wasm_bindgen::to_value(d)?;
                Reflect::set(&obj, &"payload".into(), &payload)?;
            }
            return Ok(obj.into());
        }

        // Binary format
        let array = Uint8Array::new(&data);
        let bytes = array.to_vec();

        // Verify magic
        if bytes.len() < 5 {
            return Err(JsValue::from_str("Message too short"));
        }

        for i in 0..4 {
            if bytes[i] != ZAP_MAGIC[i] {
                return Err(JsValue::from_str("Invalid ZAP magic"));
            }
        }

        let msg_type = bytes[4];
        let json_bytes = &bytes[5..];
        let json_str = String::from_utf8(json_bytes.to_vec())
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let payload: serde_json::Value = serde_json::from_str(&json_str)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;

        let obj = Object::new();
        Reflect::set(&obj, &"type".into(), &JsValue::from(msg_type))?;
        Reflect::set(&obj, &"payload".into(), &serde_wasm_bindgen::to_value(&payload)?)?;

        Ok(obj.into())
    }

    /// Encode a handshake message
    #[wasm_bindgen(js_name = encodeHandshake)]
    pub fn encode_handshake(&self, handshake: JsValue) -> Result<JsValue, JsValue> {
        self.encode(MessageType::Handshake as u8, handshake)
    }

    /// Encode a request message
    #[wasm_bindgen(js_name = encodeRequest)]
    pub fn encode_request(&self, request: JsValue) -> Result<JsValue, JsValue> {
        self.encode(MessageType::Request as u8, request)
    }

    /// Encode a response message
    #[wasm_bindgen(js_name = encodeResponse)]
    pub fn encode_response(&self, response: JsValue) -> Result<JsValue, JsValue> {
        self.encode(MessageType::Response as u8, response)
    }

    /// Encode a ping message
    #[wasm_bindgen(js_name = encodePing)]
    pub fn encode_ping(&self) -> Result<JsValue, JsValue> {
        let obj = Object::new();
        Reflect::set(&obj, &"ts".into(), &JsValue::from(js_sys::Date::now()))?;
        self.encode(MessageType::Ping as u8, obj.into())
    }

    /// Encode a pong message
    #[wasm_bindgen(js_name = encodePong)]
    pub fn encode_pong(&self, ts: f64) -> Result<JsValue, JsValue> {
        let obj = Object::new();
        Reflect::set(&obj, &"ts".into(), &JsValue::from(ts))?;
        self.encode(MessageType::Pong as u8, obj.into())
    }
}

// ============================================================================
// ID Generation
// ============================================================================

/// Generate a unique request ID
#[wasm_bindgen(js_name = generateId)]
pub fn generate_id() -> String {
    let now = js_sys::Date::now() as u64;
    let random = (js_sys::Math::random() * 1_000_000.0) as u64;
    format!("{:x}-{:x}", now, random)
}

/// Generate a unique client ID
#[wasm_bindgen(js_name = generateClientId)]
pub fn generate_client_id() -> String {
    let now = js_sys::Date::now() as u64;
    let random = (js_sys::Math::random() * 1_000_000.0) as u64;
    format!("zap-{:x}-{:x}", now, random)
}

// ============================================================================
// ZAP Client
// ============================================================================

type PendingCallback = (Function, Function);

/// ZAP Client for connecting to MCP servers
///
/// This client uses a callback-based API for maximum compatibility with
/// JavaScript async patterns.
#[wasm_bindgen]
pub struct ZapClient {
    client_id: String,
    #[allow(dead_code)]
    client_type: u8,
    capabilities: Vec<String>,
    #[allow(dead_code)]
    timeout: u32,
    #[allow(dead_code)]
    binary: bool,
    protocol: Rc<Protocol>,
    ws: Rc<RefCell<Option<WebSocket>>>,
    pending_requests: Rc<RefCell<HashMap<String, PendingCallback>>>,
    event_handlers: Rc<RefCell<HashMap<String, Vec<Function>>>>,
    url: Rc<RefCell<Option<String>>>,
}

#[wasm_bindgen]
impl ZapClient {
    /// Create a new ZAP client
    #[wasm_bindgen(constructor)]
    pub fn new(options: JsValue) -> Result<ZapClient, JsValue> {
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        let client_id = get_string_opt(&options, "clientId")
            .unwrap_or_else(generate_client_id);

        let client_type = get_number_opt(&options, "clientType")
            .unwrap_or(ClientType::McpClient as u8 as f64) as u8;

        let capabilities = get_string_array_opt(&options, "capabilities")
            .unwrap_or_else(|| vec!["tools".into(), "browser".into(), "mcp".into()]);

        let timeout = get_number_opt(&options, "timeout")
            .unwrap_or(30000.0) as u32;

        let binary = get_bool_opt(&options, "binary")
            .unwrap_or(true);

        Ok(ZapClient {
            client_id,
            client_type,
            capabilities,
            timeout,
            binary,
            protocol: Rc::new(Protocol::new(Some(binary))),
            ws: Rc::new(RefCell::new(None)),
            pending_requests: Rc::new(RefCell::new(HashMap::new())),
            event_handlers: Rc::new(RefCell::new(HashMap::new())),
            url: Rc::new(RefCell::new(None)),
        })
    }

    /// Get client ID
    #[wasm_bindgen(getter, js_name = clientId)]
    pub fn client_id(&self) -> String {
        self.client_id.clone()
    }

    /// Check if connected
    #[wasm_bindgen(getter, js_name = isConnected)]
    pub fn is_connected(&self) -> bool {
        self.ws.borrow().as_ref()
            .map(|ws| ws.ready_state() == WebSocket::OPEN)
            .unwrap_or(false)
    }

    /// Connect to a ZAP server
    /// Returns a Promise that resolves when connected
    #[wasm_bindgen]
    pub fn connect(&self, url: String) -> Promise {
        let ws_ref = self.ws.clone();
        let pending_ref = self.pending_requests.clone();
        let handlers_ref = self.event_handlers.clone();
        let url_ref = self.url.clone();
        let protocol = self.protocol.clone();
        let client_id = self.client_id.clone();
        let client_type = self.client_type;
        let capabilities = self.capabilities.clone();

        Promise::new(&mut |resolve, reject| {
            // Store URL for reconnection
            *url_ref.borrow_mut() = Some(url.clone());

            // Create WebSocket
            let ws = match WebSocket::new(&url) {
                Ok(ws) => ws,
                Err(e) => {
                    let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(&format!("Failed to create WebSocket: {:?}", e)));
                    return;
                }
            };

            ws.set_binary_type(BinaryType::Arraybuffer);

            // Store callbacks for handshake
            let resolve = Rc::new(RefCell::new(Some(resolve)));
            let reject = Rc::new(RefCell::new(Some(reject)));

            // Set up message handler
            let pending_clone = pending_ref.clone();
            let handlers_clone = handlers_ref.clone();
            let protocol_clone = protocol.clone();
            let resolve_clone = resolve.clone();
            let reject_clone = reject.clone();

            let onmessage = Closure::wrap(Box::new(move |event: MessageEvent| {
                let data = event.data();

                if let Ok(decoded) = protocol_clone.decode(data) {
                    let msg_type = Reflect::get(&decoded, &"type".into())
                        .ok()
                        .and_then(|v| v.as_f64())
                        .unwrap_or(0.0) as u8;

                    let payload = Reflect::get(&decoded, &"payload".into())
                        .unwrap_or(JsValue::NULL);

                    match msg_type {
                        2 => {
                            // HandshakeResponse
                            let accepted = Reflect::get(&payload, &"accepted".into())
                                .ok()
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);

                            if accepted {
                                if let Some(resolve) = resolve_clone.borrow_mut().take() {
                                    let _ = resolve.call0(&JsValue::NULL);
                                }
                            } else {
                                let error = Reflect::get(&payload, &"error".into())
                                    .ok()
                                    .and_then(|v| v.as_string())
                                    .unwrap_or_else(|| "Connection rejected".into());
                                if let Some(reject) = reject_clone.borrow_mut().take() {
                                    let _ = reject.call1(&JsValue::NULL, &JsValue::from_str(&error));
                                }
                            }
                        }
                        4 => {
                            // Response
                            let id = Reflect::get(&payload, &"id".into())
                                .ok()
                                .and_then(|v| v.as_string())
                                .unwrap_or_default();

                            if let Some((cb_resolve, cb_reject)) = pending_clone.borrow_mut().remove(&id) {
                                let error = Reflect::get(&payload, &"error".into()).ok();

                                if error.as_ref().map(|e| !e.is_null() && !e.is_undefined()).unwrap_or(false) {
                                    let _ = cb_reject.call1(&JsValue::NULL, &error.unwrap());
                                } else {
                                    let result = Reflect::get(&payload, &"result".into())
                                        .unwrap_or(JsValue::NULL);
                                    let _ = cb_resolve.call1(&JsValue::NULL, &result);
                                }
                            }
                        }
                        5 => {
                            // Stream
                            if let Some(handlers) = handlers_clone.borrow().get("stream") {
                                for handler in handlers {
                                    let _ = handler.call1(&JsValue::NULL, &payload);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }) as Box<dyn FnMut(MessageEvent)>);

            ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
            onmessage.forget();

            // Set up open handler
            let ws_clone = ws.clone();
            let protocol_for_open = protocol.clone();
            let capabilities_clone = capabilities.clone();
            let client_id_clone = client_id.clone();

            let onopen = Closure::wrap(Box::new(move |_event: JsValue| {
                // Send handshake
                let handshake = Object::new();
                let _ = Reflect::set(&handshake, &"version".into(), &"1.0.0".into());
                let _ = Reflect::set(&handshake, &"clientType".into(), &JsValue::from(client_type));
                let _ = Reflect::set(&handshake, &"clientId".into(), &JsValue::from_str(&client_id_clone));

                let caps = Array::new();
                for cap in &capabilities_clone {
                    caps.push(&JsValue::from_str(cap));
                }
                let _ = Reflect::set(&handshake, &"capabilities".into(), &caps);

                if let Ok(encoded) = protocol_for_open.encode(MessageType::Handshake as u8, handshake.into()) {
                    // Try to cast to ArrayBuffer, if fails try string
                    if encoded.is_instance_of::<js_sys::ArrayBuffer>() {
                        if let Ok(buffer) = encoded.dyn_into::<js_sys::ArrayBuffer>() {
                            let _ = ws_clone.send_with_array_buffer(&buffer);
                        }
                    } else if let Some(s) = encoded.as_string() {
                        let _ = ws_clone.send_with_str(&s);
                    }
                }
            }) as Box<dyn FnMut(JsValue)>);

            ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
            onopen.forget();

            // Set up error handler
            let handlers_err = handlers_ref.clone();
            let reject_err = reject.clone();

            let onerror = Closure::wrap(Box::new(move |event: ErrorEvent| {
                console::error_1(&JsValue::from_str(&format!("WebSocket error: {:?}", event.message())));
                if let Some(handlers) = handlers_err.borrow().get("error") {
                    for handler in handlers {
                        let _ = handler.call1(&JsValue::NULL, &event);
                    }
                }
                // Reject connection promise if still pending
                if let Some(reject) = reject_err.borrow_mut().take() {
                    let _ = reject.call1(&JsValue::NULL, &JsValue::from_str("WebSocket error"));
                }
            }) as Box<dyn FnMut(ErrorEvent)>);

            ws.set_onerror(Some(onerror.as_ref().unchecked_ref()));
            onerror.forget();

            // Set up close handler
            let handlers_close = handlers_ref.clone();

            let onclose = Closure::wrap(Box::new(move |event: CloseEvent| {
                console::log_1(&JsValue::from_str(&format!("WebSocket closed: code={}, reason={}", event.code(), event.reason())));
                if let Some(handlers) = handlers_close.borrow().get("disconnect") {
                    for handler in handlers {
                        let _ = handler.call1(&JsValue::NULL, &event);
                    }
                }
            }) as Box<dyn FnMut(CloseEvent)>);

            ws.set_onclose(Some(onclose.as_ref().unchecked_ref()));
            onclose.forget();

            // Store WebSocket
            *ws_ref.borrow_mut() = Some(ws);
        })
    }

    /// Close the connection
    #[wasm_bindgen]
    pub fn close(&self) {
        if let Some(ws) = self.ws.borrow_mut().take() {
            let _ = ws.close();
        }

        // Reject all pending requests
        let mut pending = self.pending_requests.borrow_mut();
        for (_, (_, reject)) in pending.drain() {
            let _ = reject.call1(&JsValue::NULL, &JsValue::from_str("Connection closed"));
        }
    }

    /// Send a request and return a Promise for the response
    #[wasm_bindgen]
    pub fn request(&self, method: String, params: JsValue) -> Promise {
        let ws_ref = self.ws.clone();
        let pending_ref = self.pending_requests.clone();
        let protocol = self.protocol.clone();

        Promise::new(&mut |resolve, reject| {
            let ws = ws_ref.borrow();
            let ws = match ws.as_ref() {
                Some(ws) if ws.ready_state() == WebSocket::OPEN => ws,
                _ => {
                    let _ = reject.call1(&JsValue::NULL, &JsValue::from_str("Not connected"));
                    return;
                }
            };

            let id = generate_id();

            // Build request
            let request = Object::new();
            let _ = Reflect::set(&request, &"id".into(), &JsValue::from_str(&id));
            let _ = Reflect::set(&request, &"method".into(), &JsValue::from_str(&method));
            if !params.is_null() && !params.is_undefined() {
                let _ = Reflect::set(&request, &"params".into(), &params);
            }

            // Store callbacks
            pending_ref.borrow_mut().insert(id, (resolve, reject.clone()));

            // Send request
            match protocol.encode(MessageType::Request as u8, request.into()) {
                Ok(encoded) => {
                    let send_result = if encoded.is_instance_of::<js_sys::ArrayBuffer>() {
                        if let Ok(buffer) = encoded.dyn_into::<js_sys::ArrayBuffer>() {
                            ws.send_with_array_buffer(&buffer)
                        } else {
                            Err(JsValue::from_str("Cast failed"))
                        }
                    } else if let Some(s) = encoded.as_string() {
                        ws.send_with_str(&s)
                    } else {
                        Err(JsValue::from_str("Unknown message format"))
                    };

                    if send_result.is_err() {
                        let _ = reject.call1(&JsValue::NULL, &JsValue::from_str("Failed to send"));
                    }
                }
                Err(e) => {
                    let _ = reject.call1(&JsValue::NULL, &e);
                }
            }
        })
    }

    /// List available tools
    #[wasm_bindgen(js_name = listTools)]
    pub fn list_tools(&self) -> Promise {
        let request_promise = self.request("tools/list".into(), JsValue::NULL);

        Promise::new(&mut |resolve, reject| {
            let resolve = Rc::new(resolve);
            let reject = Rc::new(reject);

            let resolve_clone = resolve.clone();
            let reject_clone = reject.clone();

            let then = Closure::wrap(Box::new(move |result: JsValue| {
                let tools = Reflect::get(&result, &"tools".into())
                    .unwrap_or(Array::new().into());
                let _ = resolve_clone.call1(&JsValue::NULL, &tools);
            }) as Box<dyn FnMut(JsValue)>);

            let catch = Closure::wrap(Box::new(move |error: JsValue| {
                let _ = reject_clone.call1(&JsValue::NULL, &error);
            }) as Box<dyn FnMut(JsValue)>);

            let _ = request_promise.then2(&then, &catch);

            then.forget();
            catch.forget();
        })
    }

    /// Call a tool
    #[wasm_bindgen(js_name = callTool)]
    pub fn call_tool(&self, name: String, args: JsValue) -> Promise {
        let params = Object::new();
        let _ = Reflect::set(&params, &"name".into(), &JsValue::from_str(&name));
        let _ = Reflect::set(&params, &"arguments".into(), &args);

        self.request("tools/call".into(), params.into())
    }

    /// List available resources
    #[wasm_bindgen(js_name = listResources)]
    pub fn list_resources(&self) -> Promise {
        let request_promise = self.request("resources/list".into(), JsValue::NULL);

        Promise::new(&mut |resolve, reject| {
            let resolve = Rc::new(resolve);
            let reject = Rc::new(reject);

            let resolve_clone = resolve.clone();
            let reject_clone = reject.clone();

            let then = Closure::wrap(Box::new(move |result: JsValue| {
                let resources = Reflect::get(&result, &"resources".into())
                    .unwrap_or(Array::new().into());
                let _ = resolve_clone.call1(&JsValue::NULL, &resources);
            }) as Box<dyn FnMut(JsValue)>);

            let catch = Closure::wrap(Box::new(move |error: JsValue| {
                let _ = reject_clone.call1(&JsValue::NULL, &error);
            }) as Box<dyn FnMut(JsValue)>);

            let _ = request_promise.then2(&then, &catch);

            then.forget();
            catch.forget();
        })
    }

    /// Read a resource by URI
    #[wasm_bindgen(js_name = readResource)]
    pub fn read_resource(&self, uri: String) -> Promise {
        let params = Object::new();
        let _ = Reflect::set(&params, &"uri".into(), &JsValue::from_str(&uri));
        self.request("resources/read".into(), params.into())
    }

    /// Execute a browser action
    #[wasm_bindgen]
    pub fn browser(&self, params: JsValue) -> Promise {
        self.request("browser".into(), params)
    }

    /// Navigate to URL
    #[wasm_bindgen]
    pub fn navigate(&self, url: String, tab_id: Option<u32>) -> Promise {
        let params = Object::new();
        let _ = Reflect::set(&params, &"action".into(), &JsValue::from(BrowserAction::Navigate as u8));
        let _ = Reflect::set(&params, &"url".into(), &JsValue::from_str(&url));
        if let Some(id) = tab_id {
            let _ = Reflect::set(&params, &"tabId".into(), &JsValue::from(id));
        }
        self.browser(params.into())
    }

    /// Click an element by selector
    #[wasm_bindgen]
    pub fn click(&self, selector: String, tab_id: Option<u32>) -> Promise {
        let params = Object::new();
        let _ = Reflect::set(&params, &"action".into(), &JsValue::from(BrowserAction::Click as u8));
        let _ = Reflect::set(&params, &"selector".into(), &JsValue::from_str(&selector));
        if let Some(id) = tab_id {
            let _ = Reflect::set(&params, &"tabId".into(), &JsValue::from(id));
        }
        self.browser(params.into())
    }

    /// Fill an input element
    #[wasm_bindgen]
    pub fn fill(&self, selector: String, value: String, tab_id: Option<u32>) -> Promise {
        let params = Object::new();
        let _ = Reflect::set(&params, &"action".into(), &JsValue::from(BrowserAction::Fill as u8));
        let _ = Reflect::set(&params, &"selector".into(), &JsValue::from_str(&selector));
        let _ = Reflect::set(&params, &"value".into(), &JsValue::from_str(&value));
        if let Some(id) = tab_id {
            let _ = Reflect::set(&params, &"tabId".into(), &JsValue::from(id));
        }
        self.browser(params.into())
    }

    /// Evaluate JavaScript code
    #[wasm_bindgen]
    pub fn evaluate(&self, code: String, tab_id: Option<u32>) -> Promise {
        let params = Object::new();
        let _ = Reflect::set(&params, &"action".into(), &JsValue::from(BrowserAction::Evaluate as u8));
        let _ = Reflect::set(&params, &"code".into(), &JsValue::from_str(&code));
        if let Some(id) = tab_id {
            let _ = Reflect::set(&params, &"tabId".into(), &JsValue::from(id));
        }
        self.browser(params.into())
    }

    /// Take a screenshot
    #[wasm_bindgen]
    pub fn screenshot(&self, full_page: Option<bool>, tab_id: Option<u32>) -> Promise {
        let params = Object::new();
        let _ = Reflect::set(&params, &"action".into(), &JsValue::from(BrowserAction::Screenshot as u8));
        if let Some(fp) = full_page {
            let _ = Reflect::set(&params, &"fullPage".into(), &JsValue::from(fp));
        }
        if let Some(id) = tab_id {
            let _ = Reflect::set(&params, &"tabId".into(), &JsValue::from(id));
        }
        self.browser(params.into())
    }

    /// Get browser tabs
    #[wasm_bindgen(js_name = getTabs)]
    pub fn get_tabs(&self) -> Promise {
        let params = Object::new();
        let _ = Reflect::set(&params, &"action".into(), &JsValue::from(BrowserAction::GetTabs as u8));
        self.browser(params.into())
    }

    /// Subscribe to an event
    #[wasm_bindgen]
    pub fn on(&self, event: String, handler: Function) {
        let mut handlers = self.event_handlers.borrow_mut();
        handlers
            .entry(event)
            .or_default()
            .push(handler);
    }

    /// Unsubscribe from an event
    #[wasm_bindgen]
    pub fn off(&self, event: String, handler: Function) {
        let mut handlers = self.event_handlers.borrow_mut();
        if let Some(list) = handlers.get_mut(&event) {
            list.retain(|h| h != &handler);
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_string_opt(obj: &JsValue, key: &str) -> Option<String> {
    Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_string())
}

fn get_number_opt(obj: &JsValue, key: &str) -> Option<f64> {
    Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_f64())
}

fn get_bool_opt(obj: &JsValue, key: &str) -> Option<bool> {
    Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.as_bool())
}

fn get_string_array_opt(obj: &JsValue, key: &str) -> Option<Vec<String>> {
    Reflect::get(obj, &JsValue::from_str(key))
        .ok()
        .and_then(|v| v.dyn_into::<Array>().ok())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_string())
                .collect()
        })
}

// ============================================================================
// Initialization
// ============================================================================

/// Initialize the WASM module
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();

    console::log_1(&JsValue::from_str("ZAP WASM initialized"));
}

# zap-wasm

WebAssembly bindings for the [ZAP Protocol](https://github.com/zap-proto/zap) - enabling direct browser extension to MCP server communication.

## Overview

`zap-wasm` provides high-performance Rust-based WebAssembly bindings for the ZAP (Zero-Copy App Proto) protocol. This allows browser extensions and web applications to communicate directly with MCP (Model Context Protocol) servers without intermediate proxies.

## Features

- **Binary Protocol**: Efficient ZAP binary wire format
- **WebSocket Transport**: Direct WebSocket connections to MCP servers
- **Browser Compatible**: Works in browser extensions and web applications
- **Type Safe**: Full TypeScript definitions included
- **Lightweight**: Small WASM bundle (~50KB gzipped)

## Installation

```bash
npm install @zap-proto/wasm
```

## Usage

### Browser/Bundler

```typescript
import init, { ZapClient, Protocol, generateId } from '@zap-proto/wasm';

// Initialize WASM
await init();

// Create client
const client = new ZapClient({
  clientId: 'my-extension',
  capabilities: ['tools', 'browser'],
});

// Connect to MCP server
await client.connect('ws://localhost:9999');

// List available tools
const tools = await client.listTools();
console.log('Tools:', tools);

// Call a tool
const result = await client.callTool('search', { query: 'hello world' });
console.log('Result:', result);

// Clean up
client.close();
```

### Protocol Encoding/Decoding

```typescript
import init, { Protocol, MessageType } from '@zap-proto/wasm';

await init();

const protocol = new Protocol(true); // binary mode

// Encode a request
const encoded = protocol.encode(MessageType.Request, {
  id: generateId(),
  method: 'tools/list',
});

// Decode a message
const decoded = protocol.decode(arrayBuffer);
console.log('Type:', decoded.type);
console.log('Payload:', decoded.payload);
```

## API

### ZapClient

```typescript
class ZapClient {
  constructor(options?: {
    clientId?: string;
    clientType?: number;
    capabilities?: string[];
    timeout?: number;
    binary?: boolean;
  });

  // Properties
  get clientId(): string;
  get isConnected(): boolean;

  // Methods
  connect(url: string): Promise<void>;
  close(): void;
  request(method: string, params?: any): Promise<any>;
  listTools(): Promise<Tool[]>;
  callTool(name: string, args?: Record<string, any>): Promise<ToolResult>;

  // Events
  on(event: string, handler: Function): void;
  off(event: string, handler: Function): void;
}
```

### Protocol

```typescript
class Protocol {
  constructor(binary?: boolean);

  encode(type: number, data: any): ArrayBuffer | string;
  decode(data: ArrayBuffer | string): { type: number; payload: any };
  encodeHandshake(handshake: Handshake): ArrayBuffer | string;
  encodeRequest(request: Request): ArrayBuffer | string;
  encodeResponse(response: Response): ArrayBuffer | string;
  encodePing(): ArrayBuffer | string;
  encodePong(ts: number): ArrayBuffer | string;
}
```

## Building

```bash
# Install wasm-pack
cargo install wasm-pack

# Build for web
make build

# Build for Node.js
make build-nodejs

# Build all targets
make build-all

# Run tests
make test
```

## Wire Protocol

ZAP uses a binary wire format:

```
+----------------+------+------------------+
| Magic (4 bytes)| Type | JSON Payload     |
| "ZAP\x01"      | u8   | (variable)       |
+----------------+------+------------------+
```

Message Types:
- `1` - Handshake
- `2` - HandshakeResponse
- `3` - Request
- `4` - Response
- `5` - Stream
- `6` - Ping
- `7` - Pong

## Related Packages

- [@zap-proto/zap](https://github.com/zap-proto/zap-js) - TypeScript implementation
- [zap-rust](https://github.com/zap-proto/zap-rust) - Pure Rust implementation

## License

MIT

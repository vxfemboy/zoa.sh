---
title: Building a Real-time Shoutbox with WebSockets
date: 2024-12-20
slug: websocket-shoutbox
tags: rust, websockets, tutorial
---

The shoutbox on this site uses WebSockets for real-time messaging. Here's how it works.

## Server Side (Actix)

```rust
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ShoutboxSession {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Text(text)) => {
                // Broadcast to all connected clients
                self.server.do_send(BroadcastMessage {
                    content: text.to_string(),
                    sender: self.id,
                });
            }
            Ok(ws::Message::Ping(msg)) => ctx.pong(&msg),
            _ => (),
        }
    }
}
```

## Client Side (WASM)

```javascript
const ws = new WebSocket('wss://zoa.sh/ws/shoutbox');
ws.onmessage = (event) => {
    const msg = JSON.parse(event.data);
    appendMessage(msg.username, msg.content);
};
```

## The ASCII Challenge

Messages need to fit in the box! I truncate long messages and handle Unicode width properly so the borders don't break.

> Pro tip: Always sanitize user input. XSS in a shoutbox is embarrassing.

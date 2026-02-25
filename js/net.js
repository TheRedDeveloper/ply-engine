// Ply networking JS bridge
// Provides HTTP and WebSocket support for WASM builds.

// --- HTTP ---

var ply_net_uid = 0;
var ply_net_http_requests = {};

function ply_net_http_make_request(scheme, url, body, headers) {
    var cid = ply_net_uid;
    ply_net_uid += 1;

    var method;
    if (scheme === 0) { method = 'GET'; }
    else if (scheme === 1) { method = 'POST'; }
    else if (scheme === 2) { method = 'PUT'; }
    else if (scheme === 3) { method = 'DELETE'; }

    var url_string = consume_js_object(url);
    var body_string = consume_js_object(body);
    var headers_obj = consume_js_object(headers);

    var xhr = new XMLHttpRequest();
    xhr.open(method, url_string, true);
    xhr.responseType = 'arraybuffer';

    for (var header in headers_obj) {
        if (headers_obj.hasOwnProperty(header)) {
            xhr.setRequestHeader(header, headers_obj[header]);
        }
    }

    xhr.onload = function () {
        // Forward status + body regardless of status code
        ply_net_http_requests[cid] = {
            status: this.status,
            body: new Uint8Array(this.response)
        };
    };

    xhr.onerror = function () {
        ply_net_http_requests[cid] = {
            status: 0,
            error: "Network error"
        };
    };

    xhr.ontimeout = function () {
        ply_net_http_requests[cid] = {
            status: 0,
            error: "Request timed out"
        };
    };

    xhr.send(body_string.length > 0 ? body_string : null);
    return cid;
}

function ply_net_http_try_recv(cid) {
    if (ply_net_http_requests[cid] !== undefined && ply_net_http_requests[cid] !== null) {
        var data = ply_net_http_requests[cid];
        ply_net_http_requests[cid] = null;
        return js_object(data);
    }
    return -1;
}

// --- WebSocket ---
// Supports multiple simultaneous sockets keyed by integer ID.

var PLY_WS_CONNECTED = 0;
var PLY_WS_BINARY = 1;
var PLY_WS_TEXT = 2;
var PLY_WS_ERROR = 3;
var PLY_WS_CLOSED = 4;

var ply_net_sockets = {};

function ply_net_ws_connect(socket_id, addr) {
    var addr_string = consume_js_object(addr);
    var recv_buffer = [];

    var ws = new WebSocket(addr_string);
    ws.binaryType = 'arraybuffer';

    ws.onopen = function () {
        recv_buffer.push({ "type": PLY_WS_CONNECTED });
    };

    ws.onmessage = function (msg) {
        if (typeof msg.data === "string") {
            recv_buffer.push({ "type": PLY_WS_TEXT, "data": msg.data });
        } else {
            recv_buffer.push({ "type": PLY_WS_BINARY, "data": new Uint8Array(msg.data) });
        }
    };

    ws.onerror = function (error) {
        recv_buffer.push({
            "type": PLY_WS_ERROR,
            "data": JSON.stringify(error.message || "WebSocket error")
        });
    };

    ws.onclose = function () {
        recv_buffer.push({ "type": PLY_WS_CLOSED });
    };

    ply_net_sockets[socket_id] = {
        ws: ws,
        recv_buffer: recv_buffer
    };
}

function ply_net_ws_send_binary(socket_id, data) {
    var entry = ply_net_sockets[socket_id];
    if (!entry) return;
    try {
        var array = consume_js_object(data);
        if (array.buffer !== undefined) {
            entry.ws.send(array.buffer);
        } else {
            entry.ws.send(array);
        }
    } catch (error) {
        entry.recv_buffer.push({
            "type": PLY_WS_ERROR,
            "data": JSON.stringify(error.message || "Send error")
        });
    }
}

function ply_net_ws_send_text(socket_id, text) {
    var entry = ply_net_sockets[socket_id];
    if (!entry) return;
    try {
        var text_string = consume_js_object(text);
        entry.ws.send(text_string);
    } catch (error) {
        entry.recv_buffer.push({
            "type": PLY_WS_ERROR,
            "data": JSON.stringify(error.message || "Send error")
        });
    }
}

function ply_net_ws_close(socket_id) {
    var entry = ply_net_sockets[socket_id];
    if (!entry) return;
    entry.ws.close();
    delete ply_net_sockets[socket_id];
}

function ply_net_ws_try_recv(socket_id) {
    var entry = ply_net_sockets[socket_id];
    if (!entry) return -1;
    if (entry.recv_buffer.length !== 0) {
        return js_object(entry.recv_buffer.shift());
    }
    return -1;
}

// --- Plugin registration ---

miniquad_add_plugin({
    register_plugin: function (importObject) {
        importObject.env.ply_net_http_make_request = ply_net_http_make_request;
        importObject.env.ply_net_http_try_recv = ply_net_http_try_recv;
        importObject.env.ply_net_ws_connect = ply_net_ws_connect;
        importObject.env.ply_net_ws_send_binary = ply_net_ws_send_binary;
        importObject.env.ply_net_ws_send_text = ply_net_ws_send_text;
        importObject.env.ply_net_ws_close = ply_net_ws_close;
        importObject.env.ply_net_ws_try_recv = ply_net_ws_try_recv;
    },
    on_init: function () {},
    version: 1,
    name: "ply_net"
});

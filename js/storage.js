// Ply storage JS bridge
// Provides OPFS-backed persistent storage for WASM builds.

var ply_storage_op_uid = 1;
var ply_storage_root_uid = 1;
var ply_storage_pending = {};
var ply_storage_roots = {};

function ply_storage_next_op_id() {
    var id = ply_storage_op_uid;
    ply_storage_op_uid += 1;
    return id;
}

function ply_storage_finish_ok(op_id, payload) {
    var result = payload || {};
    result.status = 1;
    ply_storage_pending[op_id] = result;
}

function ply_storage_finish_err(op_id, error) {
    var message = "Storage operation failed";
    if (error !== undefined && error !== null) {
        if (error.message !== undefined && error.message !== null) {
            message = String(error.message);
        } else {
            message = String(error);
        }
    }
    ply_storage_pending[op_id] = {
        status: 0,
        error: message
    };
}

function ply_storage_split_path(path) {
    var parts = path.split('/').filter(function (part) {
        return part.length > 0;
    });
    if (parts.length === 0) {
        throw new Error("Invalid storage path");
    }
    return parts;
}

async function ply_storage_resolve_root(root_path) {
    if (!navigator.storage || !navigator.storage.getDirectory) {
        throw new Error("OPFS is not available in this browser");
    }

    var handle = await navigator.storage.getDirectory();
    var parts = root_path.split('/').filter(function (part) {
        return part.length > 0;
    });

    for (var i = 0; i < parts.length; i += 1) {
        handle = await handle.getDirectoryHandle(parts[i], { create: true });
    }

    return handle;
}

async function ply_storage_resolve_parent_dir(root_handle, relative_path, create_dirs) {
    var parts = ply_storage_split_path(relative_path);
    var dir = root_handle;

    for (var i = 0; i < parts.length - 1; i += 1) {
        dir = await dir.getDirectoryHandle(parts[i], { create: create_dirs });
    }

    return {
        dir: dir,
        file_name: parts[parts.length - 1]
    };
}

function ply_storage_get_root(storage_id) {
    var root = ply_storage_roots[storage_id];
    if (!root) {
        throw new Error("Invalid storage handle");
    }
    return root;
}

function ply_storage_new(root_path_obj) {
    var op_id = ply_storage_next_op_id();
    var root_path = consume_js_object(root_path_obj);

    (async function () {
        try {
            var root_handle = await ply_storage_resolve_root(root_path);
            var storage_id = ply_storage_root_uid;
            ply_storage_root_uid += 1;
            ply_storage_roots[storage_id] = root_handle;
            ply_storage_finish_ok(op_id, { storage_id: storage_id });
        } catch (error) {
            ply_storage_finish_err(op_id, error);
        }
    }());

    return op_id;
}

function ply_storage_save_bytes(storage_id, relative_path_obj, data_obj) {
    var op_id = ply_storage_next_op_id();
    var relative_path = consume_js_object(relative_path_obj);
    var data = consume_js_object(data_obj);

    (async function () {
        try {
            var root_handle = ply_storage_get_root(storage_id);
            var parent = await ply_storage_resolve_parent_dir(root_handle, relative_path, true);
            var file_handle = await parent.dir.getFileHandle(parent.file_name, { create: true });
            var writable = await file_handle.createWritable();
            await writable.write(data);
            await writable.close();
            ply_storage_finish_ok(op_id);
        } catch (error) {
            ply_storage_finish_err(op_id, error);
        }
    }());

    return op_id;
}

function ply_storage_load_bytes(storage_id, relative_path_obj) {
    var op_id = ply_storage_next_op_id();
    var relative_path = consume_js_object(relative_path_obj);

    (async function () {
        try {
            var root_handle = ply_storage_get_root(storage_id);
            var parent = await ply_storage_resolve_parent_dir(root_handle, relative_path, false);
            var file_handle = await parent.dir.getFileHandle(parent.file_name, { create: false });
            var file = await file_handle.getFile();
            var array_buffer = await file.arrayBuffer();
            ply_storage_finish_ok(op_id, {
                exists: 1,
                data: new Uint8Array(array_buffer)
            });
        } catch (error) {
            if (error && error.name === "NotFoundError") {
                ply_storage_finish_ok(op_id, { exists: 0 });
                return;
            }
            ply_storage_finish_err(op_id, error);
        }
    }());

    return op_id;
}

function ply_storage_remove(storage_id, relative_path_obj) {
    var op_id = ply_storage_next_op_id();
    var relative_path = consume_js_object(relative_path_obj);

    (async function () {
        try {
            var root_handle = ply_storage_get_root(storage_id);
            var parent = await ply_storage_resolve_parent_dir(root_handle, relative_path, false);
            await parent.dir.removeEntry(parent.file_name);
            ply_storage_finish_ok(op_id);
        } catch (error) {
            ply_storage_finish_err(op_id, error);
        }
    }());

    return op_id;
}

function ply_storage_export(storage_id, relative_path_obj) {
    var op_id = ply_storage_next_op_id();
    var relative_path = consume_js_object(relative_path_obj);

    (async function () {
        try {
            var root_handle = ply_storage_get_root(storage_id);
            var parent = await ply_storage_resolve_parent_dir(root_handle, relative_path, false);
            var file_handle = await parent.dir.getFileHandle(parent.file_name, { create: false });
            var file = await file_handle.getFile();
            var array_buffer = await file.arrayBuffer();
            var blob = new Blob([array_buffer]);

            var download_url = URL.createObjectURL(blob);
            var anchor = document.createElement('a');
            anchor.href = download_url;
            anchor.download = parent.file_name;
            anchor.style.display = 'none';
            document.body.appendChild(anchor);
            anchor.click();
            anchor.remove();
            setTimeout(function () {
                URL.revokeObjectURL(download_url);
            }, 0);

            ply_storage_finish_ok(op_id);
        } catch (error) {
            ply_storage_finish_err(op_id, error);
        }
    }());

    return op_id;
}

function ply_storage_try_recv(op_id) {
    if (Object.prototype.hasOwnProperty.call(ply_storage_pending, op_id)) {
        var result = ply_storage_pending[op_id];
        delete ply_storage_pending[op_id];
        return js_object(result);
    }
    return -1;
}

miniquad_add_plugin({
    register_plugin: function (importObject) {
        importObject.env.ply_storage_new = ply_storage_new;
        importObject.env.ply_storage_save_bytes = ply_storage_save_bytes;
        importObject.env.ply_storage_load_bytes = ply_storage_load_bytes;
        importObject.env.ply_storage_remove = ply_storage_remove;
        importObject.env.ply_storage_export = ply_storage_export;
        importObject.env.ply_storage_try_recv = ply_storage_try_recv;
    },
    on_init: function () {},
    version: 1,
    name: "ply_storage"
});

// Ply Accessibility Plugin — hidden DOM for screen readers
// Uses aria-activedescendant pattern: the canvas keeps keyboard focus,
// and the hidden DOM tree is used for screen reader announcements.

var a11y_root = null;
var nodes = {};

miniquad_add_plugin({
    register_plugin: function (imp) {
        // Initialise: set up the canvas for aria-activedescendant and create
        // the hidden DOM container.
        imp.env.ply_a11y_init = function () {
            if (!a11y_root) {
                // Make the canvas a proper application landmark
                canvas.setAttribute("tabindex", "0");
                canvas.setAttribute("role", "application");
                canvas.setAttribute("aria-label", "Application");

                // Create hidden container for screen reader elements
                a11y_root = document.createElement("div");
                a11y_root.id = "ply-a11y-root";
                a11y_root.style.cssText =
                    "position:absolute;left:-9999px;width:1px;height:1px;overflow:hidden;";
                document.body.appendChild(a11y_root);

                // Tell screen readers the canvas "owns" the hidden tree
                canvas.setAttribute("aria-owns", "ply-a11y-root");
            }
        };

        // Create or update a hidden DOM node for a given element id.
        // Hidden elements are NOT focusable — the canvas stays focused,
        // and aria-activedescendant points to the current element.
        imp.env.ply_a11y_upsert_node = function (
            id,
            role_ptr,
            role_len,
            label_ptr,
            label_len,
            _tab_index,
        ) {
            var role = UTF8ToString(role_ptr, role_len);
            var label = UTF8ToString(label_ptr, label_len);
            var el = nodes[id];

            if (!el) {
                el = document.createElement("div");
                el.id = "ply-a11y-" + id;
                a11y_root.appendChild(el);
                nodes[id] = el;
            }

            if (role && role !== "none") el.setAttribute("role", role);
            else el.removeAttribute("role");

            if (label) {
                el.setAttribute("aria-label", label);
                // Also set textContent so live regions fire announcements
                // and browse-mode screen readers can discover the text.
                // Only mutate when changed to avoid spurious live-region
                // re-announcements.
                if (el.textContent !== label) el.textContent = label;
            } else {
                el.removeAttribute("aria-label");
                if (el.textContent !== "") el.textContent = "";
            }
        };

        // Set heading level (aria-level)
        imp.env.ply_a11y_set_heading_level = function (id, level) {
            var el = nodes[id];
            if (el && level >= 1 && level <= 6)
                el.setAttribute("aria-level", level);
        };

        // Set checked state
        imp.env.ply_a11y_set_checked = function (id, checked) {
            var el = nodes[id];
            if (el)
                el.setAttribute("aria-checked", checked ? "true" : "false");
        };

        // Set value + optional min/max (for sliders, progress bars)
        imp.env.ply_a11y_set_value = function (
            id,
            value_ptr,
            value_len,
            min,
            max,
        ) {
            var el = nodes[id];
            if (!el) return;
            var value = UTF8ToString(value_ptr, value_len);
            if (value) el.setAttribute("aria-valuenow", value);
            // NaN !== NaN, so this skips NaN values
            if (min === min) el.setAttribute("aria-valuemin", min);
            if (max === max) el.setAttribute("aria-valuemax", max);
        };

        // Set live-region mode (0 = off, 1 = polite, 2 = assertive)
        imp.env.ply_a11y_set_live = function (id, mode) {
            var el = nodes[id];
            if (!el) return;
            if (mode === 1) el.setAttribute("aria-live", "polite");
            else if (mode === 2) el.setAttribute("aria-live", "assertive");
            else el.removeAttribute("aria-live");
        };

        // Remove a node from the hidden tree
        imp.env.ply_a11y_remove_node = function (id) {
            var el = nodes[id];
            if (el) {
                el.remove();
                delete nodes[id];
            }
        };

        // Update aria-activedescendant on the canvas to point to the
        // given element. This tells screen readers which element is
        // active without moving browser focus away from the canvas.
        imp.env.ply_a11y_set_focus = function (id) {
            if (id === 0) {
                canvas.removeAttribute("aria-activedescendant");
            } else {
                canvas.setAttribute("aria-activedescendant", "ply-a11y-" + id);
            }
        };

        // Remove all nodes (for full rebuild)
        imp.env.ply_a11y_clear = function () {
            if (a11y_root) a11y_root.innerHTML = "";
            nodes = {};
            canvas.removeAttribute("aria-activedescendant");
        };

        // Set text content (for announcements via live regions)
        imp.env.ply_a11y_announce = function (id, text_ptr, text_len) {
            var el = nodes[id];
            if (el) el.textContent = UTF8ToString(text_ptr, text_len);
        };

        // Set aria-description
        imp.env.ply_a11y_set_description = function (
            id,
            desc_ptr,
            desc_len,
        ) {
            var el = nodes[id];
            if (el) {
                var desc = UTF8ToString(desc_ptr, desc_len);
                if (desc) el.setAttribute("aria-description", desc);
                else el.removeAttribute("aria-description");
            }
        };

        // Reorder DOM children to match layout order.
        // ids_ptr points to a u32 array of element IDs in desired order.
        // We use Int32Array because WASM passes u32 as i32 to JS, and
        // nodes[] keys are the signed interpretation of those values.
        imp.env.ply_a11y_reorder = function (ids_ptr, count) {
            if (!a11y_root || count === 0) return;
            var ids = new Int32Array(wasm_memory.buffer, ids_ptr, count);
            for (var i = 0; i < count; i++) {
                var el = nodes[ids[i]];
                if (el) a11y_root.appendChild(el);
            }
        };

        // Virtual keyboard support for mobile web.
        // Creates a hidden textarea that triggers the on-screen keyboard
        // when focused, and forwards input events to the canvas.
        var vk_input = null;
        imp.env.ply_show_virtual_keyboard = function (show) {
            if (show) {
                if (!vk_input) {
                    vk_input = document.createElement("textarea");
                    vk_input.id = "ply-virtual-keyboard";
                    vk_input.autocapitalize = "off";
                    vk_input.autocomplete = "off";
                    vk_input.spellcheck = false;
                    vk_input.style.cssText =
                        "position:fixed;left:0;top:0;width:1px;height:1px;" +
                        "opacity:0;z-index:-1;pointer-events:none;";
                    document.body.appendChild(vk_input);

                    // Forward character input back to the canvas
                    vk_input.addEventListener("input", function () {
                        var text = vk_input.value;
                        vk_input.value = "";
                        for (var i = 0; i < text.length; i++) {
                            var code = text.charCodeAt(i);
                            // Skip control characters
                            if (code >= 32) {
                                canvas.dispatchEvent(
                                    new KeyboardEvent("keypress", {
                                        charCode: code,
                                        keyCode: code,
                                        bubbles: true,
                                    })
                                );
                            }
                        }
                    });

                    // Forward control keys (Backspace, Enter, arrows, etc.)
                    // Only forward the initial press (not browser-generated
                    // repeats) — Rust has its own repeat logic.
                    // Also forward all Ctrl/Meta combos (undo, redo, copy,
                    // paste, cut, select-all) so they reach the canvas.
                    vk_input.addEventListener("keydown", function (e) {
                        var forward = [
                            "Backspace", "Delete", "Enter", "Tab", "Escape",
                            "ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown",
                            "Home", "End",
                        ];
                        var shouldForward =
                            forward.indexOf(e.key) !== -1 ||
                            e.ctrlKey || e.metaKey;
                        if (shouldForward) {
                            if (!e.repeat) {
                                // For Ctrl/Meta combos, map e.key to the correct
                                // code so non-QWERTY layouts work correctly.
                                var fwdCode = e.code;
                                if ((e.ctrlKey || e.metaKey) && e.key && e.key.length === 1) {
                                    var upper = e.key.toUpperCase();
                                    if (upper >= 'A' && upper <= 'Z') {
                                        fwdCode = 'Key' + upper;
                                    }
                                }
                                canvas.dispatchEvent(
                                    new KeyboardEvent("keydown", {
                                        key: e.key,
                                        code: fwdCode,
                                        keyCode: e.keyCode,
                                        ctrlKey: e.ctrlKey,
                                        shiftKey: e.shiftKey,
                                        altKey: e.altKey,
                                        metaKey: e.metaKey,
                                        bubbles: true,
                                    })
                                );
                            }
                            e.preventDefault();
                        }
                    });

                    // Forward keyup so the Rust key-repeat tracker knows
                    // the key was released and stops repeating.
                    vk_input.addEventListener("keyup", function (e) {
                        var fwdCode = e.code;
                        if ((e.ctrlKey || e.metaKey) && e.key && e.key.length === 1) {
                            var upper = e.key.toUpperCase();
                            if (upper >= 'A' && upper <= 'Z') {
                                fwdCode = 'Key' + upper;
                            }
                        }
                        canvas.dispatchEvent(
                            new KeyboardEvent("keyup", {
                                key: e.key,
                                code: fwdCode,
                                keyCode: e.keyCode,
                                ctrlKey: e.ctrlKey,
                                shiftKey: e.shiftKey,
                                altKey: e.altKey,
                                metaKey: e.metaKey,
                                bubbles: true,
                            })
                        );
                    });
                }
                vk_input.focus({ preventScroll: true });
            } else {
                if (vk_input) {
                    vk_input.blur();
                }
                canvas.focus();
            }
        };
    },
    on_init: function () {},
    version: 1,
    name: "ply_accessibility",
});

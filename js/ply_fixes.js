miniquad_add_plugin({
    register_plugin: function (imp) {
        // Fix Ctrl/Meta + letter shortcuts on non-QWERTY layouts
        // (e.g. QWERTZ where Z and Y are swapped). miniquad uses
        // event.code (physical key position, always QWERTY) to map
        // key codes. We patch the code property in the capture phase
        // (before miniquad's bubble-phase handler) so shortcuts like
        // Ctrl+Z resolve to the layout-aware letter, not the physical
        // QWERTY position.
        function fixLayoutCode(e) {
            if ((e.ctrlKey || e.metaKey) && e.key && e.key.length === 1) {
                var upper = e.key.toUpperCase();
                if (upper >= 'A' && upper <= 'Z') {
                    Object.defineProperty(e, 'code', {
                        value: 'Key' + upper
                    });
                }
            }
        }
        canvas.addEventListener("keydown", fixLayoutCode, true);
        canvas.addEventListener("keyup", fixLayoutCode, true);
    },
    on_init: function () {},
    version: 1,
    name: "ply_fixes",
});

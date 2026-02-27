#!/bin/bash
set -e

mkdir -p .build

curl https://raw.githubusercontent.com/TheRedDeveloper/miniquad-fix/refs/heads/main/js/gl.js -o .build/gl.js
curl https://raw.githubusercontent.com/not-fl3/quad-snd/refs/heads/master/js/audio.js -o .build/audio.js
curl https://raw.githubusercontent.com/not-fl3/sapp-jsutils/refs/heads/master/js/sapp_jsutils.js -o .build/sapp_jsutils.js

function wrap_js {
    echo "(function () {" >> .build/bundle.js
    cat $1 >> .build/bundle.js
    echo "}());" >> .build/bundle.js
}
cat .build/gl.js > .build/bundle.js
wrap_js .build/audio.js
echo "" >> .build/bundle.js
cat .build/sapp_jsutils.js >> .build/bundle.js
wrap_js net.js
wrap_js ply_fixes.js
wrap_js ply_accessibility.js

npx minify@9.2.0 .build/bundle.js > ply_bundle.js

rm -rf .build

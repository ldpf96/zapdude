(function () {
    var found = [];
    function walk(node, depth) {
        if (depth > 16 || found.length > 10) {
            return;
        }
        for (var index = 0; index < node.childNodes.length; index++) {
            var child = node.childNodes[index];
            if (child.nodeType === 3) {
                var text = String(child.nodeValue || '').trim();
                if (/\+\d{1,3}[\d ()-]{5,}/.test(text)) {
                    found.push(text.slice(0, 32));
                }
            } else if (child.nodeType === 1) {
                walk(child, depth + 1);
            }
        }
    }
    walk(document.body, 0);
    return {
        found: found,
        dialogCount: document.querySelectorAll('div[role="dialog"]').length,
        profileWrapperVisible: !!document.querySelector('button[aria-label="Close"]'),
    };
})()

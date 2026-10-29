(function () {
    var out = [];
    document.querySelectorAll('[data-testid]').forEach(function (node) {
        var id = String(node.getAttribute('data-testid') || '');
        if (/header|conversation/i.test(id) && out.length < 20) {
            var rect = node.getBoundingClientRect();
            if (rect.width > 0) {
                out.push({
                    testid: id,
                    rect: [Math.round(rect.left), Math.round(rect.top), Math.round(rect.width), Math.round(rect.height)],
                });
            }
        }
    });
    var buttons = [];
    document.querySelectorAll('#main button[aria-label], [data-testid="conversation-header"] button[aria-label]').forEach(function (n) {
        buttons.push({
            label: String(n.getAttribute('aria-label') || '').slice(0, 40),
            tag: n.tagName,
            icon: (n.querySelector('span[data-icon]') ? n.querySelector('span[data-icon]').getAttribute('data-icon') : ''),
        });
    });
    return {
        headerTestids: out,
        chatButtons: buttons.slice(0, 14),
        composerVisible: !!document.querySelector('footer div[contenteditable="true"]'),
    };
})()

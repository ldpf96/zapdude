(function () {
    var found = [];
    document.querySelectorAll('[data-testid], [role="option"], [role="listitem"], [role="row"]').forEach(function (node) {
        if (found.length > 12) {
            return;
        }
        var testid = String(node.getAttribute('data-testid') || '');
        var aria = String(node.getAttribute('aria-label') || '');
        var rect = node.getBoundingClientRect();
        if (rect.width === 0 || rect.height === 0) {
            return;
        }
        if (/phone|number|numero/i.test(testid + ' ' + aria) || /51987931332|987\s*931|Enviar mensaje/i.test(String(node.textContent || ''))) {
            found.push({
                testid: testid,
                aria: aria.slice(0, 50),
                rect: [Math.round(rect.left), Math.round(rect.top), Math.round(rect.width), Math.round(rect.height)],
                text: String(node.textContent || '').slice(0, 50),
            });
        }
    });
    return { drawerResults: found.slice(0, 10) };
})()

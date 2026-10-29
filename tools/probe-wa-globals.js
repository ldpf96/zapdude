(function () {
    var waKeys = [];
    Object.keys(window).forEach(function (key) {
        if (/^WA(i|i de)/i.test(key)) { return; }
        if (/^WA/i.test(key)) { waKeys.push(key); }
    });
    var selfChat = null;
    document.querySelectorAll('#pane-side [data-testid^="list-item"]').forEach(function (row) {
        var title = '';
        var label = String(row.getAttribute('aria-label') || '');
        var titleNode = row.querySelector('span[title]');
        if (titleNode) { title = String(titleNode.getAttribute('title') || ''); }
        if (/t[uú]/i.test(title) || /message yourself/i.test(label) || /t[uú] mismo/i.test(label)) {
            selfChat = { title: title, aria: (label || '').slice(0, 60) };
        }
    });
    return { waGlobals: waKeys.slice(0, 30), selfChat: selfChat };
})()

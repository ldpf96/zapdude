(function () {
    var buttons = [];
    document.querySelectorAll('[aria-label]').forEach(function (node) {
        var label = String(node.getAttribute('aria-label') || '').trim();
        if (!label) { return; }
        var rect = node.getBoundingClientRect();
        if (rect.width <= 0 || rect.height <= 0) { return; }
        if (rect.left > 660 && rect.top > 130) { return; }
        buttons.push({
            tag: node.tagName,
            label: label.slice(0, 36),
            rect: [Math.round(rect.left), Math.round(rect.top), Math.round(rect.width), Math.round(rect.height)],
        });
    });
    var headerPhone = [];
    document.querySelectorAll('header span, header div').forEach(function (node) {
        var text = String(node.textContent || '').trim();
        if (/\+\d[\d ()-]{6,}/.test(text)) {
            headerPhone.push(text.slice(0, 24));
        }
    });
    return {
        buttonCount: buttons.length,
        buttons: buttons.slice(0, 22),
        headerPhone: headerPhone.slice(0, 4),
    };
})()

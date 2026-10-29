(function () {
    // Calls do not use window.open anymore: the surface is in-page. Catch it
    // live: every element whose test id talks calls, plus draggable-style
    // transforms.
    var voipNodes = [];
    document.querySelectorAll('[data-testid]').forEach(function (node) {
        var testid = String(node.getAttribute('data-testid') || '');
        if (/voip|call|ring|dial|caller/i.test(testid)) {
            var rect = node.getBoundingClientRect();
            if (rect.width > 10) {
                voipNodes.push({
                    testid: testid,
                    rect: [Math.round(rect.left), Math.round(rect.top), Math.round(rect.width), Math.round(rect.height)],
                    aria: String(node.getAttribute('aria-label') || '').slice(0, 34),
                    transform: String(node.style.transform || ''),
                });
            }
        }
    });
    var transformed = [];
    document.querySelectorAll('div[style*="transform"], div[data-animate]').forEach(function (node) {
        var rect = node.getBoundingClientRect();
        if (rect.left >= 400 && rect.width > 120 && rect.height > 120 && transformed.length < 8) {
            transformed.push({
                testid: String(node.getAttribute('data-testid') || ''),
                cls: String(node.className || '').slice(0, 40),
                rect: [Math.round(rect.left), Math.round(rect.top), Math.round(rect.width), Math.round(rect.height)],
                transform: String(node.style.transform || ''),
                norm: (node.getAttribute('data-animate-media-viewer') !== null),
            });
        }
    });
    return { voipNodes: voipNodes.slice(0, 20), transformedSurfaces: transformed };
})()

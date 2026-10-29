(function () {
    // fire-call-step2: after the chat is open, click Voice and immediately
    // place a record of the discovered call surface into a DOM self-marker
    // AND window state (persisted string so a later eval can read it).
    var header = document.querySelector('div[data-testid="conversation-header"]');
    if (!header) {
        return { ok: false, error: 'no_header_now' };
    }
    var voiceButton = null;
    header.querySelectorAll('button').forEach(function (button) {
        var label = String(button.getAttribute('aria-label') || '');
        if (!voiceButton && /voice\s*call|llamada\s*de\s*voz/i.test(label)) {
            voiceButton = button;
        }
    });
    if (!voiceButton) {
        return { ok: false, error: 'no_voice_button_now' };
    }
    voiceButton.click();
    try {
        window.__zapdudeCallProbe = { firedAt: Date.now() };
    } catch (_markerError) {}
    // Two quick samples - done in SEPARATE evals by the host; here only the
    // first beat is recorded so this script never awaits.
    var now = [];
    document.querySelectorAll('[data-testid]').forEach(function (node) {
        var testid = String(node.getAttribute('data-testid') || '');
        if (/voip|ring|call/i.test(testid) && !/log|bubble/i.test(testid)) {
            var rect = node.getBoundingClientRect();
            if (rect.width > 20) {
                now.push({ testid: testid, rect: [Math.round(rect.left), Math.round(rect.top), Math.round(rect.width), Math.round(rect.height)] });
            }
        }
    });
    return { ok: true, fired: true, surfaceNodes: now.slice(0, 14) };
})()

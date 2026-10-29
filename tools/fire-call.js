(function () {
    // Fire the call and sample the UI while ringing. Step 1: click.
    var header = document.querySelector('div[data-testid="conversation-header"]');
    if (!header) {
        return { ok: false, error: 'no_header' };
    }
    var clicked = null;
    header.querySelectorAll('button').forEach(function (button) {
        var label = String(button.getAttribute('aria-label') || '');
        if (!clicked && /voice\s*call|llamada\s*de\s*voz/i.test(label)) {
            clicked = button;
        }
    });
    if (!clicked) {
        return { ok: false, error: 'no_voice_button' };
    }
    clicked.click();
    // Sample right away: the ringing surface may mount within a beat.
    var sample = function () {
        var buttons = [];
        document.querySelectorAll('button').forEach(function (button) {
            var rect = button.getBoundingClientRect();
            if (rect.width <= 0) {
                return;
            }
            var label = String(button.getAttribute('aria-label') || '').slice(0, 34);
            var testid = String(button.getAttribute('data-testid') || '');
            buttons.push(testid + '|' + label + '|' + Math.round(rect.left) + ',' + Math.round(rect.top));
        });
        return { buttons: buttons.slice(0, 26) };
    };
    var result = sample();
    setTimeout(function () {
        try {
            var late = sample();
            result.lateButtons = late.buttons.slice(0, 20);
        } catch (_sampleError) {}
    }, 1200);
    result.ok = true;
    return result;
})()

(function () {
    var drawer;
    try {
        drawer = document.querySelector('div[data-testid="new-chat-drawer"]');
    } catch (_drawerError) {}
    if (!drawer) {
        return { ok: false, error: 'no_drawer' };
    }
    var input = drawer.querySelector('input');
    if (!input) {
        return { ok: false, error: 'no_input' };
    }
    input.focus({ preventScroll: true });
    try {
        var setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
        setter.call(input, '+51987931332');
        input.dispatchEvent(new Event('input', { bubbles: true }));
        input.dispatchEvent(new Event('change', { bubbles: true }));
    } catch (error) {
        return { ok: false, error: 'typed_failed', detail: String((error && error.message) || error) };
    }
    return { ok: true, typed: '+51987931332' };
})()
